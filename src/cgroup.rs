/// Cgroups (Control Groups) to provide resource limiting, prioritization, and accounting
/// for container processes.

use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// Cgroups v2 mount point (unified hierarchy)
const CGROUP_V2_MOUNT: &str = "/sys/fs/cgroup";

/// Cgroup configuration for container
#[derive(Debug, Clone)]
pub struct CgroupConfig {
    pub container_id: String,
    /// CPU quota in microseconds per period (e.g., 100000 = 1 CPU)
    pub cpu_quota: Option<i64>,
    /// CPU period in microseconds (default: 100000 = 100ms)
    pub cpu_period: Option<u64>,
    /// CPU shares/weight (1-10000, default: 100)
    pub cpu_weight: Option<u64>,
    /// Memory limit in bytes
    pub memory_limit: Option<i64>,
    /// Memory + swap limit in bytes
    pub memory_swap: Option<i64>,
    /// Maximum number of PIDs
    pub pids_max: Option<i64>,
}

impl CgroupConfig {
    /// Create from OCI spec
    pub fn from_oci_spec(
        spec: &oci_spec::runtime::Spec,
        container_id: &str,
    ) -> Result<Self> {
        let linux = spec.linux().as_ref();

        let mut config = Self {
            container_id: container_id.to_string(),
            cpu_quota: None,
            cpu_period: None,
            cpu_weight: None,
            memory_limit: None,
            memory_swap: None,
            pids_max: None,
        };

        // Parse resource limits from OCI spec
        if let Some(linux) = linux {
            if let Some(resources) = linux.resources() {
                // CPU limits
                if let Some(cpu) = resources.cpu() {
                    config.cpu_quota = cpu.quota();
                    config.cpu_period = cpu.period();
                    config.cpu_weight = cpu.shares().map(|s| s as u64);
                }

                // Memory limits
                if let Some(memory) = resources.memory() {
                    config.memory_limit = memory.limit();
                    config.memory_swap = memory.swap();
                }

                // PIDs limit
                if let Some(pids) = resources.pids() {
                    config.pids_max = Some(pids.limit());
                }
            }
        }

        Ok(config)
    }
}

/// Cgroup manager for container resource control
pub struct CgroupManager {
    /// Path to the container's cgroup directory
    cgroup_path: PathBuf,
}

impl CgroupManager {
    /// Create a new cgroup manager for a container
    pub fn new(container_id: &str) -> Result<Self> {
        // Verify cgroups v2 is available
        if !is_cgroup_v2_available() {
            bail!("cgroups v2 is not available on this system");
        }

        let cgroup_path = get_cgroup_path(container_id);

        Ok(Self { cgroup_path })
    }

    /// Create cgroup hierarchy and apply resource limits
    #[cfg(target_os = "linux")]
    pub fn setup(&self, config: &CgroupConfig) -> Result<()> {
        log::info!("Setting up cgroup: {:?}", self.cgroup_path);

        // Create cgroup directory
        fs::create_dir_all(&self.cgroup_path)
            .with_context(|| format!("Failed to create cgroup directory: {:?}", self.cgroup_path))?;

        // Apply resource limits
        self.apply_cpu_limits(config)?;
        self.apply_memory_limits(config)?;
        self.apply_pids_limit(config)?;

        log::info!("Cgroup setup complete: {}", config.container_id);
        Ok(())
    }

    /// Stub for non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    pub fn setup(&self, config: &CgroupConfig) -> Result<()> {
        log::warn!("Cgroups not supported on this platform");
        log::debug!("Cgroup config: {:?}", config);
        Ok(())
    }

    /// Move a process to this cgroup
    #[cfg(target_os = "linux")]
    pub fn add_process(&self, pid: i32) -> Result<()> {
        let procs_file = self.cgroup_path.join("cgroup.procs");

        fs::write(&procs_file, format!("{}", pid))
            .with_context(|| format!("Failed to add PID {} to cgroup: {:?}", pid, procs_file))?;

        log::debug!("Added process {} to cgroup: {:?}", pid, self.cgroup_path);
        Ok(())
    }

    /// Stub for non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    pub fn add_process(&self, pid: i32) -> Result<()> {
        log::debug!("Skipping cgroup process addition (not on Linux): pid={}", pid);
        Ok(())
    }

    /// Clean up cgroup (called on container deletion)
    #[cfg(target_os = "linux")]
    pub fn cleanup(&self) -> Result<()> {
        if self.cgroup_path.exists() {
            // Kill all processes in cgroup first (if any remain)
            let procs_file = self.cgroup_path.join("cgroup.procs");
            if procs_file.exists() {
                if let Ok(pids_str) = fs::read_to_string(&procs_file) {
                    for pid_str in pids_str.lines() {
                        if let Ok(pid) = pid_str.parse::<i32>() {
                            // Send SIGKILL to any remaining processes
                            let _ = nix::sys::signal::kill(
                                nix::unistd::Pid::from_raw(pid),
                                nix::sys::signal::Signal::SIGKILL,
                            );
                        }
                    }
                }
            }

            // Remove cgroup directory
            fs::remove_dir(&self.cgroup_path)
                .with_context(|| format!("Failed to remove cgroup directory: {:?}", self.cgroup_path))?;

            log::info!("Cleaned up cgroup: {:?}", self.cgroup_path);
        }

        Ok(())
    }

    /// Stub for non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    pub fn cleanup(&self) -> Result<()> {
        log::debug!("Skipping cgroup cleanup (not on Linux)");
        Ok(())
    }

    /// Apply CPU resource limits
    #[cfg(target_os = "linux")]
    fn apply_cpu_limits(&self, config: &CgroupConfig) -> Result<()> {
        // Apply CPU quota and period
        if let (Some(quota), Some(period)) = (config.cpu_quota, config.cpu_period) {
            let max_file = self.cgroup_path.join("cpu.max");
            let content = format!("{} {}", quota, period);

            fs::write(&max_file, content)
                .with_context(|| format!("Failed to write cpu.max: {:?}", max_file))?;

            log::debug!("Applied CPU limit: quota={}, period={}", quota, period);
        }

        // Apply CPU weight (shares)
        if let Some(weight) = config.cpu_weight {
            let weight_file = self.cgroup_path.join("cpu.weight");

            fs::write(&weight_file, format!("{}", weight))
                .with_context(|| format!("Failed to write cpu.weight: {:?}", weight_file))?;

            log::debug!("Applied CPU weight: {}", weight);
        }

        Ok(())
    }

    /// Apply memory resource limits
    #[cfg(target_os = "linux")]
    fn apply_memory_limits(&self, config: &CgroupConfig) -> Result<()> {
        // Apply memory limit
        if let Some(limit) = config.memory_limit {
            let max_file = self.cgroup_path.join("memory.max");

            fs::write(&max_file, format!("{}", limit))
                .with_context(|| format!("Failed to write memory.max: {:?}", max_file))?;

            log::info!("Applied memory limit: {} bytes", limit);
        }

        // Apply memory + swap limit
        if let Some(swap) = config.memory_swap {
            let swap_file = self.cgroup_path.join("memory.swap.max");

            fs::write(&swap_file, format!("{}", swap))
                .with_context(|| format!("Failed to write memory.swap.max: {:?}", swap_file))?;

            log::debug!("Applied swap limit: {} bytes", swap);
        }

        Ok(())
    }

    /// Apply PIDs limit
    #[cfg(target_os = "linux")]
    fn apply_pids_limit(&self, config: &CgroupConfig) -> Result<()> {
        if let Some(max_pids) = config.pids_max {
            let max_file = self.cgroup_path.join("pids.max");

            fs::write(&max_file, format!("{}", max_pids))
                .with_context(|| format!("Failed to write pids.max: {:?}", max_file))?;

            log::debug!("Applied PIDs limit: {}", max_pids);
        }

        Ok(())
    }
}

pub fn is_cgroup_v2_available() -> bool {
    // Check if cgroup v2 mount point exists
    let v2_mount = Path::new(CGROUP_V2_MOUNT);
    if !v2_mount.exists() {
        return false;
    }

    let controllers_file = v2_mount.join("cgroup.controllers");
    controllers_file.exists()
}

fn get_cgroup_path(container_id: &str) -> PathBuf {
    // Create cgroup under rundmc/ namespace
    PathBuf::from(CGROUP_V2_MOUNT)
        .join("rundmc")
        .join(container_id)
}

/// Ensure the rundmc cgroup namespace exists
#[cfg(target_os = "linux")]
pub fn ensure_rundmc_namespace() -> Result<()> {
    let rundmc_path = PathBuf::from(CGROUP_V2_MOUNT).join("rundmc");

    if !rundmc_path.exists() {
        fs::create_dir_all(&rundmc_path)
            .with_context(|| format!("Failed to create rundmc cgroup namespace: {:?}", rundmc_path))?;

        log::info!("Created rundmc cgroup namespace: {:?}", rundmc_path);
    }

    Ok(())
}

/// Stub for non-Linux platforms
#[cfg(not(target_os = "linux"))]
pub fn ensure_rundmc_namespace() -> Result<()> {
    log::debug!("Skipping cgroup namespace creation (not on Linux)");
    Ok(())
}
