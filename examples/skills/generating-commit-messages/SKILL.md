---
name: generating-commit-messages
description: Generates clear, descriptive commit messages by analyzing git diffs. Use when writing commit messages or reviewing staged changes.
---

# Generating Commit Messages

Analyzes git changes and generates conventional commit messages.

## Instructions

1. Run `git diff --staged` to see changes
2. Analyze the changes and suggest a commit message with:
   - **Summary**: Under 50 characters, present tense
   - **Description**: What changed and why
   - **Affected components**: List modified areas

## Format

```
<type>(<scope>): <summary>

<description>
```

## Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Adding or modifying tests
- `chore`: Maintenance tasks
- `perf`: Performance improvements
- `style`: Code style changes (formatting, etc.)

## Best Practices

- Use present tense ("add feature" not "added feature")
- Explain WHAT and WHY, not HOW
- Reference issue numbers when relevant
- Keep first line under 50 characters
- Separate subject from body with blank line
- Use imperative mood

## Examples

**Input:** Added user authentication with JWT tokens
**Output:**
```
feat(auth): implement JWT-based authentication

Add login endpoint with token generation and validation middleware.
Includes token refresh logic and secure storage.

Closes #123
```

**Input:** Fixed bug where dates displayed incorrectly
**Output:**
```
fix(reports): correct date formatting in timezone conversion

Use UTC timestamps consistently across report generation.
Previously used local time which caused discrepancies for users
in different timezones.
```
