---
name: integration-testing
description: Creates and runs integration tests for component interactions. Use when testing multiple modules working together.
---

# Integration Testing

Tests how multiple components work together.

## Instructions

When creating integration tests:

1. **Identify integration points**
   - API endpoints
   - Database interactions
   - Service communication
   - External dependencies

2. **Set up test environment**
   - Use test databases
   - Mock external services
   - Clean state between tests

3. **Write test scenarios**
   ```rust
   #[tokio::test]
   async fn test_user_registration_flow() {
       // Setup
       let app = spawn_app().await;
       
       // Act
       let response = app.post_register(json!({
           "email": "test@example.com",
           "password": "password123"
       })).await;
       
       // Assert
       assert!(response.status().is_success());
   }
   ```

## Best Practices

- Test realistic user scenarios
- Use actual databases when possible
- Clean up resources after tests
- Run in isolated environments
