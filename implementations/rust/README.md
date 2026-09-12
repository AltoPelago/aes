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

Run the phase-separated T0 scalar comparison across Film, Telex, portable JSON,
and resident AES validation with:

```bash
npm run bench:scalar
```

The comparison labels syntax, provisional-owned, complete-validation, and
encoding boundaries independently. Its raw JSON engine rows intentionally omit
AES validation; the adapter and validated-adapter rows expose the additional
work explicitly.

Count heap allocations for the corresponding native Film, Telex, and resident
AES operations with:

```bash
npm run profile:allocations
```

The profiling allocator is a development-only dependency. Each operation runs
in an isolated child process so its count has one unambiguous lifetime; setup
and one warm-up pass occur before profiling begins.

The Rust and JavaScript implementations intentionally do not call each other or
share codec source. Their common authorities are the transport-neutral portable
AES event contract, the published Telex encoding, and the shared vectors.

## Film v1 draft reference

`film` implements the selected table-free binary layout defined by the Film v1
normative draft. It provides borrowed Film views, explicit owned
materialization and complete validation, canonical stream context, framed
records, the fixed kind table, structured datatypes, provenance, spans, named
extensions, Film-local limits, and direct Telex transcoders over the existing
portable AES record model. The historical `film_candidate_a` module re-exports
this surface temporarily for local prototype compatibility.

This module is reference code in an unpublished crate. Its reader claims the
immutable `film-cts-v1-snapshot-0.1` target and passes the repository-local
mutable 72-vector Film candidate; later local additions do not alter that
snapshot claim. The writer remains available for conformance tooling and
experimentation, but must not be enabled for durable interchange until reader
deployment and ecosystem compatibility review close the separate writer gate.

`film_candidate_b` is an archived, deliberately stateful comparator. It
replaces each address with the longest UTF-8 prefix shared with the previous
address in the same address plane plus an inline suffix. It uses the distinct
experimental `O_B FF 00` preamble and can never be represented as
`film.aes=1`.

Run its focused tests and native benchmark with:

```bash
cargo test --locked --manifest-path implementations/rust/Cargo.toml --test film
cargo test --locked --manifest-path implementations/rust/Cargo.toml --test film_api
cargo test --locked --manifest-path implementations/rust/Cargo.toml --test film_conformance
cargo test --locked --manifest-path implementations/rust/Cargo.toml --test film_candidate_b
cargo run --release --locked --example bench_film_candidate_a --manifest-path implementations/rust/Cargo.toml
cargo run --release --locked --example compare_film_layouts --manifest-path implementations/rust/Cargo.toml -- path/to/input.telex.aes
```

Set `FILM_BENCH_OUTPUT_DIR` or `FILM_COMPARE_OUTPUT_DIR` to retain generated
Telex and candidate bytes for equal external-compression comparisons.

The fuzz-only crate under `fuzz/` sends arbitrary CTS-seeded bytes through the
borrowed and validated Film decoders under libFuzzer and AddressSanitizer. It is
kept outside the reference crate's runtime and test dependency graph. Run the
bounded local gate with `npm run fuzz:film`; continuing and corpus-management
commands are documented in [`fuzz/README.md`](./fuzz/README.md).
