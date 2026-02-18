use rundmc::rootfs::RootfsConfig;
use std::path::PathBuf;

#[test]
fn test_rootfs_config_creation() {
    let config = RootfsConfig {
        path: PathBuf::from("/tmp/rootfs"),
        readonly: false,
    };
    assert_eq!(config.path, PathBuf::from("/tmp/rootfs"));
    assert!(!config.readonly);
}

#[test]
fn test_rootfs_config_readonly() {
    let config = RootfsConfig {
        path: PathBuf::from("/tmp/rootfs"),
        readonly: true,
    };
    assert!(config.readonly);
}
