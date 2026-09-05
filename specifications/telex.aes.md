# `telex.aes` v0

Scope: textual framing, escaping, canonical encoding, and decoding of portable
AES event streams.

Format version: `0`

File suffix: `.telex.aes`

Portable event contract: [`aes.events.md`](./aes.events.md)

## 1. Purpose

`telex.aes` is the textual interchange encoding for an ordered Assignment
Event Stream.

Its job is intentionally narrow:

> Encode portable AES records so another component can recover them without
> parsing AEON source or depending on a producer's in-memory object shape.

The transport-neutral AES contract defines record meaning, value kinds,
structure, profiles, projections, provenance, ordering, and semantic fidelity.
Telex defines only their text framing, escaping, canonical bytes, and syntax
diagnostics. The future `film.aes` will encode the same records with different
physical tradeoffs.

This draft is a candidate to discuss and test. Format version `0` is not a
compatibility promise.

## 2. Design constraints

Telex should be:

- UTF-8 text;
- streamable one record at a time;
- expressible as flat `field=value` lines;
- deterministic in canonical form;
- independent of JSON and host-language data models;
- able to carry every portable AES field without reinterpretation; and
- small enough that a syntax implementation can be audited end to end.

Compactness is secondary. Source formats may infer context; Telex writes the
decoded portable record explicitly.

## 3. Example

Given this illustrative AEON input:

```aeon
customer:object = {
  name:string = "Alice"
  balance:decimal = %010.00
}
```

a Telex projection could be:

```text
telex.aes=0

path=$.customer
kind=ObjectNode
datatype=object
origin=sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
span=0:73

path=$.customer.name
kind=StringLiteral
datatype=string
value=Alice
origin=sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
span=22:43

path=$.customer.balance
kind=RadixLiteral
datatype=decimal
value=010.00
origin=sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
span=46:71
```

The records and their field meanings come from the portable AES contract.
Telex treats every payload, including `010.00`, as text. The repeated digest is
illustrative; a real producer computes it from the exact source bytes.

## 4. Physical format

### 4.1 Encoding

A Telex file is UTF-8 without a byte-order mark.

Canonical Telex uses LF (`U+000A`) line endings and ends with one LF. A decoder
MAY accept CRLF and normalize it before parsing. It MUST reject a bare CR and
invalid UTF-8.

Telex performs no Unicode normalization. A conforming implementation preserves
the decoded Unicode scalar sequence exactly.

### 4.2 Stream header

The first line is exactly:

```text
telex.aes=0
```

This preamble establishes both Telex encoding version `0` and its static
mapping to portable event contract `aes.events.v0`. A Telex reader does not
need a second contract declaration. Future Telex versions state their event
contract mapping in their own specification.

The preamble may be followed by one profile declaration:

```text
profile=aes.partial.v0
```

and one optional projection declaration:

```text
projection=aeon.document.v0
```

`profile` and `projection` are the independent context axes defined by
`aes.events.v0`. A canonical stream places `profile` before `projection`.
Omitting profile selects `aes.complete.v0`; omitting projection selects the
ordinary body-only stream.

The stream header is followed by a blank line when at least one record follows.
A canonical zero-record stream ends with exactly one LF immediately after its
last stream-header line. Therefore `telex.aes=0\n` is the canonical empty
default stream; an explicit profile or projection, when present, becomes the
last line before that LF. There is no trailing blank line.

A tolerant decoder may accept EOF without the final LF or one or more blank
lines after zero-record metadata, but marks either form non-canonical. Header-
plane records are records and therefore require the ordinary blank separator.
Future Telex versions use another preamble value rather than an inferred
feature set.

Profile and projection identifiers use Telex payload escaping. Telex v0 permits
at most one declaration of each and rejects an empty identifier. A syntax
reader preserves an unknown non-empty identifier; semantic validation rejects
unsupported context.

### 4.3 Record framing

A record is a non-empty stanza of `field=value` lines. One or more blank lines
separate stanzas. A canonical encoder emits exactly one blank line between
records.

Each stanza encodes one portable AES record without adding a record-type field.
The first `=` on a line separates the field name from its payload; later `=`
characters belong to the payload and need no escaping.

Field names use this ASCII grammar:

```text
segment = [a-z][a-z0-9-]*
field   = segment ("." segment)*
```

Consequently, every dotted segment begins with a lowercase ASCII letter;
`x.owner.flag` and `x.base42.flag` are valid, while `x.42base.flag` is not.
Empty payloads are valid. Empty field names, duplicate fields in one record,
and lines without `=` are invalid.

There is no separate header/body delimiter among records. Each stanza selects
its plane with exactly one `header` or `path` field. Ordinary blank stanza
separators apply within and between planes. Semantic validation forbids a
`header` record after the first `path` record; an empty header plane has no
marker.

There are no comments. Comments would introduce a second information channel
and complicate canonicalization.

### 4.4 Payload escaping

Payloads are single logical lines. Telex uses this escape vocabulary:

| Escape | Decoded scalar |
| --- | --- |
| `\\` | backslash |
| `\n` | line feed |
| `\r` | carriage return |
| `\t` | horizontal tab |
| `\0` | null |
| `\u{H...}` | Unicode scalar written as 1-6 uppercase hexadecimal digits |

An unescaped C0 control or DEL is invalid. A surrogate, a value above
`U+10FFFF`, and an unknown or malformed escape are invalid.

A canonical encoder emits backslash as `\\`; emits null, tab, LF, and CR as
`\0`, `\t`, `\n`, and `\r`; emits every other C0 control and DEL as the shortest
`\u{H...}` form with uppercase hexadecimal digits and no leading zeroes; and
emits every other Unicode scalar directly. Thus U+000B becomes `\u{B}`, U+007F
becomes `\u{7F}`, and U+0041 becomes `A`.

Lowercase or padded Unicode escapes are decodable but non-canonical. They are
not malformed merely because their spelling is non-canonical.

This escaping is Telex-specific. It is not JSON, JavaScript, Rust, or AEON
source escaping.

### 4.5 Syntax diagnostic codes

Portable vectors identify syntax failures by stable code. Diagnostic prose is
not compared for conformance.

| Code | Condition |
| --- | --- |
| `TELEX_BOM` | forbidden byte-order mark |
| `TELEX_BARE_CR` | carriage return not followed by line feed |
| `TELEX_INVALID_PREAMBLE` | missing, malformed, or unsupported preamble |
| `TELEX_EMPTY_PROFILE` | empty explicit profile identifier |
| `TELEX_EMPTY_PROJECTION` | empty explicit projection identifier |
| `TELEX_DUPLICATE_STREAM_FIELD` | profile or projection occurs more than once |
| `TELEX_MISSING_HEADER_SEPARATOR` | missing blank line before records |
| `TELEX_INVALID_FIELD_LINE` | record line is not `field=value` |
| `TELEX_INVALID_FIELD_NAME` | field name violates Telex grammar |
| `TELEX_DUPLICATE_FIELD` | field occurs more than once in one record |
| `TELEX_UNESCAPED_CONTROL` | payload contains an unescaped control scalar |
| `TELEX_INCOMPLETE_ESCAPE` | payload ends during an escape |
| `TELEX_UNKNOWN_ESCAPE` | escape is outside the Telex vocabulary |
| `TELEX_UNTERMINATED_UNICODE_ESCAPE` | Unicode escape has no closing brace |
| `TELEX_INVALID_UNICODE_ESCAPE` | Unicode escape digits are malformed |
| `TELEX_INVALID_UNICODE_SCALAR` | payload or escape is not a Unicode scalar |
| `TELEX_INVALID_DATATYPE` | compact datatype descriptor cannot be expanded |
| `TELEX_DATATYPE_LIMIT` | a datatype generic-depth, argument-count, clarifier-count, or component-count limit is exceeded |
| `TELEX_LIMIT_EXCEEDED` | a caller-selected Telex physical or stream resource limit is exceeded |

A bare CR uses `TELEX_BARE_CR`. An LF ends the current physical line, so
following payload text without `=` is a new malformed line and uses
`TELEX_INVALID_FIELD_LINE`. Other unescaped C0 controls in a payload use
`TELEX_UNESCAPED_CONTROL`. Telex has no generic trailing-character error:
everything after the first `=` belongs to the payload and may instead fail
later AES field validation.

## 5. Mapping the portable AES contract

Each decoded stanza is one record defined by
[`aes.events.md`](./aes.events.md). Telex performs no type inference, reference
resolution, source evaluation, or nested AST reconstruction while parsing.

The standard physical core field names are:

```text
header
path
kind
datatype
identity
value
origin
span
```

The physical `datatype` line is the one deliberate compact mapping in Telex. It
combines the logical AES `datatype`, `generics`, and `clarifiers` fields using
canonical AEON datatype-descriptor syntax without the leading `:`:

```text
datatype=csv["."]
datatype=list<int>
datatype=null<string>
datatype=tuple<tuple<int, 2>[16], string>["x", "x"]
```

A decoder expands each such line into a base-name string and the two ordered
arrays defined by `aes.events.v0`. A datatype argument becomes a recursive
descriptor, a numeric generic argument becomes a tagged `NumberLiteral`, and a
clarifier becomes a tagged `StringLiteral` or `NumberLiteral`. Numeric payloads
remain strings. An encoder performs the inverse mapping. `generics` and
`clarifiers` are therefore logical AES fields, not separate Telex lines; using
either as a physical field name fails with `TELEX_INVALID_DATATYPE`.

A record without `datatype` has neither logical array. A decoded record with
`datatype` always has both arrays, even when empty. Descriptor array order and
duplicates are preserved. The Telex decoder applies the generic-depth and
datatype-component limits selected by its consumer before accepting a record.
Telex does not derive either value from the stream's semantic profile.

Field presence, payload meaning, value kinds, canonical paths, identity,
provenance, completeness, ordering, and the `aeon.document.v0` control plane
are AES rules. A Telex syntax parser may successfully decode a record or stream
that later fails those semantic rules.

Telex syntax preserves supplied record order exactly. It never groups records,
sorts by address, or infers an order from path structure.

## 6. Canonical Telex

A canonical encoder emits:

1. the exact version preamble;
2. `profile` when explicitly supplied;
3. `projection` when explicitly supplied;
4. one blank line before the first record;
5. records in their supplied order;
6. core fields in the order below;
7. extension fields in Unicode-code-point order after core fields;
8. the shortest canonical escape for every payload scalar;
9. one blank line between records; and
10. exactly one final LF.

Core field order is:

```text
header
path
kind
datatype
identity
value
origin
span
```

A tolerant decoder may accept non-canonical field order, multiple stanza
separators, CRLF, and lowercase or padded hexadecimal in Unicode escapes. It
exposes that the input was non-canonical when canonical bytes matter.
`TELEX_INVALID_UNICODE_ESCAPE` is reserved for an undecodable escape; a
canonical verifier rejects an otherwise decodable spelling through the
canonicality result or comparison with canonical re-encoding, not through a
lowercase-specific syntax diagnostic.

Canonicalization changes only Telex representation. It does not reorder AES
records or rewrite decoded AES scalar strings, including WTC anchor and
reference spelling. It does reconstruct the compact `datatype` line from the
expanded logical fields, using `, ` between ordered array entries and JSON
double-quoted spelling for string clarifiers.

## 7. Validation layers

Telex separates three checks:

1. **Telex syntax:** UTF-8, framing, field-name grammar, duplicates, and
   escapes.
2. **AES record shape:** address cardinality, required and allowed fields, and
   locally defined value-kind rules.
3. **AES stream semantics:** selected context, address and identity uniqueness,
   ordering, ancestry, container compatibility, and profile rules.

A tiny parser may implement only layer 1. It must not claim AES conformance
merely because it can split fields.

The reference `validateTelex` helper implements the event-local rules defined
by the portable contract. Under `aes.complete.v0`, it additionally checks path
and identity uniqueness, ancestry, parent compatibility, and node-head
placement. Under `aes.partial.v0`, it omits body cross-record checks. Under
`aeon.document.v0`, it independently checks header ordering, ancestry, and
compatibility.

`checkTelexCompleteness(input)` and `checkPrefixCompleteness(records)` provide
lighter ancestry diagnostics without claiming full profile validation.

Canonical payload grammars owned by other contracts remain separate validation
points. The reference helper does not substitute host-specific rules for them.
AES semantic diagnostic codes are defined in
[`aes.events.md`](./aes.events.md#12-validation-and-diagnostics).

## 8. Unknown fields and versioning

A Telex v0 syntax parser accepts and preserves every syntactically valid field,
including unknown fields. Canonical output includes them after core fields.
Telex does not interpret the `x.<owner>.<name>` convention or treat such fields
as optional.

Semantic consumers apply the AES extension rules. A generic Telex relay may
preserve unknown data without claiming semantic conformance.

Opening a standalone file with an unsupported Telex version fails explicitly.
Profile and projection negotiation belongs to an enclosing protocol such as
`poem.aes`. A new required Telex framing feature requires another Telex version;
a new required AES core field requires another event-contract version.

## 9. Security and resource bounds

Codecs and validators accept caller-supplied normalized integer limits for:

- input bytes;
- line bytes;
- fields per record;
- record count;
- decoded payload bytes;
- canonical path depth and Unicode-scalar length when semantic validation is enabled;
- datatype generic depth;
- generic arguments per generic list;
- clarifier values per clarifier list; and
- total descriptor, generic-argument, and clarifier components per record.

Decoded-payload bytes are the cumulative UTF-8 byte length of decoded profile,
projection, and record-field payloads. Physical line byte counts exclude the LF
or CRLF delimiter. Limits are inclusive: an observed count equal to its limit
is accepted and the first greater count is rejected.

Numeric values are consumer implementation policy rather than Telex format or
AES semantic-profile rules. Implementations document their defaults and expose
the normalized effective values. A profile claim in a limits file is metadata;
it does not let an input stream select or relax those limits.

Syntax decoding performs no reference resolution, schema loading, network
access, datatype execution, or source-language evaluation.

## 10. Draft 1 exit criteria

Draft 1 should not be declared until the settled Telex syntax and referenced
AES event rules exist as conformance vectors in at least two independent
implementations.

The repository-local `conformance/telex/v0` manifest remains a mutable v0
candidate with no snapshot identity. Stable conformance is split by authority:
`telex-cts-v0-snapshot-0.1` owns 50 syntax, canonicalization, and format-limit
vectors, while `aes-events-cts-v0-snapshot-0.1` owns 38 transport-neutral event
and profile-validation vectors. Both immutable manifests are published in the
shared CTS repository with per-suite content digests and pass independently in
the JavaScript and Rust implementations. This satisfies the conformance
prerequisite for Draft 1; lifecycle promotion remains a separate publication
decision.

Transport media types and external registration are outside the v0 format
decision gates.

## 11. Relationship to AES semantics

Telex is an encoding, not the owner of event or value semantics. It preserves
the portable scalar strings, structured datatype components, and ordering
defined by the AES contract.

The Aeonic Semantic Language may later compare, convert, measure, or calculate
with those values. No such operation changes how Telex frames a record.
