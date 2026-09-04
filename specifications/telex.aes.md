# `telex.aes` Draft 0

Status: exploratory draft

Format version: `0`

File suffix: `.telex.aes`

## 1. Purpose

`telex.aes` is the textual interchange format for an ordered Assignment Event
Stream.

Its job is intentionally narrow:

> Encode already-recognized AES events so another component can reconstruct
> them without parsing AEON source, applying type inference, or depending on a
> producer's in-memory object shape.

Telex and the future `film.aes` encode the same portable AES event model. Telex
optimizes for trivial implementation, inspection, fixtures, diffs, and protocol
debugging. Film will optimize for binary transport and size.

This draft is a candidate to discuss and test. The version number `0` is not a
compatibility promise.

## 2. Design constraints

Telex should be:

- UTF-8 text;
- streamable one record at a time;
- expressible as flat `field=value` lines;
- deterministic in canonical form;
- independent of JSON and host-language data models;
- explicit about value kind and declared datatype;
- able to carry all information in the portable AES event contract; and
- small enough that a syntax implementation can be audited end to end.

Compactness is secondary. An AEON document may be smaller because its syntax
allows context to be inferred. Telex makes that context explicit.

## 3. Example

Given this illustrative AEON input:

```aeon
customer:object = {
  name:string = "Alice"
  balance:decimal = %010.00
}
```

A Telex projection could be:

```text
telex.aes=0

path=$.customer
kind=object
datatype=object
origin=sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
span=0:73

path=$.customer.name
kind=string
datatype=string
value=Alice
origin=sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
span=22:43

path=$.customer.balance
kind=radix
datatype=decimal
value=010.00
origin=sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
span=46:71
```

The object event carries its own representation kind. Its children are ordinary
subsequent events. `value=010.00` is a textual payload whose meaning is fixed by
`kind=radix` and `datatype=decimal`; it is not parsed as a Telex number.
The repeated digest is illustrative; a real producer computes it from the exact
source bytes.

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

The preamble may be followed by one profile declaration:

```text
profile=aes.partial.v0
```

It may also carry one optional projection declaration:

```text
projection=aeon.document.v0
```

`profile` selects AES validation constraints. `projection` selects an explicit
source-to-AES projection contract and is a separate axis. A canonical stream
places `profile` before `projection` when both are present. Omitting
`projection` selects the ordinary body-only AES stream; there is no implicit
source-language projection.

The stream header is followed by a blank line when at least one record follows.
Future format versions use a different preamble value, not an inferred feature
set. The profile selects semantic constraints within that format version.

An omitted profile declaration means `aes.complete.v0`. This default is
normative, not a request for profile negotiation. A producer that requires
cross-event constraints to be relaxed must declare `aes.partial.v0` explicitly.

Profile and projection payloads use Telex payload escaping. Draft 0 permits at
most one declaration of each and rejects an empty identifier. Syntax readers
preserve unknown non-empty identifiers; semantic consumers reject profile or
projection identifiers they do not support.

### 4.3 Record framing

A record is a non-empty stanza of `field=value` lines. One or more blank lines
separate stanzas. A canonical encoder emits exactly one blank line between
records. A body record is an AES event. An explicitly selected projection may
also define control records, such as the AEON header records in section 5.11.

The first `=` on a line separates the field name from its payload. Later `=`
characters belong to the payload and do not need escaping.

Field names use lowercase ASCII letters, digits, `-`, and `.`. A field and each
dotted field segment begin with a letter. Empty payloads are valid. Empty field
names, duplicate fields in one record, and lines without `=` are invalid.

There are no comments. Comments would create a second information channel and
would complicate byte-for-byte canonicalization.

### 4.4 Payload escaping

Payloads are single logical lines. The Telex escape vocabulary is:

| Escape | Decoded scalar |
| --- | --- |
| `\\\\` | backslash |
| `\\n` | line feed |
| `\\r` | carriage return |
| `\\t` | horizontal tab |
| `\\0` | null |
| `\\u{H...}` | Unicode scalar written as 1-6 uppercase hexadecimal digits |

An unescaped C0 control or DEL is invalid. A surrogate, a value above
`U+10FFFF`, an unknown escape, and a lowercase or padded canonical Unicode
escape are invalid in canonical input.

Canonical encoders use the short escapes where available, escape other C0
controls and DEL with `\\u{...}`, and emit all other Unicode scalars directly.

Escaping is defined by Telex. It is not JSON, JavaScript, Rust, or AEON source
escaping.

### 4.5 Syntax diagnostic codes

Portable conformance vectors identify syntax failures by stable code. Error
prose is diagnostic presentation and is not compared for conformance.

| Code | Condition |
| --- | --- |
| `TELEX_BOM` | forbidden byte-order mark |
| `TELEX_BARE_CR` | carriage return not followed by line feed |
| `TELEX_INVALID_PREAMBLE` | missing, malformed, or unsupported preamble |
| `TELEX_EMPTY_PROFILE` | empty explicit profile identifier |
| `TELEX_EMPTY_PROJECTION` | empty explicit projection identifier |
| `TELEX_DUPLICATE_STREAM_FIELD` | profile or projection declared more than once |
| `TELEX_MISSING_HEADER_SEPARATOR` | missing blank line before records |
| `TELEX_INVALID_FIELD_LINE` | record line is not `field=value` |
| `TELEX_INVALID_FIELD_NAME` | field name violates the Telex grammar |
| `TELEX_DUPLICATE_FIELD` | field occurs more than once in one record |
| `TELEX_UNESCAPED_CONTROL` | payload contains an unescaped control scalar |
| `TELEX_INCOMPLETE_ESCAPE` | payload ends during an escape |
| `TELEX_UNKNOWN_ESCAPE` | escape is not in the Telex vocabulary |
| `TELEX_UNTERMINATED_UNICODE_ESCAPE` | Unicode escape has no closing brace |
| `TELEX_INVALID_UNICODE_ESCAPE` | Unicode escape digits are malformed |
| `TELEX_INVALID_UNICODE_SCALAR` | payload or escape does not represent a Unicode scalar |

## 5. Portable event profile

The following is the Draft 0 candidate event profile. It needs reconciliation
with the AES specification and the implementations before promotion.

### 5.1 Core fields

| Field | Presence | Meaning |
| --- | --- | --- |
| `header` | address-dependent | canonical address in an explicitly selected header plane |
| `path` | address-dependent | canonical SANSA data address of a body binding |
| `kind` | required | portable value-kind token |
| `datatype` | optional | declared datatype, without inference |
| `identity` | optional | structural identity carried by the binding |
| `value` | kind-dependent | decoded textual payload |
| `origin` | optional | immutable identity of the exact source bytes |
| `span` | optional; requires `origin` | original source byte range |

Every stanza has exactly one address field: `path` for a body event or
`header` for a control-plane record defined by an explicit projection. A stanza
with neither or both is invalid. The address field provides the distinction, so
Draft 0 has no additional `event=assignment` or record-type discriminator.

`key` is not transported because it is derivable from the final segment of the
selected address. A normalized selector path is also derived data and is not
transported.

The order of `path` records is the AES event order. An encoder MUST NOT sort
records by either address.

### 5.2 Datatypes

`datatype` carries the complete canonical datatype descriptor as one textual
payload. It is not limited to a bare type name.

For example:

```text
datatype=csv["."]
datatype=list<int>
datatype=null<string>
```

The AEON binding `:` is not part of the descriptor. Generic arguments and
clarifiers are retained in their canonical form because they are declared
semantic claims, not source formatting.

A datatype remains only on the event where it was declared. AES does not infer
or propagate `int` from `list<int>` onto the list's child events. Interpreting a
container generic, validating it against descendants, or assigning inherited
meaning is downstream work.

Telex treats the descriptor as a payload. Its grammar and canonical rendering
belong to the portable AES/Aeonic type contract rather than the Telex framing
grammar.

### 5.3 Spans

The portable span form is:

```text
start-byte:end-byte
```

Both positions are zero-based UTF-8 byte offsets into the exact source resource
identified by provenance. The start is inclusive and the end is exclusive. A
valid span has `start-byte <= end-byte`, and both endpoints fall on UTF-8 scalar
boundaries.

Line and column coordinates are not transported. They are derived diagnostic
views and different host languages otherwise tend to count bytes, Unicode
scalars, or UTF-16 code units inconsistently.

Both provenance fields are optional. Their permitted combinations are:

| `origin` | `span` | Meaning |
| --- | --- | --- |
| absent | absent | no portable source evidence |
| present | absent | exact source known, exact record location unknown |
| present | present | exact source and byte range known |
| absent | present | invalid |

Draft 0 defines one origin form:

```text
origin=sha256:<64 lowercase hexadecimal digits>
```

The digest is computed over the exact, unnormalized source byte sequence.
Consequently it includes an accepted source BOM, original line endings, and
every other source byte. It identifies source evidence; it is not a document or
structural identity. A record-local origin permits one stream to combine
records derived from different immutable sources without a source table or
stream-wide assumption. Repetition is intentional in Telex; Film may compress
repeated origins without changing the AES contract.

A source-backed producer may emit `origin` alone when it cannot establish an
exact range. A record created without source evidence omits both fields rather
than inventing `0:0` or a placeholder digest. Telex syntax treats both as
ordinary payloads, while the AES record-shape validator enforces their grammar
and dependency. Semantic value hashes exclude `origin` and `span`;
provenance-aware signature profiles may explicitly include them.

### 5.4 Value kinds

The portable value-kind vocabulary and its `value` payloads are:

| `kind` | `value` payload |
| --- | --- |
| `string` | decoded Unicode string; an empty payload is valid |
| `number` | canonical finite numeric text |
| `infinity` | `Infinity` or `-Infinity` |
| `nan` | `NaN` or `-NaN` |
| `null` | recognized sentinel or decoded custom reason |
| `boolean` | `true` or `false` |
| `toggle` | `yes`, `no`, `on`, or `off` |
| `hex` | lowercase hexadecimal digits, without `#` or visual underscores |
| `radix` | canonical radix payload, without `%` or visual underscores |
| `encoding` | encoding payload without `&`; padding is preserved |
| `separator` | canonical separator payload without `^` |
| `sansa-address` | canonical SANSA address |
| `date` | canonical date text |
| `time` | canonical time text |
| `datetime` | canonical date-time text |
| `wtc` | canonical world-time-context text |
| `object` | absent |
| `list` | absent |
| `tuple` | absent |
| `node` | absent |
| `node-head` | node tag |
| `clone-reference` | canonical target path |
| `pointer-reference` | canonical target path |

`value` is required for every kind except `object`, `list`, `tuple`, and
`node`, for which it is absent. Payloads are strings at the Telex layer even
when their kind gives them numeric, temporal, or other semantics.

AEON source spelling is normalized before the event enters AES. In particular,
quoted strings, backtick strings, and trimtick strings all use `kind=string`.
A trimtick payload has already undergone its trimming operation; delimiter
width and indentation are source mechanics rather than properties of the
resulting value. Numeric separators and the leading AEON sigils for hex,
radix, encoding, and separator values are likewise not transported.

The `separator` payload remains an opaque canonical value in AES. Its declared
datatype and clarifiers may assign structure or interpretation downstream;
Telex does not split it into a nested payload.

Container events carry their kind but no nested value tree. Every member or
item follows as an event at its own descendant canonical path. The same rule
applies recursively to object members, list and tuple elements, node children,
and values reached through attribute address spaces.

A node head uses `value` for its tag. A reference uses `value` for its canonical
target path. Nulls use `value` for their recognized sentinel or decoded custom
reason. No kind introduces a kind-specific field name.

Clone and pointer references remain distinct kinds even when they target the
same canonical path:

```text
path=$.copy
kind=clone-reference
value=$.source

path=$.alias
kind=pointer-reference
value=$.source
```

The AEON `~` and `~>` sigils are not retained in `value`; their distinction is
carried by `kind`. A Telex decoder preserves the symbolic reference and does not
resolve or materialize it.

The table defines the portable AES value representation. A generic Telex syntax
parser need not validate it, but an AES event-shape validator must.

### 5.5 Anonymous heads and structural identity

An anonymous typed or attributed value does not introduce a wrapper event or a
special value kind. Its datatype, structural identity, and value kind belong to
the event at its indexed path.

For example:

```aeon
values:list = [
  \item-1\@{source:string = "user"}:int = 3
]
```

projects as:

```text
path=$.values
kind=list
datatype=list

path=$.values[0]
kind=number
datatype=int
identity=item-1
value=3

path=$.values[0].@.source
kind=string
datatype=string
value=user
```

The `identity` payload omits the AEON backslash delimiters. It is occurrence
metadata and does not alter the event path or stream order. Telex does not
invent an identity when none was supplied. Document-level uniqueness is an AES
semantic constraint rather than Telex framing syntax.

### 5.6 Nodes

A node is an ordered structural container. Its node heads are ordinary flat
descendant events, addressed by index. A `node-head` carries the tag in `value`
and owns that head's ordered content values.

Current AEON syntax produces one head at index zero:

```aeon
a:node = <tag("hello", 2)>
```

```text
path=$.a
kind=node
datatype=node

path=$.a[0]
kind=node-head
value=tag

path=$.a[0][0]
kind=string
value=hello

path=$.a[0][1]
kind=number
value=2
```

Binding-head metadata remains on the `node` event. Metadata written on the AEON
node head belongs to its `node-head` event, so both heads remain independently
representable without a new namespace:

```text
$.a.@.x
$.a[0].@.x
```

Telex does not impose a cardinality of exactly one node head. The flat model can
represent an empty node or multiple ordered heads without changing its path or
event grammar. Whether a source language or negotiated AES profile permits
zero, one, or many heads is a semantic constraint. This preserves room for
future forms equivalent to `<>` and `<<tag>, <tag>>` while current AEON remains
single-headed.

Nested nodes follow directly from the same rule. For
`a = <tag(<tag>, <tag>)>`, the inner node containers occur at `$.a[0][0]` and
`$.a[0][1]`; their heads occur at `$.a[0][0][0]` and `$.a[0][1][0]`.

#### 5.6.1 AEON source paths and AES event paths

AEON source paths address values in the source-language structure. Portable
AES paths address events in the expanded flat structure. They are separate
path domains even when their canonical strings happen to be equal.

An AEON adapter translates paths using the structure of the document, not by
rewriting index strings in isolation. Let `S` be the AEON source path of a node
and `E(S)` its translated AES event path:

| Source occurrence | Portable AES event path |
| --- | --- |
| node container at `S` | `E(S)` |
| the node's synthetic head | `E(S)[0]` |
| node child at `S[i]` | `E(S)[0][i]` |
| node-head attribute | beneath `E(S)[0].@` |

Member, attribute, list-index, and tuple-index segments otherwise retain their
meaning. Translation recurses, so every crossed node boundary introduces a
head index. For example:

| AEON source path | Portable AES event path |
| --- | --- |
| `$.a` | `$.a` |
| `$.a[0]` | `$.a[0][0]` |
| `$.a[0][0]` where both indexed occurrences are node children | `$.a[0][0][0][0]` |

The payload of a `clone-reference` or `pointer-reference` is in the portable
AES event-path domain. During AEON-to-AES projection, `~a[0]` therefore becomes
`kind=clone-reference,value=$.a[0][0]` when `a` is a node. `~>a[0]` produces the
same translated payload with `kind=pointer-reference`. Structural identity is
never an input to translation or path comparison.

Reverse projection removes a head index only when the represented parent is a
node and the following index addresses that head's content. A direct portable
reference to a synthetic node-head event, such as `$.a[0]`, has no reference
form in current AEON. AES may represent it, but an AEON serializer must report
that target as unrepresentable rather than reinterpret it as `~a[0]`.

Consequently translation requires structural context. A partial stream that
does not carry enough ancestry needs external profile state before it can be
translated. A legacy stream in which `$.a[0]` meant the first node child also
requires an explicit versioned compatibility adapter; it must not be silently
read as the revised node-head event.

#### 5.6.2 Node-head source span

When source provenance is available, the `node-head` span covers the complete
AEON node head. It begins at the first byte of the tag token and ends
immediately after the last head component: the datatype when present,
otherwise the attribute block or structural identity when present, otherwise
the tag token. The range includes the complete quoted tag token, identity
delimiters, attributes, datatype arguments, and any intervening source bytes.

The span excludes the opening `<`, trivia before the tag, child-list
delimiters, children, the closing `>`, and any surrounding binding-head syntax.
For example, the node-head evidence in
`<tag\HEAD\@{role = "button"}:node("hello")>` is the byte range containing
`tag\HEAD\@{role = "button"}:node`.

The outer `node`, its `node-head`, and descendant attribute events may have
overlapping source ranges. An adapter that knows the source identity but only
has the whole node-literal span emits `origin` without `span` for the
`node-head`; it must not substitute the whole node span.

### 5.7 Flat structure and attributes

AES has one flat event model. It does not embed attribute collections inside an
owning event and Telex does not have an attribute-specific payload grammar.

For example:

```aeon
a@{a={a=1, b@{a=2}=2}}=0
```

is represented by events at:

```text
$.a
$.a.@.a
$.a.@.a.a
$.a.@.a.b
$.a.@.a.b.@.a
```

Each event uses the same `path`, `kind`, datatype, identity, value, and
provenance fields as any other event.

The `.@` segment is part of the canonical path. Telex preserves it but does not
interpret attribute ownership or scope. SANSA and downstream consumers own
address-space interpretation, just as consumers interpret object members,
indexed elements, and node children.

Telex preserves event order exactly. It does not derive a new order from path
structure and it never groups, sorts, or nests attribute-space events.

### 5.8 Losslessness and source lexemes

The word lossless applies only when its boundary is stated. This specification
distinguishes portable record fidelity, portable semantic losslessness, and
exact source fidelity.

#### 5.8.1 Portable record fidelity

For a supported Telex version, encoding and decoding preserve the stream
profile and projection, record order, every address, and every field payload.
This includes provenance and syntactically valid unknown fields when a generic
format tool relays them without claiming semantic conformance.

Canonicalization may change Telex whitespace, field order, separator count,
line endings, and escape spelling. It is record-lossless because decoding the
canonical result yields the same header and ordered records; it is not a
byte-preserving transformation of tolerant input.

#### 5.8.2 Portable semantic losslessness

A projection is semantically lossless only relative to its explicitly selected
AES event profile and projection. Portable semantic equivalence preserves:

- the `path` or `header` address plane and address;
- `kind`, canonical `value`, `datatype`, and structural `identity`;
- every profile-significant extension; and
- the event order that the selected profile declares authoritative.

`origin` and `span` are provenance rather than inputs to portable semantic
equivalence. Dropping them loses record and provenance fidelity, even though it
does not change the portable semantic value. An unknown field prevents a
semantic-losslessness claim unless the active profile defines its meaning; a
generic relay can still preserve it with record fidelity.

The default body-only projection can be semantically lossless for the AES body
without claiming to preserve the complete AEON document. When header semantics
must survive, the `aeon.document.v0` projection is part of the equivalence
boundary.

For an AEON-representable stream, a reconstructed canonical AEON document is a
valid semantic round trip when projecting it again produces an equivalent AES
stream under the same profile and projection. Equality with the original AEON
source bytes is not required.

#### 5.8.3 Exact source fidelity

AES does not carry the exact original source token or a `lexeme` field. It
intentionally discards or normalizes source-authoring choices such as
whitespace, comments, quote and escape choice, trimtick delimiters and
indentation, numeric separator spelling, header shorthand, raw AST shape, a
source BOM, and line endings.

`kind`, `datatype`, and `value` preserve the recognized portable distinctions.
`origin` and `span` identify source evidence but do not embed it. Exact source
bytes can be recovered only when the separately retained artifact whose bytes
match `origin` is available; they cannot be reconstructed from AES records
alone.

Portable AES is therefore semantically lossless with respect to its selected
event profile and projection. It is not an exact source-roundtrip format. This
boundary also prevents canonical Telex bytes from changing merely because two
sources use different spellings for the same portable AES value.

### 5.9 Event order

Event order is significant and Telex preserves it exactly. AES core does not
impose one universal structural sorting algorithm: a source projection, a
materialized snapshot, an incremental transaction, and an append-only ledger
may each have a different authoritative order.

A canonical AEON document projection uses depth-first preorder. For each
binding it emits:

1. the binding event;
2. the binding's attribute subtrees in declaration order;
3. structural descendant subtrees in declaration or ascending index order; and
4. the next sibling binding.

Each parent therefore precedes its descendants and each projected subtree is
contiguous. Object members and attribute entries preserve declaration order.
List and tuple items, node heads, and node-head content preserve ascending
index order. Paths, datatypes, values, and structural identities are never used
as implicit sort keys.

Other producer profiles may define another order. Telex canonicalization only
canonicalizes the encoding of the supplied sequence; it never changes that
sequence.

Signature profiles must state which sequence they cover. A semantic document
signature may first project events into the canonical order defined by its
profile. A ledger signature covers the original event order exactly. Telex
does not infer one signing mode from the event content.

### 5.10 Completeness and profiles

`aes.complete.v0` is the default AES profile. It requires a complete,
self-contained stream that a consumer can navigate without external state. A
profile declaration is optional only because omission selects
`aes.complete.v0`; omission does not select a partial mode.

Under `aes.complete.v0`, every non-root structural prefix has a material event
and each parent kind is compatible with its child segment. Parents are never
inferred or synthesized. An attribute event requires its owning event, but
`.@` does not require a synthetic attribute-space container event. Node content
requires both its `node` and `node-head` ancestry. Event shape, path uniqueness,
and the other portable event requirements in this specification also apply.
The document root `$` is not itself an event, so only a member event may occur
directly beneath it; an indexed or attribute-space event requires a represented
owner below the root.

`aes.partial.v0` retains event-local validity but relaxes cross-event
constraints. Its events still require canonical paths, known value kinds,
correct kind-dependent value presence, valid core fields, and any other rule that can
be decided from that event alone. The stream may omit ancestors, repeat paths
or structural identities, and carry events in delivery or ledger order. Parent
compatibility is not asserted even when a parent happens to be present.

For example, a partial stream may contain an event at `$.a.b` without carrying
`$.a`; it can represent an incremental event, filtered stream, transaction
fragment, subscription, or ledger entry without claiming to be independently
navigable AES state. Arbitrary `field=value` stanzas are not thereby valid
partial AES events.

Transaction and ledger profiles may establish completeness against prior state
plus the supplied segment rather than against the segment alone. They must be
selected explicitly. A canonical AEON document projection is complete.

Completeness is an AES profile constraint, not a Telex syntax constraint. A
syntax parser therefore may successfully decode an incomplete default-profile
stream so that it can report the semantic failure.

The reference codec exposes `checkTelexCompleteness(input)` and
`checkPrefixCompleteness(records)` as lightweight diagnostics. They report
missing structural prefixes without reordering events. They do not check
container-kind compatibility, uniqueness, references, or any other claim of a
complete AES profile. Callers checking an explicit header plane also provide
its projection context.

The normative completeness rule belongs to AES. `aeon.gp.profile.v1` explicitly
declares that its AES projection satisfies `aes.complete.v0`, even though the same
profile would be selected by omission in a Telex stream. The GP profile
references this AES-owned rule rather than redefining it.

### 5.11 Optional AEON document projection

AEON-to-AES projection is body-only by default. Recognized `aeon:header` or
shorthand `aeon:*` declarations are consumed while parsing and validating the
AEON source but are not silently injected into the portable body event stream.

A producer that needs semantic header preservation selects the encoding-neutral
`aeon.document.v0` projection explicitly:

```text
telex.aes=0
projection=aeon.document.v0

header=$.["aeon:mode"]
kind=string
value=strict

path=$.a
kind=number
value=1
```

The projection normalizes structured and shorthand AEON headers into the same
flat records. Each header record uses `header` instead of `path`, followed by
the ordinary `kind`, `datatype`, `identity`, `value`, and provenance fields.
Its canonical address begins with one non-empty quoted `aeon:` member. Nested
header values are flattened beneath that member:

```text
header=$.["aeon:conventions"]
kind=list

header=$.["aeon:conventions"][0]
kind=string
value=aeon.gp.security.v1
```

Header and body address spaces are disjoint, so a body binding cannot collide
with control metadata at the same textual address. Header records precede all
body events and preserve AEON header declaration order, using depth-first
preorder for descendants. Telex never reorders either plane.

`aes.complete.v0` and `aes.partial.v0` continue to govern the body. Whenever
`aeon.document.v0` is selected, the included header plane is complete:
addresses are unique, every structural ancestor is present, and parent kinds
are compatible. This remains true when the body profile is partial. A document
projection may contain no header records when the source had no AEON header.

A `header` record without this projection is invalid rather than ignored.
Unknown projections remain syntax-preservable but fail semantic validation.
The AEON `aeon:profile` declaration is an advisory source/application claim;
it does not select or replace the stream's AES `profile` declaration.

Body semantic hashes and body signatures exclude `header` records. A document
signature includes the ordered header plane followed by the ordered body plane
and must declare that broader scope. Exact structured-versus-shorthand syntax,
original header lexemes, and other source-authoring choices are not preserved
by this semantic projection.

The reference validators use these stable semantic diagnostic codes:

| Code | Condition |
| --- | --- |
| `AES_UNSUPPORTED_PROJECTION` | selected projection is not supported |
| `AES_MISSING_ADDRESS` | record has neither `path` nor `header` |
| `AES_MULTIPLE_ADDRESSES` | record has both `path` and `header` |
| `AES_HEADER_REQUIRES_PROJECTION` | `header` occurs without `aeon.document.v0` |
| `AES_INVALID_HEADER_PATH` | header address is malformed or lacks its leading quoted `aeon:` member |
| `AES_HEADER_ORDER` | header record occurs after a body event |
| `AES_INVALID_ORIGIN` | origin is not a canonical Draft 0 source digest |
| `AES_SPAN_REQUIRES_ORIGIN` | span occurs without source identity |

## 6. Canonical form

A canonical Telex encoder emits:

1. the exact version preamble;
2. the profile declaration when one was explicitly supplied;
3. the projection declaration when one was explicitly supplied;
4. one blank line before the first record;
5. header records followed by body events in their semantic stream order;
6. fields in the order below;
7. extension fields in Unicode-code-point order after core fields;
8. the shortest canonical escape for each payload scalar;
9. one blank line between records; and
10. exactly one final LF.

Core field order:

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
separators, CRLF, and lowercase hexadecimal in Unicode escapes. It must expose
that the input was non-canonical when canonical bytes matter.

## 7. Validation layers

Telex deliberately separates three checks:

1. **Syntax:** UTF-8, framing, field grammar, duplicates, and escapes.
2. **Record shape:** exactly one address, required and allowed fields, and
   value-kind rules.
3. **Stream semantics:** projection selection, canonical addresses,
   plane-specific uniqueness, ordering, structural consistency, datatypes,
   references, and profile rules.

A tiny Telex parser may implement only layer 1. It must not claim AES
conformance merely because it can split fields.

The reference `validateTelex` helper implements the event-local checks whose
grammars are defined in this draft. For `aes.complete.v0`, it additionally checks
path and structural-identity uniqueness, required ancestry, parent/child
container compatibility, and node-head placement. For `aes.partial.v0`, it omits
those body cross-event checks. For `aeon.document.v0`, it independently checks
header ordering and complete header ancestry and compatibility.

Canonical payload grammars owned elsewhere remain separate validation points.
In particular, the helper does not substitute implementation-specific rules
for canonical numeric, radix, encoding, separator, SANSA-address, temporal,
world-time-context, datatype, or structural-identity grammars that this draft
does not define. Full semantic conformance requires those owning contracts once
published.

## 8. Unknown fields and versions

Draft 0 syntax parsers accept and preserve any syntactically valid field. They
include unknown fields in canonical output and never interpret or discard them.
Extension fields use the convention `x.<owner>.<name>` and sort with other
non-core fields after the core field list. The `x` prefix does not mean that a
field is optional or safe to ignore.

A standalone AES semantic decoder rejects every unknown field unless an
explicitly selected profile registers that exact field and defines its
validation. Enabling one extension does not enable other fields from the same
owner. Extensions cannot redefine core fields.

A generic relay, inspector, canonicalizer, or store may preserve and forward
unknown fields without understanding them, but it cannot claim semantic AES
conformance for those events. A component must not sign a transformed event
after silently dropping unknown fields.

Silently discarding unknown data would make round trips lossy. Treating an
unknown field as harmless without profile knowledge could change meaning.

Profile and projection negotiation belong to an enclosing protocol such as
`poem.aes`. Opening a standalone file with an unsupported version fails
explicitly. A new required core field requires a new Telex version; it is not
introduced as an extension to `telex.aes=0`.

## 9. Security and resource bounds

Decoders must accept caller-supplied limits for at least:

- input bytes;
- line bytes;
- fields per record;
- record count;
- decoded payload bytes; and
- canonical path depth.

Syntax decoding performs no reference resolution, schema loading, network
access, datatype execution, or source-language evaluation.

## 10. Draft 1 exit criteria

Draft 1 should not be declared until the settled format and profile rules exist
as conformance vectors in at least two independent implementations, not only as
prose. Transport-specific media types and external registration are outside the
Draft 0 format decision gates.

The repository-local `conformance/telex/v0` manifest is the mutable Draft 0
development snapshot. The JavaScript and Rust reference implementations run it
independently. Draft 1 still requires both to pass the eventual frozen snapshot,
not merely this mutable development copy.

## 11. Relationship to the Aeonic Semantic Language

Telex records representation without applying higher-order value behavior.

For example, Telex can carry that a payload has kind `radix`, datatype
`decimal`, and value `010.00`. The Aeonic Semantic Language owns questions such
as equality, ordering, conversion, precision, and arithmetic for that value.

This keeps the layers distinct:

```text
Telex / Film
    carry recognized values

Aeonic Semantic Language
    defines shared operations over values

AEOS / SANSA / Tonics / storage
    apply those operations in their own domains
```
