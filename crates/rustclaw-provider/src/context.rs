//! RustClaw Context Management
//!
//! Implements state-of-the-art context management strategies for LLM conversations
//! based on 2025-2026 research findings.
//!
//! Strategies:
//! - Sliding window with observation masking
//! - LLM-based summarization at threshold
//! - Hybrid approach combining both

use chrono::{DateTime, Utc};
use rustclaw_types::{ChatMessage, Role, ToolCall};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, info};
use uuid::Uuid;

// ============================================================================
// Configuration
// ============================================================================

/// Default context window size (in tokens, approximate)
const DEFAULT_CONTEXT_WINDOW: usize = 128_000;

/// Percentage of context to trigger compression (70-80% recommended)
const COMPRESSION_THRESHOLD: f32 = 0.75;

/// Number of recent turns to always keep in full detail
const RECENT_TURNS_TO_KEEP: usize = 10;

// ============================================================================
// Message Types
// ============================================================================

/// A conversation turn with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: String,
    pub role: Role,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub token_count: usize,
    pub is_summarized: bool,
    pub is_masked: bool,
}

impl ConversationTurn {
    pub fn user(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::User,
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
            timestamp: Utc::now(),
            token_count: 0,
            is_summarized: false,
            is_masked: false,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
            timestamp: Utc::now(),
            token_count: 0,
            is_summarized: false,
            is_masked: false,
        }
    }

    pub fn assistant_with_tools(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Assistant,
            content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
            timestamp: Utc::now(),
            token_count: 0,
            is_summarized: false,
            is_masked: false,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        Self {
            id: Uuid::new_v4().to_string(),
            role: Role::Tool,
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            timestamp: Utc::now(),
            token_count: 0,
            is_summarized: false,
            is_masked: false,
        }
    }

    /// Estimate token count (rough approximation: 1 token ≈ 4 chars)
    pub fn estimate_tokens(&mut self) -> usize {
        let mut count = 0;
        if let Some(ref content) = self.content {
            count += content.len() / 4;
        }
        if let Some(ref calls) = self.tool_calls {
            for call in calls {
                count += call.function.name.len() / 4;
                count += call.function.arguments.len() / 4;
            }
        }
        self.token_count = count.max(1);
        self.token_count
    }

    /// Convert to API message format
    pub fn to_chat_message(&self) -> ChatMessage {
        ChatMessage {
            role: self.role.clone(),
            content: self.content.clone(),
            name: None,
            tool_calls: self.tool_calls.clone(),
            tool_call_id: self.tool_call_id.clone(),
        }
    }

    /// Create a masked version (placeholder for old content)
    pub fn masked(&self) -> Self {
        let mut masked = self.clone();
        masked.is_masked = true;
        masked.content = Some("[Previous context omitted for brevity]".to_string());
        masked.tool_calls = None;
        masked.token_count = 10; // Minimal tokens
        masked
    }
}

// ============================================================================
// Context Manager
// ============================================================================

/// Summary of a conversation segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub turns_covered: Vec<String>,
    pub summary: String,
    pub key_facts: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub token_count: usize,
}

/// Context management strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextStrategy {
    /// Keep all messages (no compression)
    None,
    /// Use sliding window with observation masking
    SlidingWindow,
    /// Use LLM summarization
    Summarization,
    /// Hybrid: masking + occasional summarization
    Hybrid,
}

/// Conversation context manager
pub struct ContextManager {
    /// Conversation turns
    turns: VecDeque<ConversationTurn>,
    /// Summaries of old conversations
    summaries: Vec<ConversationSummary>,
    /// Current strategy
    strategy: ContextStrategy,
    /// Maximum context window (tokens)
    max_tokens: usize,
    /// Number of recent turns to always keep
    recent_turns: usize,
    /// System prompt
    system_prompt: String,
    /// Total estimated tokens
    total_tokens: usize,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            turns: VecDeque::new(),
            summaries: Vec::new(),
            strategy: ContextStrategy::Hybrid,
            max_tokens: DEFAULT_CONTEXT_WINDOW,
            recent_turns: RECENT_TURNS_TO_KEEP,
            system_prompt: String::new(),
            total_tokens: 0,
        }
    }

    pub fn with_strategy(mut self, strategy: ContextStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Add a turn to the conversation
    pub fn add_turn(&mut self, mut turn: ConversationTurn) {
        turn.estimate_tokens();
        self.total_tokens += turn.token_count;
        self.turns.push_back(turn);

        // Check if compression needed
        if self.should_compress() {
            self.compress();
        }
    }

    /// Check if compression is needed
    fn should_compress(&self) -> bool {
        let threshold = (self.max_tokens as f32 * COMPRESSION_THRESHOLD) as usize;
        self.total_tokens > threshold && self.turns.len() > self.recent_turns
    }

    /// Compress context using the configured strategy
    fn compress(&mut self) {
        match self.strategy {
            ContextStrategy::None => {
                debug!("Compression disabled, skipping");
            }
            ContextStrategy::SlidingWindow => {
                self.apply_sliding_window();
            }
            ContextStrategy::Summarization => {
                // Note: Actual summarization requires LLM call, done externally
                info!("Summarization triggered but requires external LLM call");
            }
            ContextStrategy::Hybrid => {
                self.apply_hybrid_compression();
            }
        }
    }

    /// Apply sliding window with observation masking
    fn apply_sliding_window(&mut self) {
        let turns_to_mask = self.turns.len().saturating_sub(self.recent_turns);

        if turns_to_mask == 0 {
            return;
        }

        info!(
            "Applying observation masking to {} old turns",
            turns_to_mask
        );

        let mut tokens_saved = 0;

        for i in 0..turns_to_mask {
            if let Some(turn) = self.turns.get_mut(i) {
                if !turn.is_masked && !turn.is_summarized {
                    let old_tokens = turn.token_count;
                    *turn = turn.masked();
                    tokens_saved += old_tokens.saturating_sub(turn.token_count);
                }
            }
        }

        self.total_tokens = self.total_tokens.saturating_sub(tokens_saved);
        debug!("Saved {} tokens via masking", tokens_saved);
    }

    /// Apply hybrid compression (masking + summarization)
    fn apply_hybrid_compression(&mut self) {
        // First apply sliding window
        self.apply_sliding_window();

        // If still over threshold, mark for summarization
        let threshold = (self.max_tokens as f32 * 0.9) as usize;
        if self.total_tokens > threshold {
            info!("Context still high after masking, summarization recommended");
        }
    }

    /// Create a summary of old turns (to be called with LLM)
    pub fn get_turns_to_summarize(&self) -> Vec<&ConversationTurn> {
        let skip_recent = self.recent_turns.max(5);
        self.turns
            .iter()
            .rev()
            .skip(skip_recent)
            .filter(|t| !t.is_summarized)
            .collect()
    }

    /// Apply a summary (replacing old turns)
    pub fn apply_summary(&mut self, summary: ConversationSummary) {
        let token_count = summary.token_count;
        
        // Remove summarized turns
        let summarized_ids: std::collections::HashSet<_> =
            summary.turns_covered.iter().collect();

        let mut removed_tokens = 0;
        self.turns.retain(|t| {
            if summarized_ids.contains(&t.id) {
                removed_tokens += t.token_count;
                false
            } else {
                true
            }
        });

        // Add summary as a system-like message
        let summary_turn = ConversationTurn {
            id: summary.id.clone(),
            role: Role::System,
            content: Some(format!("[Conversation Summary]\n{}", summary.summary)),
            tool_calls: None,
            tool_call_id: None,
            timestamp: summary.timestamp,
            token_count: summary.token_count,
            is_summarized: true,
            is_masked: false,
        };

        self.total_tokens = self.total_tokens
            .saturating_sub(removed_tokens)
            .saturating_add(token_count);
        self.turns.push_front(summary_turn);
        self.summaries.push(summary);

        info!("Applied summary, saved {} tokens", removed_tokens.saturating_sub(token_count));
    }

    /// Get all messages for API call
    pub fn get_messages(&self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        // Add system prompt
        if !self.system_prompt.is_empty() {
            messages.push(ChatMessage::system(&self.system_prompt));
        }

        // Add summaries as context
        for summary in &self.summaries {
            messages.push(ChatMessage::system(format!(
                "[Previous Conversation Summary]\n{}\nKey facts: {}",
                summary.summary,
                summary.key_facts.join(", ")
            )));
        }

        // Add conversation turns
        for turn in &self.turns {
            messages.push(turn.to_chat_message());
        }

        messages
    }

    /// Get conversation statistics
    pub fn stats(&self) -> ContextStats {
        ContextStats {
            total_turns: self.turns.len(),
            total_summaries: self.summaries.len(),
            estimated_tokens: self.total_tokens,
            max_tokens: self.max_tokens,
            utilization: self.total_tokens as f32 / self.max_tokens as f32,
            masked_turns: self.turns.iter().filter(|t| t.is_masked).count(),
            summarized_turns: self.turns.iter().filter(|t| t.is_summarized).count(),
        }
    }

    /// Clear all context
    pub fn clear(&mut self) {
        self.turns.clear();
        self.summaries.clear();
        self.total_tokens = 0;
        info!("Context cleared");
    }

    /// Check if context is getting full
    pub fn is_near_capacity(&self) -> bool {
        self.total_tokens > (self.max_tokens as f32 * 0.7) as usize
    }

    /// Get token utilization percentage
    pub fn utilization(&self) -> f32 {
        (self.total_tokens as f32 / self.max_tokens as f32) * 100.0
    }
}

// ============================================================================
// Statistics
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub total_turns: usize,
    pub total_summaries: usize,
    pub estimated_tokens: usize,
    pub max_tokens: usize,
    pub utilization: f32,
    pub masked_turns: usize,
    pub summarized_turns: usize,
}

impl std::fmt::Display for ContextStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Context: {}/{} tokens ({:.1}%), {} turns, {} masked, {} summarized",
            self.estimated_tokens,
            self.max_tokens,
            self.utilization * 100.0,
            self.total_turns,
            self.masked_turns,
            self.summarized_turns
        )
    }
}

// ============================================================================
// Summarization Prompt Generator
// ============================================================================

/// Generate a prompt for LLM-based summarization
pub fn generate_summarization_prompt(turns: &[&ConversationTurn]) -> String {
    let conversation: String = turns
        .iter()
        .map(|t| {
            let role = match t.role {
                Role::System => "System",
                Role::User => "User",
                Role::Assistant => "Assistant",
                Role::Tool => "Tool",
            };
            format!("{}: {}", role, t.content.as_deref().unwrap_or("[tool call]"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"Summarize the following conversation segment concisely. 
Extract key facts, decisions made, and important context.
Format your response as JSON with these fields:
- summary: A 2-3 sentence summary of the conversation
- key_facts: An array of important facts, names, or decisions mentioned

Conversation:
{}

Respond only with valid JSON."#,
        conversation
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager() {
        let mut manager = ContextManager::new()
            .with_strategy(ContextStrategy::SlidingWindow)
            .with_max_tokens(1000);

        // Add some turns
        for i in 0..20 {
            manager.add_turn(ConversationTurn::user(format!("Message {}", i)));
            manager.add_turn(ConversationTurn::assistant(format!("Response {}", i)));
        }

        let stats = manager.stats();
        assert!(stats.total_turns > 0);
        println!("{}", stats);
    }

    #[test]
    fn test_sliding_window() {
        let mut manager = ContextManager::new()
            .with_strategy(ContextStrategy::SlidingWindow)
            .with_max_tokens(500)
            .with_system_prompt("You are helpful.");

        // Add many turns to trigger compression
        for i in 0..30 {
            let long_content = "x".repeat(100);
            manager.add_turn(ConversationTurn::user(format!("{}: {}", i, long_content)));
        }

        let stats = manager.stats();
        println!("After compression: {}", stats);
        assert!(stats.masked_turns > 0);
    }

    #[test]
    fn test_token_estimation() {
        let mut turn = ConversationTurn::user("Hello world, this is a test message.");
        let tokens = turn.estimate_tokens();
        println!("Estimated tokens: {}", tokens);
        assert!(tokens > 0);
    }
}
