# `film.aes` v1

Status: normative draft; not a released conformance target

Scope: binary framing, canonical encoding, and decoding of portable AES event
streams.

Format version: `1`

File suffix: `.film.aes`

Preamble: `4F 5F 5F FF 01` (`O__`, binary sentinel, version `1`)

Portable event contract: [`aes.events.md`](./aes.events.md)

## 1. Purpose and authority

`film.aes` is the canonical binary interchange encoding for an ordered
Assignment Event Stream. It carries the same logical stream context and records
as [`telex.aes`](./telex.aes.md) without reproducing Telex text or AEON source.

The transport-neutral AES contract owns record fields, value kinds, paths,
identity, profiles, projections, provenance, ordering, and semantic validity.
Film owns only:

- the binary preamble and format version;
- stream-context encoding;
- record and field framing;
- fixed control and kind codes;
- binary primitive encodings;
- canonical Film bytes;
- Film-local resource counters; and
- Film syntax and canonicality diagnostics.

Film v1 maps statically to `aes.events.v1`. An incompatible binary grammar or
mapping requires another Film format version. A new AES event contract does not
silently change the meaning of Film v1.

## 2. Design invariants

Film v1 has these invariants:

1. Every accepted Film stream recovers one ordered portable AES stream without
   type inference or source evaluation.
2. Records are inline and physically independent: given its complete framed
   bytes and the stream context, a record requires no dictionary, preceding
   address, reset state, or other record to decode.
3. Canonical Film has one accepted physical representation and no tolerant
   noncanonical decoding mode.
4. AES scalar values remain canonical Unicode strings. Film does not substitute
   host integers, floating-point values, booleans, or binary numeric formats.
5. Supplied event order and explicit stream-context declaration presence are
   preserved.
6. Physical canonicality does not imply AES semantic validity, authorization,
   transaction completion, or commit.

Film is semantically lossless relative to the selected portable AES profile and
projection. It does not preserve an originating AEON document's exact spelling,
comments, whitespace, trimtick boundaries, or nested parser representation.

## 3. Binary notation and primitive encodings

All offsets and lengths in this specification count bytes. Multi-byte integer
fields use the canonical unsigned LEB128 primitive defined below. Control,
kind, tag, and version fields are fixed single bytes and do not use LEB128.

### 3.1 Canonical unsigned LEB128

An unsigned LEB128 value consists of one to ten bytes. Bits `0` through `6` of
each byte carry the next seven low-order value bits. Bit `7` is one when another
byte follows and zero on the final byte.

The encoded domain is `0` through `2^64 - 1`. A conforming decoder MUST reject:

- an encoding longer than ten bytes;
- a tenth byte whose payload is greater than one;
- a tenth byte with its continuation bit set;
- an encoding that exceeds the `u64` domain; and
- any representation longer than the shortest encoding of its value.

Examples:

| Value | Bytes |
| ---: | --- |
| `0` | `00` |
| `1` | `01` |
| `127` | `7F` |
| `128` | `80 01` |
| `255` | `FF 01` |

Thus `80 00` is not another spelling of zero; it is noncanonical Film.

An implementation checks conversion to its host address-space type before
allocation, slicing, addition, or pointer arithmetic. The ten-byte bound is a
grammar invariant rather than a configurable resource limit.

### 3.2 Film strings

The ordinary Film string representation is:

```text
string := utf8-byte-length:uLEB128 utf8-bytes[utf8-byte-length]
```

The length counts encoded UTF-8 bytes, not Unicode scalar values. The bytes MUST
be valid UTF-8 and are preserved without Unicode normalization. Whether an
empty string is valid depends on the logical field. A decoder checks
`max_field_bytes` before reading, buffering, or allocating the declared bytes.

### 3.3 Fixed bytes

The following are always one byte:

- Film version;
- stream-context control;
- record control;
- kind code;
- datatype generic tag; and
- datatype clarifier tag.

Reserved fixed-byte values and reserved bits are invalid. They are not
forward-compatible aliases.

## 4. Stream structure

A Film v1 stream has this physical shape:

```text
stream := preamble stream-context record*

preamble := 4F 5F 5F FF 01

stream-context :=
  context-control:u8
  [profile:string]
  [projection:string]

record := record-payload-length:uLEB128 record-payload[record-payload-length]
```

There is no whole-stream byte length, record count, final marker, padding,
alignment, checksum, or trailer.

### 4.1 Preamble

Every Film v1 stream begins with exactly:

```text
4F 5F 5F FF 01
```

The first three bytes display as `O__`. Byte `FF` is the binary sentinel and is
invalid as a standalone UTF-8 byte. Byte `01` is the fixed Film format version.

A reader MUST validate all five bytes before interpreting stream context. A
different final byte is not a feature bitmap; it identifies another or
unsupported format version. Film has no byte-order mark.

### 4.2 Stream context

One mandatory context-control byte immediately follows the preamble:

| Bit | Meaning when one |
| ---: | --- |
| `0` | an explicit profile string follows |
| `1` | an explicit projection string follows after profile, when present |
| `2`-`7` | reserved; MUST be zero |

When bit `0` is zero, the effective profile is `aes.complete.v1`. When bit `1`
is zero, the stream has the body-only projection. A present profile or
projection string MUST be non-empty.

Syntax decoding preserves an unknown non-empty identifier. AES semantic
validation rejects an unsupported profile or projection. The profile and
projection axes remain independent.

Explicit declaration presence is part of canonical stream context. Therefore
an omitted profile and an explicitly encoded `aes.complete.v1` profile have the
same effective AES behavior but remain byte-distinct Film streams.

The canonical default zero-record stream is exactly:

```text
4F 5F 5F FF 01 00
```

Exact EOF after the context fields represents zero records.

### 4.3 Record framing

Each record is one canonical unsigned-LEB128 payload length followed by exactly
that many payload bytes. The length excludes its own bytes.

A record payload length MUST be greater than zero. The decoder checks
`max_record_bytes` before reading, buffering, allocating, or skipping the
payload. The payload boundary must be consumed exactly by the record grammar.

Exact EOF after a complete record terminates the stream. EOF inside a record
length, declared payload, nested descriptor, string, origin, or span is
truncation.

The outer record length permits a framing-only relay to copy or skip a complete
bounded record without understanding its fields. It does not make an unknown
kind or malformed record a portable AES event.

## 5. Record payload

The record payload has this fixed positional layout:

```text
record-payload :=
  record-control:u8
  kind:u8
  address:string
  [datatype-descriptor]
  [identity:string]
  [value:string]
  [origin:bytes[32]]
  [span-start:uLEB128 span-end:uLEB128]
  extension*

extension := name:string value:string
```

Core fields have no individual field tags. Presence comes from record control
except for value, whose presence derives from kind. The fixed order above is
canonical and cannot be rearranged.

### 5.1 Record control

| Bit | Meaning when one |
| ---: | --- |
| `0` | address maps to logical `header`; otherwise it maps to `path` |
| `1` | datatype descriptor is present |
| `2` | identity is present |
| `3` | origin is present |
| `4` | span is present |
| `5`-`7` | reserved; MUST be zero |

Span presence requires origin presence. The control byte does not select AES
profile, record validity, transaction role, encryption, or compression.

### 5.2 Address

Address is one ordinary Film string containing the canonical SANSA address used
by the selected logical `path` or `header` field. Record-control bit `0` selects
the address plane; a record cannot carry both fields.

Film framing validates length and UTF-8. AES validation owns non-emptiness,
canonical SANSA syntax, path limits, header-path restrictions, and structural
meaning. Film v1 has no binary path-segment grammar, prefix compression, or
reference to a previous address.

`SansaAddressLiteral`, `CloneReference`, and `PointerReference` payloads are
ordinary value strings. They do not reuse the address field encoding or create
an implicit target link.

### 5.3 Identity and value

Identity and value both use the ordinary Film string representation:

```text
identity := string
value    := string
```

Identity syntax, identity uniqueness, and whether an empty value is valid come
from `aes.events.v1`. Value bytes encode the canonical AES string payload for
the selected kind. Film never substitutes a host-native representation.

Value presence is derived from kind. `ObjectNode`, `ListNode`, `TupleLiteral`,
and `NodeLiteral` omit value. Every other assigned Film v1 kind requires value,
including `NodeHead`, `CloneReference`, and `PointerReference`.

## 6. Kind codes

Film kind codes are local to this wire version. They preserve the corresponding
`aes.events.v1` kind exactly.

| Code | AES kind | Value |
| ---: | --- | --- |
| `01` | `StringLiteral` | required |
| `02` | `NumberLiteral` | required |
| `03` | `InfinityLiteral` | required |
| `04` | `NaNLiteral` | required |
| `05` | `NullLiteral` | required |
| `06` | `BooleanLiteral` | required |
| `07` | `ToggleLiteral` | required |
| `08` | `HexLiteral` | required |
| `09` | `RadixLiteral` | required |
| `0A` | `EncodingLiteral` | required |
| `0B` | `SeparatorLiteral` | required |
| `0C` | `SansaAddressLiteral` | required |
| `0D` | `DateLiteral` | required |
| `0E` | `TimeLiteral` | required |
| `0F` | `DateTimeLiteral` | required |
| `10` | `WTCDateTimeLiteral` | required |
| `11` | `ObjectNode` | omitted |
| `12` | `ListNode` | omitted |
| `13` | `TupleLiteral` | omitted |
| `14` | `NodeLiteral` | omitted |
| `15` | `NodeHead` | required |
| `16` | `CloneReference` | required |
| `17` | `PointerReference` | required |

Code `00` and codes `18` through `FF` are unassigned and invalid in Film v1. A
semantic decoder rejects them because it cannot determine the logical kind or
value-presence contract. A framing-only relay may copy or skip the complete
outer record opaquely but cannot expose it as AES or claim canonical record
validation.

Adding an AES core kind requires another event-contract version and a Film
version mapped to that contract. Unassigned codes are not an extension space.

## 7. Datatype descriptors

When record-control bit `1` is one, one framed datatype descriptor follows the
address:

```text
datatype-descriptor := descriptor-byte-length:uLEB128 descriptor-body

descriptor-body :=
  base-name:string
  generic-count:uLEB128
  generic[generic-count]
  clarifier-count:uLEB128
  clarifier[clarifier-count]

generic :=
  00 datatype-descriptor
  02 number-value:string

clarifier :=
  01 string-value:string
  02 number-value:string
```

The descriptor byte length counts only the descriptor body and MUST be consumed
exactly. Every nested datatype generic carries its own descriptor length.

The base name MUST be non-empty. `generic-count` and `clarifier-count` are
literal entry counts. The decoder reads exactly those counts and rejects an
unknown tag, missing entry, or trailing descriptor byte.

Generic tag `00` carries a recursive datatype descriptor. Generic tag `02`
carries a `NumberLiteral` string. Clarifier tag `01` carries a `StringLiteral`
string and tag `02` carries a `NumberLiteral` string. Numeric payloads remain
strings and are validated by AES rather than converted to host numbers.

Order and duplicates are preserved in both arrays. An absent datatype control
bit means logical `datatype`, `generics`, and `clarifiers` are absent. A present
descriptor always yields a base datatype and both arrays, even when their
counts are zero.

The consumer checks descriptor byte length and the selected shared generic
depth, generic argument, clarifier value, and datatype component limits before
descending or allocating. Limit exhaustion rejects the descriptor; it never
truncates the logical arrays.

## 8. Extensions

After all present core fields, every remaining byte in a record payload belongs
to an ordered sequence of extension pairs:

```text
extension := name:string value:string
```

There is no extension count or terminator; the record boundary ends the
sequence. A name without its paired value is truncation.

Film v1 extension names use this ASCII grammar:

```text
segment        := [a-z][a-z0-9-]*
extension-name := "x" "." segment "." segment ("." segment)*
```

The first segment is literal `x`, the second identifies the owner, and at least
one following segment names the field. `x.example.claim` is valid;
`example.claim`, `x.42base.claim`, and `x.example` are invalid.

Extensions MUST occur in ascending Unicode-code-point order by complete name.
Because names are ASCII, this is also ascending UTF-8 byte order. Duplicate or
out-of-order names are invalid.

Extension values are opaque Unicode strings at the Film layer. Syntax readers
and generic relays preserve every valid pair. Semantic acceptance requires the
selected trusted profile or consumer context to register the exact field name.
Unknown extension meaning cannot be inferred from its owner or spelling.

## 9. Provenance and spans

When record-control bit `3` is one, origin is exactly 32 inline bytes containing
the SHA-256 digest. It has no algorithm tag and no length prefix. Decoding
reconstructs the portable value:

```text
sha256:<64 lowercase hexadecimal digits>
```

Film v1 can use this fixed representation because `aes.events.v1` admits only
that origin form. Another origin algorithm requires another governing contract
and incompatible Film mapping.

When record-control bit `4` is one, span immediately follows origin and is:

```text
span := start-byte:uLEB128 end-byte:uLEB128
```

`start-byte` MUST be less than `end-byte`. Offsets retain their AES meaning as a
half-open range in the exact source bytes identified by origin. Origin without
span is valid; span without origin is invalid.

Film performs no source retrieval, digest verification, UTF-8 boundary audit,
or authority inference. Those operations remain with the source-backed AES
audit contract. `NodeHead` span meaning remains the tag token range defined by
`aes.events.v1`.

## 10. Canonical Film

Film v1 accepts only one physical representation. A decoder MUST reject rather
than normalize a noncanonical form.

Canonical Film requires:

1. the exact five-byte preamble and mandatory context byte;
2. zero reserved context and record-control bits;
3. explicit profile and projection only in their fixed order;
4. shortest unsigned-LEB128 integers and lengths;
5. exact declared byte boundaries;
6. valid UTF-8 without normalization;
7. positive record payload lengths;
8. fixed positional core-field order;
9. an assigned kind and matching kind-derived value presence;
10. exact datatype counts, tags, and nested boundaries;
11. exactly 32 origin bytes when present;
12. origin whenever span is present and `start < end`;
13. strictly ordered, non-duplicate extension names;
14. supplied event order without implicit sorting; and
15. exact EOF without padding, alignment, terminators, or trailing data.

Film v1 has no dictionaries, path prefixes, string tables, aliases, table
resets, compression mode, or record-local alternate spellings. External
compression may wrap the complete canonical Film bytes but does not change
their identity.

For accepted values:

```text
decode(encode(logical-stream)) = logical-stream
encode(decode(canonical-bytes)) = canonical-bytes
```

Physical canonicality remains separate from AES semantic validity. Canonical
bytes do not authorize an unknown extension, make an unsupported profile valid,
repair an invalid path, or satisfy completeness.

## 11. Validation layers and decoded-record usability

A Film consumer distinguishes at least these layers:

1. **Framing:** preamble, context and record byte boundaries can be located
   within active byte limits.
2. **Film record decoding:** control, kind, fields, descriptors, provenance,
   extensions, UTF-8, and physical canonicality are valid.
3. **AES event-local validation:** the decoded logical record satisfies its
   address, field, datatype, identity, value-kind, and provenance rules.
4. **AES stream validation:** final EOF and the selected profile and projection
   checks succeed across the complete ordered record stream.

A framing view is not an AES event. A locally validated event remains
provisional until final stream validation. A completed-stream result exists only
after the caller declares final input, the decoder reaches exact EOF, and every
selected cross-record rule succeeds.

Consumers may inspect provisional events. Replay, database ingestion, mutation,
and other side effects requiring a complete stream MUST stage them until final
acceptance or use an enclosing transaction protocol. Decoder yield, local
validity, or Film EOF alone does not constitute authorization or commit.

## 12. Incremental decoding, truncation, and recovery

Before final input is declared, an incomplete integer, field, record, or stream
context produces a need-more-input state. After the caller declares final EOF,
the same condition is `FILM_TRUNCATED`.

Any malformed, noncanonical, semantically impossible, or limit-exceeding input
stops ordinary decoding at the first invalid byte. Diagnostics SHOULD include:

- absolute byte offset;
- zero-based record ordinal when known; and
- the enclosing component, such as context, record length, datatype, or span.

Film v1 has no in-band resynchronization marker or recovery mode. A decoder MUST
NOT scan for `4F 5F 5F FF 01` after corruption because those bytes may appear in
a length-delimited payload. Recovery requires a trusted enclosing journal,
container, or transport boundary.

Film detects EOF inside a declared structure. It cannot detect removal of one
or more complete final records when the remaining prefix is independently valid.
Detecting that case requires independently established expected length, record
count, digest, completion evidence, or transaction framing.

## 13. Resource limits

Film applies the shared structural counters selected by the consumer for the
decoded `aes.events.v1` stream. Those include event count, path depth and
characters, attribute depth, represented value nesting, string and key-segment
code points, list and tuple items, generic depth and arguments, clarifiers, and
datatype components.

Film v1 additionally defines these format-local counters:

| Counter | Measurement |
| --- | --- |
| `max_input_bytes` | all bytes in one standalone Film stream, including preamble and context |
| `max_record_bytes` | record payload bytes, excluding its length prefix |
| `max_field_bytes` | bytes after one field-length prefix, including context strings, addresses, descriptor bodies, and extension names or values |
| `max_buffered_bytes` | bytes retained internally by an incremental codec, including encoder scratch and excluding caller-owned input and returned borrowed views |

Limits are inclusive. An observation equal to its limit is accepted; the first
greater observation is rejected. Consumers select effective limits and may
impose lower immutable safety ceilings. A limits-file profile claim is metadata
and cannot allow an input stream to select or relax resource policy.

The proposed AltoPelago defaults for a future limits-file revision that claims
Film support are:

| Counter | Value |
| --- | ---: |
| `max_input_bytes` | `67108864` (64 MiB) |
| `max_record_bytes` | `16777216` (16 MiB) |
| `max_field_bytes` | `4194304` (4 MiB) |
| `max_buffered_bytes` | `16777216` (16 MiB) |

These values do not modify the already published `altopelago.aeonic-limits.v1`
`1.0.0` file. A later version may encode an unbounded consumer choice as
`!"unBound"` and an implementation-selected choice as `!"useImplementation"`
according to the limits contract.

There is no configurable varint limit because ten bytes is a grammar maximum.
There are no table limits and no decoded-expansion counter because Film v1 uses
neither dictionaries nor decompression.

Implementations MUST check every length conversion and addition before
allocation or pointer arithmetic. Film-format exhaustion uses
`FILM_LIMIT_EXCEEDED`; shared logical exhaustion retains the AES diagnostic
defined by `aes.events.v1`.

## 14. Encoder and API contract

Record and descriptor lengths precede their payloads. A conforming encoder uses
one of these strategies while producing the same canonical bytes:

1. calculate the exact encoded length before writing once to a forward-only
   sink;
2. buffer one bounded record or descriptor before writing its shortest length
   and payload; or
3. backpatch a seekable destination without reserved width or padding.

A forward-only encoder MUST NOT emit a placeholder, overlong integer, or
fixed-width length. Public APIs expose checked sizing, bounded buffering, or
both. A record encoder alone does not imply that its bytes form a standalone
Film stream; complete streams also require preamble and context.

Native implementations SHOULD support lifetime-bounded borrowed record views
and explicit owned materialization. They MUST document when a borrowed view
expires and MUST NOT hide a copy behind an API described as borrowed.

JavaScript and WASM use `Uint8Array` or an equivalent exact byte view. Records
returned across a WASM boundary are owned unless a callback-scoped or equivalent
lifetime mechanism prevents them from outliving backing memory.

Exact function and class names are implementation-specific. Framing,
provisional events, and completed streams nevertheless use distinct result
types or states rather than an easy-to-ignore validation flag.

## 15. Transcoding with Telex

A Telex-to-Film or Film-to-Telex transcoder operates on portable AES stream
context and records. It MUST preserve:

- effective profile and projection;
- whether each context value was explicitly declared;
- supplied event order;
- every logical core field;
- datatype, generic and clarifier arrays;
- identity;
- provenance and spans; and
- every registered extension pair.

Transcoding requires neither AEON source nor AEON parsing, schema inference,
reference resolution, nested AST reconstruction, nor an intermediate JSON
object graph.

A streaming transcoder may produce provisional bytes only into a staging sink.
Durable publication waits for completed source-stream validation or an enclosing
transaction that supplies equivalent atomicity.

Equivalent canonical Telex and Film streams decode to the same logical portable
AES stream. Their physical bytes are intentionally different.

## 16. Integrity, signatures, encryption, and transactions

Film v1 carries no intrinsic checksum, digest, signature, encryption flag,
integrity trailer, or evidence record.

Three coverage layers remain distinct:

1. **Exact Film bytes:** an enclosing contract may digest or sign the complete
   canonical bytes from the first preamble byte through the final record byte.
   That evidence binds Film version, declaration presence, record order,
   provenance, extensions, and every physical choice.
2. **Portable AES semantics:** transport-independent evidence uses
   [`aes.integrity.v1`](./aes.integrity.md) and `aes.signature.v1` after Film
   decoding and AES validation. Equivalent Telex and Film streams can therefore
   produce the same semantic evidence.
3. **Transaction and application state:** expected completion, authorization,
   replay policy, atomic application, and commit belong to
   [`aes.transaction.v1`](./aes.transactions.md), ASP, or another enclosing
   protocol.

Evidence remains outside the covered Film stream to avoid recursion. The
enclosing evidence identifies the representation and whether it covers exact
bytes or portable semantics. Valid bytes or evidence do not themselves
authorize mutation or commit.

Encryption belongs to a versioned AEON envelope or carrier profile, not Film
control bytes. Such a contract must define plaintext and ciphertext coverage,
authenticated visible metadata, and processing order. Until that contract
exists, encryption composition remains deferred. After decryption, the inner
bytes independently satisfy Film syntax, canonicality, limits, and AES
validation.

Film frames assignment events. It does not define transaction completion or
commit.

## 17. Syntax diagnostics

Portable Film vectors compare stable diagnostic codes rather than prose.
Implementations may expose more specific internal causes while mapping them to
these format-level categories:

| Code | Condition |
| --- | --- |
| `FILM_INVALID_PREAMBLE` | preamble bytes or Film version are unsupported |
| `FILM_TRUNCATED` | final input ends inside a required integer, field, context, descriptor, or record |
| `FILM_NONCANONICAL` | a decodable physical value uses a forbidden alternate spelling or order |
| `FILM_INTEGER_OVERFLOW` | an integer exceeds the Film `u64` domain or host-safe range |
| `FILM_INVALID_UTF8` | a declared string is not valid UTF-8 |
| `FILM_INVALID_CONTEXT` | context bits, declaration presence, or context value shape is invalid |
| `FILM_INVALID_RECORD` | record control, field presence, boundary, origin, or span shape is invalid |
| `FILM_INVALID_KIND` | kind code is unassigned or cannot determine the value contract |
| `FILM_INVALID_DATATYPE` | datatype framing, count agreement, tag, or boundary is invalid |
| `FILM_INVALID_EXTENSION` | extension name, order, duplicate status, or pair boundary is invalid |
| `FILM_LIMIT_EXCEEDED` | a caller-selected Film format counter is exceeded |

AES event-local and stream-semantic failures retain the diagnostics defined by
[`aes.events.v1`](./aes.events.md#12-validation-and-diagnostics). A syntax error
must not be relabeled as an AES semantic failure merely because both occur at
the same record.

## 18. Security considerations

Film decoding performs no schema loading, datatype execution, reference
resolution, network access, source retrieval, decompression, decryption,
signature verification, or mutation.

Decoders fail closed on malformed lengths, integer overflow, invalid UTF-8,
unknown tags, reserved bits, kind/value disagreement, invalid provenance shape,
duplicate or out-of-order extensions, truncation, and active resource limits.

Length-prefixed framing does not make untrusted allocation safe by itself. A
decoder checks the declared length and all arithmetic before allocating or
advancing. Implementations SHOULD fuzz arbitrary bytes, nested descriptors,
boundary lengths, chunk divisions, and complete-record removal.

Film files and records carry no implicit document authority. Structural
identity does not become path identity, provenance does not become mutation
authority, and a cryptographically valid enclosing signature does not replace
host authorization.

## 19. Conformance and lifecycle

This document is a normative draft. It defines the intended Film v1 bytes but
is not yet a released conformance target. The Rust Candidate A implementation
is prototype evidence and cannot establish conformance by itself.

Film v1 is ready for release only after:

1. a language-neutral mutable CTS covers positive, negative, canonicality,
   boundary, truncation, limits, datatype, provenance, extension, context, and
   Telex-equivalence cases;
2. at least two independent decoders pass the same candidate vectors;
3. fuzzing covers arbitrary bytes, integer and nested descriptor boundaries,
   and incremental chunk divisions;
4. a specification snapshot and CTS snapshot receive immutable identifiers;
5. reader support is deployed and compatibility-reviewed before durable writers
   are enabled; and
6. the canonical specification source is promoted through the Aeonite specs
   publication process.

Repository-local Film vectors remain mutable development material until their
manifest identifies an immutable shared snapshot. Compatible added coverage
advances the CTS snapshot; an incompatible wire change advances the Film format
version.

Transport media types and external registration are outside Film v1.

## 20. Complete scalar example

This canonical Telex stream:

```text
telex.aes=1

path=$.message
kind=StringLiteral
value=hello
```

maps to the following Film bytes:

```text
4F 5F 5F FF 01 00
12
00 01
09 24 2E 6D 65 73 73 61 67 65
05 68 65 6C 6C 6F
```

The lines above are explanatory only; Film contains no line boundaries. The
record payload length is `12` hexadecimal, or 18 decimal. Its payload contains
record control `00`, kind `01`, the nine-byte address `$.message`, and the
five-byte value `hello`.
