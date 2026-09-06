# Film v0 binary encoding proposal

Status: deferred proposal

Scope: binary framing and encoding of portable AES event streams

Proposed file suffix: `.film.aes`

Portable event contract: [`aes.events.v0`](../specifications/aes.events.md)

## 1. Summary

`film.aes` is the compact binary encoding of the portable AES event model.
It carries the same logical records as Telex and does not define a second event
model.

Conceptually:

```text
portable AES records
        /       \
       v         v
 telex.aes    film.aes
 UTF-8 text    binary
```

Film is intended for high-throughput interchange, storage, replication, IPC,
and other stream-oriented uses where human-readable framing is not required.
It should be inexpensive to encode, support bounded sequential decoding, and
permit zero-copy access to length-delimited payloads where the host allows it.

This proposal records the architecture and the questions that must be resolved
before a Film specification is written. It deliberately does not assign wire
bytes, numeric field codes, or a frozen magic sequence.

## 2. Authority and relationship to AES

The portable AES event contract remains authoritative for:

- record fields and their logical shapes;
- value kinds and canonical payloads;
- paths and address planes;
- datatype, generic, and clarifier structure;
- structural identity;
- provenance and spans;
- record order;
- profiles and projections; and
- semantic validation and diagnostic codes.

Film will own only:

- its binary preamble and version marker;
- stream-context encoding;
- record and field framing;
- numeric tags and binary primitive encodings;
- canonical Film bytes;
- format-local limits; and
- Film syntax and canonicality diagnostics.

A Film decoder must recover the same logical stream context and records that a
Telex decoder recovers. Transcoding between canonical Telex and canonical Film
must not require AEON source, an AEON parser, schema inference, or downstream
semantic interpretation.

Film v0 should map statically to `aes.events.v0`, just as Telex v0 does. An
incompatible mapping or binary grammar requires another Film format version.

## 3. Required semantic invariants

Binary encoding does not change the AES value model.

- Every portable scalar field still decodes to a Unicode string.
- `generics` and `clarifiers` remain ordered structured arrays.
- A `NumberLiteral` remains its canonical textual value at the AES boundary;
  Film must not silently convert it to an IEEE-754 or host integer value.
- `CloneReference` and `PointerReference` remain distinct kinds.
- `NodeLiteral` and `NodeHead` remain distinct records.
- Structural identity remains independent of path identity.
- Record order is preserved exactly.
- Unknown extensions retain their complete logical field name and value.
- Origin and span remain optional, and span remains invalid without origin.
- Exact AEON spelling and layout remain outside AES semantic losslessness.

Film may represent a logical string with compact binary machinery. For example,
it may store a SHA-256 origin digest as 32 bytes rather than 64 hexadecimal
characters. The decoder must nevertheless expose the logical AES value in the
form required by `aes.events.v0`.

## 4. Goals

Film v0 should:

1. Encode every valid `aes.events.v0` stream without loss.
2. Decode incrementally from a byte stream with bounded buffering.
3. Frame each record so a decoder can skip or reject it without scanning for a
   textual delimiter.
4. Use deterministic canonical bytes.
5. Preserve supplied record order for ledgers, signatures, and replay.
6. Avoid an intermediate object graph when producer and consumer can operate
   on borrowed event views.
7. Support efficient native and WASM implementations.
8. Fail closed on malformed lengths, integer overflow, truncation, duplicate
   fields, invalid UTF-8, and unsupported required semantics.
9. Keep the first implementation small enough to audit.
10. Reuse AES structural limits without redefining their meaning.

## 5. Non-goals

Film v0 should not:

- define different event semantics from Telex;
- encode an implementation-specific AST or JSON object layout;
- infer kinds, datatypes, parents, or reference targets;
- replace Tape's externally contextualized value-stream role;
- guarantee exact AEON source round-tripping;
- require global compression or a whole-stream materialization pass;
- provide schemas, query execution, or Aeonic Semantic Language operations;
- make transport framing, encryption, checksums, or signatures intrinsic to
  every Film stream; or
- promise that a JavaScript object graph can cross a WASM boundary without
  conversion cost.

Compression, transport integrity, encryption, indexing, and random-access
containers may wrap or extend Film later. They should not complicate the v0
event stream unless measurements show a compelling need.

## 6. Proposed stream architecture

The initial direction is a self-identifying stream header followed by
length-delimited records:

```text
+-----------------------+
| Film magic + version  |
+-----------------------+
| stream context        |  profile and projection
+-----------------------+
| record length         |
| record payload        |
+-----------------------+
| record length         |
| record payload        |
+-----------------------+
| ...                   |
+-----------------------+
```

The exact bytes remain a decision gate. The architecture should have these
properties:

- the preamble rejects accidental interpretation as another format;
- the version establishes the event-contract mapping;
- omission of profile selects `aes.complete.v0`;
- omission of projection selects the ordinary body-only stream;
- a canonical empty stream consists only of its required stream header;
- each record has one explicit byte length;
- lengths use one canonical representation;
- records never depend on a following record for physical decoding; and
- truncation is detectable at the exact enclosing field or record.

An external transport may divide a Film stream into its own frames. Film record
boundaries and transport-frame boundaries are independent.

## 7. Proposed record architecture

The working direction is a compact tagged record rather than a serialized host
object. A record would contain:

1. one address-plane tag and address payload;
2. one value-kind tag;
3. a presence map or tagged sequence for optional core fields; and
4. zero or more explicitly named extension fields.

Conceptually:

```text
record := length {
  address-plane
  address
  kind
  optional-core-fields
  extension-fields
}
```

Core field names and value kinds should use versioned numeric codes. Extension
fields must retain their `x.<owner>.<name>` names because their meaning cannot
be assigned by the Film codec.

Length-delimited fields allow safe skipping and bounded decoding. Physical
skipping does not imply semantic acceptance: an AES semantic consumer still
rejects an unknown field unless the active profile registers it. A generic
relay may preserve an extension without claiming to understand it.

The design must reject duplicate logical fields even if different physical
forms could decode to the same field.

## 8. Primitive representations

The likely v0 primitives are:

- unsigned integers encoded with a unique shortest varint representation;
- byte strings encoded as `length + bytes`;
- AES strings encoded as length-delimited UTF-8 without normalization;
- fixed-size raw bytes where the logical field fixes an exact size; and
- small enums encoded as versioned unsigned tags.

Canonical Film rejects overlong varints, impossible lengths, out-of-range
tags, invalid UTF-8 in textual fields, and trailing bytes inside a framed
value.

Fixed-width integers may be preferable for selected hot fields if benchmarks
show that predictable decoding outweighs their larger representation. The
choice between varints and fixed-width values is not yet resolved.

## 9. Datatypes, generics, and clarifiers

Film should encode the logical datatype structure directly. It must not use the
combined Telex datatype descriptor as its internal representation.

Conceptually:

```text
datatype-descriptor := {
  base-name
  generic-count
  generic-argument...
  clarifier-count
  clarifier...
}

generic-argument := datatype-descriptor | tagged-number-literal
clarifier       := tagged-string-literal | tagged-number-literal
```

Counts are lengths, while nested datatype descriptors contribute to generic
depth. Film uses the same `max_generic_arguments`, `max_clarifier_values`,
`max_generic_depth`, and `max_datatype_components` counters as other AES
ingress paths.

An implementation may offer borrowed descriptor views. It must never truncate
a descriptor to satisfy a consumer limit.

## 10. Paths and addresses

Both body `path` and control-plane `header` addresses belong to the canonical
SANSA address domain defined by the AES contract. Film must distinguish the two
planes without inserting a synthetic field into the decoded event.

There are two viable v0 encodings:

1. length-prefixed canonical UTF-8 addresses; or
2. binary address segments with an exact, bidirectional mapping to canonical
   SANSA strings.

The first is simpler, easy to audit, and cheap to transcode. The second may
reduce size and accelerate structural navigation, but expands Film's dependency
on SANSA grammar and introduces more canonicality rules.

Prefix compression and address dictionaries must remain optional design
candidates until representative AES-DB, ledger, document, and IPC workloads
are measured. If adopted, references must be backward-only or defined before
use so sequential decoding remains possible.

## 11. Values and kinds

Value kinds should use a fixed v0 numeric tag table. Value presence continues
to follow `aes.events.v0`; Film does not carry a value-presence flag that can
contradict the selected kind.

Value payloads should initially use their canonical AES string form. This
preserves arbitrary precision, lexical distinctions that remain semantic, and
straightforward Telex transcoding.

A later Film version could add canonical accelerators for selected values, but
only if the logical AES value remains uniquely recoverable. Multiple binary
spellings for one logical event would weaken canonical Film and should be
avoided.

## 12. Provenance

The v0 provenance direction is:

- encode the origin algorithm as a small tag;
- encode a SHA-256 digest as its 32 raw bytes;
- encode span start and end as unsigned byte offsets; and
- reconstruct the logical `sha256:<lowercase-hex>` origin on ordinary decode.

Repeated origins are common. Film may support a bounded, streaming-safe origin
table, but a table is not required for the first prototype. Any table must be
defined before reference, have deterministic canonical insertion order, and be
subject to explicit entry and byte limits.

Film does not verify source bytes merely by decoding provenance. Source-backed
origin and span audits remain AES consumer operations.

## 13. Ordering and canonical bytes

Film always preserves record sequence. It must not sort records implicitly.

Canonical Film should specify at least:

- one preamble and version representation;
- one stream-context order;
- one integer representation;
- one core-field order or presence-map layout;
- one extension-field ordering rule within each record;
- one UTF-8 representation without Unicode normalization;
- no duplicate fields;
- no unused flag bits;
- no unreferenced or multiply representable table entries; and
- no trailing padding or data.

Canonical byte order inside a record is independent of event-stream order.
Profiles that sign canonical AES semantics and profiles that sign exact supplied
stream order remain distinct. A signature over Film bytes also binds the Film
version and every physical encoding choice; a semantic AES signature should be
defined over the transport-neutral expanded record model.

## 14. Streaming, buffering, and recovery

A decoder should be able to:

1. validate the preamble and stream context;
2. read one bounded record length;
3. decode or skip that record;
4. release record-local scratch storage; and
5. continue with the next record.

Semantic validation may retain cross-record state for uniqueness,
prefix-completeness, reference targets, and profile ordering. That state is not
a Film framing requirement.

The initial direction is fail-fast decoding without an in-band resynchronizing
marker. Recovery after corrupt bytes belongs to a containing journal, transport,
or chunk format that can identify a trusted next boundary. False resynchronizing
matches inside arbitrary payloads would otherwise complicate both security and
canonicality.

## 15. Limits and security

Film consumes the shared structural counters defined by the selected consumer
policy, including event count, field count, path depth and characters, decoded
payload bytes, generic structure, clarifiers, and datatype components.

Film also needs format-local counters. Candidate counters include:

- `max_input_bytes`;
- `max_record_bytes`;
- `max_field_bytes`;
- `max_varint_bytes`;
- `max_table_entries`;
- `max_table_bytes`; and
- `max_buffered_bytes`.

The final list and defaults require prototypes. These counters belong to Film
or its enclosing transport, not to an AES semantic profile. As with the shared
AltoPelago limits file, profile names attached to a limits set are descriptive
claims rather than authority over consumer policy.

Every decoder must check length conversion and addition before allocation or
pointer arithmetic. It must reject integer overflow, impossible nesting,
truncated frames, invalid tags, invalid presence bits, invalid UTF-8, duplicate
fields, table cycles, forward references where forbidden, and decoded values
that exceed the active limits.

## 16. Implementation model

The preferred native API should support both owned records and borrowed views:

```text
Film bytes
    |
    +--> validating iterator of borrowed event views
    |
    +--> owned portable AES records when requested
```

A producer already holding portable records should write directly into one
growable byte buffer or caller-provided sink. A producer parsing AEON inside the
same native or WASM runtime should be able to project events directly into the
Film encoder without constructing a JavaScript event graph.

JavaScript APIs should use `Uint8Array` for Film bytes. A JS-to-WASM encoder
that begins with ordinary JavaScript objects must count object conversion and
boundary transfer in its end-to-end performance claims. A slower bridge should
not be published merely because the native codec is fast.

Rust is the preferred first reference implementation because it can provide a
safe streaming surface while allowing carefully audited zero-copy or SIMD work
inside isolated hot paths. A conforming implementation is not language-bound.

## 17. Benchmark and comparison contract

Film performance claims should separate operations that have different
boundaries:

- resident logical records to encoded bytes;
- bytes to borrowed records;
- bytes to owned records;
- bytes to semantic validation result;
- Film canonicalization;
- Telex-to-Film and Film-to-Telex transcoding; and
- end-to-end JavaScript-to-WASM calls.

The first benchmark suite should reuse the Telex workloads at 100, 10,000, and
100,000 events. Each implementation must produce logically equivalent decoded
records. Comparisons should report:

- median elapsed time after warm-up;
- encoded byte length;
- events and megabytes per second;
- peak or counted allocations where available;
- whether input and output cross a runtime boundary;
- whether validation is included; and
- a stable digest of the canonical output.

Native Rust and TypeScript resident-record encoding are analogous language-local
measurements. A JavaScript-object-to-WASM measurement is a separate end-to-end
API comparison. JSON, Telex, and Film results should be labelled by the semantic
work each operation performs rather than presented as interchangeable parsers.

## 18. Conformance and compatibility

Film should not become a released conformance target until:

1. the binary grammar and canonical form are specified;
2. a language-neutral CTS covers positive, negative, boundary, truncation, and
   canonicality cases;
3. Telex-to-Film-to-Telex vectors prove logical equivalence;
4. at least two independent decoders pass the same frozen CTS snapshot;
5. fuzzing covers arbitrary bytes, nested datatype structures, lengths, and
   table references; and
6. reader support is deployed before any durable writer is enabled.

Film follows the portable AES compatibility contract. A Film carrier is not a
legacy-event adapter, and recognizing `.film.aes` by filename alone is not a
safe substitute for validating the binary preamble.

## 19. Decision gates for later work

The following gates should be taken one at a time when Film work resumes:

1. **Preamble and versioning:** exact magic, version representation, and static
   mapping to `aes.events.v0`.
2. **Integer primitive:** shortest varints, fixed-width integers, or a measured
   hybrid.
3. **Record framing:** length prefix width, maximum record size, and whether a
   record includes a field count.
4. **Core layout:** presence bitmap versus tagged core fields, and the stable
   numeric field table.
5. **Kind table:** numeric assignments and behavior for unknown kind codes.
6. **Datatype layout:** recursive descriptor tags, counts, and canonicality.
7. **Address encoding:** canonical UTF-8 paths versus binary SANSA segments.
8. **Compression tables:** whether v0 includes bounded backward references for
   paths, strings, datatypes, or origins.
9. **Extensions:** canonical named-extension representation and generic relay
   preservation.
10. **Provenance:** raw digest representation and whether repeated origins use
    a table in v0.
11. **Canonical Film:** complete uniqueness rules for every valid byte stream.
12. **Limits:** format-local counter names, defaults, and exhaustion errors.
13. **Streaming recovery:** fail-fast only or an independently framed chunk
    profile.
14. **Integrity boundary:** how byte signatures, semantic signatures, and
    enclosing transport integrity identify what they cover.
15. **API boundary:** borrowed native iteration, owned materialization, WASM
    typed arrays, and transcoders.
16. **Prototype selection:** compare at least two plausible layouts before
    freezing the wire format.

## 20. Recommended prototype sequence

When this proposal is resumed:

1. Freeze a small corpus of representative portable AES streams.
2. Implement a minimal length-delimited Rust prototype using UTF-8 addresses,
   numeric core tags, and no compression tables.
3. Implement a second prototype that compresses the dominant repeated data
   identified by measurement, not assumption.
4. Compare size, throughput, allocation, streaming behavior, and implementation
   complexity.
5. Exercise both through native Rust and `Uint8Array` WASM APIs.
6. Select the simpler layout unless the more complex form has a material,
   repeatable advantage on representative workloads.
7. Write the normative `film.aes` specification and shared CTS only after the
   layout decision.

This sequence keeps Film grounded in the established AES contract while
leaving its physical design open to evidence.
