# AES integrity, relay, and durable-codec audit

Status: implementation audit, 2026-09-07

## Outcome

Portable AES encoding paths preserve the named `aes.events.v0` record model,
effective profile/projection declarations, record order, and expanded datatype
fields. No reviewed implementation currently defines an encoding-neutral
portable AES logical-byte or signature contract. Existing hashes and
fingerprints remain valid only in their explicitly named legacy or
application-specific domains.

The audit found one serialized compatibility boundary in Aeon Tonics that
accepted untagged TypeScript assignment-event JSON. That route now requires and
emits `contract: "aeon.typescript.assignment-events.v0"`; raw arrays and
untagged `{ events }` objects fail closed. Telex remains the portable
interchange route.

## Boundary rule

- Same-process `AssignmentEvent` APIs may retain their implementation-native
  type while they remain internal to one implementation.
- A serialized implementation-native event array must carry a named source
  contract and must not claim portable AES compatibility.
- A portable boundary uses a validated `aes.events.v0` carrier such as
  `telex.aes=0`, including its effective profile, projection, and record order.
- A hash, fingerprint, receipt, or signature is portable AES evidence only if
  its versioned integrity contract defines and binds the complete logical
  scope. A SHA-256 digest alone does not make that claim.

## Reviewed surfaces

| Surface | Classification | Audit result |
| --- | --- | --- |
| TypeScript and Rust AEON integrity-envelope canonical hashes | Legacy AEON source projection | Retained for compatibility and relabelled. They sort source paths and hash canonical AEON values while excluding the top-level envelope; they do not bind event kind, structural identity, expanded datatype fields, AES profile/projection, or an ordering-policy identifier. They are not portable AES hashes. |
| TypeScript, Rust, Python, and PHP Telex codecs | Portable AES encoding | Telex headers identify the wire version and effective context; validated records retain expanded datatype structure internally, and canonical Telex preserves the selected record sequence. Encoding canonicalization is not a portable semantic-signature definition. |
| ASP strict portable read view and Telex export | Portable read-only projection | The named strict view carries its AES contract, profile, projection, adapter identity, ordered records, and conversion report. Telex is produced only from a validated strict view. No actionable Telex ingress exists. |
| ASP portable read-view `contentFingerprint` | Application-specific derived-content check | Covers the complete named read-view derivation content, including its ordered records and report. Documentation already excludes semantic-hash, signature, authorization, and source-artifact claims. |
| ASP portable mutation journals and receipts | Application-specific durable evidence | Requests, plans, transactions, receipts, journals, and locks use closed, versioned candidate protocols and domain-separated canonical-JSON fingerprints. These fingerprints detect drift/recovery mismatch; they are not AES semantic hashes, generic AET evidence, or signatures. |
| ASP migration transition authoring | Legacy AEON artifact identity | Uses the TypeScript legacy AEON canonical assignment projection for stable source-manifest/reference identities. Documentation now states that this is not portable AES integrity evidence. |
| Aeon Tonics `aeon-edit export-aes` and `aes-diff --from-aes` | Legacy serialized compatibility route | Now emits/requires `aeon.typescript.assignment-events.v0`. Untagged inputs fail closed. `export-telex`/`--from-telex` remain the portable routes. |
| Aeon Tonics `aes.patch v1` | Tool-local review/application artifact | Ordered path-keyed operations are retained for the existing tool contract. The patch is not a portable AES stream, AET carrier, ledger representation, digest scope, or signature scope. |
| Aeon Tonics signed-ledger prototype | Separate ledger protocol | Signs its own `aeon.ledger.entry` canonical-JSON payload. Any future AES ledger entry must name and bind an approved exact-order AES integrity contract; the current protocol is not silently upgraded. |
| Aeon Tooling and CTS JSON fixtures | Internal process/test control | No public serialized portable AES boundary was found. Test-control JSON must not be advertised as an interchange contract. |

## Portable integrity work that remains open

The existing specifications establish what must be bound but deliberately do
not yet define the deterministic logical bytes. A separately approved,
versioned integrity contract must settle all of the following before portable
AES signing is enabled:

1. An encoding-neutral deterministic mapping for `aes.events.v0`, including
   closed field presence/absence rules and expanded `datatype`, `generics`, and
   `clarifiers`.
2. Explicit binding of the effective semantic profile, projection, and limits
   claim, including defaults that were not written on the Telex wire.
3. Distinct policies for canonical-semantic ordering and exact supplied-order
   ledger signing. Path sorting is not a general ledger rule.
4. Header scope: body hashes exclude the header plane, while a full-document
   policy binds the ordered header plane followed by the ordered body plane.
5. Provenance and span inclusion, plus the required immutable source identity
   when source-backed evidence is claimed.
6. Extension-field admission and their deterministic inclusion or rejection.
7. Domain separation, digest/signature algorithm identifiers, key identity,
   signature encoding, and independent cross-language vectors.
8. Composition with AEON encryption and signature envelopes, including whether
   a policy covers plaintext, ciphertext, or a separately defined composition.

Until that contract exists, implementations must not compute a purported
portable AES signature by signing canonical Telex bytes, a legacy
`AssignmentEvent` hash, an ASP content fingerprint, or an application journal
fingerprint.

## Definition of done for this audit

- Portable encoders and read projections have been distinguished from hashes
  and signatures.
- Reviewed durable fingerprints are tied to their owning protocol and are not
  promoted as portable AES evidence.
- The discovered untagged Tonics compatibility boundary now identifies its
  implementation-native contract and rejects ambiguous input.
- Legacy AEON canonical hash wording no longer calls the result an AES hash.
- The remaining portable logical-byte/signature contract is recorded as a
  separate, explicit design and conformance task.
