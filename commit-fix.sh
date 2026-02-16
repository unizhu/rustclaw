#!/bin/bash
cd "$(dirname "$0")"
git add -A
git commit -m "fix: Clean up Provider type and service initialization

- Remove duplicate provider.rs file (was unused)
- Simplify Provider constructor matching in service.rs
- Update DEFAULT_CONFIG to not include empty api_key/base_url
- Fix filter logic for Option<String> fields"
git push
