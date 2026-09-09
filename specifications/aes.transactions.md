# Assignment Event Transaction Contract v1

Scope: transport-neutral prepared transaction bodies, non-actionable carrier
validation, exact-order transaction integrity, the initial ASP scalar-value
replacement application, and composition with trusted hosts and AEON security
envelopes.

Transaction contract: `aes.transaction.v1`

Logical envelope: `aes.transaction.envelope.v1`

Integrity contract: `aes.transaction.integrity.v1`

Initial application contract: `aes.application.asp.scalar-replacement.v1`

This contract sits above `aes.events.v1`. It does not change Telex framing,
infer mutation intent from event content, authorize a write, or extend ASP v0
storage. A transaction becomes actionable only when a trusted consumer supports
every named sub-contract, validates the complete envelope, independently
authorizes the operation, prepares the target-specific candidate, and commits
it atomically.

## 1. Purpose and boundaries

An Assignment Event Transaction (AET) is a bounded prepared unit that binds an
ordered AES payload to an explicit application, target, preconditions,
preparation policy, limits claim, authorization context, assertions, and
integrity policy.

The contract owns:

- the closed logical transaction-body and envelope fields;
- distinct semantic-intent, prepared-attempt, and transaction identities;
- exact binding of the effective AES event context and supplied record order;
- standard target, revision-precondition, assertion, limits-claim,
  authorization-context, and preparation shapes;
- deterministic transaction-integrity and signature inputs; and
- the initial mapping contract for ASP scalar-value replacement.

It does not own Telex or Film bytes, endpoint negotiation, authenticated actor
identity, key trust, application-specific authorization decisions, AEOS schema
selection, ASP persistence, commit receipts, or encryption.

`telex.aes=1` remains an event-stream encoding. A Telex stream never becomes
actionable by containing a transaction-looking path or value. A physical AET
carrier must identify `aes.transaction.envelope.v1` before decoding its body;
Poem, AEON, or an API may provide that physical mapping separately.

## 2. Logical envelope

The logical envelope is a map with exactly:

| Field | Presence | Meaning |
| --- | --- | --- |
| `envelope` | required | `aes.transaction.envelope.v1` |
| `body` | required | one validated `aes.transaction.v1` body |
| `evidence` | required | null or transaction integrity/signature evidence |

The explicit null distinguishes an unsigned carrier from a truncated carrier.
An unsigned transaction may be structurally inspected, but a consumer policy
that requires integrity evidence rejects it before authorization or target
access.

Unknown envelope fields are invalid. Encryption requires a later envelope
contract or registered carrier security profile; ciphertext is not placed in
`body` under this version.

## 3. Transaction body

The body is a map with exactly the following core fields plus explicitly
registered `x.<owner>.<name>` extensions:

| Field | Logical value |
| --- | --- |
| `transaction` | `aes.transaction.v1` |
| `id` | non-empty transaction identity |
| `intent` | non-empty semantic-intent identity |
| `attempt` | non-empty prepared-attempt identity |
| `events` | `aes.events.v1` |
| `profile` | explicit effective AES profile |
| `projection` | explicit effective projection, or null for body-only |
| `ordering` | `aes.order.exact.v1` |
| `application` | application map |
| `target` | target map |
| `preconditions` | ordered precondition-map list |
| `preparation` | preparation map |
| `authorization` | authorization-context map |
| `limits` | limits-claim map |
| `assertions` | assertions map |
| `records` | ordered `aes.events.v1` record list |
| `integrity` | transaction-integrity policy map |

`id`, `intent`, and `attempt` are distinct strings of 1 through 256 Unicode
scalar values. They do not identify the later concrete ASP transaction or
commit receipt unless a target adapter explicitly defines and records that
mapping.

All effective event context is explicit even when the same values could have
been omitted from a Telex preamble. `ordering` is fixed to exact supplied order
in v1. Reordering produces a different transaction and requires new integrity
evidence.

Unknown core fields fail closed. A registered extension is covered by
transaction integrity and validated by its owning profile. A generic relay may
preserve an unknown extension but cannot claim semantic AET validation or make
the transaction actionable.

## 4. Standard component maps

### 4.1 Application

`application` contains exactly:

```text
{ contract: <non-empty registered identifier> }
```

The named contract defines what the records do. Omission never implies create,
replace, merge, delete, insert, move, patch, or upsert. Base AET validation does
not make an unknown application actionable.

### 4.2 Target

`target` contains exactly:

| Field | Meaning |
| --- | --- |
| `contract` | target-contract identifier |
| `id` | opaque target resource identity |
| `boundary` | target-defined atomic boundary |

The initial ASP target contract is `aes.target.asp.v1`. Its `id` is the trusted
ASP database identity and its `boundary` is the canonical source scope that
will be locked and revision-checked. The initial scalar application requires
`boundary="$"`.

### 4.3 Preconditions

`preconditions` is ordered. In v1 each entry has exactly `contract`, `scope`,
and canonical unsigned-decimal `revision` fields. The named contract owns the
meaning of that revision check. The initial registered contract is the ASP
revision form below; a different field shape requires a later transaction
extension or version.

```text
{
  contract: "aes.precondition.asp-revision.v1",
  scope: "$",
  revision: "42"
}
```

`revision` is a canonical unsigned decimal string. It is rechecked inside the
same atomic boundary as commit. Matching it during initial inspection is not
sufficient.

### 4.4 Preparation

#### 4.4.1 Identity preparation

The initial `aes.preparation.identity.v1` map contains only its `contract`.
It asserts that the supplied record order and values are already the final
prepared payload. Any transformation or reordering selects another registered
preparation contract and produces a new `attempt`, transaction `id`, and digest.

Preparation declarations are descriptive evidence. They never cause a
consumer to execute document-supplied code.

#### 4.4.2 Source-backed preparation

`aes.preparation.source-backed.v1` registers the operation gate for an
application that claims every payload record has exact retained source
evidence. Its map also contains only `contract`; record-local origins bind the
artifacts and do not expose storage locators in the transaction body.

Before readiness, the trusted consumer must resolve every distinct record
origin to exact bytes, verify its SHA-256 digest, require valid UTF-8, and check
every present span for bounds and scalar boundaries. Every record must carry an
origin. An origin-only record is valid evidence that the exact artifact was
retained while making no range claim. Missing bytes or a missing record origin
reports `AES_SOURCE_REQUIRED`; mismatch and invalid ranges retain their AES
event diagnostics. Local event validity remains independent of artifact
availability, but a transaction selecting this preparation cannot become
ready-for-authorization until the audit is valid and complete.

The audit alone does not prove that record semantics were parsed from the
claimed span. An application making that stronger assertion must name and rerun
the source projection that derives the event. A target adapter applies these
retention rules:

- an unchanged target occurrence may retain its already-verified provenance;
- a changed target occurrence always drops its previous provenance;
- verified payload provenance may become result provenance only when the named
  application maps that payload record to the result occurrence and performs
  any claimed source projection; otherwise it is evidence only; and
- ASP operation `origin` supplied by trusted host context is audit metadata,
  not AES source provenance, and is never synthesized from transported fields.

The registered scalar-replacement application continues to require identity
preparation and forbids payload `origin` and `span`; it makes no source-retention
claim.

### 4.5 Authorization context

`authorization` contains exactly:

| Field | Meaning |
| --- | --- |
| `contract` | trusted host authorization-context contract |
| `context` | opaque non-empty context identity |

The initial identifier is `aes.authorization.host-context.v1`. The field binds
the transaction to a host-known authorization context; it is not a credential
and never self-authorizes. A consumer matches it to authenticated host state and
still performs request and prepared-plan authorization.

### 4.6 Limits claim

`limits` contains exactly:

| Field | Meaning |
| --- | --- |
| `contract` | `aes.limits.claim.v1` |
| `id` | immutable named limit-set line |
| `version` | exact limit-set revision |

This is a covered claim about the limits used for preparation. It cannot select
or relax consumer limits. The consumer independently selects processing
limits, rejects unsupported claims when policy requires agreement, and may
always apply stricter immutable safety ceilings. Format-local parsing ceilings
apply before this field can be trusted.

### 4.7 Assertions

`assertions` contains exactly `eventCount` and `containers`.
`eventCount` is a canonical unsigned decimal string equal to `records.length`.
`containers` is an ordered list whose entries contain exactly:

| Field | Meaning |
| --- | --- |
| `path` | canonical body event path of the asserted container |
| `payloadDirectItemCount` | canonical unsigned decimal direct-child count in the payload |
| `resultDirectItemCount` | canonical unsigned decimal expected count after application |

Counts are assertions, not container `value` fields. An application that does
not define container assertion semantics requires an empty list.

### 4.8 Integrity policy

`integrity` contains exactly:

```text
{
  contract: "aes.transaction.integrity.v1",
  digest: "sha256"
}
```

The policy is body content and is therefore itself bound by the digest.
Evidence carrying the resulting hash remains outside the body.

## 5. Event payload

`records` is the final ordered logical `aes.events.v1` sequence. Validation
uses the explicit `profile` and `projection` from the body. Transport-specific
combined datatype spellings are expanded before transaction validation and
integrity encoding.

AET does not change event semantics. Structural identity remains metadata,
node heads remain explicit `[0]` events, attributes remain flat paths, and
provenance remains optional and record-local. Every present event field,
including `origin`, `span`, and registered extensions, is covered by transaction
integrity. Transaction v1 has no provenance-exclusion switch.

The base transaction carrier preserves duplicate records and addresses because
an application contract owns their meaning. The initial scalar application
forbids them by requiring exactly one record.

## 6. Initial scalar-value replacement application

`aes.application.asp.scalar-replacement.v1` is the only application registered
by this draft. It maps to the already-tested ASP v0 scalar-replacement behavior
without widening ASP storage.

Its transaction body additionally requires:

- `profile="aes.partial.v1"` and `projection=null`;
- `application.contract="aes.application.asp.scalar-replacement.v1"`;
- `target.contract="aes.target.asp.v1"` and `target.boundary="$"`;
- exactly one `aes.precondition.asp-revision.v1` entry for scope `$`;
- `preparation.contract="aes.preparation.identity.v1"`;
- `authorization.contract="aes.authorization.host-context.v1"`;
- an empty container-assertion list and `eventCount="1"`; and
- exactly one body record containing only `path`, `kind`, and `value`.

The record `kind` must be a scalar value-bearing kind admitted by the existing
ASP adapter. Its path and kind identify the exact expected portable occurrence;
only its string `value` is replacement material. The adapter preserves existing
datatype components, identity, attributes, siblings, representation kind, and
order. It drops stale source provenance from changed storage owners unless the
trusted host supplies a fresh target provenance record.

The target adapter resolves the event path through the versioned source/event
path map, reads the bound database, checks the declared revision and kind,
prepares the same one-operation ASP v0 candidate used by the existing closed
application, validates that complete candidate, authorizes both request and
resolved plan, and commits with an atomic revision precondition.

Node-head mutation, container or tuple replacement, reference retargeting,
insertion, deletion, movement, datatype changes, identity changes, and
multi-record application are not part of this application contract. Their
existing implementation candidates are not promoted by this carrier.

## 7. Transaction integrity

`aes.transaction.integrity.v1` reuses the deterministic structural mapping
`E(value)` from `aes.integrity.v1`. It does not reuse the event-stream integrity
input map or its domain.

The transaction logical bytes are:

```text
UTF8("aes.transaction.integrity.v1") || 0x00 || E(body)
```

Here `0x00` is one zero octet. `body` is the complete logical body map after
event decoding, default expansion, extension admission, and component
validation. Map keys use unsigned UTF-8 byte order; lists retain order; strings
retain their Unicode scalar sequence. No runtime JSON serialization or Telex
bytes participate.

The digest is SHA-256 and its textual form is 64 lowercase hexadecimal digits.
Any change to target, application, identities, event context, order, records,
preconditions, preparation, authorization context, limits claim, assertions,
integrity policy, or extension fields changes the digest.

The optional envelope evidence map contains exactly:

| Field | Logical value |
| --- | --- |
| `integrity` | `aes.transaction.integrity.v1` |
| `digest` | `sha256` |
| `hash` | canonical transaction digest |
| `signatures` | ordered signature-entry list |

An empty `signatures` list is valid digest-only evidence. Each signature entry
contains exactly `signature`, `alg`, `kid`, and `sig`, where `signature` is
`aes.transaction.signature.v1`. `sig` is a non-empty string in the encoding
selected by the trusted signature profile.

Signature input is:

```text
UTF8("aes.transaction.signature.v1") || 0x00 || E({
  signature: "aes.transaction.signature.v1",
  integrity: "aes.transaction.integrity.v1",
  digest: "sha256",
  hash: <transaction digest>,
  alg: <exact algorithm identifier>,
  kid: <exact opaque key identifier>
})
```

`sig` is excluded to avoid recursion. Algorithm approval, signature encoding,
key discovery, revocation, quorum policy, and timestamp authority belong to the
trusted security profile.

## 8. AEON and Poem composition

When AEON carries an AET, the transaction body is ordinary covered document
content. The final `aeon:envelope` may carry the transaction evidence, reusing
AEON signature field vocabulary while explicitly identifying
`aes.transaction.integrity.v1` and `aes.transaction.signature.v1`. It must not
label the transaction digest as `aeon.gp.integrity.v1`.

The security envelope is excluded from transaction bytes to avoid recursion.
An envelope attached to the original source artifact is separate lineage
evidence and does not authenticate the prepared transaction.

Poem may later define endpoint framing, negotiation, streaming, replies, and
delivery guarantees around `aes.transaction.envelope.v1`. That work does not
change `telex.aes=1` or this logical envelope. Encryption remains blocked until
a carrier profile defines plaintext/ciphertext coverage, authenticated visible
metadata, and processing order.

## 9. Validation and application lifecycle

1. Apply physical carrier and resource limits before trusting body claims.
2. Establish `aes.transaction.envelope.v1` and validate its closed shape.
3. Validate the complete `aes.transaction.v1` body and event stream.
4. Resolve every named application, target, precondition, preparation,
   authorization, limits, extension, integrity, and security contract against
   trusted consumer capabilities.
5. Verify integrity evidence when required by consumer policy.
6. If source-backed preparation is selected, resolve and audit every exact
   source artifact and require complete record coverage.
7. Resolve and authenticate trusted host context; document fields do not grant
   authority.
8. Authorize the request, read the exact target state, and prepare the concrete
   candidate.
9. Validate the resolved plan and complete candidate, then authorize the plan.
10. Recheck preconditions and commit atomically.
11. Return a separate versioned receipt or a failure with no partial commit.

A consumer may inspect, relay, or reject a well-formed transaction without
supporting its application. Structural validity, supported contracts, verified
evidence, authorization, and successful commit are separate states.

## 10. Diagnostics

| Code | Condition |
| --- | --- |
| `AES_TRANSACTION_ENVELOPE_INVALID` | envelope shape or identifier is invalid |
| `AES_TRANSACTION_BODY_INVALID` | body fields, types, identities, or context are invalid |
| `AES_TRANSACTION_UNSUPPORTED_CONTRACT` | a required named sub-contract is unsupported |
| `AES_TRANSACTION_EXTENSION_UNREGISTERED` | a body extension has no selected registration |
| `AES_TRANSACTION_EVENT_INVALID` | records fail their explicit AES event context |
| `AES_TRANSACTION_ASSERTION_FAILED` | event or container counts do not match |
| `AES_TRANSACTION_APPLICATION_INVALID` | payload violates the selected application contract |
| `AES_TRANSACTION_INTEGRITY_INVALID` | integrity evidence is malformed or unsupported |
| `AES_TRANSACTION_INTEGRITY_MISMATCH` | supplied and recomputed transaction digests differ |
| `AES_TRANSACTION_SIGNATURE_INVALID` | signature entry or signature input is invalid |
| `AES_SOURCE_REQUIRED` | source-backed preparation lacks complete exact artifacts |
| `AES_TRANSACTION_UNAUTHORIZED` | trusted host authorization rejects the request or plan |
| `AES_TRANSACTION_STALE` | an atomic target precondition no longer holds |

## 11. Conformance and publication status

Candidate vectors cover closed shape, distinct identities, explicit event
context, exact record order, scalar-application narrowing, revision and count
assertions, target/authorization/limits binding, deterministic logical bytes,
tamper detection, digest-only evidence, signature-context binding, unknown
fields, unsupported-contract separation, and source-backed preparation
readiness against exact retained artifacts.

The carrier and initial application remain normative drafts until independent
implementations pass a shared immutable CTS snapshot and the ASP bridge proves
that the same transaction body lowers to the already-tested closed scalar
application without adding Wire, CLI, bare-event, or Telex mutation ingress.
