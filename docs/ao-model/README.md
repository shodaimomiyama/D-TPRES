# AO Model Documentation

This directory contains technical documents about how AO (Arweave Operations) platform integrates with FORMIX.

## Contents

### Core Documents

- **[AO Process Model and Stateless Execution](./ao_process_model.md)**
  Comprehensive guide covering AO's stateless execution model, Compute Units (CU), state management strategies, and CosmWasm AO-specific patterns. Essential reading for developers working with AO processes.

### Key Concepts Covered

1. **Stateless Execution Model**
   - No memory persistence between messages
   - Message-driven architecture
   - Compute Unit distribution

2. **Process Management**
   - Process spawning with WASM modules
   - Module loading from Arweave
   - Multi-role process design

3. **State Management**
   - Explicit state persistence to Arweave
   - ProcessEntity as global state
   - Lazy loading patterns
   - Event sourcing and KV storage patterns (CosmWasm AO)

4. **Performance Optimization**
   - Batch operations
   - Message-scope caching
   - State size management

## External Resources

- [AO Network](https://ao.arweave.net/) - Official AO platform
- [Arweave](https://www.arweave.org/) - Permanent storage layer
- [FORMIX Architecture](../PRD.md) - Overall system architecture

## Related Documents

- [Domain Layer Overview](../domain/domain_overview.md) - Domain entities and AO integration
- [Repository Implementations](../domain/infrastructure/repository_implementations.md) - Arweave storage patterns
- [Service Overview](../service/services_overview.md) - AO process implementations
