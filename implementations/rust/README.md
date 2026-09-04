# Rust Telex reference implementation

This crate independently implements Draft 0 `telex.aes` parsing,
canonicalization, and AES profile validation.

The default stream contains body events only. The optional
`aeon.document.v0` projection adds flat `header` records in a disjoint address
plane; it remains independent of the complete/partial AES profile selection.
Optional source provenance is record-local: `origin` may stand alone, while
`span` requires a canonical SHA-256 origin.

The library has no runtime dependencies. `serde_json` is a test-only dependency
used to load the language-neutral vectors from `conformance/telex/v0`.

Run the Rust vector suite with:

```bash
cargo test --locked --manifest-path implementations/rust/Cargo.toml
```

The Rust and JavaScript implementations intentionally do not call each other or
share codec source. Their common authorities are the transport-neutral portable
AES event contract, the Telex encoding draft, and the shared vectors.
