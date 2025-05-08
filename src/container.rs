// use nix::sched::CloneFlags;
use nix::unistd::{fork, ForkResult, execvp};
use std::ffi::CString;

use crate::oci::load_config;

pub fn run(bundle: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("{}/config.json", bundle);
    let config = load_config(&config_path);

    // unshare(CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWWNS);

    match unsafe { fork()? } {
        ForkResult::Child => {
            let args: Vec<CString> = config.unwrap().process.args.iter().map(|a| CString::new(a.as_str()).unwrap()).collect();
            execvp(&args[0], &args)?;
            Ok(())
        }
        ForkResult::Parent { child } => {
            nix::sys::wait::waitpid(child, None)?;
            Ok(())
        }
    }
}
