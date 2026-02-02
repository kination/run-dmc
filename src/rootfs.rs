/// This module handles setting up the container's root filesystem,
/// including pivot_root, mounting /proc, /sys, /dev, and handling bind mounts.

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// Rootfs configuration
#[derive(Debug, Clone)]
pub struct RootfsConfig {
    /// Path to the rootfs directory (from OCI bundle)
    pub path: PathBuf,
    /// Whether rootfs should be readonly
    pub readonly: bool,
}

impl RootfsConfig {
    /// Create from OCI spec
    pub fn from_oci_spec(spec: &oci_spec::runtime::Spec, bundle_path: &Path) -> Result<Self> {
        let root = spec.root().as_ref().context("No root defined in OCI spec")?;

        let rootfs_path = if root.path().is_absolute() {
            root.path().to_path_buf()
        } else {
            bundle_path.join(root.path())
        };

        Ok(Self {
            path: rootfs_path,
            readonly: root.readonly().unwrap_or(false),
        })
    }
}

/// Setup container's root filesystem using pivot_root
///
/// This function is only available on Linux and requires CAP_SYS_ADMIN capability.
/// On non-Linux platforms or when running without privileges, this is a stub.
#[cfg(target_os = "linux")]
pub fn setup_rootfs(config: &RootfsConfig) -> Result<()> {
    let new_root = &config.path;

    log::info!("Setting up rootfs: {:?}", new_root);

    // Verify new root exists
    if !new_root.exists() {
        bail!("Rootfs path does not exist: {:?}", new_root);
    }

    // Step 1: Bind mount new root to itself (required for pivot_root)
    mount_bind(new_root, new_root)?;
    log::debug!("Bind mounted rootfs to itself");

    // Step 2: Create directory for old root
    let put_old = new_root.join(".pivot_root");
    fs::create_dir_all(&put_old)
        .with_context(|| format!("Failed to create pivot directory: {:?}", put_old))?;

    // Step 3: Change to new root directory
    std::env::set_current_dir(new_root)
        .with_context(|| format!("Failed to chdir to new root: {:?}", new_root))?;
    log::debug!("Changed directory to new root");

    // Step 4: Pivot root using syscall
    pivot_root_syscall(".", ".pivot_root")?;
    log::info!("pivot_root successful");

    // Step 5: Change to new root (now at /)
    std::env::set_current_dir("/")
        .context("Failed to chdir to / after pivot_root")?;

    // Step 6: Unmount old root
    unmount_detach("/.pivot_root")?;
    log::debug!("Unmounted old root");

    // Step 7: Remove pivot directory
    fs::remove_dir("/.pivot_root")
        .context("Failed to remove pivot directory")?;

    // Step 8: Setup essential mounts
    setup_proc()?;
    setup_sys()?;
    setup_dev()?;

    // Step 9: Apply readonly if configured
    if config.readonly {
        make_rootfs_readonly()?;
    }

    log::info!("Rootfs setup complete");
    Ok(())
}

/// Stub for non-Linux platforms
#[cfg(not(target_os = "linux"))]
pub fn setup_rootfs(config: &RootfsConfig) -> Result<()> {
    log::warn!("Rootfs isolation not supported on this platform");
    log::debug!("Rootfs config: {:?}", config);
    Ok(())
}

/// Pivot root syscall wrapper (Linux only)
#[cfg(target_os = "linux")]
fn pivot_root_syscall(new_root: &str, put_old: &str) -> Result<()> {
    use std::ffi::CString;

    let new_root_c = CString::new(new_root)?;
    let put_old_c = CString::new(put_old)?;

    unsafe {
        if libc::syscall(libc::SYS_pivot_root, new_root_c.as_ptr(), put_old_c.as_ptr()) != 0 {
            return Err(anyhow::anyhow!(
                "pivot_root failed: {}",
                std::io::Error::last_os_error()
            ));
        }
    }

    Ok(())
}

/// Mount bind helper (Linux only)
#[cfg(target_os = "linux")]
fn mount_bind(source: &Path, target: &Path) -> Result<()> {
    use std::ffi::CString;

    let source_c = CString::new(source.to_str().unwrap())?;
    let target_c = CString::new(target.to_str().unwrap())?;
    let fstype = CString::new("").unwrap();

    unsafe {
        if libc::mount(
            source_c.as_ptr(),
            target_c.as_ptr(),
            fstype.as_ptr(),
            libc::MS_BIND | libc::MS_REC,
            std::ptr::null(),
        ) != 0
        {
            return Err(anyhow::anyhow!(
                "Failed to bind mount {:?} to {:?}: {}",
                source,
                target,
                std::io::Error::last_os_error()
            ));
        }
    }

    Ok(())
}

/// Unmount with detach flag (Linux only)
#[cfg(target_os = "linux")]
fn unmount_detach(target: &str) -> Result<()> {
    use std::ffi::CString;

    let target_c = CString::new(target)?;

    unsafe {
        if libc::umount2(target_c.as_ptr(), libc::MNT_DETACH) != 0 {
            return Err(anyhow::anyhow!(
                "Failed to unmount {}: {}",
                target,
                std::io::Error::last_os_error()
            ));
        }
    }

    Ok(())
}

/// Mount /proc inside the container (Linux only)
#[cfg(target_os = "linux")]
fn setup_proc() -> Result<()> {
    use std::ffi::CString;

    let proc_path = Path::new("/proc");

    // Create /proc if it doesn't exist
    if !proc_path.exists() {
        fs::create_dir_all(proc_path).context("Failed to create /proc directory")?;
    }

    let source = CString::new("proc")?;
    let target = CString::new("/proc")?;
    let fstype = CString::new("proc")?;

    unsafe {
        if libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            fstype.as_ptr(),
            libc::MS_NOEXEC | libc::MS_NOSUID | libc::MS_NODEV,
            std::ptr::null(),
        ) != 0
        {
            return Err(anyhow::anyhow!(
                "Failed to mount /proc: {}",
                std::io::Error::last_os_error()
            ));
        }
    }

    log::debug!("Mounted /proc");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn setup_proc() -> Result<()> {
    log::debug!("Skipping /proc mount (not on Linux)");
    Ok(())
}

/// Mount /sys inside the container (Linux only, or use cross builds)
#[cfg(target_os = "linux")]
fn setup_sys() -> Result<()> {
    use std::ffi::CString;

    let sys_path = Path::new("/sys");

    // Create /sys if it doesn't exist
    if !sys_path.exists() {
        fs::create_dir_all(sys_path).context("Failed to create /sys directory")?;
    }

    let source = CString::new("sysfs")?;
    let target = CString::new("/sys")?;
    let fstype = CString::new("sysfs")?;

    unsafe {
        if libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            fstype.as_ptr(),
            libc::MS_NOEXEC | libc::MS_NOSUID | libc::MS_NODEV | libc::MS_RDONLY,
            std::ptr::null(),
        ) != 0
        {
            return Err(anyhow::anyhow!(
                "Failed to mount /sys: {}",
                std::io::Error::last_os_error()
            ));
        }
    }

    log::debug!("Mounted /sys");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn setup_sys() -> Result<()> {
    log::debug!("Skipping /sys mount (not on Linux)");
    Ok(())
}

/// Setup /dev with minimal required devices (Linux only)
#[cfg(target_os = "linux")]
fn setup_dev() -> Result<()> {
    use std::ffi::CString;

    let dev_path = Path::new("/dev");

    // Create /dev if it doesn't exist
    if !dev_path.exists() {
        fs::create_dir_all(dev_path).context("Failed to create /dev directory")?;
    }

    let source = CString::new("tmpfs")?;
    let target = CString::new("/dev")?;
    let fstype = CString::new("tmpfs")?;
    let data = CString::new("mode=755,size=65536k")?;

    unsafe {
        if libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            fstype.as_ptr(),
            libc::MS_NOSUID | libc::MS_STRICTATIME,
            data.as_ptr() as *const libc::c_void,
        ) != 0
        {
            return Err(anyhow::anyhow!(
                "Failed to mount tmpfs on /dev: {}",
                std::io::Error::last_os_error()
            ));
        }
    }

    // Create essential device directories
    fs::create_dir_all("/dev/pts").context("Failed to create /dev/pts")?;
    fs::create_dir_all("/dev/shm").context("Failed to create /dev/shm")?;

    log::debug!("Mounted /dev");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn setup_dev() -> Result<()> {
    log::debug!("Skipping /dev mount (not on Linux)");
    Ok(())
}

/// Make the rootfs readonly (Linux only)
#[cfg(target_os = "linux")]
fn make_rootfs_readonly() -> Result<()> {
    use std::ffi::CString;

    let target = CString::new("/")?;

    unsafe {
        if libc::mount(
            std::ptr::null(),
            target.as_ptr(),
            std::ptr::null(),
            libc::MS_REMOUNT | libc::MS_RDONLY | libc::MS_BIND,
            std::ptr::null(),
        ) != 0
        {
            return Err(anyhow::anyhow!(
                "Failed to remount rootfs as readonly: {}",
                std::io::Error::last_os_error()
            ));
        }
    }

    log::info!("Rootfs remounted as readonly");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn make_rootfs_readonly() -> Result<()> {
    log::debug!("Skipping readonly remount (not on Linux)");
    Ok(())
}

/// Handle bind mounts from OCI spec
pub fn setup_mounts(spec: &oci_spec::runtime::Spec) -> Result<()> {
    if let Some(mounts) = spec.mounts() {
        log::debug!("Processing {} mounts from OCI spec", mounts.len());
        for mount_spec in mounts {
            setup_single_mount(mount_spec)?;
        }
    }
    Ok(())
}

/// Setup a single mount from OCI mount spec (Linux only)
#[cfg(target_os = "linux")]
fn setup_single_mount(mount_spec: &oci_spec::runtime::Mount) -> Result<()> {
    use std::ffi::CString;

    let destination = mount_spec.destination();
    let source = mount_spec.source().as_ref().map(|s| s.as_path());
    let fs_type = mount_spec.typ().as_ref().map(|s| s.as_str());

    // Create mount point if it doesn't exist
    if !destination.exists() {
        fs::create_dir_all(destination)
            .with_context(|| format!("Failed to create mount point: {:?}", destination))?;
    }

    // Parse mount flags
    let mut flags: libc::c_ulong = 0;
    if let Some(options) = mount_spec.options() {
        for opt in options {
            match opt.as_str() {
                "ro" | "readonly" => flags |= libc::MS_RDONLY,
                "nosuid" => flags |= libc::MS_NOSUID,
                "nodev" => flags |= libc::MS_NODEV,
                "noexec" => flags |= libc::MS_NOEXEC,
                "bind" => flags |= libc::MS_BIND,
                "rbind" => flags |= libc::MS_BIND | libc::MS_REC,
                "private" => flags |= libc::MS_PRIVATE,
                "shared" => flags |= libc::MS_SHARED,
                "slave" => flags |= libc::MS_SLAVE,
                _ => {}
            }
        }
    }

    let source_c = source
        .and_then(|s| s.to_str())
        .map(|s| CString::new(s).ok())
        .flatten();
    let target_c = CString::new(destination.to_str().unwrap())?;
    let fstype_c = fs_type.map(|s| CString::new(s).ok()).flatten();

    unsafe {
        if libc::mount(
            source_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            target_c.as_ptr(),
            fstype_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
            flags,
            std::ptr::null(),
        ) != 0
        {
            return Err(anyhow::anyhow!(
                "Failed to mount {:?} to {:?}: {}",
                source,
                destination,
                std::io::Error::last_os_error()
            ));
        }
    }

    log::debug!("Mounted: {:?} -> {:?} (type: {:?})", source, destination, fs_type);
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn setup_single_mount(_mount_spec: &oci_spec::runtime::Mount) -> Result<()> {
    log::debug!("Skipping mount (not on Linux)");
    Ok(())
}

// Tests moved to tests/rootfs_tests.rs
