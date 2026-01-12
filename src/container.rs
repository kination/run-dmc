use std::ffi::CString;
use std::path::PathBuf;
use anyhow::{Result, Context, bail};
use nix::unistd::{fork, ForkResult, execvp};
use crate::oci::load_config;

pub fn create(bundle: PathBuf, _container_id: String) -> Result<()> {
    let config_path = bundle.join("config.json");
    let spec = load_config(&config_path).context("Failed to load OCI config from bundle")?;

    let process = spec.process().as_ref().context("No process defined in config")?;
    let args = process.args().as_ref().context("No args defined in process")?;

    println!("Creating container with args: {:?}", args);

    // TODO: Setup Namespaces (CLONE_NEWPID, etc.) - Linux only
    #[cfg(target_os = "linux")]
    {
        // simplistic unshare for now
        // nix::sched::unshare(nix::sched::CloneFlags::CLONE_NEWUTS | ...)?;
    }
    #[cfg(not(target_os = "linux"))]
    {
         println!("WARNING: Not on Linux, skipping namespace isolation.");
    }

    // Fork and Exec
    // In a real OCI runtime, 'create' sets up the environment and pauses.
    // 'start' then signals it to continue.
    // For this minimal MVP, we are effectively doing 'run' logic here but just spawning.
    
    // UNSAFE: fork is unsafe in multi-threaded programs.
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // Child process
            let process = spec.process().as_ref().context("No process config")?;
            
            // 1. Environment Variables
            if let Some(envs) = process.env() {
                for e in envs {
                    if let Some((k, v)) = e.split_once('=') {
                        unsafe {
                            std::env::set_var(k, v);
                        }
                    }
                }
            }

            // 2. Working Directory
            let cwd = process.cwd();
            let cwd_cstr = CString::new(cwd.to_str().unwrap_or("/")).unwrap();
            unsafe {
                libc::chdir(cwd_cstr.as_ptr());
            }

            let c_args: Vec<CString> = args.iter()
                .map(|s| CString::new(s.as_str()).unwrap_or_default())
                .collect();
            
            if let Some(cmd) = c_args.first() {
                // TODO: pivot_root, mounts, etc. (See CHILD_BLOCK_KR.md)
                let _ = execvp(cmd, &c_args);
            }
            // If exec fails or no args
            std::process::exit(1);
        }
        Ok(ForkResult::Parent { child: _ }) => {
            // Parent process returns immediately for 'create' 
            // In full OCI, we would register the container state handling here.
             println!("Container process spawned.");
             Ok(())
        }
        Err(e) => bail!("Fork failed: {}", e),
    }
}
