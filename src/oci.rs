use oci_spec::runtime::Spec;
use std::path::Path;
use anyhow::{Context, Result};

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Spec> {
    Spec::load(path).context("Failed to load OCI config")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_load_valid_config() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.json");
        let file = File::create(&file_path).unwrap();
        serde_json::to_writer(file, &serde_json::json!({
            "ociVersion": "1.0.0",
            "process": {
                "cwd": "/",
                "args": ["sh"],
                "user": {
                    "uid": 0,
                    "gid": 0
                }
            },
            "root": {
                "path": "rootfs"
            }
        })).unwrap();

        let spec = load_config(&file_path).unwrap();
        assert_eq!(spec.version(), "1.0.0");
        assert_eq!(spec.process().as_ref().unwrap().args().as_ref().unwrap()[0], "sh");
    }
}
