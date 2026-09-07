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
`audit_aes_source_provenance` verifies exact retained bytes, valid UTF-8, range
bounds, and scalar boundaries while distinguishing missing artifacts from
invalid evidence.

The crate also implements the candidate transport-neutral
`aes.transaction.v0` body, support-gated logical envelope, exact-order
transaction digest, and signature input. Generic inspection can report that a
verified transaction is ready for trusted authorization, but always reports
`actionable=false`. A transaction selecting
`aes.preparation.source-backed.v0` additionally remains unready until its
record provenance audit is complete.

The Telex codec has no runtime dependencies. Portable integrity uses the
`sha2` crate for SHA-256 rather than implementing a cryptographic primitive in
this repository. `serde_json` is a test-only dependency used to load the
language-neutral vectors from `conformance/`.

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
