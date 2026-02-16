//! Skills registry for managing multiple skills
//!
//! Implements progressive disclosure architecture:
//! - Phase 1: Scan directories and load metadata only
//! - Phase 2: Load full skill content on demand

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::skill::Skill;

/// Skills registry managing all available skills
pub struct SkillsRegistry {
    /// All discovered skills (metadata only initially)
    skills: HashMap<String, Skill>,
    /// Skills directories to scan
    directories: Vec<PathBuf>,
}

impl SkillsRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            directories: Vec::new(),
        }
    }

    /// Add a skills directory to scan
    pub fn add_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.directories.push(dir.into());
        self
    }

    /// Add personal skills directory: ~/.rustclaw/skills/
    pub fn with_personal_skills(self) -> Self {
        if let Some(home) = dirs::home_dir() {
            self.add_directory(home.join(".rustclaw").join("skills"))
        } else {
            warn!("Could not find home directory for personal skills");
            self
        }
    }

    /// Add project skills directory: ./.rustclaw/skills/
    pub fn with_project_skills(self) -> Self {
        self.add_directory(PathBuf::from(".rustclaw/skills"))
    }

    /// Scan all configured directories and discover skills (Phase 1: Discovery)
    pub fn discover(&mut self) -> Result<()> {
        info!(
            "Starting skills discovery in {} directories",
            self.directories.len()
        );

        let directories = self.directories.clone();

        for dir in &directories {
            if !dir.exists() {
                debug!("Skills directory does not exist: {:?}", dir);
                continue;
            }

            if !dir.is_dir() {
                warn!("Skills path is not a directory: {:?}", dir);
                continue;
            }

            self.scan_directory(dir)?;
        }

        info!("Discovered {} skills", self.skills.len());
        Ok(())
    }

    /// Scan a single directory for skills
    fn scan_directory(&mut self, dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory {:?}", dir))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // Try to load skill metadata
            match Skill::metadata_from_dir(&path) {
                Ok(skill) => {
                    let name = skill.name().to_string();
                    debug!("Discovered skill: {} at {:?}", name, path);
                    self.skills.insert(name, skill);
                }
                Err(e) => {
                    debug!("Skipping {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// Get a skill by name (returns metadata only if not yet loaded)
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Get a mutable skill by name (can load content)
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Skill> {
        self.skills.get_mut(name)
    }

    /// Load full content for a specific skill (Phase 2: Activation)
    pub fn load_skill(&mut self, name: &str) -> Result<&Skill> {
        let skill = self
            .skills
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", name))?;

        skill.load_content()?;
        Ok(skill)
    }

    /// Get all skill names
    pub fn skill_names(&self) -> impl Iterator<Item = &String> {
        self.skills.keys()
    }

    /// Get number of skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Generate skills list for LLM system prompt
    /// Format:
    /// Available skills (use /{skill-name} or call the Skill tool):
    /// - skill-name: Description of what this skill does and when to use it
    /// - another-skill: Another description...
    pub fn generate_system_prompt(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("\n\nAvailable skills (use /{skill-name} to activate):\n");

        // Sort skills by name for consistent ordering
        let mut sorted_skills: Vec<_> = self.skills.values().collect();
        sorted_skills.sort_by_key(|s| s.name());

        for skill in sorted_skills {
            prompt.push_str(&skill.to_summary());
            prompt.push('\n');
        }

        prompt
    }

    /// Generate a concise skills list for embedding in tool descriptions
    pub fn generate_skills_list(&self) -> String {
        if self.skills.is_empty() {
            return "No skills available".to_string();
        }

        let mut list = String::new();
        let mut sorted_skills: Vec<_> = self.skills.values().collect();
        sorted_skills.sort_by_key(|s| s.name());

        for (i, skill) in sorted_skills.iter().enumerate() {
            if i > 0 {
                list.push_str(", ");
            }
            list.push_str(&format!("{}: {}", skill.name(), skill.description()));
        }

        list
    }
}

impl Default for SkillsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = SkillsRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_generate_system_prompt_empty() {
        let registry = SkillsRegistry::new();
        let prompt = registry.generate_system_prompt();
        assert!(prompt.is_empty());
    }

    #[test]
    fn test_generate_skills_list_empty() {
        let registry = SkillsRegistry::new();
        let list = registry.generate_skills_list();
        assert_eq!(list, "No skills available");
    }
}
