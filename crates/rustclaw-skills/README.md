# RustClaw Skills System

Production-ready skills system implementing **progressive disclosure architecture** based on 2025-2026 AI agent research and best practices from Claude Code, OpenAI Codex, Anthropic, and Spring AI.

## 🎯 Overview

Skills are modular capabilities that extend your AI agent's expertise without bloating the context window. They follow a **two-phase loading strategy**:

- **Phase 1 (Discovery)**: Load only skill names and descriptions at startup
- **Phase 2 (Activation)**: Load full skill content on-demand when task matches

This approach allows you to have dozens of skills available while keeping context lean.

## 🚀 Quick Start

### 1. Create a Skill

Create a directory with a `SKILL.md` file:

```bash
mkdir -p ~/.rustclaw/skills/my-first-skill
```

Create `~/.rustclaw/skills/my-first-skill/SKILL.md`:

```markdown
---
name: my-first-skill
description: Brief description of what this skill does and when to use it.
---

# My First Skill

Instructions for the AI agent go here.

## What to Do
- Step 1
- Step 2
- Step 3

## Examples
Show example inputs and outputs.
```

### 2. Configure Skills Directories

In `rustclaw.toml`:

```toml
[skills]
directories = [
    "~/.rustclaw/skills",           # Personal skills
    "./.rustclaw/skills",           # Project skills
    "./examples/skills"             # Example skills
]
```

### 3. Start RustClaw

Skills are automatically discovered and loaded:

```
INFO Discovered 3 skills
```

The LLM will now see available skills in its system prompt and can activate them when appropriate.

## 📚 Skill Structure

### YAML Frontmatter (Required)

Every `SKILL.md` must start with YAML frontmatter:

```yaml
---
name: skill-name           # Max 64 chars, lowercase/numbers/hyphens only
description: What and when # Max 1024 chars, describes WHAT and WHEN
---
```

**Best Practices**:
- Use **gerund form** for names: `generating-commits`, `reviewing-code`
- Description must include both **WHAT** the skill does and **WHEN** to use it
- Be concise - every token competes for context space

### Skill Body (The Instructions)

After the frontmatter, write instructions for the AI:

```markdown
---
name: code-reviewer
description: Reviews code for best practices. Use when reviewing or analyzing code.
---

# Code Reviewer

## Instructions

When reviewing code:
1. Check for security vulnerabilities
2. Verify best practices
3. Suggest improvements

## Output Format

[Expected output structure]
```

## 🎨 Progressive Disclosure Patterns

### Pattern 1: Quick Start + References

Main skill has basics, detailed info in separate files:

```
my-skill/
├── SKILL.md           # Quick start guide
├── reference/
│   ├── api.md         # Detailed API docs
│   └── examples.md    # Usage examples
└── scripts/
    └── helper.py      # Utility scripts
```

In `SKILL.md`:

```markdown
## Quick Start

Basic usage instructions here.

## Advanced Features

- **API Reference**: See [reference/api.md](reference/api.md)
- **Examples**: See [reference/examples.md](reference/examples.md)
```

### Pattern 2: Domain Organization

For skills covering multiple domains:

```
bigquery-skill/
├── SKILL.md
└── reference/
    ├── finance.md
    ├── sales.md
    └── product.md
```

## 📝 Best Practices (2025-2026 Research)

### 1. Conciseness is King ⭐

**Good** (50 tokens):
```markdown
## Extract PDF text
Use pdfplumber for text extraction:

import pdfplumber
with pdfplumber.open("file.pdf") as pdf:
    text = pdf.pages[0].extract_text()
```

**Bad** (150 tokens):
```markdown
## Extract PDF text
PDF (Portable Document Format) files are a common file format that contains
text, images, and other content. To extract text from a PDF, you'll need to
use a library. There are many libraries available for PDF processing, but we
recommend pdfplumber because it's easy to use...
```

**Rule**: Assume the AI is brilliant. Only add context it doesn't have.

### 2. Set Appropriate Degrees of Freedom

Match instruction specificity to task fragility:

- **LOW FREEDOM**: For fragile operations (database migrations, deployments)
  - Use exact scripts, specific commands
  
- **MEDIUM FREEDOM**: For preferred patterns with some variation (report generation)
  - Use templates with configurable parameters
  
- **HIGH FREEDOM**: For tasks with multiple valid approaches (code review)
  - Use text-based instructions and heuristics

### 3. Use Feedback Loops

For quality-critical operations:

```markdown
## Document Editing Process

1. Make your edits
2. **Validate immediately**: `python validate.py`
3. **If validation fails**:
   - Fix the issues
   - Run validation again
   - Do NOT proceed until validation passes
4. Rebuild the document
```

### 4. Write Discovery-Friendly Descriptions

The description is your skill's elevator pitch. It must answer:
- **WHAT** does this skill do?
- **WHEN** should it be used?

**Good**:
```
description: Extracts text and tables from PDF files. Use when working with PDFs, forms, or document extraction.
```

**Bad**:
```
description: Helps with documents  # Too vague
```

## 🗂️ Skill Locations

### Personal Skills (`~/.rustclaw/skills/`)
- Private to you
- Available everywhere
- Not version controlled

### Project Skills (`./.rustclaw/skills/`)
- Shared with your team
- Checked into git
- Project-specific knowledge

### Plugin Skills
- Bundled with MCP plugins
- Tool-specific expertise
- Activated when plugin is enabled

## 🔍 How It Works

### Phase 1: Discovery (Startup)

```rust
// Scan directories and load metadata only
SkillsRegistry::new()
    .add_directory("~/.rustclaw/skills")
    .discover()?;

// Registry now has all skill names and descriptions
// Total tokens: ~50-100 per skill
```

### Phase 2: Activation (On-Demand)

When user request matches a skill's description:

```rust
// Load full content only when needed
let skill = registry.load_skill("code-reviewer")?;
// Now SKILL.md content is in memory
```

### Phase 3: Execution

AI follows skill instructions, loading referenced files as needed.

## 📋 Example Skills

See `examples/skills/` directory for:

1. **code-reviewer** - Reviews code for best practices
2. **generating-commit-messages** - Creates conventional commit messages
3. **brainstorming** - Explores requirements before implementation

## 🛠️ Advanced Features

### Validation Scripts

Skills can include helper scripts:

```
my-skill/
├── SKILL.md
└── scripts/
    ├── validate.py
    └── analyze.py
```

Reference in skill:

```markdown
## Validation

Run the validation script:
```bash
python scripts/validate.py input.json
```
```

### Templates

Include templates for output structure:

```
report-skill/
├── SKILL.md
└── templates/
    └── report.md
```

## 📊 Performance

- **Startup overhead**: ~50-100 tokens per skill (metadata only)
- **Activation cost**: Full skill content only when matched
- **Scalability**: Can have 50+ skills with minimal context impact

## 🧪 Testing Skills

Create evaluation scenarios:

```json
{
  "skill": "code-reviewer",
  "query": "Review this authentication function",
  "files": ["auth.py"],
  "expected_behavior": [
    "Checks for SQL injection",
    "Identifies authentication flaws",
    "Suggests improvements"
  ]
}
```

## 📖 References

- [Agent Skills Specification](https://agentskills.io)
- [Anthropic Best Practices Guide](https://www.anthropic.com/research/building-effective-agents)
- [Spring AI Agent Skills](https://spring.io/blog/2026/01/13/spring-ai-generic-agent-skills)
- [Mastering Agentic Skills](https://medium.com/spillwave-solutions/mastering-agentic-skills-the-complete-guide-to-building-effective-agent-skills-d3fe57a058f1)

## 🤝 Contributing

To contribute skills:

1. Follow the naming convention (gerund form)
2. Include clear descriptions (WHAT + WHEN)
3. Keep instructions concise
4. Test with real scenarios
5. Add examples

## 📄 License

MIT License - See LICENSE file for details
