# Repository Guidelines

## Project Structure & Module Organization
- `client/`: Rust AO/WASM contract wrapper. `src/domain` holds core types, `src/service` orchestrates processes, `di.rs` wires dependencies, `lib.rs` exports entrypoints.
- `docs/` and `discuss/`: architecture notes and design discussions—sync guideline updates with these.
- `ao/`: scripts and manifests for shipping the contract; treat as source of truth for environment settings.
- `mcp/`: auxiliary Model Context Protocol servers (Context7); see `mcp/context7/Makefile`.
- `test/`: integration fixtures; use for scenario harnesses instead of mixing into library modules.
- `images/` hosts diagrams referenced from docs; keep binary assets out of `client/`.

## Build, Test, and Development Commands
- `make check` / `cargo check`: fast type checking; run before pushing.
- `make fmt`: enforces repo-wide `rustfmt` config.
- `make clippy` and `make clippy-strict`: default lint gate and hardened audit profile (warnings -> errors). Use strict profile before tagging releases.
- `make test` / `cargo test --all-targets`: executes Rust unit/integration suites.
- `make start-context7` / `make stop-context7`: spin up/down the local MCP server when validating agent flows.

## Coding Style & Naming Conventions
- Rust edition 2021; spaces only, 4-space indent, 100-char max width.
- Prefer `snake_case` for functions/modules, `UpperCamelCase` for types, and clear enum variant names; avoid placeholders listed in `clippy.toml`.
- Keep constructor wiring in `di.rs`; domain logic stays in `domain/`, external integrations in `service/`.
- Always run `cargo fmt --all` and address any `cargo clippy` diagnostics before requesting review.

## Testing Guidelines
- Write focused `#[cfg(test)]` modules beside the code they exercise; place cross-cutting scenarios under `test/`.
- When adding async workflows, include deterministic state seeds and assert both success and failure branches.
- Aim to cover holder selection, kFrag validation, and AO message handlers; add regression tests for every security-sensitive path.
- Capture command output when relevant (e.g., `cargo test -- --nocapture`) in PR notes.

## Commit & Pull Request Guidelines
- Use descriptive, present-tense subjects (e.g., "Add AO message handler for kFrag intake"); add multi-line bodies for rationale or doc references. English or Japanese is acceptable—stay consistent within a commit.
- One logical change per commit; update docs and `ao/` artefacts alongside code.
- PRs must include: summary of behavior changes, linked issues/tasks, steps to reproduce/test evidence, and screenshots or logs for observable flows.
- Request review only after `make check fmt clippy test` succeed; mention any skipped checks explicitly.

## Security & Configuration Tips
- Never commit secrets; reference them via environment variables documented in `ao/`.
- Pin toolchains with `rust-toolchain.toml`; run `rustup update` cautiously and document version bumps.
- Prefer deterministic randomness sources and mark unsafe code with clear justification and tests.
