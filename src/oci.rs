use oci_spec::runtime::Spec;
use std::path::Path;
use anyhow::{Context, Result, bail};

/// Load and validate OCI runtime specification from config.json
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Spec> {
    let spec = Spec::load(&path).with_context(|| {
        format!("Failed to load OCI config from: {}", path.as_ref().display())
    })?;

    validate_spec(&spec)?;
    Ok(spec)
}

/// Validate required fields and OCI spec version compatibility
fn validate_spec(spec: &Spec) -> Result<()> {
    // Validate OCI version
    let version = spec.version();
    if !version.starts_with("1.") {
        bail!(
            "Unsupported OCI spec version: {}. Expected 1.x.x",
            version
        );
    }

    // Validate process configuration
    let process = spec
        .process()
        .as_ref()
        .context("Missing required field: process")?;

    // Validate process args
    let args = process
        .args()
        .as_ref()
        .context("Missing required field: process.args")?;

    if args.is_empty() {
        bail!("process.args cannot be empty");
    }

    // Validate process user
    let user = process.user();
    let _uid = user.uid();
    let _gid = user.gid();

    // Validate root configuration
    let root = spec
        .root()
        .as_ref()
        .context("Missing required field: root")?;

    let root_path = root.path();
    if root_path.as_os_str().is_empty() {
        bail!("root.path cannot be empty");
    }

    Ok(())
}

