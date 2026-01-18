use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context, bail};
use nix::unistd::{fork, ForkResult, Pid};
use nix::sys::signal::{kill as send_signal, Signal};
use crate::oci::load_config;
use crate::state::{ContainerState, ContainerStatus};

/// Create a container (OCI create command)
/// Sets up the container environment but does not start the process
pub fn create(
    bundle: PathBuf,
    container_id: String,
    root: PathBuf,
    pid_file: Option<PathBuf>,
) -> Result<()> {
    // Check if container already exists
    if ContainerState::exists(&root, &container_id) {
        bail!("Container {} already exists", container_id);
    }

    // Load and validate OCI config
    let config_path = bundle.join("config.json");
    let spec = load_config(&config_path)
        .context("Failed to load OCI config from bundle")?;

    // Create initial state
    let mut state = ContainerState::new(
        container_id.clone(),
        bundle.canonicalize().context("Failed to canonicalize bundle path")?,
        spec.version().to_string(),
    );

    // Save creating state
    state.save(&root)?;

    // TODO: In a proper OCI implementation, we would:
    // 1. Create namespaces
    // 2. Set up cgroups
    // 3. Prepare rootfs
    // 4. Fork the init process but keep it paused
    // 5. Transition to 'created' state
    //
    // For now, we'll do a simplified version

    let process = spec.process().as_ref().context("No process defined in config")?;
    let args = process.args().as_ref().context("No args defined in process")?;

    log::info!(
        "Creating container: id={}, command={:?}, bundle={:?}",
        container_id,
        args,
        bundle
    );

    // Fork the container process
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // Child process - this will become the container init process
            // TODO: Set up namespaces, pivot_root, etc.

            // For now, just pause waiting for start signal
            // In a real implementation, we'd use a pipe or socket to wait
            std::thread::sleep(std::time::Duration::from_millis(100));

            // This is a placeholder - the child should wait for start command
            std::process::exit(0);
        }
        Ok(ForkResult::Parent { child }) => {
            // Parent process
            let pid = child.as_raw();

            // Update state to created
            state.status = ContainerStatus::Created;
            state.pid = Some(pid);
            state.save(&root)?;

            // Write PID file if requested
            if let Some(pid_path) = pid_file {
                fs::write(&pid_path, format!("{}", pid))
                    .with_context(|| format!("Failed to write PID file: {}", pid_path.display()))?;
                log::debug!("PID file written: {:?}", pid_path);
            }

            log::info!("Container created: id={}, pid={}, status=created", container_id, pid);
            Ok(())
        }
        Err(e) => bail!("Fork failed: {}", e),
    }
}

/// Start a container (OCI start command)
/// Signals the paused container process to begin execution
pub fn start(container_id: &str, root: &PathBuf) -> Result<()> {
    let mut state = ContainerState::load(root, container_id)?;

    // Validate state transition
    match state.status {
        ContainerStatus::Created => {
            // Valid transition
        }
        ContainerStatus::Running => {
            bail!("Container {} is already running", container_id);
        }
        ContainerStatus::Stopped => {
            bail!("Container {} has stopped and cannot be started", container_id);
        }
        ContainerStatus::Creating => {
            bail!("Container {} is still being created", container_id);
        }
    }

    // TODO: Signal the paused process to continue execution
    // For now, we just update the state
    state.status = ContainerStatus::Running;
    state.save(root)?;

    log::info!("Container started: id={}, pid={:?}, status=running", container_id, state.pid);
    Ok(())
}

/// Query container state (OCI state command)
pub fn state(container_id: &str, root: &PathBuf) -> Result<()> {
    let state = ContainerState::load(root, container_id)?;

    log::debug!("Querying state: id={}, status={:?}", container_id, state.status);

    // Output state as JSON to stdout (per OCI spec)
    let json = serde_json::to_string_pretty(&state)
        .context("Failed to serialize state")?;

    println!("{}", json);
    Ok(())
}

/// Send a signal to container (OCI kill command)
pub fn kill(container_id: &str, signal_str: &str, root: &PathBuf, _all: bool) -> Result<()> {
    let state = ContainerState::load(root, container_id)?;

    let pid = state.pid.context("Container has no PID")?;

    // Parse signal
    let signal = parse_signal(signal_str)?;

    // Send signal to process
    let nix_pid = Pid::from_raw(pid);
    send_signal(nix_pid, signal)
        .with_context(|| format!("Failed to send signal {} to PID {}", signal_str, pid))?;

    log::info!("Signal sent: id={}, signal={}, pid={}", container_id, signal_str, pid);

    // If we sent SIGKILL or the process might have terminated, check if we should update state
    if signal == Signal::SIGKILL || signal == Signal::SIGTERM {
        // TODO: We should have a reaper that updates state when process exits
        // For now, we don't update state here
    }

    Ok(())
}

/// Delete a container (OCI delete command)
pub fn delete(container_id: &str, root: &PathBuf, force: bool) -> Result<()> {
    let state = ContainerState::load(root, container_id)?;

    log::debug!(
        "Deleting container: id={}, status={:?}, force={}",
        container_id,
        state.status,
        force
    );

    // Validate state - can only delete stopped containers
    match state.status {
        ContainerStatus::Stopped => {
            log::debug!("Container is stopped, safe to delete");
        }
        ContainerStatus::Created => {
            log::debug!("Container is in created state, allowing deletion");
        }
        ContainerStatus::Running => {
            if !force {
                bail!("Cannot delete running container {}. Stop it first.", container_id);
            }
            log::warn!("Force deleting running container: id={}", container_id);
            // Force delete - kill the process first
            if let Some(pid) = state.pid {
                let _ = send_signal(Pid::from_raw(pid), Signal::SIGKILL);
                log::debug!("Sent SIGKILL to pid={}", pid);
            }
        }
        ContainerStatus::Creating => {
            if !force {
                bail!("Cannot delete container {} that is being created", container_id);
            }
            log::warn!("Force deleting container in creating state: id={}", container_id);
        }
    }

    // TODO: Clean up resources:
    // - Remove cgroups
    // - Unmount filesystems
    // - Clean up network interfaces
    // - etc.

    // Delete state
    ContainerState::delete(root, container_id)?;

    log::info!("Container deleted: id={}", container_id);
    Ok(())
}

/// Parse signal name or number to Signal enum
pub fn parse_signal(signal_str: &str) -> Result<Signal> {
    // Try parsing as number first
    if let Ok(num) = signal_str.parse::<i32>() {
        return Signal::try_from(num)
            .with_context(|| format!("Invalid signal number: {}", num));
    }

    // Parse as name (with or without SIG prefix)
    let name = if signal_str.starts_with("SIG") {
        signal_str
    } else {
        // Try with SIG prefix
        &format!("SIG{}", signal_str)
    };

    match name {
        "SIGTERM" | "TERM" => Ok(Signal::SIGTERM),
        "SIGKILL" | "KILL" => Ok(Signal::SIGKILL),
        "SIGINT" | "INT" => Ok(Signal::SIGINT),
        "SIGHUP" | "HUP" => Ok(Signal::SIGHUP),
        "SIGQUIT" | "QUIT" => Ok(Signal::SIGQUIT),
        "SIGUSR1" | "USR1" => Ok(Signal::SIGUSR1),
        "SIGUSR2" | "USR2" => Ok(Signal::SIGUSR2),
        "SIGSTOP" | "STOP" => Ok(Signal::SIGSTOP),
        "SIGCONT" | "CONT" => Ok(Signal::SIGCONT),
        _ => bail!("Unknown signal: {}", signal_str),
    }
}

