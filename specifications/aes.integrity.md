# Portable AES Integrity Contract v0

Scope: deterministic logical bytes, digests, ordering policies, coverage,
provenance, signatures, and AEON security-envelope composition for portable AES.

Contract identifier: `aes.integrity.v0`

Ordering policies: `aes.order.canonical-semantic.v0`, `aes.order.exact.v0`

Signature-input contract: `aes.signature.v0`

This contract operates on validated `aes.events.v0` records. It is independent
of Telex, Film, JSON, AEON source spelling, and implementation-native event
objects. An encoding decoder first recovers the transport-neutral stream; this
contract then produces identical logical bytes from that stream.

## 1. Purpose and authority

This document owns:

- the deterministic structural byte mapping used for portable AES integrity;
- binding of the event contract, effective profile and projection, ordering
  policy, scope, provenance policy, digest algorithm, and ordered records;
- canonical-semantic and exact supplied-order policies;
- SHA-256 digest production and comparison;
- the domain-separated signature input; and
- composition rules for carrying AES evidence in an AEON security envelope.

It does not define event validity, Telex or Film bytes, private-key handling,
trusted-key discovery, algorithm approval, authorization, encryption, ledger
storage, or Assignment Event Transaction envelopes. Those are defined by the
separate [`aes.transaction.v0`](./aes.transactions.md) draft.

`aeon.gp.integrity.v1` remains the AEON final-document-state hash. Its existing
path/value serialization and signatures are not portable AES evidence and are
not upgraded or reinterpreted by this contract.

## 2. Integrity input

The logical integrity input is a map with exactly these fields:

| Field | Logical value |
| --- | --- |
| `integrity` | `aes.integrity.v0` |
| `events` | `aes.events.v0` |
| `profile` | effective non-empty AES profile identifier |
| `projection` | effective projection identifier, or null for body-only |
| `ordering` | selected ordering-policy identifier |
| `scope` | `aes.scope.body.v0` or `aes.scope.document.v0` |
| `provenance` | `aes.provenance.excluded.v0` or `aes.provenance.included.v0` |
| `digest` | `sha256` |
| `records` | the policy-produced record sequence |

Every field is present in the logical input. Carrier omission does not become
integrity-input omission: an omitted AES profile is expanded to
`aes.complete.v0`, and an omitted projection becomes logical null.

The base digest does not bind a consumer's runtime resource limits. Limits do
not change AES event meaning. An enclosing application or transaction contract
that makes an effective-limits claim binds that separately as part of its own
logical input; it must not inject local limits into this map.

## 3. Scope

`aes.scope.body.v0` includes body records and excludes every header record. It
still binds the effective projection, so the same body selected from two
different projections is not silently treated as the same integrity input.

`aes.scope.document.v0` includes the ordered header plane followed by the body
plane and requires `projection=aeon.document.v0`. A later projection may
register its own compatible document-scope rule. Document scope fails when no
selected projection defines a header plane.

Scope filtering occurs before provenance processing and ordering.

## 4. Provenance policy

`aes.provenance.excluded.v0` removes `origin` and `span` from every covered
record. It is the semantic-value policy and does not attest source bytes.

`aes.provenance.included.v0` retains both fields exactly when present. A signer
claiming source-backed verification must additionally perform the source
artifact audit defined by `aes.events.v0`; syntactic inclusion alone does not
prove that an artifact was available or checked.

All other core fields and all present extension fields remain covered. A field
is never omitted merely because the signer does not understand it.

## 5. Ordering policies

### 5.1 `aes.order.exact.v0`

Exact order preserves the supplied order of the covered records. Repeated
addresses and repeated identical records remain distinct ordered occurrences.
This is the policy for ledgers, prepared transactions, delivery evidence, and
any application where chronology or repeated writes are significant.

A relay that changes record order produces different logical bytes and cannot
retain the former digest or signature as evidence over the new sequence.

### 5.2 `aes.order.canonical-semantic.v0`

Canonical-semantic order asserts that supplied record order is not part of the
evidence being signed. It is valid only when every covered address is unique
within its address plane. A duplicate `(plane, address)` fails with
`AES_INTEGRITY_AMBIGUOUS_CANONICAL_ORDER`; implementations must not use record
contents as a tie-breaker to conceal repeated writes.

Covered records are sorted by:

1. plane: header before body;
2. the unsigned UTF-8 bytes of the canonical `header` or `path` string; and
3. no further key, because addresses are unique within a plane.

Byte strings are compared lexicographically by unsigned octet. A shorter byte
string sorts first when it is an exact prefix. No locale, Unicode normalization,
host collation, numeric conversion, or path-segment interpretation is applied.
Indexed semantic order remains represented by each canonical indexed address.

Selecting this policy is an application assertion. It does not make order
insignificant under every AES profile and must not replace exact order when
chronology, repeated occurrences, or a ledger sequence matters.

## 6. Covered record structure

Each record is encoded as a map of its present logical fields after the selected
scope and provenance projections. Absence is represented by an absent map key;
it is distinct from an empty string, empty array, and null.

Datatype information is always the expanded `aes.events.v0` structure:

- `datatype` is the base-name string;
- `generics` is an ordered list of datatype maps or tagged numeric-literal maps;
- `clarifiers` is an ordered list of tagged string/numeric-literal maps.

The Telex combined datatype descriptor is never an integrity input. Extension
field names and string values are included like other record map members. An
integrity implementation validates the selected AES profile and registered
extension surface before claiming semantic conformance.

Portable scalar strings are encoded exactly as their Unicode scalar sequence.
No normalization, case folding, numeric parsing, null reinterpretation, or
reference resolution occurs. The record's normative `kind` is independently
covered, so equal string payloads under distinct recognized representation
kinds remain distinct integrity inputs. Source-only fields such as `raw` are
not part of `aes.events.v0` and fail validation instead of entering the digest.

## 7. Deterministic structural byte mapping

All marker and length characters below are ASCII. String payloads are UTF-8.
Lengths count bytes, not Unicode scalar values or host string units. Unsigned
decimal integers use `0` or a digit from `1` through `9` followed by digits;
leading zeroes are forbidden.

The recursive encoding `E(value)` is:

| Logical value | Bytes |
| --- | --- |
| null | `n` |
| string | `s` + decimal UTF-8 byte length + `:` + UTF-8 bytes |
| list | `l` + decimal item count + `:` + each item encoded in order |
| map | `m` + decimal member count + `:` + each encoded key then encoded value |

Map keys are strings and are emitted in unsigned UTF-8 byte order. Duplicate
map keys are invalid. Maps are therefore independent of host insertion order.
Counts and string lengths make delimiters inside strings inert; no escaping is
performed.

The complete logical bytes are:

```text
UTF8("aes.integrity.v0") || 0x00 || E(integrity-input-map)
```

Here `0x00` is one zero octet. Implementations reject
unpaired UTF-16 surrogates or any other input that is not a Unicode scalar
sequence rather than replacing it during UTF-8 encoding.

This mapping is an integrity encoding only. It is not an AES interchange format
and does not compete with Telex or Film.

## 8. Digest

AES v0 defines `sha256`. The digest is SHA-256 over the complete logical bytes.
Its portable textual form is exactly 64 lowercase hexadecimal digits.

The `digest=sha256` member is inside the hashed integrity-input map. Algorithm
substitution therefore changes the preimage as well as the verification
operation. Other algorithms require a new registered digest identifier and
test vectors; accepting a host crypto name is insufficient.

Digest comparison is exact over the 32 digest bytes. Text decoders first reject
non-canonical hexadecimal spelling.

## 9. Signature input

`aes.signature.v0` reuses the AEON signature-entry vocabulary where possible,
but defines a separate coverage rule. A signature entry supplies:

- signature algorithm `alg`;
- opaque non-empty key identifier `kid`;
- signature bytes `sig`; and
- the associated AES integrity evidence.

The signature-context map has exactly:

| Field | Logical value |
| --- | --- |
| `signature` | `aes.signature.v0` |
| `integrity` | `aes.integrity.v0` |
| `digest` | `sha256` |
| `hash` | the 64-character lowercase digest string |
| `alg` | the exact signature algorithm identifier |
| `kid` | the exact opaque key identifier |

Signature input bytes are:

```text
UTF8("aes.signature.v0") || 0x00 || E(signature-context-map)
```

`sig` is excluded to avoid recursion. Binding `alg` and `kid` prevents metadata
substitution without producing new valid evidence. Algorithm approval, key
resolution, revocation, multi-signature policy, and timestamp authority remain
security-profile concerns.

## 10. AEON security-envelope composition

An AEON document may carry AES integrity evidence inside its ordinary
`aeon:envelope`. The carrier must preserve all integrity-input context fields,
the digest, and signature fields. `aeon.gp.signature.v1` field names may be
reused, but an entry must identify `aes.signature.v0`; it must not claim that an
AES digest is an `aeon.gp.integrity.v1` final-state hash.

The envelope remains outside the covered AES record stream. For document scope,
the `aeon.document.v0` header plane and body plane are covered, while the
security envelope containing the evidence is excluded to avoid recursion.

Signing does not authorize an AES application or ASP write. The separate
`aes.transaction.integrity.v0` contract reuses this structural mapping while
binding transaction identity, target, application/preparation contracts,
preconditions, assertions, effective limits claims, authorization context, and
extensions under a distinct domain.

Encryption composition remains deferred. A profile must state whether this
contract covers plaintext or a separately defined authenticated ciphertext
structure before encrypted AES evidence is issued.

## 11. Processing procedure

To compute AES integrity evidence:

1. Establish and validate `aes.events.v0`, its effective profile and projection,
   and any registered extension fields.
2. Require explicit ordering, scope, and provenance policies.
3. Select records by scope.
4. Remove `origin` and `span` only when the provenance policy requires it.
5. Apply the selected ordering policy, rejecting ambiguous canonical order.
6. Construct the complete integrity-input map with expanded defaults.
7. Encode it using the deterministic structural mapping.
8. Compute SHA-256 and render lowercase hexadecimal evidence.

Verification repeats the same procedure from trusted policy inputs and compares
the digest. It does not trust a carrier's context declarations merely because
they accompany a matching digest; the verifier decides which contracts and
policies are acceptable for the operation.

## 12. Diagnostics

| Code | Condition |
| --- | --- |
| `AES_INTEGRITY_CONTEXT_REQUIRED` | ordering, scope, provenance, or stream context is absent |
| `AES_INTEGRITY_UNSUPPORTED_CONTRACT` | integrity or event contract is unsupported |
| `AES_INTEGRITY_UNSUPPORTED_ORDERING` | ordering policy is unknown |
| `AES_INTEGRITY_UNSUPPORTED_SCOPE` | scope is unknown or unavailable for the projection |
| `AES_INTEGRITY_UNSUPPORTED_PROVENANCE` | provenance policy is unknown |
| `AES_INTEGRITY_UNSUPPORTED_DIGEST` | digest identifier is unknown |
| `AES_INTEGRITY_AMBIGUOUS_CANONICAL_ORDER` | canonical-semantic input repeats an address in one plane |
| `AES_INTEGRITY_INVALID_LOGICAL_VALUE` | a covered field cannot be represented by the structural mapping |
| `AES_INTEGRITY_DIGEST_MISMATCH` | supplied and recomputed digests differ |
| `AES_SIGNATURE_CONTEXT_INVALID` | signature algorithm, key identifier, or digest context is invalid |

## 13. Conformance

Independent implementations use the same logical input vectors and compare both
complete logical bytes and SHA-256 digests. Required vectors cover:

- empty streams with expanded defaults;
- canonical-semantic equality across reordered unique records;
- exact-order inequality across the same reorder;
- duplicate-address rejection versus exact-order preservation;
- expanded generics and clarifiers;
- body versus document scope;
- semantic versus provenance-inclusive evidence;
- representation-kind distinctions for equal scalar payload strings;
- rejection of source-only raw spelling at the integrity boundary;
- extension-field preservation and map-key ordering;
- non-ASCII UTF-8 byte lengths; and
- signature-context binding of algorithm and key identity.

Vectors are owned by an AES integrity CTS lane, not by Telex. Telex and Film
decoders pass their recovered logical stream to that lane without contributing
encoding bytes.
