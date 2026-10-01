# Releasing

This repository publishes the Rust reference implementation as
[`altopelago-aes-telex`](https://crates.io/crates/altopelago-aes-telex). The
repository-root npm package remains private; the public npm implementation
package, `@altopelago/aeon-aes`, is released from the
[`AltoPelago/aeon`](https://github.com/AltoPelago/aeon) workspace.

Package versions are independent of the AES event-contract, Telex wire-format,
Film wire-format, and CTS snapshot versions. See [VERSIONING.md](./VERSIONING.md).

## Rust compatibility declaration

`altopelago-aes-telex` 0.2 supports:

- `aes.events.v1`, including the complete and partial v1 profiles;
- `telex.aes=1` parsing, canonicalization, encoding, and validation;
- the immutable `aes-events-cts-v1-snapshot-0.1` and
  `telex-cts-v1-snapshot-0.1` targets; and
- the Film v1 reader, writer, and transcoder surfaces claimed in
  `film-cts-v1-snapshot-0.2`.

Film's durable-writer gate remains in force. Publishing the crate does not make
the Film writer suitable for durable interchange. Candidate B and Candidate C
remain explicitly experimental comparison surfaces.

## crates.io trusted publisher

The crates.io trusted publisher for `altopelago-aes-telex` must use:

- GitHub owner: `AltoPelago`
- repository: `aes`
- workflow: `rust-publish.yml`
- environment: `crates-io`

Protect the `crates-io` GitHub environment with an appropriate reviewer. The
workflow uses crates.io OIDC and does not require a long-lived registry token.
Manual workflow dispatch verifies and packages the crate but never publishes.

## Rust release checklist

1. Ensure the corresponding canonical specifications are on
   `aeonite-org/aeonite-specs` main.
2. Ensure the claimed immutable snapshots are on `aeonite-org/aeonite-cts`
   main.
3. Update the Rust manifest version, `implementations/rust/Cargo.lock`, the Rust
   CTS claim-set version, and this changelog in one reviewed pull request.
4. Run `npm run public:check` with `AEONITE_CTS_ROOT` pointing to that CTS
   checkout.
5. Run `npm run package:rust` and inspect the packaged file list. Only the Rust
   sources, crate README, license, and Cargo metadata should ship.
6. Merge the reviewed release commit to `main` and wait for GitHub Actions to
   pass.
7. Create a signed annotated tag named `aes-telex/vX.Y.Z` on that exact main
   commit and push the tag.
8. Approve the protected `crates-io` environment deployment. The tag workflow
   verifies the tag, publishes the crate, and runs a clean registry-consumer
   smoke test.
9. Confirm the version on crates.io and docs.rs before releasing any downstream
   package that depends on it.

Do not publish the crate directly from a developer machine. The signed-tag
workflow is the release path and the record of what was published.

## Repository boundary

The repository-root `package.json` must retain `private: true`. Normative
specifications and immutable CTS snapshots are released from their authority
repositories, not from this repository.
