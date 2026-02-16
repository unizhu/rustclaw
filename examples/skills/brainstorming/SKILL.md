---
name: brainstorming
description: Explores user intent, requirements and design before implementation. Use this before any creative work - creating features, building components, or modifying behavior.
---

# Brainstorming

You MUST use this skill before any creative work to explore user intent, requirements and design before implementation.

## When to Use

Use this skill when:
- Creating new features
- Building new components
- Adding new functionality
- Modifying existing behavior
- Starting any non-trivial implementation

## Process

Follow this structured approach:

### 1. Understand the Request

Ask clarifying questions to understand:
- What is the core problem being solved?
- Who are the users/stakeholders?
- What are the success criteria?
- Are there any constraints (technical, time, resources)?
- What is the scope?

### 2. Explore Requirements

Gather detailed requirements:
- **Functional requirements**: What should it do?
- **Non-functional requirements**: Performance, security, usability
- **Edge cases**: What could go wrong?
- **Integration points**: How does it fit with existing systems?

### 3. Design Exploration

Propose multiple approaches:
- Option 1: [Approach A]
  - Pros: ...
  - Cons: ...
  - Complexity: Low/Medium/High
  
- Option 2: [Approach B]
  - Pros: ...
  - Cons: ...
  - Complexity: Low/Medium/High

### 4. Get User Feedback

Present options to user and ask:
- Which approach aligns best with your goals?
- Are there trade-offs you're willing to accept?
- Do you have preferences or constraints?

### 5. Create Action Plan

Once approach is agreed:
- Break down into implementation steps
- Identify dependencies
- Estimate effort
- Define testing strategy

## Example

**User Request:** "Add user authentication"

**Your Response:**
Before implementing, let me understand:

1. **Authentication Method**: What type of auth do you need?
   - Email/password
   - OAuth (Google, GitHub, etc.)
   - API keys
   - Other?

2. **Session Management**: How should sessions be handled?
   - JWT tokens
   - Session cookies
   - Server-side sessions

3. **Security Requirements**: 
   - MFA support needed?
   - Password strength requirements?
   - Session timeout preferences?

4. **User Experience**:
   - Remember me functionality?
   - Password reset flow?
   - Email verification?

Let me propose two approaches:

**Option A: JWT-based stateless auth**
- Pros: Scalable, works well with microservices
- Cons: Token revocation is complex

**Option B: Session-based auth**
- Pros: Simple to implement, easy to revoke
- Cons: Requires server state, harder to scale

Which approach interests you? Or would you like to explore other options?
