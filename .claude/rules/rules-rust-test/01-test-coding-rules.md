# Global Test Coding Rules

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
