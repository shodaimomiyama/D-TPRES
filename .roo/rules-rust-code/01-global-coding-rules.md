# Global Coding Rules

Basic rules should be applied to the entire project.

## 1. Rust version and Best Practices

**Why?:** Rust is evolving. Using outdated practices or deprecated features can lead to compatibility issues, performance degradation, or security risks.

**Rule:**

- Always check the Rust version in `rust-toolchain.toml`.
- Code according to the best practices of the current Rust version.
- Avoid using deprecated libraries or functions. Check official docs or release notes.

## 2. Import Path Resolution

**Why?:** Consistent import paths improve readability and module dependency management.

**Rule:**

- Import modules using the `mod` keyword in Rust
- Relative imports (`../`, `./`) are generally prohibited.
- Double-check similar domain/path names.
- Ensure there is a blank line between standard/third-party modules and internally defined modules.

### Example (Import Path Resolution)

```rust:src/usecase/*.rs
// OK Example 
use anyhow::Result;
use ethers_core::types::U256;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::{
    adapter::queue::QueueAdapter,
    entity::order::{Order, OrderBatch, OrderDTO},
    repository::order::OrderRepository,
    value::token_pair::TokenPair,
};
```

## 3. Comment Convention

**Why?** Code should be readable by anyone. However, excessive or poorly written comments can be noise.

**Rule:**

- Don't leave comments on import statements, like `// Import xx` or `// 〇〇を追加`.
- Write comments in English
- Don't write comments describing "What this code does", just leave comments showing "Why we do this"
- Avoid redundant comments in the codebase. Delete all comments except those explaining 'why' before completing tasks. Keep comments concise.

### Example (Comment Convention)

```rust
// Bad Case:
// Define Usecase struct
pub struct Usecases {
    pub order: Arc<OrderUsecase>,
}

// Good Case:
// Aggregate usecases so that DI can be simple
pub fn new_usecases(repos: Arc<Repositories>, adapters: Arc<Adapters>) -> Usecases {
    let order_usecase = Arc::new(OrderUsecase::new(
        repos.order.clone(),
        adapters.queue.clone(),
    ));

    Usecases {
        order: order_usecase,
    }
}
```

## 4. Logging

**Why?** Logs are crucial for monitoring and troubleshooting. Structured logging (e.g., JSON) is easier to analyze.

**Rule:**

- Log events using key-value pairs instead of embedding variables directly into formatted strings. This provides context that can be easily parsed.
- Use the field syntax provided by tracing macros (e.g., event!(Level::INFO, key = value, message = "...") or shorthand macros like info!(key = value, "message"))
- Use tracing::span! or the #[tracing::instrument] attribute macro to define logical units of work or contexts.

### Example (avoid Bad case, and apply Good case)

```rust
use tracing::{info, warn, Level};
use uuid::Uuid;

fn process_order(order_id: Uuid, user_id: &str, item_count: u32) {
    // Bad: Unstructured log - harder to parse and filter reliably
    println!("Processing order {} for user {} with {} items", order_id, user_id, item_count);

    // Good: Structured log using tracing
    info!(
        order_id = %order_id,
        user_id,
        item_count,
        "Processing customer order"
    );

    if item_count == 0 {
        // Another structured event
        warn!(order_id = %order_id, "Order received with zero items");
    }

    info!(order_id = %order_id, "Order processed successfully");
}
```

## 5. Cargo version and best practices

**Why?** Rust ecosystem evolves. Using outdated practices or deprecated features can lead to compatibility issues, performance degradation, or security risks.

**Rule:**

- Always check the Cargo and dependencies version in `Cargo.toml` and `rust-toolchain.toml`.
- Code according to the best practices of the current Cargo version.
- Avoid using deprecated libraries or functions. Check official docs or release notes.

### Example

```toml
[toolchain]
channel = "1.86.0"  // check this version
components = ["rustc", "cargo"]
profile = "minimal"
targets = []
```

## 6. When to Edit `src/di.rs`

**Why?:** Dependency Injection (DI) setup is mainly in `src/di.rs`, but each layer (Usecase, Infra) also has its DI container. Usually, adding new features only requires modifying layer-specific containers. Unnecessary edits to `src/di.rs` can cause conflicts or obscure the overall DI structure.

**Rule:**

- Edit `src/di.rs` only for changes affecting the overall DI structure, like adding **new dependency components** (e.g., new DB connection, new external service client).

- Adding new Usecases, Repositories, Controllers typically only requires updates in their respective layer's DI setup (e.g., `src/usecase/usecases.rs`, `src/infrastructure/repositories.rs`), not `di.rs`.

## 7. Entities Definition & Responsibilities

**Why?:** Entities represent core business concepts (DDD). Encapsulating state and behavior within Entities increases cohesion and maintainability.

**Rule:**

- **Identify Entities:** Entities correspond to tables defined in the domain model
- **File per Entity:** Create a separate file for each Entity
- **No External Dependencies:** Entities must be pure domain objects. **Do not add annotations** related to external concerns like JSON (`json:"..."`) or DB persistence (`db:"..."`). These belong to Interface/Infra layers.
- **Encapsulation:** Entity fields **must be private**. Access/modification occurs only through methods.
- **Mandatory Constructor:** **Always provide a constructor** (`NewUser`) responsible for ensuring the Entity is created in a valid state (satisfying invariants).
- **Use Constructor:** **Always use the constructor** to create Entity instances, **except** when reconstructing from persistence in Infra layer Repositories. This guarantees invariants.
- **State Change Methods:** Implement operations that change state as Entity methods. Check domain rules and invariants within these methods.

### Example (Entity Definition)

```rust
use uuid::Uuid;

// NG Case: Public fields violate encapsulation rule.
// Also, annotations like `serde` belong in Infra layer, not Entities.
// #[derive(serde::Serialize, serde::Deserialize)] 
pub struct UserNok {
    pub id: Uuid, // NG: Public field
    pub name: String, // NG: Public field
    // #[serde(rename = "emailAddress")] // Avoid this
    pub email: String, // NG: Public field
}

// OK Case: Private fields ensure encapsulation.
// Access and modification are controlled via methods.
#[derive(Debug, Clone, PartialEq, Eq)] // Basic derives are acceptable
pub struct User {
    id: Uuid, // OK: Private (default)
    name: String, // OK: Private
    email: String, // OK: Private
}

impl User {
    // Constructor to create a new User instance with validation
    pub fn new(id: Uuid, name: String, email: String) -> Result<Self, String> {
        if name.is_empty() {
            return Err("User name cannot be empty".to_string());
        }
        if !email.contains('@') { // Simple email format check
            return Err("Invalid email format".to_string());
        }
        Ok(Self { id, name, email })
    }

    // Getter for the id field
    pub fn id(&self) -> Uuid {
        self.id
    }

    // Getter for the name field
    pub fn name(&self) -> &str {
        &self.name
    }

    // Getter for the email field
    pub fn email(&self) -> &str {
        &self.email
    }

    // Example method to change the email, potentially with validation
    pub fn change_email(&mut self, new_email: String) -> Result<(), String> {
        if !new_email.contains('@') {
            return Err("Invalid email format".to_string());
        }
        self.email = new_email;
        Ok(())
    }
}
```
