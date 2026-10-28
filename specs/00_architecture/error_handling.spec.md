# SPECIFICATION: UNIFIED ERROR HANDLING TAXONOMY
Spec ID: `[SPEC-ERR-001]`
Status: APPROVED
Domain: Error Handling & Fault Isolation

---

## 1. Overview
Defines typed error hierarchies across all workspace crates using `thiserror`. Forbids `unwrap()` and `expect()` in production code.

---

## 2. Requirements `[REQ-ERR-001]`
1. Every crate must define domain-specific error enums using `thiserror::Error`.
2. Error variants must provide descriptive, localized-friendly error messages.
3. System/OS API failures must wrap underlying OS error codes without losing provenance.
4. Production functions returning fallible operations must return `Result<T, TypedError>`.

---

## 3. Rust Contracts `[CONTRACT-ERR-001]`
```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Invalid frame dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("Buffer overflow: capacity {capacity} exceeded")]
    BufferOverflow { capacity: usize },

    #[error("Audio format mismatch: expected {expected_sample_rate}Hz, got {actual_sample_rate}Hz")]
    AudioFormatMismatch { expected_sample_rate: u32, actual_sample_rate: u32 },

    #[error("Configuration I/O error: {0}")]
    ConfigIo(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Target monitor or window not found: {0}")]
    TargetNotFound(String),

    #[error("Permission denied or access restricted by operating system")]
    PermissionDenied,

    #[error("DirectX / GPU device lost or reset")]
    DeviceLost,

    #[error("Platform unsupported: {0}")]
    UnsupportedPlatform(String),

    #[error("Internal capture error: {0}")]
    Internal(String),
}
```

---

## 4. Non-Functional Requirements `[NFR-ERR-001]`
1. Zero panics in production: zero `unwrap()` or `expect()`.
2. Error conversions (`#[from]`) must be zero-cost until an error actually occurs.

---

## 5. Verification Criteria `[TEST-ERR-001]`
1. Automated CI check grepping for `.unwrap()` and `.expect()` in non-test source files.
2. Unit tests validating that all error enums format properly with `to_string()`.
