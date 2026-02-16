//! Skill definition and parsing
//!
//! Each skill is a folder containing SKILL.md with YAML frontmatter

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Maximum allowed name length (from Anthropic spec)
const MAX_NAME_LENGTH: usize = 64;
/// Maximum allowed description length (from Anthropic spec)
const MAX_DESCRIPTION_LENGTH: usize = 1024;

/// Skill metadata extracted from YAML frontmatter
#[derive(Debug, Clone, Deserialize)]
pub struct SkillMetadata {
    /// Skill name (max 64 chars, lowercase letters/numbers/hyphens only)
    pub name: String,
    /// Skill description (max 1024 chars, describes WHAT and WHEN)
    pub description: String,
}

/// A complete skill with metadata and content
#[derive(Debug, Clone)]
pub struct Skill {
    /// Skill metadata
    pub metadata: SkillMetadata,
    /// Full path to skill directory
    pub path: PathBuf,
    /// Full SKILL.md content (loaded on demand)
    pub content: Option<String>,
}

impl Skill {
    /// Load skill from a directory
    pub fn from_dir(dir: &Path) -> Result<Self> {
        let skill_file = dir.join("SKILL.md");

        if !skill_file.exists() {
            return Err(anyhow!("SKILL.md not found in {:?}", dir));
        }

        let content = fs::read_to_string(&skill_file)
            .with_context(|| format!("Failed to read {:?}", skill_file))?;

        let (metadata, _) = parse_skill_content(&content)
            .with_context(|| format!("Failed to parse skill from {:?}", skill_file))?;

        // Validate metadata
        validate_metadata(&metadata)?;

        Ok(Self {
            metadata,
            path: dir.to_path_buf(),
            content: Some(content),
        })
    }

    /// Load only metadata from a directory (Phase 1: Discovery)
    pub fn metadata_from_dir(dir: &Path) -> Result<Self> {
        let skill_file = dir.join("SKILL.md");

        if !skill_file.exists() {
            return Err(anyhow!("SKILL.md not found in {:?}", dir));
        }

        let content = fs::read_to_string(&skill_file)
            .with_context(|| format!("Failed to read {:?}", skill_file))?;

        let (metadata, _) = parse_skill_content(&content)
            .with_context(|| format!("Failed to parse skill from {:?}", skill_file))?;

        // Validate metadata
        validate_metadata(&metadata)?;

        Ok(Self {
            metadata,
            path: dir.to_path_buf(),
            content: None, // Don't load full content yet
        })
    }

    /// Load full content if not already loaded (Phase 2: Activation)
    pub fn load_content(&mut self) -> Result<()> {
        if self.content.is_some() {
            return Ok(());
        }

        let skill_file = self.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_file)
            .with_context(|| format!("Failed to read {:?}", skill_file))?;

        self.content = Some(content);
        Ok(())
    }

    /// Get the skill directory name
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Get the skill description
    pub fn description(&self) -> &str {
        &self.metadata.description
    }

    /// Generate a concise summary for LLM system prompt
    /// Format: "- {name}: {description}"
    pub fn to_summary(&self) -> String {
        format!("- {}: {}", self.metadata.name, self.metadata.description)
    }
}

/// Parse skill content to extract frontmatter metadata and body
fn parse_skill_content(content: &str) -> Result<(SkillMetadata, String)> {
    // Extract YAML frontmatter
    let frontmatter_re = Regex::new(r"^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$")
        .map_err(|e| anyhow!("Failed to compile regex: {}", e))?;

    let captures = frontmatter_re
        .captures(content)
        .ok_or_else(|| anyhow!("No valid YAML frontmatter found"))?;

    let yaml_str = captures
        .get(1)
        .ok_or_else(|| anyhow!("Failed to extract frontmatter"))?
        .as_str();

    let body = captures.get(2).map(|m| m.as_str()).unwrap_or("");

    let metadata: SkillMetadata =
        serde_yaml::from_str(yaml_str).with_context(|| "Failed to parse YAML frontmatter")?;

    Ok((metadata, body.to_string()))
}

/// Validate skill metadata according to Anthropic specification
fn validate_metadata(metadata: &SkillMetadata) -> Result<()> {
    // Validate name
    if metadata.name.is_empty() {
        return Err(anyhow!("Skill name cannot be empty"));
    }

    if metadata.name.len() > MAX_NAME_LENGTH {
        warn!(
            "Skill name '{}' exceeds {} characters (was {}), may be truncated",
            metadata.name,
            MAX_NAME_LENGTH,
            metadata.name.len()
        );
    }

    // Name should be lowercase letters, numbers, and hyphens only
    let name_re = Regex::new(r"^[a-z0-9-]+$")
        .map_err(|e| anyhow!("Failed to compile name validation regex: {}", e))?;

    if !name_re.is_match(&metadata.name) {
        return Err(anyhow!(
            "Skill name '{}' must contain only lowercase letters, numbers, and hyphens",
            metadata.name
        ));
    }

    // Validate description
    if metadata.description.is_empty() {
        return Err(anyhow!("Skill description cannot be empty"));
    }

    if metadata.description.len() > MAX_DESCRIPTION_LENGTH {
        warn!(
            "Skill '{}' description exceeds {} characters (was {}), may be truncated",
            metadata.name,
            MAX_DESCRIPTION_LENGTH,
            metadata.description.len()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_content() {
        let content = r#"---
name: code-reviewer
description: Reviews code for best practices and security. Use when reviewing or analyzing code.
---

# Code Reviewer

This skill helps review code.
"#;

        let (metadata, body) = parse_skill_content(content).unwrap();
        assert_eq!(metadata.name, "code-reviewer");
        assert_eq!(
            metadata.description,
            "Reviews code for best practices and security. Use when reviewing or analyzing code."
        );
        assert!(body.contains("# Code Reviewer"));
    }

    #[test]
    fn test_validate_metadata() {
        let valid = SkillMetadata {
            name: "valid-skill-name".to_string(),
            description: "A valid description".to_string(),
        };
        assert!(validate_metadata(&valid).is_ok());

        let invalid_name = SkillMetadata {
            name: "Invalid_Name".to_string(),
            description: "A description".to_string(),
        };
        assert!(validate_metadata(&invalid_name).is_err());
    }
}
