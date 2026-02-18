use rundmc::namespace::NamespaceConfig;

#[test]
fn test_namespace_config_default() {
    let config = NamespaceConfig::default();
    assert!(config.pid);
    assert!(config.mount);
    assert!(config.uts);
    assert!(config.ipc);
    assert!(config.network);
    assert!(!config.user); // User namespace disabled by default
}

#[test]
#[cfg(target_os = "linux")]
fn test_to_clone_flags() {
    let config = NamespaceConfig {
        pid: true,
        mount: true,
        uts: false,
        ipc: false,
        network: true,
        user: false,
    };

    let flags = config.to_clone_flags();
    assert!(flags & libc::CLONE_NEWPID != 0);
    assert!(flags & libc::CLONE_NEWNS != 0);
    assert!(flags & libc::CLONE_NEWNET != 0);
    assert!(flags & libc::CLONE_NEWUTS == 0);
    assert!(flags & libc::CLONE_NEWIPC == 0);
    assert!(flags & libc::CLONE_NEWUSER == 0);
}

#[test]
#[cfg(not(target_os = "linux"))]
fn test_to_clone_flags_stub() {
    let config = NamespaceConfig {
        pid: true,
        mount: true,
        uts: false,
        ipc: false,
        network: true,
        user: false,
    };

    let flags = config.to_clone_flags();
    assert_eq!(flags, 0); // Stub always returns 0
}
