use anyhow::Result;
use async_openai::{Client, config::OpenAIConfig};
use rustclaw_types::{Message, MessageContent, Provider as ProviderType};
use tracing::{info, warn};

/// Provider service for interacting with LLM providers
pub struct ProviderService {
    provider: ProviderType,
}

impl ProviderService {
    /// Create a new provider service
    pub fn new(provider: ProviderType) -> Self {
        info!("Provider service initialized with: {:?}", provider);
        Self { provider }
    }

    /// Complete a conversation using the configured provider
    pub async fn complete(&self, messages: &[Message], prompt: &str) -> Result<String> {
        match &self.provider {
            ProviderType::OpenAI { model } => self.complete_openai(model, messages, prompt).await,
            ProviderType::Ollama { model, base_url } => {
                self.complete_ollama(model, base_url, messages, prompt).await
            }
        }
    }

    /// Complete using OpenAI
    async fn complete_openai(
        &self,
        model: &str,
        messages: &[Message],
        prompt: &str,
    ) -> Result<String> {
        use async_openai::types::{
            ChatCompletionRequestSystemMessageArgs,
            ChatCompletionRequestUserMessageArgs,
            CreateChatCompletionRequestArgs,
        };

        let client = Client::new();

        // Build conversation history
        let mut chat_messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a helpful AI assistant.")
                .build()?
                .into(),
        ];

        // Add previous messages
        for msg in messages {
            let content = match &msg.content {
                MessageContent::Text(text) => text,
            };
            chat_messages.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(content.clone())
                    .build()?
                    .into(),
            );
        }

        // Add current prompt
        chat_messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(prompt.to_string())
                .build()?
                .into(),
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(chat_messages)
            .build()?;

        let response = client.chat().create(request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone().unwrap_or_default())
        } else {
            warn!("OpenAI returned no choices");
            Ok("I couldn't generate a response.".to_string())
        }
    }

    /// Complete using Ollama
    async fn complete_ollama(
        &self,
        model: &str,
        base_url: &str,
        messages: &[Message],
        prompt: &str,
    ) -> Result<String> {
        use async_openai::types::{
            ChatCompletionRequestSystemMessageArgs,
            ChatCompletionRequestUserMessageArgs,
            CreateChatCompletionRequestArgs,
        };

        // Create custom config for Ollama
        let config = OpenAIConfig::new().with_api_base(base_url);
        let client = Client::with_config(config);

        // Build conversation history
        let mut chat_messages = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a helpful AI assistant.")
                .build()?
                .into(),
        ];

        // Add previous messages
        for msg in messages {
            let content = match &msg.content {
                MessageContent::Text(text) => text,
            };
            chat_messages.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(content.clone())
                    .build()?
                    .into(),
            );
        }

        // Add current prompt
        chat_messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(prompt.to_string())
                .build()?
                .into(),
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(chat_messages)
            .build()?;

        let response = client.chat().create(request).await?;

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone().unwrap_or_default())
        } else {
            warn!("Ollama returned no choices");
            Ok("I couldn't generate a response.".to_string())
        }
    }
}
