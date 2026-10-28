# SPEC-DRIVEN DEVELOPMENT (SDD) PROTOCOL & LIFE CYCLE
ID: `RULE-SDD-001`
Status: MANDATORY / SUPREME CONSTITUTION

---

## 1. Philosophical Axiom
**Code is a derived artifact of specifications.**
No production code, trait, public structure, or typed error may be written without an approved, formal specification residing in `specs/`. Unanchored code is strictly prohibited.

---

## 2. Traceability Tag Taxonomy
Every artifact, specification, commit, and source module must utilize the canonical tag system:

| Tag Prefix | Description | Example | Target Location |
| :--- | :--- | :--- | :--- |
| `[SPEC-<MODULE>-<NUM>]` | Formal specification document identifier | `[SPEC-CORE-001]` | `specs/` markdown headers |
| `[REQ-<MODULE>-<NUM>]` | Atomic, verifiable functional requirement | `[REQ-CORE-001]` | Specification body & docstrings |
| `[CONTRACT-<MODULE>-<NUM>]`| Rust trait, type signature, public API contract | `[CONTRACT-CORE-001]` | Rust traits, structs, enums |
| `[NFR-<MODULE>-<NUM>]` | Non-functional constraint (latency, CPU, memory, RAII) | `[NFR-CORE-001]` | Spec NFR section & benchmarks |
| `[TEST-<MODULE>-<NUM>]` | Automated unit, integration, or property test | `[TEST-CORE-001]` | Test functions in `tests/` or unit tests |

---

## 3. The 5-Step Lifecycle of SDD

```
┌─────────────────────────────────────────────────────────────┐
│ 1. DRAFT SPECIFICATION (specs/*.spec.md)                   │
│    Define Reqs, Contracts, NFRs, Acceptance Criteria       │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. CONTRACT-FIRST DESIGN (crates/*/src/lib.rs & traits)     │
│    Declare Traits, Structs, Enums with Typed Errors        │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. TDD & IMPLEMENTATION (tests/ & src/)                     │
│    Write Red Tests, Implement Green Logic, Refactor         │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. QA, SAFETY & DOC AUDIT                                   │
│    Cargo Check, Cargo Doc (0 warnings), Clippy, RAII Audit │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. PROMOTION & PACKAGING                                    │
│    Semantic Version Bump, Release Notes, Installer Bundling │
└─────────────────────────────────────────────────────────────┘
```

### Step 1: Draft Specification
- File created in `specs/<category>/<topic>.spec.md`.
- Must contain: Title, Metadata, Overview, Functional Requirements (`[REQ-...]`), Rust API Contracts (`[CONTRACT-...]`), Non-Functional Requirements (`[NFR-...]`), Acceptance Criteria & Test Matrix (`[TEST-...]`).

### Step 2: Contract-First Design
- Translate `[CONTRACT-...]` into public Rust traits, types, and typed error enums (`thiserror`).
- Add comprehensive Rustdoc with GitHub Flavored Markdown and doctests.
- Ensure traits obey SOLID principles (especially DIP and ISP).

### Step 3: TDD & Implementation
- Write unit/integration tests tagged with `[TEST-...]`.
- Implement internal logic satisfying the requirements.
- Never use `unwrap()` or `expect()` in production code paths.

### Step 4: QA, Safety & Doc Audit
- Run `cargo test --workspace`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.
- Run `cargo doc --workspace --no-deps`.
- Verify RAII `Drop` implementations for all OS/native resources.

### Step 5: Promotion & Packaging
- Bump version using Semantic Versioning (`MAJOR.MINOR.PATCH`).
- Generate release packages/installers (Inno Setup for Windows, tarball/deb/app bundles).
- Never deliver raw bare executables from `target/`.
