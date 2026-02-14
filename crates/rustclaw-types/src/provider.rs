use serde::{Deserialize, Serialize};

/// LLM Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Provider {
    OpenAI {
        model: String,
    },
    Ollama {
        model: String,
        base_url: String,
    },
}

impl Provider {
    pub fn openai(model: impl Into<String>) -> Self {
        Self::OpenAI {
            model: model.into(),
        }
    }

    pub fn ollama(model: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::Ollama {
            model: model.into(),
            base_url: base_url.into(),
        }
    }

    pub fn default_openai() -> Self {
        Self::openai("gpt-4-turbo-preview")
    }

    pub fn default_ollama() -> Self {
        Self::ollama("llama2", "http://localhost:11434")
    }
}

impl Default for Provider {
    fn default() -> Self {
        Self::default_openai()
    }
}
