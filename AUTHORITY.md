# Authority

This repository is the public implementation-adjacent home of the Assignment
Event Stream (AES) family. It contains reference codecs, local conformance
development material, policies, proposals, and working documentation.

## Normative specifications

Normative AES specifications are published by
[`aeonite-org/aeonite-specs`](https://github.com/aeonite-org/aeonite-specs/tree/main/sources/aes/v1).
Their declared lifecycle and normativity determine the public contract.

The Markdown documents under [`specifications/`](./specifications/) are
readable working/reference copies. They do not override the canonical AEON
sources.

[`specifications/film.aes.md`](./specifications/film.aes.md) is a normative
draft working copy. Until it is promoted through the canonical specs process
and receives an immutable CTS baseline, neither that file nor the Rust Film
prototypes establish a released Film conformance claim.

## Conformance

Immutable shared conformance snapshots are published by
[`aeonite-org/aeonite-cts`](https://github.com/aeonite-org/aeonite-cts).

The vectors under [`conformance/`](./conformance/) are mutable development
material unless their manifest explicitly identifies an immutable shared
snapshot. Passing repository-local vectors does not by itself establish a
public conformance claim.

## Implementations and proposals

This repository owns the behavior of its JavaScript and Rust reference
implementations. Other AES implementations remain authoritative for their own
package and runtime behavior.

Proposal documents describe possible future work. They are not normative and
must not be treated as extending a published AES or Telex contract.
