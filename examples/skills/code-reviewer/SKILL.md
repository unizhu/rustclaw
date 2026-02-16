---
name: code-reviewer
description: Reviews code for best practices, security issues, and performance. Use when user asks to review, analyze, or audit code.
---

# Code Reviewer

Reviews code following industry best practices and security guidelines.

## Instructions

When reviewing code:

1. **Security Analysis**
   - Check for SQL injection vulnerabilities
   - Identify XSS risks
   - Verify proper authentication/authorization
   - Look for sensitive data exposure

2. **Code Quality**
   - Assess code readability and maintainability
   - Check for proper error handling
   - Identify potential null pointer exceptions
   - Evaluate naming conventions

3. **Performance**
   - Identify potential performance bottlenecks
   - Check for inefficient algorithms
   - Look for memory leaks

4. **Best Practices**
   - Verify SOLID principles
   - Check for code duplication (DRY)
   - Assess test coverage needs

## Output Format

```markdown
## Code Review Summary

### Critical Issues
- [List critical security or functionality issues]

### Warnings
- [List potential problems or code smells]

### Suggestions
- [List improvement recommendations]

### Positive Observations
- [Highlight good practices found]
```

## Examples

**Input:** Review this authentication function
**Output:** Detailed analysis covering security vulnerabilities, error handling, and improvement suggestions.
