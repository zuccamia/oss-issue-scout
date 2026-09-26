use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Watched {
    pub watched: Vec<String>,
}

pub fn load(path: impl AsRef<Path>) -> Result<Watched> {
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}
