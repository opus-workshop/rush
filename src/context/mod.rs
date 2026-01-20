// Project and Git context detection
// TODO: Implement project type detection and Git integration

use std::path::Path;

#[derive(Debug, Clone)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Unknown,
}

pub struct Context {
    project_type: ProjectType,
}

impl Context {
    pub fn new() -> Self {
        Self {
            project_type: ProjectType::Unknown,
        }
    }

    pub fn detect_project(&mut self, _path: &Path) -> ProjectType {
        // TODO: Check for Cargo.toml, package.json, etc.
        ProjectType::Unknown
    }

    pub fn get_project_type(&self) -> &ProjectType {
        &self.project_type
    }
}
