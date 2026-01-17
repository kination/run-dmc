use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Container state as defined by OCI runtime spec
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    /// The container is being created
    Creating,
    /// The container process has been created but not started
    Created,
    /// The container process is running
    Running,
    /// The container process has exited
    Stopped,
}

/// Container state structure following OCI runtime spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerState {
    /// OCI version
    #[serde(rename = "ociVersion")]
    pub oci_version: String,
    /// Container ID
    pub id: String,
    /// Container status
    pub status: ContainerStatus,
    /// PID of the container process (if running)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<i32>,
    /// Path to the bundle directory
    pub bundle: PathBuf,
    /// Annotations from the config (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
}

impl ContainerState {
    /// Create a new container state
    pub fn new(id: String, bundle: PathBuf, oci_version: String) -> Self {
        Self {
            oci_version,
            id,
            status: ContainerStatus::Creating,
            pid: None,
            bundle,
            annotations: None,
        }
    }

    /// Get the path to the state file for a container
    pub fn state_file_path(root: &Path, container_id: &str) -> PathBuf {
        root.join(container_id).join("state.json")
    }

    /// Get the directory for container state
    pub fn container_dir(root: &Path, container_id: &str) -> PathBuf {
        root.join(container_id)
    }

    /// Load container state from disk
    pub fn load(root: &Path, container_id: &str) -> Result<Self> {
        let state_file = Self::state_file_path(root, container_id);

        if !state_file.exists() {
            bail!("Container {} does not exist", container_id);
        }

        let contents = fs::read_to_string(&state_file)
            .with_context(|| format!("Failed to read state file: {}", state_file.display()))?;

        let state: ContainerState = serde_json::from_str(&contents)
            .context("Failed to parse state file")?;

        Ok(state)
    }

    /// Save container state to disk atomically
    pub fn save(&self, root: &Path) -> Result<()> {
        let container_dir = Self::container_dir(root, &self.id);
        let state_file = Self::state_file_path(root, &self.id);

        // Create container directory if it doesn't exist
        fs::create_dir_all(&container_dir)
            .with_context(|| format!("Failed to create container directory: {}", container_dir.display()))?;

        // Write to temporary file first for atomic operation
        let temp_file = state_file.with_extension("tmp");
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize state")?;

        fs::write(&temp_file, json)
            .with_context(|| format!("Failed to write temporary state file: {}", temp_file.display()))?;

        // Atomic rename
        fs::rename(&temp_file, &state_file)
            .with_context(|| format!("Failed to rename state file: {}", state_file.display()))?;

        Ok(())
    }

    /// Delete container state from disk
    pub fn delete(root: &Path, container_id: &str) -> Result<()> {
        let container_dir = Self::container_dir(root, container_id);

        if container_dir.exists() {
            fs::remove_dir_all(&container_dir)
                .with_context(|| format!("Failed to remove container directory: {}", container_dir.display()))?;
        }

        Ok(())
    }

    /// Check if a container exists
    pub fn exists(root: &Path, container_id: &str) -> bool {
        Self::state_file_path(root, container_id).exists()
    }
}

