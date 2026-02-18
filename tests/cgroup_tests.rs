use rundmc::cgroup::{CgroupConfig, CgroupManager, is_cgroup_v2_available};

#[test]
fn test_cgroup_config_from_empty_spec() {
    // Test cgroup config with no resource limits
    let spec = oci_spec::runtime::Spec::default();
    let config = CgroupConfig::from_oci_spec(&spec, "test-container").unwrap();

    assert_eq!(config.container_id, "test-container");
    assert_eq!(config.cpu_quota, None);
    assert_eq!(config.cpu_period, None);
    assert_eq!(config.memory_limit, None);
}

#[test]
#[cfg(target_os = "linux")]
fn test_cgroup_v2_detection() {
    // This test checks if cgroup v2 is available on the system
    // On Linux systems with cgroups v2, this should return true
    let is_available = is_cgroup_v2_available();

    // We can't assert true/false as it depends on the system
    // But we can verify it doesn't panic
    println!("Cgroups v2 available: {}", is_available);
}

#[test]
#[cfg(not(target_os = "linux"))]
fn test_cgroup_v2_detection_stub() {
    // On non-Linux systems, should return false
    let is_available = is_cgroup_v2_available();
    assert!(!is_available, "Cgroups v2 should not be available on non-Linux");
}

#[test]
#[cfg(target_os = "linux")]
fn test_cgroup_manager_creation() {
    // Test that we can create a cgroup manager
    // This might fail if not running as root or cgroups v2 not available
    match CgroupManager::new("test-cgroup-manager") {
        Ok(_) => {
            println!("CgroupManager created successfully");
        }
        Err(e) => {
            println!("CgroupManager creation failed (expected if not root or no cgroupv2): {}", e);
        }
    }
}

#[test]
fn test_cgroup_config_with_cpu_limits() {
    use oci_spec::runtime::{Spec, LinuxBuilder, LinuxResourcesBuilder, LinuxCpuBuilder};

    let cpu = LinuxCpuBuilder::default()
        .quota(100000)
        .period(100000u64)
        .shares(1024u64)
        .build()
        .unwrap();

    let resources = LinuxResourcesBuilder::default()
        .cpu(cpu)
        .build()
        .unwrap();

    let linux = LinuxBuilder::default()
        .resources(resources)
        .build()
        .unwrap();

    let spec = oci_spec::runtime::SpecBuilder::default()
        .linux(linux)
        .build()
        .unwrap();

    let config = CgroupConfig::from_oci_spec(&spec, "test-container").unwrap();

    assert_eq!(config.cpu_quota, Some(100000));
    assert_eq!(config.cpu_period, Some(100000));
    assert_eq!(config.cpu_weight, Some(1024));
}

#[test]
fn test_cgroup_config_with_memory_limits() {
    use oci_spec::runtime::{Spec, LinuxBuilder, LinuxResourcesBuilder, LinuxMemoryBuilder};

    let memory = LinuxMemoryBuilder::default()
        .limit(536870912) // 512 MB
        .swap(1073741824) // 1 GB
        .build()
        .unwrap();

    let resources = LinuxResourcesBuilder::default()
        .memory(memory)
        .build()
        .unwrap();

    let linux = LinuxBuilder::default()
        .resources(resources)
        .build()
        .unwrap();

    let spec = oci_spec::runtime::SpecBuilder::default()
        .linux(linux)
        .build()
        .unwrap();

    let config = CgroupConfig::from_oci_spec(&spec, "test-container").unwrap();

    assert_eq!(config.memory_limit, Some(536870912));
    assert_eq!(config.memory_swap, Some(1073741824));
}

#[test]
fn test_cgroup_config_with_pids_limit() {
    use oci_spec::runtime::{Spec, LinuxBuilder, LinuxResourcesBuilder, LinuxPidsBuilder};

    let pids = LinuxPidsBuilder::default()
        .limit(1024)
        .build()
        .unwrap();

    let resources = LinuxResourcesBuilder::default()
        .pids(pids)
        .build()
        .unwrap();

    let linux = LinuxBuilder::default()
        .resources(resources)
        .build()
        .unwrap();

    let spec = oci_spec::runtime::SpecBuilder::default()
        .linux(linux)
        .build()
        .unwrap();

    let config = CgroupConfig::from_oci_spec(&spec, "test-container").unwrap();

    assert_eq!(config.pids_max, Some(1024));
}

#[test]
#[cfg(not(target_os = "linux"))]
fn test_cgroup_operations_stub() {
    // Test that cgroup operations don't panic on non-Linux platforms
    let config = CgroupConfig {
        container_id: "test-stub".to_string(),
        cpu_quota: Some(100000),
        cpu_period: Some(100000),
        cpu_weight: Some(100),
        memory_limit: Some(536870912),
        memory_swap: Some(1073741824),
        pids_max: Some(1024),
    };

    // These should succeed as stubs
    if let Ok(manager) = CgroupManager::new("test-stub") {
        assert!(manager.setup(&config).is_ok());
        assert!(manager.add_process(1234).is_ok());
        assert!(manager.cleanup().is_ok());
    }
}
