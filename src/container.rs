use std::path::PathBuf;
use std::fs;
use std::os::unix::io::{AsRawFd, RawFd};
use anyhow::{Result, Context, bail};
use nix::unistd::{fork, ForkResult, Pid, pipe, close, read};
use nix::sys::signal::{kill as send_signal, Signal};
use crate::oci::load_config;
use crate::state::{ContainerState, ContainerStatus};
use crate::namespace::{NamespaceConfig, setup_namespaces, set_hostname};
use crate::rootfs::{RootfsConfig, setup_rootfs, setup_mounts};

/// Create a container (OCI create command)
/// Set up container environment, but not starting the process
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

    let process = spec.process().as_ref().context("No process defined in config")?;
    let args = process.args().as_ref().context("No args defined in process")?;

    log::info!(
        "Create container: id={}, command={:?}, bundle={:?}",
        container_id,
        args,
        bundle
    );

    // Parse namespace and rootfs configuration from OCI spec
    let ns_config = NamespaceConfig::from_oci_spec(&spec);
    let rootfs_config = RootfsConfig::from_oci_spec(&spec, &bundle)?;

    log::info!("Namespace config: {:?}", ns_config);
    log::info!("Rootfs config: {:?}", rootfs_config);

    // Create synchronization pipe to separate create/start
    // Child will block reading from this pipe until 'start' command
    let (sync_read, sync_write) = pipe()
        .context("Failed to create synchronization pipe")?;

    let sync_read_fd = sync_read.as_raw_fd();
    let sync_write_fd = sync_write.as_raw_fd();

    // Fork container process
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // Child process - this will become the container init process
            drop(sync_write); // Close write end in child

            // Run container initialization in child
            if let Err(e) = child_init(
                &spec,
                &ns_config,
                &rootfs_config,
                sync_read_fd,
            ) {
                log::error!("Container initialization failed: {}", e);
                std::process::exit(1);
            }

            // Should never reach here
            std::process::exit(0);
        }
        Ok(ForkResult::Parent { child }) => {
            // Parent process
            drop(sync_read); // Close read end in parent

            let pid = child.as_raw();

            // Update state to created
            state.status = ContainerStatus::Created;
            state.pid = Some(pid);

            // Store sync pipe FD for start command
            // In a real implementation, we'd store this in a runtime structure
            // For now, we'll store it in a file
            let sync_file = root.join(&container_id).join("sync_fd");
            fs::write(&sync_file, format!("{}", sync_write_fd))
                .context("Failed to save sync fd")?;

            state.save(&root)?;

            // Write PID file if requested
            if let Some(pid_path) = pid_file {
                fs::write(&pid_path, format!("{}", pid))
                    .with_context(|| format!("Failed to write PID file: {}", pid_path.display()))?;
                log::debug!("PID file written: {:?}", pid_path);
            }

            log::info!("Container created: id={}, pid={}, status=created", container_id, pid);

            // Keep sync_write FD open - we'll close it in start command
            // DON'T close it here or child will unblock immediately
            std::mem::forget(sync_write); // Prevent auto-close

            Ok(())
        }
        Err(e) => bail!("Fork failed: {}", e),
    }
}

/// Child process initialization
/// This runs in forked child process, and setting up container environment
fn child_init(
    spec: &oci_spec::runtime::Spec,
    ns_config: &NamespaceConfig,
    rootfs_config: &RootfsConfig,
    sync_fd: RawFd,
) -> Result<()> {
    log::debug!("Child process init");

    // Step 1: Setup namespaces
    setup_namespaces(ns_config)
        .context("Failed to setup namespaces")?;

    // Step 2: Setup hostname (if UTS namespace is enabled)
    if ns_config.uts {
        if let Some(hostname) = spec.hostname() {
            set_hostname(hostname)
                .context("Failed to set hostname")?;
        }
    }

    // Step 3: Setup rootfs with pivot_root
    setup_rootfs(rootfs_config)
        .context("Failed to setup rootfs")?;

    // Step 4: Setup additional mounts from OCI spec
    setup_mounts(spec)
        .context("Failed to setup mounts")?;

    // Step 5: Wait for start signal
    // Block here until parent closes the write end of the pipe
    log::debug!("Waiting for start signal...");
    let mut buf = [0u8; 1];
    match read(sync_fd, &mut buf) {
        Ok(0) => {
            // EOF - pipe closed, we can continue
            log::debug!("Start signal received (pipe closed)");
        }
        Ok(_) => {
            // Unexpected data
            log::warn!("Unexpected data on sync pipe");
        }
        Err(e) => {
            log::error!("Error reading sync pipe: {}", e);
            bail!("Failed to wait for start signal: {}", e);
        }
    }
    close(sync_fd).ok();

    // Step 6: Execute the container process
    let process = spec.process().as_ref().context("No process in spec")?;
    execute_container_process(process)
        .context("Failed to execute container process")?;

    // Should never reach here
    Ok(())
}

/// Execute the container's main process
fn execute_container_process(process: &oci_spec::runtime::Process) -> Result<()> {
    let args = process.args().as_ref().context("No args in process")?;

    if args.is_empty() {
        bail!("Process args is empty");
    }

    let program = &args[0];
    let argv: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    log::info!("Executing container process: {:?}", argv);

    // Set environment variables
    if let Some(env_vars) = process.env() {
        for env in env_vars {
            if let Some((key, value)) = env.split_once('=') {
                unsafe {
                    std::env::set_var(key, value);
                }
            }
        }
    }

    // Set working directory
    let cwd = process.cwd();
    std::env::set_current_dir(cwd)
        .with_context(|| format!("Failed to change directory to {:?}", cwd))?;

    // TODO: Set user/group IDs, capabilities, rlimits, etc.

    // Execute the process
    // This replaces the current process image
    nix::unistd::execvp(
        &std::ffi::CString::new(program.as_str())?,
        &argv.iter()
            .map(|&s| std::ffi::CString::new(s))
            .collect::<Result<Vec<_>, _>>()?,
    )
    .with_context(|| format!("Failed to exec {:?}", program))?;

    // Should never reach here
    Ok(())
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

    // Read the sync FD from file
    let sync_file = root.join(container_id).join("sync_fd");
    let sync_fd_str = fs::read_to_string(&sync_file)
        .context("Failed to read sync fd file")?;
    let sync_fd: RawFd = sync_fd_str.trim().parse()
        .context("Failed to parse sync fd")?;

    // Close the sync pipe to signal the child to continue
    // When we close the write end, the child's read() will return EOF
    log::debug!("Closing sync pipe (fd={}) to signal start", sync_fd);
    close(sync_fd)
        .context("Failed to close sync pipe")?;

    // Remove sync fd file
    fs::remove_file(&sync_file)
        .context("Failed to remove sync fd file")?;

    // Update state to running
    state.status = ContainerStatus::Running;
    state.save(root)?;

    log::info!("Container started: id={}, pid={:?}, status=running", container_id, state.pid);
    Ok(())
}

/// Query container state (OCI 'state' command)
pub fn state(container_id: &str, root: &PathBuf) -> Result<()> {
    let state = ContainerState::load(root, container_id)?;

    log::debug!("Querying state: id={}, status={:?}", container_id, state.status);

    // Output state as JSON to stdout
    let json = serde_json::to_string_pretty(&state)
        .context("Failed to serialize state")?;

    println!("{}", json);
    Ok(())
}

/// Send a signal to container (OCI 'kill' command)
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
    }

    Ok(())
}

/// Delete container (OCI 'delete' command)
pub fn delete(container_id: &str, root: &PathBuf, force: bool) -> Result<()> {
    let state = ContainerState::load(root, container_id)?;

    log::info!(
        "Delete container: id={}, status={:?}, force={}",
        container_id,
        state.status,
        force
    );

    // Validate state - only stopped containers can be deleted
    match state.status {
        ContainerStatus::Stopped => {
            log::info!("Container is stopped, so safe to delete");
        }
        ContainerStatus::Created => {
            log::info!("Container is in created state, allowing deletion");
        }
        ContainerStatus::Running => {
            if !force {
                bail!("Cannot delete running container {}. Stop it first.", container_id);
            }
            log::warn!("Force delete running container: id={}", container_id);
            // Force delete - kill the process first
            if let Some(pid) = state.pid {
                let _ = send_signal(Pid::from_raw(pid), Signal::SIGKILL);
                log::info!("Sent SIGKILL to pid={}", pid);
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

