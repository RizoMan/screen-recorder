# TESTING, CODE SAFETY, AND RESOURCE LIFECYCLE (RAII)
ID: `RULE-SAFE-001`
Status: MANDATORY / IRONCLAD

---

## 1. Zero Panics / Zero `unwrap()` & `expect()` in Production
- **Rule**: In all production crates (`rec-core`, `rec-engine`, `rec-platform-*`, `rec-app`), calling `.unwrap()` or `.expect()` is **STRICTLY PROHIBITED**.
- **Enforcement**:
  - All errors must be modeled as typed error enums powered by `thiserror::Error`.
  - Propagate errors idiomatically via the `?` operator.
  - Conversions between foreign errors (OS, Win32, PipeWire, I/O) and domain errors must use `#[from]` attributes or explicit `map_err`.
  - `.unwrap()` and `.expect()` are only permitted inside `#[cfg(test)]` modules or test files under `tests/`.

---

## 2. Unsafe Boundaries & FFI Isolation
- **Rule**: `rec-core` and `rec-engine` must enforce `#![forbid(unsafe_code)]`.
- **Platform Crates**: When FFI calls are unavoidable (WinRT, DXGI, Direct3D, CoreAudio, PipeWire, ScreenCaptureKit):
  1. The `unsafe` block must be kept to the absolute minimal number of expressions.
  2. Every single `unsafe` block **MUST** be immediately preceded by a `// SAFETY:` comment explaining:
     - Why the pointer/handle is valid and aligned.
     - Why invariants of the external API are upheld.
     - How lifetime requirements and ownership transfer are respected.
  3. All `unsafe` internals must be wrapped in 100% safe Rust abstraction layers before being exposed to `rec-engine` or `rec-app`.

---

## 3. Mandatory RAII (Resource Acquisition Is Initialization)
- **Rule**: Every system handle, capture session, GPU texture, audio stream, and open file descriptor **MUST** implement the `Drop` trait (or be owned by an existing RAII wrapper).
- **Invariants**:
  - If a process encounters an error or thread panic, the stack unwind must trigger `Drop` to release native resources (e.g. `Release()` on COM/WinRT interfaces, `pw_stream_destroy`, CoreAudio stop).
  - Explicit manual teardown methods like `close()` or `stop()` must be accompanied by safe, idempotent `Drop` implementations that ensure resources are freed if `stop()` was not explicitly called.

---

## 4. Testing Standards and Coverage
- Every functional requirement (`[REQ-...]`) must have at least one corresponding unit or integration test (`[TEST-...]`).
- Tests must cover:
  - Happy path workflows.
  - Error conditions and edge cases (e.g., unplugged audio device, monitor disconnection, resolution resize).
  - Ring buffer overflows, underruns, and PTS jitter.
- Stress testing: ring buffers and channels must be verified against high-frequency throughput without deadlocks or data races.
