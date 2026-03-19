pub mod flake_gen;
pub mod process_compose;

use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub struct DevShellConfig {
    pub name: String,
    pub build_inputs: Vec<String>,
    pub env_vars: IndexMap<String, String>,
    pub shell_hook_lines: Vec<String>,
}

impl DevShellConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            build_inputs: Vec::new(),
            env_vars: IndexMap::new(),
            shell_hook_lines: Vec::new(),
        }
    }

    /// Add a nix package, deduplicating.
    pub fn add_input(&mut self, pkg: &str) {
        if !self.build_inputs.iter().any(|p| p == pkg) {
            self.build_inputs.push(pkg.to_string());
        }
    }

    /// Merge another config into this one.
    pub fn merge(&mut self, other: &DevShellConfig) {
        for pkg in &other.build_inputs {
            self.add_input(pkg);
        }
        for (k, v) in &other.env_vars {
            self.env_vars.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for line in &other.shell_hook_lines {
            if !self.shell_hook_lines.contains(line) {
                self.shell_hook_lines.push(line.clone());
            }
        }
    }
}
