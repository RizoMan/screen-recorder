---
name: sdd-workflow
description: Standard Operating Procedure for Spec-Driven Development (SDD) lifecycle, traceability taxonomy, contract drafting, and quality governance in Rust.
---

# Spec-Driven Development (SDD) Workflow Skill

This skill defines the operational procedure for developing features, refactoring, and maintaining the Screen Recorder codebase under strict SDD rules.

## Core Directives
1. **Never write code before specification**: Every PR, feature, trait, or public API must start as a specification in `specs/`.
2. **Tag Everything**: Use the canonical tags:
   - `[SPEC-<MODULE>-<NUM>]`
   - `[REQ-<MODULE>-<NUM>]`
   - `[CONTRACT-<MODULE>-<NUM>]`
   - `[NFR-<MODULE>-<NUM>]`
   - `[TEST-<MODULE>-<NUM>]`
3. **No Unanchored Code**: Code without an originating requirement tag is dead/orphan code and must be removed.

## Step-by-Step SOP

### Phase 1: Spec Drafting (`specs/`)
1. Create a markdown spec file in `specs/<category>/<feature_name>.spec.md`.
2. Add document header with `[SPEC-<MODULE>-<NUM>]`.
3. Detail atomic requirements `[REQ-<MODULE>-<NUM>]`.
4. Detail trait signatures and typed errors `[CONTRACT-<MODULE>-<NUM>]`.
5. Specify performance, memory, and latency budgets `[NFR-<MODULE>-<NUM>]`.
6. Define manual and automated test criteria `[TEST-<MODULE>-<NUM>]`.

### Phase 2: Contract-First Definition
1. Place traits and data types in `rec-core` or the orchestrator `rec-engine`.
2. Define typed errors using `thiserror`.
3. Annotate public items with Rustdoc including `# Arguments`, `# Returns`, `# Errors`, `# Examples`.
4. Enforce `#![forbid(unsafe_code)]` in pure core/engine crates.

### Phase 3: TDD Implementation
1. Write red unit tests referencing `[TEST-...]`.
2. Implement backend or engine logic satisfying `[REQ-...]`.
3. Enforce 0 `unwrap()` and 0 `expect()` in production.
4. Ensure RAII cleanup by implementing `Drop` on native resources.

### Phase 4: Localization & Validation
1. If any user-facing message is added, add it immediately to `rec_core::i18n::I18nKey` with both English and Spanish translations.
2. Run `cargo test --workspace` to ensure parity tests and contract tests pass.
3. Run `cargo doc --workspace --no-deps` ensuring 0 warnings.
4. Run `cargo clippy --workspace --all-targets -- -D warnings`.

### Phase 5: Versioning and Packaging
1. Perform semantic version bump if needed.
2. Build installer artifacts using packaging scripts.
