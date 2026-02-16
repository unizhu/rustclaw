//! `RustClaw` Skills System
//!
//! Production-ready skills system implementing progressive disclosure architecture
//! based on 2025-2026 AI agent research and best practices from Claude Code,
//! OpenAI Codex, and Anthropic's agent patterns.
//!
//! ## Features
//!
//! - Progressive disclosure: Load skill metadata at startup, full content on demand
//! - YAML frontmatter support for skill metadata (name, description)
//! - Multiple skills directories (personal, project, plugin)
//! - Automatic skill discovery and registration
//! - LLM-friendly skill descriptions for semantic matching
//!
//! ## Architecture
//!
//! Phase 1 (Discovery): At startup, load only name and description from each SKILL.md
//! Phase 2 (Activation): When task matches, load full SKILL.md content
//! Phase 3 (Execution): Agent follows instructions, loads referenced files as needed

#![deny(unsafe_code, dead_code, unused_imports, unused_variables, missing_docs)]

pub mod registry;
pub mod skill;

pub use registry::SkillsRegistry;
pub use skill::Skill;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{Skill, SkillsRegistry};
}
