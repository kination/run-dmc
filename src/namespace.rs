//! Linux namespace management
//!
//! This module handles creation and setup of Linux namespaces for container isolation.
//! Supports: PID, Mount, UTS, IPC, Network, and User namespaces.

use anyhow::{Context, Result};
use nix::unistd::{Gid, Uid};
use std::fs;

/// Namespace configuration from OCI spec
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    pub pid: bool,
    pub mount: bool,
    pub uts: bool,
    pub ipc: bool,
    pub network: bool,
    pub user: bool,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            pid: true,
            mount: true,
            uts: true,
            ipc: true,
            network: true,
            user: false, // User namespace requires UID/GID mapping
        }
    }
}

impl NamespaceConfig {
    /// Parse namespace configuration from OCI spec
    pub fn from_oci_spec(spec: &oci_spec::runtime::Spec) -> Self {
        let mut config = Self::default();

        if let Some(linux) = spec.linux() {
            if let Some(namespaces) = linux.namespaces() {
                // Reset all to false, then enable based on spec
                config.pid = false;
                config.mount = false;
                config.uts = false;
                config.ipc = false;
                config.network = false;
                config.user = false;

                for ns in namespaces {
                    match ns.typ() {
                        oci_spec::runtime::LinuxNamespaceType::Pid => config.pid = true,
                        oci_spec::runtime::LinuxNamespaceType::Mount => config.mount = true,
                        oci_spec::runtime::LinuxNamespaceType::Uts => config.uts = true,
                        oci_spec::runtime::LinuxNamespaceType::Ipc => config.ipc = true,
                        oci_spec::runtime::LinuxNamespaceType::Network => config.network = true,
                        oci_spec::runtime::LinuxNamespaceType::User => config.user = true,
                        _ => {}
                    }
                }
            }
        }

        config
    }

    /// Convert to libc flags for unshare syscall (Linux only)
    #[cfg(target_os = "linux")]
    pub fn to_clone_flags(&self) -> i32 {
        let mut flags = 0;

        if self.pid {
            flags |= libc::CLONE_NEWPID;
        }
        if self.mount {
            flags |= libc::CLONE_NEWNS;
        }
        if self.uts {
            flags |= libc::CLONE_NEWUTS;
        }
        if self.ipc {
            flags |= libc::CLONE_NEWIPC;
        }
        if self.network {
            flags |= libc::CLONE_NEWNET;
        }
        if self.user {
            flags |= libc::CLONE_NEWUSER;
        }

        flags
    }

    #[cfg(not(target_os = "linux"))]
    pub fn to_clone_flags(&self) -> i32 {
        0 // Stub for non-Linux
    }
}

/// Create namespaces using unshare syscall (Linux only)
///
/// This should be called in the child process after fork
#[cfg(target_os = "linux")]
pub fn setup_namespaces(config: &NamespaceConfig) -> Result<()> {
    let flags = config.to_clone_flags();

    log::debug!("Setting up namespaces: {:?}", config);

    // User namespace must be created first if enabled
    if config.user {
        unsafe {
            if libc::unshare(libc::CLONE_NEWUSER) != 0 {
                return Err(anyhow::anyhow!("Failed to create user namespace: {}",
                    std::io::Error::last_os_error()));
            }
        }
        log::debug!("Created user namespace");
    }

    // Create other namespaces
    let other_flags = flags & !libc::CLONE_NEWUSER;
    if other_flags != 0 {
        unsafe {
            if libc::unshare(other_flags) != 0 {
                return Err(anyhow::anyhow!("Failed to create namespaces: {}",
                    std::io::Error::last_os_error()));
            }
        }
        log::debug!("Created namespaces with flags: {}", other_flags);
    }

    Ok(())
}

/// Stub for non-Linux platforms
#[cfg(not(target_os = "linux"))]
pub fn setup_namespaces(config: &NamespaceConfig) -> Result<()> {
    log::warn!("Namespace isolation not supported on this platform");
    log::debug!("Namespace config: {:?}", config);
    Ok(())
}

/// Setup UID/GID mappings for user namespace
///
/// This is required when using user namespaces for rootless containers
pub fn setup_user_namespace_mappings(
    container_uid: Uid,
    container_gid: Gid,
    host_uid: Uid,
    host_gid: Gid,
) -> Result<()> {
    let pid = nix::unistd::getpid();

    // Write UID mapping
    let uid_map = format!("/proc/{}/uid_map", pid);
    let uid_mapping = format!("{} {} 1", container_uid, host_uid);
    fs::write(&uid_map, uid_mapping)
        .with_context(|| format!("Failed to write UID mapping to {}", uid_map))?;
    log::debug!("UID mapping configured: {} -> {}", container_uid, host_uid);

    // Disable setgroups (required before GID mapping)
    let setgroups = format!("/proc/{}/setgroups", pid);
    fs::write(&setgroups, "deny")
        .with_context(|| format!("Failed to write to {}", setgroups))?;

    // Write GID mapping
    let gid_map = format!("/proc/{}/gid_map", pid);
    let gid_mapping = format!("{} {} 1", container_gid, host_gid);
    fs::write(&gid_map, gid_mapping)
        .with_context(|| format!("Failed to write GID mapping to {}", gid_map))?;
    log::debug!("GID mapping configured: {} -> {}", container_gid, host_gid);

    Ok(())
}

/// Set container hostname in UTS namespace
pub fn set_hostname(hostname: &str) -> Result<()> {
    use std::ffi::CString;
    let c_hostname = CString::new(hostname)
        .context("Invalid hostname string")?;

    unsafe {
        #[cfg(target_os = "linux")]
        {
            if libc::sethostname(c_hostname.as_ptr(), hostname.len()) != 0 {
                return Err(anyhow::anyhow!("Failed to set hostname: {}",
                    std::io::Error::last_os_error()));
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            if libc::sethostname(c_hostname.as_ptr(), hostname.len() as libc::c_int) != 0 {
                return Err(anyhow::anyhow!("Failed to set hostname: {}",
                    std::io::Error::last_os_error()));
            }
        }
    }

    log::debug!("Hostname set to: {}", hostname);
    Ok(())
}

// Tests moved to tests/namespace_tests.rs
