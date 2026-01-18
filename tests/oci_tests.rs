use rundmc::oci::load_config;
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

#[test]
fn test_invalid_json() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.json");
    std::fs::write(&file_path, "{ invalid json }").unwrap();

    let result = load_config(&file_path);
    assert!(result.is_err());
}

#[test]
fn test_missing_process() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer(file, &serde_json::json!({
        "ociVersion": "1.0.0",
        "root": {
            "path": "rootfs"
        }
    })).unwrap();

    let result = load_config(&file_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("process"));
}

#[test]
fn test_missing_args() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer(file, &serde_json::json!({
        "ociVersion": "1.0.0",
        "process": {
            "cwd": "/",
            "user": {
                "uid": 0,
                "gid": 0
            }
        },
        "root": {
            "path": "rootfs"
        }
    })).unwrap();

    let result = load_config(&file_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("args"));
}

#[test]
fn test_empty_args() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer(file, &serde_json::json!({
        "ociVersion": "1.0.0",
        "process": {
            "cwd": "/",
            "args": [],
            "user": {
                "uid": 0,
                "gid": 0
            }
        },
        "root": {
            "path": "rootfs"
        }
    })).unwrap();

    let result = load_config(&file_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
}

#[test]
fn test_unsupported_version() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.json");
    let file = File::create(&file_path).unwrap();
    serde_json::to_writer(file, &serde_json::json!({
        "ociVersion": "2.0.0",
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

    let result = load_config(&file_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported OCI spec version"));
}

#[test]
fn test_missing_root() {
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
        }
    })).unwrap();

    let result = load_config(&file_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("root"));
}
