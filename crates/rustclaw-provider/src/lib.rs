//! RustClaw Provider Service
//!
//! This module provides a unified interface for interacting with LLM providers
//! (OpenAI, Ollama, etc.) with full support for tool calling.

pub mod context;

use anyhow::{anyhow, Result};
use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatChoice, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTool, ChatCompletionToolArgs,
    ChatCompletionToolType, CreateChatCompletionRequestArgs,
};
use async_openai::Client;
use rustclaw_types::{CompletionResponse, Message, MessageContent, Provider, Tool, ToolCall, ToolResult};
use std::collections::HashMap;
use tracing::{debug, info, warn};

// ============================================================================
// Tool Registry
// ============================================================================

/// A function that can be called by the model
pub trait ToolFunction: Send + Sync {
    /// Get the tool definition
    fn definition(&self) -> Tool;
    
    /// Execute the tool with the given arguments
    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value>;
}

/// Registry of available tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn ToolFunction>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn ToolFunction>) {
        let name = tool.definition().function.name.clone();
        info!("Registering tool: {}", name);
        self.tools.insert(name, tool);
    }

    /// Get all tool definitions for the API
    pub fn get_tools(&self) -> Vec<Tool> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Check if we have any tools
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Execute a tool by name
    pub fn execute(&self, name: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        self.tools
            .get(name)
            .ok_or_else(|| anyhow!("Unknown tool: {}", name))?
            .execute(args)
    }

    /// Execute a tool call
    pub fn execute_call(&self, call: &ToolCall) -> ToolResult {
        match serde_json::from_str(&call.function.arguments) {
            Ok(args) => match self.execute(&call.function.name, args) {
                Ok(result) => ToolResult::from_json(call.id.clone(), &result),
                Err(e) => ToolResult::new(
                    call.id.clone(),
                    serde_json::json!({"error": e.to_string()}).to_string(),
                ),
            },
            Err(e) => ToolResult::new(
                call.id.clone(),
                serde_json::json!({"error": format!("Failed to parse arguments: {}", e)}).to_string(),
            ),
        }
    }
}

// ============================================================================
// Provider Service
// ============================================================================

/// Provider service for interacting with LLM providers
pub struct ProviderService {
    provider: Provider,
    tools: ToolRegistry,
    system_prompt: String,
    max_tool_iterations: usize,
}

impl ProviderService {
    /// Create a new provider service without tools
    pub fn new(provider: Provider) -> Self {
        Self {
            provider,
            tools: ToolRegistry::new(),
            system_prompt: "You are a helpful AI assistant.".to_string(),
            max_tool_iterations: 5,
        }
    }

    /// Create a new provider service with tools
    pub fn with_tools(provider: Provider, tools: ToolRegistry) -> Self {
        Self {
            provider,
            tools,
            system_prompt: "You are a helpful AI assistant.".to_string(),
            max_tool_iterations: 5,
        }
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Set the maximum number of tool iterations
    pub fn with_max_tool_iterations(mut self, max: usize) -> Self {
        self.max_tool_iterations = max;
        self
    }

    /// Get a reference to the tool registry
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Get a mutable reference to the tool registry
    pub fn tools_mut(&mut self) -> &mut ToolRegistry {
        &mut self.tools
    }

    /// Complete a conversation (simple text-only interface)
    pub async fn complete(&self, messages: &[Message], prompt: &str) -> Result<String> {
        let response = self.complete_with_tools(messages, prompt, None).await?;
        Ok(response.content.unwrap_or_default())
    }

    /// Complete a conversation with tool calling support
    pub async fn complete_with_tools(
        &self,
        messages: &[Message],
        prompt: &str,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<CompletionResponse> {
        let client = self.create_client()?;
        
        // Build chat messages
        let chat_messages = self.build_messages(messages, prompt, tool_results)?;
        
        // Build request
        let request = if !self.tools.is_empty() {
            let tools = self.build_tools_for_api()?;
            debug!("Sending {} tools to API", tools.len());
            CreateChatCompletionRequestArgs::default()
                .model(self.model_name())
                .messages(chat_messages)
                .tools(tools)
                .build()?
        } else {
            CreateChatCompletionRequestArgs::default()
                .model(self.model_name())
                .messages(chat_messages)
                .build()?
        };

        debug!("Sending completion request to {}", self.provider_name());

        let response = client.chat().create(request).await?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No choices returned from API"))?;

        self.parse_response(choice)
    }

    /// Execute tool calls and return results
    pub async fn execute_tool_calls(&self, tool_calls: &[ToolCall]) -> Vec<ToolResult> {
        tool_calls.iter().map(|call| self.tools.execute_call(call)).collect()
    }

    /// Complete with automatic tool execution using configured max iterations
    pub async fn complete_agentic_default(
        &self,
        messages: &[Message],
        prompt: &str,
    ) -> Result<String> {
        self.complete_agentic(messages, prompt, self.max_tool_iterations).await
    }

    /// Complete with automatic tool execution (agentic loop)
    pub async fn complete_agentic(
        &self,
        messages: &[Message],
        prompt: &str,
        max_iterations: usize,
    ) -> Result<String> {
        let current_messages = messages.to_vec();
        let mut current_prompt = prompt.to_string();
        let mut tool_results = None;

        for iteration in 0..max_iterations {
            debug!("Agentic iteration {} of {}", iteration + 1, max_iterations);
            
            let response = self
                .complete_with_tools(&current_messages, &current_prompt, tool_results.take())
                .await?;

            if !response.has_tool_calls() {
                return Ok(response.content.unwrap_or_default());
            }

            // Execute tool calls
            let results = self.execute_tool_calls(&response.tool_calls).await;
            
            // Log tool executions
            for (call, result) in response.tool_calls.iter().zip(results.iter()) {
                info!(
                    "Tool executed: {} -> {}",
                    call.function.name,
                    if result.output.len() > 100 {
                        &result.output[..100]
                    } else {
                        &result.output
                    }
                );
            }

            // Prepare for next iteration
            tool_results = Some(results);
            
            // Clear prompt for subsequent iterations (already in context)
            if iteration > 0 {
                current_prompt = String::new();
            }
        }

        warn!("Max tool iterations reached");
        Ok("I've reached the maximum number of tool operations. Please let me know if you need me to continue.".to_string())
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    fn create_client(&self) -> Result<Client<OpenAIConfig>> {
        let base_url = match &self.provider {
            Provider::OpenAI { base_url, .. } => base_url.clone(),
            Provider::Ollama { base_url, .. } => Some(base_url.clone()),
        };

        let client = if let Some(url) = base_url {
            debug!("Using API base URL: {}", url);
            Client::with_config(OpenAIConfig::new().with_api_base(url))
        } else {
            Client::new()
        };

        Ok(client)
    }

    fn model_name(&self) -> &str {
        match &self.provider {
            Provider::OpenAI { model, .. } => model,
            Provider::Ollama { model, .. } => model,
        }
    }

    fn provider_name(&self) -> &str {
        match &self.provider {
            Provider::OpenAI { .. } => "OpenAI",
            Provider::Ollama { .. } => "Ollama",
        }
    }

    fn build_messages(
        &self,
        messages: &[Message],
        prompt: &str,
        tool_results: Option<Vec<ToolResult>>,
    ) -> Result<Vec<async_openai::types::ChatCompletionRequestMessage>> {
        let mut chat_messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(self.system_prompt.clone())
                .build()?
                .into(),
        ];

        // Add conversation history
        for msg in messages {
            let content = match &msg.content {
                MessageContent::Text(text) => text.clone(),
            };
            chat_messages.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(content)
                    .build()?
                    .into(),
            );
        }

        // Add current prompt if provided
        if !prompt.is_empty() {
            chat_messages.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt)
                    .build()?
                    .into(),
            );
        }

        // Add tool results if provided
        if let Some(results) = tool_results {
            for result in results {
                chat_messages.push(
                    async_openai::types::ChatCompletionRequestToolMessageArgs::default()
                        .content(result.output.clone())
                        .tool_call_id(result.tool_call_id.clone())
                        .build()?
                        .into(),
                );
            }
        }

        Ok(chat_messages)
    }

    fn build_tools_for_api(&self) -> Result<Vec<ChatCompletionTool>> {
        self.tools
            .get_tools()
            .into_iter()
            .map(|tool| {
                Ok(ChatCompletionToolArgs::default()
                    .r#type(ChatCompletionToolType::Function)
                    .function(async_openai::types::FunctionObject {
                        name: tool.function.name,
                        description: Some(tool.function.description),
                        parameters: Some(tool.function.parameters),
                        strict: tool.function.strict,
                    })
                    .build()?)
            })
            .collect()
    }

    fn parse_response(&self, choice: &ChatChoice) -> Result<CompletionResponse> {
        let message = &choice.message;
        
        let content = message.content.clone();
        
        let tool_calls: Vec<ToolCall> = message
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        call_type: "function".to_string(),
                        function: rustclaw_types::FunctionCall {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();

        let finish_reason = choice
            .finish_reason
            .as_ref()
            .map(|r| format!("{:?}", r).to_lowercase())
            .unwrap_or_else(|| "unknown".to_string());

        debug!(
            "Response parsed: content={}, tool_calls={}, finish_reason={}",
            content.as_deref().unwrap_or("none"),
            tool_calls.len(),
            finish_reason
        );

        Ok(CompletionResponse {
            content,
            tool_calls,
            finish_reason,
        })
    }
}

// ============================================================================
// Built-in Example Tools
// ============================================================================

/// A simple echo tool for testing
pub struct EchoTool;

impl ToolFunction for EchoTool {
    fn definition(&self) -> Tool {
        Tool::function(
            "echo",
            "Echo back the input message",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo back"
                    }
                },
                "required": ["message"],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let message = args
            .get("message")
            .and_then(|m| m.as_str())
            .ok_or_else(|| anyhow!("Missing 'message' argument"))?;
        Ok(serde_json::json!({ "echoed": message }))
    }
}

/// Current time tool
pub struct CurrentTimeTool;

impl ToolFunction for CurrentTimeTool {
    fn definition(&self) -> Tool {
        Tool::function(
            "get_current_time",
            "Get the current date and time",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "Optional timezone (e.g., 'UTC', 'America/New_York')"
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let _tz = args.get("timezone").and_then(|t| t.as_str()).unwrap_or("UTC");
        let now = chrono::Utc::now();
        Ok(serde_json::json!({
            "time": now.to_rfc3339(),
            "timezone": "UTC"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(EchoTool));
        
        let tools = registry.get_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "echo");
    }

    #[test]
    fn test_echo_tool() {
        let tool = EchoTool;
        let result = tool.execute(serde_json::json!({"message": "hello"})).unwrap();
        assert_eq!(result["echoed"], "hello");
    }

    #[test]
    fn test_time_tool() {
        let tool = CurrentTimeTool;
        let result = tool.execute(serde_json::json!({})).unwrap();
        assert!(result.get("time").is_some());
    }
}
