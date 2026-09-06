# Rust Telex reference implementation

This crate independently implements the v0 `telex.aes` candidate's parsing,
canonicalization, and AES profile validation.

Its decoded records expose a base-name `datatype`, recursive ordered
`generics`, and ordered tagged `clarifiers`. The codec combines those values
into one compact Telex `datatype=` line; numeric argument payloads remain
strings.
`TelexLimits::default()` exposes the published AltoPelago Telex limits. The
existing parse, encode, canonicalize, and validate functions use those defaults;
their `*_with_limits` variants accept an already normalized `TelexLimits`
value. Limits-file parsing and inheritance remain the caller's trusted
configuration concern. Bounds cover encoded input and line bytes, fields,
events, cumulative decoded payload bytes, paths, and all datatype dimensions;
limits are enforced without truncation.

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

Run the native bulk-operation benchmark with:

```bash
cargo run --release --example bench_telex --manifest-path implementations/rust/Cargo.toml
```

The Rust and JavaScript implementations intentionally do not call each other or
share codec source. Their common authorities are the transport-neutral portable
AES event contract, the Telex encoding draft, and the shared vectors.
