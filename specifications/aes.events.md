# Portable AES Event Contract v0

Scope: transport-neutral AES event records, paths, profiles, projections,
ordering, provenance, fidelity, and validation.

Contract identifier: `aes.events.v0`

Profiles: `aes.complete.v0`, `aes.partial.v0`

This document is transport-neutral. It defines the portable Assignment Event
Stream model encoded by `telex.aes`, the future `film.aes`, and other AES
transports. It does not define a byte encoding.

## 1. Purpose and authority

Portable AES is an ordered stream of flat records describing already-recognized
values. A consumer can reconstruct the represented event structure without
parsing AEON source, inferring value kinds, or depending on a producer's
in-memory AST or object model.

This document owns:

- the portable record fields and value-kind vocabulary;
- flat structure, attributes, node heads, and structural identity;
- event paths and AEON source-path translation;
- provenance and spans;
- event ordering;
- completeness profiles;
- optional control-plane projections;
- semantic-losslessness boundaries; and
- AES semantic diagnostic codes.

Encodings own framing, escaping, canonical bytes, and syntax diagnostics.
Source languages own their grammar and projection into this model. SANSA owns
canonical address grammar and navigation semantics. The Aeonic Semantic
Language owns operations over recognized values.

## 2. Stream model and context

An AES stream is an ordered sequence of records under the `aes.events.v0`
contract plus two independent context axes:

- `profile` selects validation and completeness claims;
- `projection` selects which source surface is represented.

After a carrier has established `aes.events.v0`, omitting the profile selects
`aes.complete.v0`. A stream that relaxes cross-record completeness must
explicitly select `aes.partial.v0` or another profile. Omitting the projection
selects the ordinary body-only stream.

The defaults do not identify the contract. An untagged legacy JSON record or
array is not portable AES merely because it resembles this record shape. Every
encoding, API, or storage surface binds its records to `aes.events.v0` before
interpreting them; legacy conversion follows the separate
[`aes.compatibility.md`](./aes.compatibility.md) contract.

An encoding may place these identifiers in a stream header, protocol envelope,
media-type parameter, or equivalent transport metadata. They are stream
context, not event records.

Portable scalar fields are Unicode strings. `generics` and `clarifiers` are
ordered arrays with the structured values defined below. An encoding may use a
more compact physical representation, but decoding must recover the same
logical fields, scalar strings, arrays, stream context, and record order.

## 3. Record shape

### 3.1 Core fields

| Field | Presence | Meaning |
| --- | --- | --- |
| `header` | address-dependent | canonical address in an explicitly selected header plane |
| `path` | address-dependent | canonical SANSA data address of a body event |
| `kind` | required | portable value-kind token |
| `datatype` | optional | declared base datatype name |
| `generics` | datatype-dependent | ordered generic-argument array |
| `clarifiers` | datatype-dependent | ordered tagged-clarifier array |
| `identity` | optional | structural occurrence identity |
| `value` | kind-dependent | decoded textual payload |
| `origin` | optional | immutable identity of exact source bytes |
| `span` | optional; requires `origin` | source byte range |

Every record has exactly one address field: `path` for a body event or `header`
for a control-plane record defined by an explicit projection. A record with
neither or both is invalid. No additional assignment-event discriminator is
required.

`key` is not transported because it is derivable from the final segment of the
selected address. Normalized selector paths and other cached navigation forms
are also derived and are not transported.

### 3.2 Extensions

Portable extension fields use `x.<owner>.<name>` identifiers. The prefix does
not make an extension optional, understood, or safe to ignore. A semantic
consumer rejects an unknown field unless the active profile registers that
exact field and defines its meaning and validation.

A generic relay may preserve unknown fields without understanding them. It
cannot claim semantic AES conformance for those records or sign a transformed
record after silently dropping a field. A new required core field requires a
new event-contract version rather than an extension.

## 4. Datatypes and values

### 4.1 Datatypes

`datatype` contains only the declared base name, without a source-language
binding marker. When `datatype` is present, `generics` and `clarifiers` are both
present as arrays, including when either array is empty. When `datatype` is
absent, both arrays are absent.

These AEON declarations therefore project as follows:

```text
:csv["."]       -> datatype="csv",  generics=[], clarifiers=[{kind="StringLiteral", value="."}]
:list<int>      -> datatype="list", generics=[{datatype="int", generics=[], clarifiers=[]}], clarifiers=[]
:null<string>   -> datatype="null", generics=[{datatype="string", generics=[], clarifiers=[]}], clarifiers=[]
```

A datatype generic argument is either a recursively structured datatype
descriptor or a tagged `NumberLiteral`. A recursive descriptor has exactly the
same three fields: non-empty `datatype`, ordered `generics`, and ordered
`clarifiers`. A tagged value has exactly `kind` and `value`. A clarifier is a
tagged `StringLiteral` or `NumberLiteral`.
Tagged literal values are canonical strings, including numeric values; an AES
boundary never converts them to host numeric types.

```text
datatype = "tuple"
generics = [
  { datatype = "int", generics = [], clarifiers = [] },
  { kind = "NumberLiteral", value = "9007199254740993" }
]
clarifiers = [
  { kind = "StringLiteral", value = "x" },
  { kind = "NumberLiteral", value = "16" }
]
```

Array order is significant and duplicate entries are preserved. An encoding
may combine the three logical fields, as Telex does, but it must expand them on
decode and construct the combined form from them on encode. Semantic hashes and
signatures over AES records bind the expanded logical structure, not an
encoding-specific combined spelling.

AES v0 implementations guard recursive generic descriptors. Generic depth is
the number of nested generic arguments that themselves contain generics:
`list<int>` has depth `0`, `list<list<int>>` has depth `1`, and
`list<list<list<int>>>` has depth `2`. The consumer selects the active maximum
as processing policy. Exceeding it rejects the record; it never truncates the
generic tree.

Generic depth is an event-local resource counter, not a completeness rule or a
semantic-profile setting. This contract defines how it is measured but does
not prescribe a numeric consumer limit. A profile mentioned by a limits file
is a claim or metadata association only; the consumer still selects both.

A datatype remains only on the event where it was declared. AES does not infer
or propagate `int` from `list<int>` onto child events. Datatype interpretation,
generic constraints, clarifier meaning, and inherited meaning are downstream
concerns.

### 4.2 Value kinds

| `kind` | `value` payload |
| --- | --- |
| `StringLiteral` | decoded Unicode string; empty is valid |
| `NumberLiteral` | canonical finite numeric text |
| `InfinityLiteral` | `Infinity` or `-Infinity` |
| `NaNLiteral` | `NaN` or `-NaN` |
| `NullLiteral` | recognized sentinel or decoded custom reason |
| `BooleanLiteral` | `true` or `false` |
| `ToggleLiteral` | `yes`, `no`, `on`, or `off` |
| `HexLiteral` | lowercase hexadecimal digits without `#` or visual underscores |
| `RadixLiteral` | canonical radix payload without `%` or visual underscores |
| `EncodingLiteral` | encoding payload without `&`; padding is preserved |
| `SeparatorLiteral` | canonical separator payload without `^` |
| `SansaAddressLiteral` | canonical SANSA address |
| `DateLiteral` | canonical date text |
| `TimeLiteral` | canonical time text |
| `DateTimeLiteral` | canonical date-time text without a world-time-context reference |
| `WTCDateTimeLiteral` | canonical world-time-context text |
| `ObjectNode` | absent |
| `ListNode` | absent |
| `TupleLiteral` | absent |
| `NodeLiteral` | absent |
| `NodeHead` | node tag |
| `CloneReference` | canonical target event path |
| `PointerReference` | canonical target event path |

`value` is required for every kind except `ObjectNode`, `ListNode`,
`TupleLiteral`, and `NodeLiteral`, for which it is absent. Every value payload
is a string at the portable boundary even when its kind gives it numeric or
temporal meaning.

Container records do not carry nested value trees. Every represented member or
item is a separate descendant record. A node head uses `value` for its tag. A
reference uses `value` for its target path. A null uses `value` for its
recognized sentinel or decoded custom reason. No kind introduces a
kind-specific field name.

Source spelling is normalized before the event enters AES. Quoted, backtick,
and trimtick strings all become `kind=StringLiteral`; trimtick width and
indentation are source mechanics. Numeric separators and the leading AEON
sigils for hex, radix, encoding, and separator values are not transported.

### 4.3 References

Clone and pointer references are distinct kinds even when they have the same
target:

```text
path=$.copy
kind=CloneReference
value=$.source

path=$.alias
kind=PointerReference
value=$.source
```

AEON's `~` and `~>` sigils are represented by `kind`, not retained in `value`.
Portable AES preserves a symbolic reference and does not resolve or materialize
it.

Under `aes.complete.v0`, a syntactically valid target must identify exactly one
body event in the same stream. Forward references are valid because existence
is checked across the complete stream rather than against event order. Header
addresses are not reference targets. AES checks existence only; cycle policy
and clone or pointer materialization remain downstream concerns.

Under `aes.partial.v0`, the target need only be a canonical event path. It may
be absent because the surrounding transaction, subscription, or ledger state
can supply the referenced occurrence.

### 4.4 WTC preservation

A `WTCDateTimeLiteral` event carries the complete canonical WTC text in
`value`. Its temporal anchor and temporal reference remain one opaque payload:

| Anchor form | Example payload |
| --- | --- |
| civil | `2025-01-01T09:30&local` |
| explicit offset | `2025-01-01T09:30+11:00&Australia/Melbourne` |
| UTC | `2024-12-31T22:30Z&-36.7590183/144.2826718` |

Each anchor form is independently valid with the exact lowercase `local`
reference, a named reference, or a geographic
`latitude/longitude[/height]` reference. `Local` and `LOCAL` are not aliases.
Authored case in other named references is preserved.

AES does not convert an offset to `Z`, resolve a named reference, normalize
geographic components, or otherwise rewrite the payload. Offset and UTC forms
remain lexically distinct even when they identify the same instant.

`conflictAuthority` is trusted consumer resolution policy. It is not a WTC
payload component, AES core field, or document authority defined by
`aeon.document.v0`. A source member or attribute with that spelling remains
ordinary untrusted data. Carrying an extension claim does not grant authority.

## 5. Flat structure and identity

### 5.1 Structural occurrence identity

`identity` belongs to the headed occurrence represented by its record. It is
optional, omits AEON's surrounding backslash delimiters, and is never invented
merely to serialize an event.

Identity does not alter event paths, path equality, navigation, selection,
ordering, or parentage. Under `aes.complete.v0`, non-empty identities are unique
across the complete document, including ordinary bindings, attribute entries,
anonymous child heads, and node heads.

### 5.2 Anonymous values

An anonymous typed, attributed, or identified value does not introduce a
wrapper event. Its datatype components, identity, kind, and value belong to the
event at its indexed path:

```text
path=$.values[0]
kind=NumberLiteral
datatype=int
generics=[]
clarifiers=[]
identity=item-1
value=3
```

Its attribute descendants occur beneath `$.values[0].@`.

### 5.3 Attributes

Attributes use the same flat record shape recursively. Their paths contain the
canonical `.@` selector; no embedded attribute map or synthetic attribute-space
container is transported.

For example, `a@{a={a=1, b@{a=2}=2}}=0` produces records at:

```text
$.a
$.a.@.a
$.a.@.a.a
$.a.@.a.b
$.a.@.a.b.@.a
```

AES preserves the address. Attribute ownership and scope interpretation belong
to SANSA and downstream consumers.

### 5.4 Nodes

A node is a value-less ordered `NodeLiteral` container. Its heads are indexed
`NodeHead` descendants that carry their tags and own their ordered content.
Current AEON produces exactly one head at index zero:

```text
path=$.a
kind=NodeLiteral
datatype=node
generics=[]
clarifiers=[]

path=$.a[0]
kind=NodeHead
value=tag

path=$.a[0][0]
kind=StringLiteral
value=hello
```

Binding-head metadata remains on the outer `NodeLiteral` record. Metadata
written on the AEON node head belongs to the indexed `NodeHead` record. Their
attributes are independently representable at `$.a.@.x` and `$.a[0].@.x`.

Portable AES does not require exactly one node head. It can represent zero or
multiple ordered heads so future source forms do not require a new event shape.
AEON and other source profiles may impose their own cardinality.

In the portable path domain, head `h` is at `[h]` below its node and child `i`
of that head is at `[h][i]`. For example, a second head is `$.a[1]` and its
first child is `$.a[1][0]`.

For `a = <tag(<tag>, <tag>)>`, the inner node containers occur at
`$.a[0][0]` and `$.a[0][1]`; their heads occur at `$.a[0][0][0]` and
`$.a[0][1][0]`.

## 6. Paths and AEON projection

Portable AES paths are canonical SANSA data addresses. The document root `$`
is not itself an event.

AEON source paths address values in the source-language structure. AES event
paths address records in the expanded flat structure. Translation is
structure-aware, not a blind rewrite of index strings.

Let `S` be an AEON source path for a node and `E(S)` its AES event path:

| Source occurrence | AES event path |
| --- | --- |
| node container at `S` | `E(S)` |
| synthetic node head | `E(S)[0]` |
| node child at `S[i]` | `E(S)[0][i]` |
| NodeHead attribute | beneath `E(S)[0].@` |

This table is specific to current AEON, whose source paths expose the children
of one implicit head. It is not a source-neutral meaning for `S[i]`. A future
multi-head AEON syntax or another source language must version and define its
own structure-aware mapping into the portable `[head-index][child-index]`
layout.

Member, attribute, list-index, and tuple-index segments otherwise retain their
meaning. Translation recurses across every node boundary:

| AEON source path | AES event path |
| --- | --- |
| `$.a` | `$.a` |
| `$.a[0]` | `$.a[0][0]` |
| `$.a[0][0]` where both indexed occurrences are node children | `$.a[0][0][0][0]` |

Reference payloads use the AES event-path domain. When `a` is a node, AEON
`~a[0]` becomes `kind=CloneReference,value=$.a[0][0]`; `~>a[0]` produces the
same target with `kind=PointerReference`. Identity never participates in
translation or path comparison.

Reverse projection removes a head index only when structural context proves
that the represented parent is a node and the following index addresses its
content. A direct AES reference to a synthetic head such as `$.a[0]` has no
reference form in current AEON. An AEON serializer reports it as
unrepresentable rather than reinterpreting it as `~a[0]`.

A partial stream without sufficient ancestry needs external profile state for
translation. A legacy stream in which `$.a[0]` meant the first node child needs
an explicit versioned compatibility adapter and must not be silently read as
the revised node head.

## 7. Provenance and spans

The portable span form is `start-byte:end-byte`. Both positions are zero-based
UTF-8 byte offsets into the exact source identified by `origin`; start is
inclusive and end is exclusive. `start-byte` must be less than `end-byte`.
Line and column views are derived and are not transported.

The allowed combinations are:

| `origin` | `span` | Meaning |
| --- | --- | --- |
| absent | absent | no portable source evidence |
| present | absent | exact source known, exact record range unknown |
| present | present | exact source and byte range known |
| absent | present | invalid |

AES v0 defines one origin form:

```text
sha256:<64 lowercase hexadecimal digits>
```

The digest covers the exact unnormalized source bytes, including any accepted
source BOM and original line endings. It identifies evidence, not document or
structural identity. Origin is record-local so one stream can combine records
from multiple immutable sources. Encodings may compress repetition without
changing the decoded contract.

A source-less record omits both fields. A producer with known source but no
exact range carries `origin` alone rather than inventing a zero span. Semantic
value hashes exclude `origin` and `span`; a provenance-aware signature profile
may explicitly include them.

Local event validation checks canonical integer syntax, the non-empty ordered
range, and the presence and syntax of `origin`. It does not load an external
artifact. When exact source bytes are available, a source-backed provenance
audit additionally verifies the SHA-256 digest, range bounds, and that both
endpoints fall on UTF-8 scalar boundaries. An unavailable artifact leaves
provenance unverified without making the locally valid record invalid.

If an available artifact does not match `origin`, the audit reports
`AES_ORIGIN_MISMATCH`. If its range is out of bounds or either endpoint splits
a UTF-8 scalar, it reports `AES_INVALID_SPAN`.

### 7.1 NodeHead span

A source-backed NodeHead span begins at the first byte of its tag token and
ends after its last head component: datatype, attribute block, structural
identity, or tag in that precedence. It includes quoted tag delimiters,
identity delimiters, attributes, datatype arguments, and intervening bytes.

It excludes `<`, trivia before the tag, child delimiters and children, `>`, and
surrounding binding-head syntax. In
`<tag\HEAD\@{role = "button"}:node("hello")>`, the head range contains
`tag\HEAD\@{role = "button"}:node`.

Overlapping ranges for the outer node, node head, and attribute descendants are
valid. An adapter that knows only the complete node-literal span carries
`origin` without `span` for the head; it does not substitute the whole node
range.

## 8. Ordering

Event order is significant and every encoding preserves it exactly. AES does
not impose one universal sort: source projections, snapshots, transactions,
and ledgers may each have different authoritative sequences.

Canonical AEON document projection uses depth-first preorder. For each binding
it emits:

1. the binding event;
2. the binding's attribute subtrees in declaration order;
3. structural descendant subtrees in declaration or ascending index order; and
4. the next sibling binding.

Each parent precedes its descendants and each subtree is contiguous. Object
members and attributes preserve declaration order. List and tuple items, node
heads, and NodeHead content preserve ascending index order. Paths, datatype
components, values, and identities are never implicit sort keys.

A signature profile states which sequence it covers. A semantic document
signature may cover a profile-defined canonical projection; a ledger signature
covers supplied order exactly. AES does not infer a signing mode from content.

## 9. Completeness profiles

### 9.1 `aes.complete.v0`

`aes.complete.v0` is the default. It claims a self-contained stream that a
consumer can navigate without external state.

The completeness claim does not select or alter processor resource limits.

Every non-root structural prefix has a material record and each parent kind is
compatible with its child segment. Parents are never inferred. An attribute
requires its owner but no synthetic `.@` container. Node content requires both
its `NodeLiteral` and `NodeHead` ancestry. Addresses and structural identities
are unique.

Every `CloneReference` and `PointerReference` target identifies a body event in
the stream. Target existence does not impose target-before-reference ordering.

Only a member record may occur directly beneath the unrepresented `$` root.
Named children require `ObjectNode` parents. Indexed children require
`ListNode`, `TupleLiteral`, or `NodeHead` parents. Every direct indexed child of
a `NodeLiteral` is a `NodeHead`, and a `NodeHead` cannot be an ordinary list or
tuple item.

An AEON canonical document projection satisfies this profile.
`aeon.gp.profile.v1` declares `aes.complete.v0` explicitly even though omission
selects the same default.

### 9.2 `aes.partial.v0`

`aes.partial.v0` retains event-local validity but relaxes body cross-record
constraints. Records still require canonical addresses, known kinds,
kind-correct value presence, valid core fields, and other locally decidable
rules.

The stream may omit ancestors, repeat paths or identities, and use delivery or
ledger order. Parent compatibility is not asserted even when a parent happens
to be present. This supports filtered streams, transaction fragments,
subscriptions, and ledger entries without treating arbitrary field maps as AES
events.

Transaction and ledger profiles may establish completeness against prior state
plus a supplied segment. They must be selected explicitly.

## 10. Optional AEON document projection

Default AEON-to-AES projection is body-only. Parsed `aeon:header` and shorthand
`aeon:*` declarations are not injected into the body stream.

`aeon.document.v0` explicitly adds a flat header control plane. Header records
use `header` instead of `path`; their canonical address begins with one
non-empty quoted `aeon:` member:

```text
header=$.["aeon:mode"]
kind=StringLiteral
value=strict

header=$.["aeon:conventions"]
kind=ListNode

header=$.["aeon:conventions"][0]
kind=StringLiteral
value=aeon.gp.security.v1
```

Structured and shorthand AEON headers normalize to the same records. Header
records use the ordinary kind, datatype components, identity, value, and
provenance fields. They precede body records, preserve source declaration
order, and use depth-first preorder for descendants.

Header and body addresses are disjoint. When this projection is selected, the
header plane is complete independently: addresses are unique, all ancestors
exist, and parent kinds are compatible. This remains true when the body profile
is partial. An AEON document with no header may produce an empty header plane.

No record or delimiter represents the plane boundary. Each record selects its
plane through `header` or `path`; once a body record occurs, a later header
record is invalid. If the header plane is empty, the first record may be a body
record.

An AEON source `aeon:profile` claim does not select or replace the AES stream
profile. Unknown projections fail semantic validation.

Body semantic hashes and body signatures exclude header records. A document
signature includes the ordered header plane followed by the ordered body plane
and declares that broader scope. This projection preserves header semantics,
not structured-versus-shorthand spelling or exact source lexemes.

## 11. Fidelity boundaries

### 11.1 Portable record fidelity

For a supported encoding version, encode/decode preserves stream profile and
projection, record order, every address, every scalar field, and every
structured datatype component. This includes provenance and syntactically
valid unknown fields relayed without semantic interpretation.

Encoding canonicalization may alter physical whitespace, field order, line
endings, and escape spelling. It is record-lossless when decoding the canonical
result yields the same stream context and ordered records; it need not preserve
tolerant input bytes.

### 11.2 Portable semantic losslessness

A projection is semantically lossless only relative to its selected profile and
projection. Equivalence preserves:

- address plane and address;
- kind, canonical value, expanded datatype components, and identity;
- every profile-significant extension; and
- the order the selected profile declares authoritative.

Origin and span are provenance, not semantic-equivalence inputs. Dropping them
loses record and provenance fidelity without changing the portable semantic
value. An unknown field prevents a semantic-losslessness claim unless the
active profile defines it.

Body-only projection can be semantically lossless for the body without claiming
to preserve the complete AEON document. Header semantics require
`aeon.document.v0` in the equivalence boundary.

For an AEON-representable stream, reconstructed canonical AEON is a valid
semantic round trip when projecting it again yields an equivalent AES stream
under the same context. Original source-byte equality is not required.

### 11.3 Exact source fidelity

Portable AES does not carry raw lexemes. It intentionally discards or
normalizes whitespace, comments, quote and escape choices, trimtick delimiters
and indentation, numeric separator spelling, header shorthand, raw AST shape,
BOM, and line endings.

Origin and span identify evidence but do not embed it. Exact source bytes are
recoverable only from a separately retained artifact whose bytes match origin;
they cannot be reconstructed from AES records alone.

Portable AES is semantically lossless with respect to its selected event
profile and projection. It is not an exact source-roundtrip format.

## 12. Validation and diagnostics

Validation has two transport-neutral layers after an encoding has decoded its
syntax:

1. record shape: address cardinality, required and allowed fields, core field
   grammar, and kind-dependent value presence;
2. stream semantics: selected context, address uniqueness, ordering, ancestry,
   container compatibility, identity uniqueness, and profile rules.

Canonical payload grammars owned by SANSA, AEON Core, the Aeonic type contract,
or the Aeonic Semantic Language remain separate validation points. AES v0
reference validators locally enforce only the payload rules defined here,
including exact lowercase WTC `local`.

### 12.1 Local diagnostics

Portable local diagnostics use these stable codes; prose is not normative:

| Code | Condition |
| --- | --- |
| `AES_UNSUPPORTED_PROFILE` | selected profile is unsupported |
| `AES_UNSUPPORTED_PROJECTION` | selected projection is unsupported |
| `AES_INVALID_EVENT` | host input is not a record |
| `AES_INVALID_PAYLOAD` | a scalar field payload is not a string |
| `AES_MISSING_ADDRESS` | record has neither `path` nor `header` |
| `AES_MULTIPLE_ADDRESSES` | record has both `path` and `header` |
| `AES_MISSING_FIELD` | required `kind` is absent |
| `AES_UNKNOWN_FIELD` | field is neither core nor registered |
| `AES_INVALID_PATH` | body address is not canonical |
| `AES_INVALID_HEADER_PATH` | header address is malformed or lacks its leading quoted `aeon:` member |
| `AES_HEADER_REQUIRES_PROJECTION` | `header` occurs without `aeon.document.v0` |
| `AES_HEADER_ORDER` | header record occurs after a body record |
| `AES_UNKNOWN_KIND` | value kind is not portable |
| `AES_MISSING_VALUE` | a valued kind lacks `value` |
| `AES_UNEXPECTED_VALUE` | a value-less kind carries `value` |
| `AES_INVALID_VALUE` | locally specified value payload is invalid |
| `AES_INVALID_REFERENCE` | reference target is not a canonical body event path |
| `AES_MISSING_REFERENCE_TARGET` | complete stream omits a syntactically valid reference target |
| `AES_EMPTY_FIELD` | optional datatype or identity is present but empty |
| `AES_DATATYPE_COMPONENTS` | datatype, generics, and clarifiers do not occur together as required |
| `AES_INVALID_DATATYPE` | datatype name, generic tree, or tagged clarifier is malformed |
| `AES_DATATYPE_DEPTH` | logical datatype structure exceeds the active generic-depth limit |
| `AES_DATATYPE_LIMIT` | a logical datatype argument-count, clarifier-count, or component-count limit is exceeded |
| `AES_LIMIT_EXCEEDED` | a caller-selected portable event-model resource limit is exceeded |
| `AES_INVALID_ORIGIN` | origin is not a canonical AES v0 source digest |
| `AES_SPAN_REQUIRES_ORIGIN` | span occurs without origin |
| `AES_INVALID_SPAN` | span syntax or ordering is invalid |
| `AES_DUPLICATE_PATH` | complete plane repeats an address |
| `AES_DUPLICATE_IDENTITY` | complete stream repeats an identity |
| `AES_MISSING_PARENT` | complete plane omits required ancestry |
| `AES_INCOMPATIBLE_PARENT` | child segment is incompatible with parent kind |
| `AES_INVALID_NODE_HEAD` | node head is not a direct indexed child of a node |

### 12.2 Source-backed audit diagnostics

These diagnostics require access to the exact external source artifact and are
not emitted by a local record validator:

| Code | Condition |
| --- | --- |
| `AES_ORIGIN_MISMATCH` | available source bytes do not match the declared origin digest |

`AES_INVALID_SPAN` is also used by a source-backed audit when a range exceeds
the artifact or splits a UTF-8 scalar boundary.

## 13. Relationship to the Aeonic Semantic Language

Portable AES recognizes and transports value distinctions. It does not define
all operations over those values.

For example, AES can carry `kind=RadixLiteral`, `datatype=decimal`, empty
`generics` and `clarifiers`, and `value=010.00`. The Aeonic Semantic Language
owns equality, comparison, ordering, conversion, measurement, and later
arithmetic for that value.

```text
recognized producer value
          |
          v
   portable AES event
      /          \
     v            v
telex.aes      film.aes
     \            /
      v          v
  Aeonic Semantic Language
```

This separation lets every AES encoding carry the same event without making an
encoding the owner of event semantics.
