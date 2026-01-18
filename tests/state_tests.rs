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
    assert_eq!(state.status, ContainerStatus::Creating);
    assert_eq!(state.pid, None);
    assert_eq!(state.bundle, PathBuf::from("/tmp/bundle"));
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
    state.status = ContainerStatus::Running;
    state.pid = Some(12345);

    // Save state
    state.save(root).unwrap();

    // Load state
    let loaded = ContainerState::load(root, "test-container").unwrap();

    assert_eq!(loaded.id, state.id);
    assert_eq!(loaded.status, state.status);
    assert_eq!(loaded.pid, state.pid);
    assert_eq!(loaded.bundle, state.bundle);
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
fn test_load_nonexistent() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let result = ContainerState::load(root, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}
