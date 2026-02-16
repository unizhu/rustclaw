#!/bin/bash
cd "$(dirname "$0")"
git add -A
git commit -m "fix: Pass API key to OpenAI client for OpenRouter support

- Add api_key field to Provider enum
- Update create_client() to pass API key to OpenAIConfig
- Change OpenAIConfig.api_key from String to Option<String>
- Add openai_with_api_key() and openai_full() constructors
- Fixes 502 error when using OpenRouter with custom base URL"
git push
