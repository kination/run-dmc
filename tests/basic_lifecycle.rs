use std::process::Command;
use std::fs::File;
use tempfile::tempdir;
use serde_json::json;

#[test]
fn test_create_command() {
    // 1. Setup bundle
    let dir = tempdir().unwrap();
    let bundle_path = dir.path();
    let config_path = bundle_path.join("config.json");

    // 2. Setup root directory for container state
    let root_dir = tempdir().unwrap();
    let root_path = root_dir.path();

    let config = json!({
        "ociVersion": "1.0.0",
        "process": {
            "cwd": ".",
            "args": ["echo", "hello_from_container"],
            "env": ["PATH=/bin:/usr/bin"],
            "user": {
                "uid": 0,
                "gid": 0
            }
        },
        "root": {
            "path": "rootfs"
        }
    });

    let f = File::create(&config_path).expect("failed to create config file");
    serde_json::to_writer(f, &config).expect("failed to write config");

    // 3. Run the runtime binary
    // Using cargo run to ensure we run the latest code
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--quiet",
            "--",
            "--root",
            root_path.to_str().unwrap(),
            "create",
            "--bundle",
            bundle_path.to_str().unwrap(),
            "test_container_id"
        ])
        .output()
        .expect("Failed to execute runtime");

    // 4. Verify output
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    assert!(output.status.success(), "Runtime failed to execute");

    // Verify container was created
    assert!(stdout.contains("Container test_container_id created with PID"));
}
