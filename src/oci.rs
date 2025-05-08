use serde::{Deserialize};
use std::fs::File;

#[derive(Deserialize)]
pub struct Process {
    pub args: Vec<String>,
    pub cwd: String
}

#[derive(Deserialize)]
pub struct OciConfig {
    pub process: Process
}

pub fn load_config(path: &str) -> Result<OciConfig, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let config = serde_json::from_reader(file)?;
    Ok(config)
}
