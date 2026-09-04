# Rust Telex reference implementation

This crate independently implements Draft 0 `telex.aes` parsing,
canonicalization, and AES profile validation.

The library has no runtime dependencies. `serde_json` is a test-only dependency
used to load the language-neutral vectors from `conformance/telex/v0`.

Run the Rust vector suite with:

```bash
cargo test --locked --manifest-path implementations/rust/Cargo.toml
```

The Rust and JavaScript implementations intentionally do not call each other or
share codec source. Their common authority is the Telex draft and its portable
vectors.
