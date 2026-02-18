use rundmc::state::{ContainerState, ContainerStatus};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_state_creation() {
    let state = ContainerState::new(
        "test-container".to_string(),
        PathBuf::from("/tmp/bundle"),
        "1.0.0".to_string(),
    );

    assert_eq!(state.id, "test-container");
    assert_eq!(state.bundle, PathBuf::from("/tmp/bundle"));
    assert_eq!(state.oci_version, "1.0.0");
    assert_eq!(state.status, ContainerStatus::Creating);
    assert_eq!(state.pid, None);
}

#[test]
fn test_state_save_and_load() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let mut state = ContainerState::new(
        "test-container".to_string(),
        PathBuf::from("/tmp/bundle"),
        "1.0.0".to_string(),
    );
    state.status = ContainerStatus::Created;
    state.pid = Some(12345);

    // Save
    state.save(root).unwrap();

    // Load
    let loaded = ContainerState::load(root, "test-container").unwrap();
    assert_eq!(loaded.id, state.id);
    assert_eq!(loaded.bundle, state.bundle);
    assert_eq!(loaded.status, ContainerStatus::Created);
    assert_eq!(loaded.pid, Some(12345));
}

#[test]
fn test_state_exists() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    assert!(!ContainerState::exists(root, "nonexistent"));

    let state = ContainerState::new(
        "test-container".to_string(),
        PathBuf::from("/tmp/bundle"),
        "1.0.0".to_string(),
    );
    state.save(root).unwrap();

    assert!(ContainerState::exists(root, "test-container"));
}

#[test]
fn test_state_delete() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let state = ContainerState::new(
        "test-container".to_string(),
        PathBuf::from("/tmp/bundle"),
        "1.0.0".to_string(),
    );
    state.save(root).unwrap();

    assert!(ContainerState::exists(root, "test-container"));

    ContainerState::delete(root, "test-container").unwrap();

    assert!(!ContainerState::exists(root, "test-container"));
}

#[test]
fn test_state_status_serialization() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let statuses = vec![
        ContainerStatus::Creating,
        ContainerStatus::Created,
        ContainerStatus::Running,
        ContainerStatus::Stopped,
    ];

    for (i, status) in statuses.iter().enumerate() {
        let mut state = ContainerState::new(
            format!("container-{}", i),
            PathBuf::from("/tmp/bundle"),
            "1.0.0".to_string(),
        );
        state.status = status.clone();
        state.save(root).unwrap();

        let loaded = ContainerState::load(root, &format!("container-{}", i)).unwrap();
        assert_eq!(loaded.status, *status);
    }
}
