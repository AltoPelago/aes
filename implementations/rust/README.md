# Rust Telex reference implementation

This crate independently implements the published v1 `telex.aes` parsing,
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
`aeon.document.v1` projection adds flat `header` records in a disjoint address
plane; it remains independent of the complete/partial AES profile selection.
Optional source provenance is record-local: `origin` may stand alone, while
`span` requires a canonical SHA-256 origin.
`audit_aes_source_provenance` verifies exact retained bytes, valid UTF-8, range
bounds, and scalar boundaries while distinguishing missing artifacts from
invalid evidence.

The crate also implements the candidate transport-neutral
`aes.transaction.v1` body, support-gated logical envelope, exact-order
transaction digest, and signature input. Generic inspection can report that a
verified transaction is ready for trusted authorization, but always reports
`actionable=false`. A transaction selecting
`aes.preparation.source-backed.v1` additionally remains unready until its
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
AES event contract, the published Telex encoding, and the shared vectors.

## Experimental Film Candidate A

`film_candidate_a` exercises the table-free binary layout currently recorded in
the Film roadmap. It implements canonical stream context, framed records, the
fixed kind table, structured datatypes, provenance, spans, named extensions,
Film-local limits, and direct Telex transcoders over the existing portable AES
record model.

This module is research code in an unpublished crate. It is not a released Film
specification or conformance target, and its bytes must not be used for durable
interchange before the Film v1 layout and CTS are frozen.

`film_candidate_b` is a deliberately stateful comparator. It replaces each
address with the longest UTF-8 prefix shared with the previous address in the
same address plane plus an inline suffix. It uses the distinct experimental
`O_B FF 00` preamble and can never be represented as `film.aes=1`.

Run its focused tests and native benchmark with:

```bash
cargo test --locked --manifest-path implementations/rust/Cargo.toml --test film_candidate_a
cargo test --locked --manifest-path implementations/rust/Cargo.toml --test film_candidate_b
cargo run --release --locked --example bench_film_candidate_a --manifest-path implementations/rust/Cargo.toml
cargo run --release --locked --example compare_film_layouts --manifest-path implementations/rust/Cargo.toml -- path/to/input.telex.aes
```

Set `FILM_BENCH_OUTPUT_DIR` or `FILM_COMPARE_OUTPUT_DIR` to retain generated
Telex and candidate bytes for equal external-compression comparisons.
