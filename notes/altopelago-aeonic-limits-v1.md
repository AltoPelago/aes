# AltoPelago Aeonic Limits v1

Status: informative AES-adjacent implementation contract.

Limits identifier: `altopelago.aeonic-limits.v1`

Published limits version: `1.0.0`

Concrete file:
[`policies/altopelago.aeonic-limits.v1.aeon`](../policies/altopelago.aeonic-limits.v1.aeon)

This document defines the common limits-file shape used by AltoPelago AEON,
AES, Telex, and later Film implementations. It is not an AES semantic profile,
an AEON language rule, or a portable conformance claim. Consumers independently
select both their semantic profiles and their processing limits.

## 1. Ownership and authority

A limits file is trusted consumer configuration. Input documents, stream
headers, and document-selected profiles cannot select, replace, or relax it.

A limits file may contain `profile_claims` as descriptive metadata. A claim
means that the named limit set was designed, tested, or deployed with those
profiles. It does not activate a profile, prove conformance, or give the named
profile authority over the limits.

The published AltoPelago v1 limit set is:

```aeon
limits_id = "altopelago.aeonic-limits.v1"
limits_version = "1.0.0"

profile_claims = [
  "aeon.gp.profile.v1"
  "aes.complete.v0"
  "aes.partial.v0"
]

structure = {
  max_attribute_depth = 1
  max_generic_depth = 1
  max_generic_arguments = 32
  max_clarifier_values = 1
  max_datatype_components = 64
  max_value_nesting_depth = 256
  max_path_depth = 1024
  max_string_codepoints = 1048576
  max_key_segment_codepoints = 1024
  max_list_items = 65536
  max_tuple_items = 65536
  max_path_characters = 8192
}

processing = {
  max_events = 100000
  max_reference_depth = 64
  max_materialized_weight = 1000000
}

formats = {
  aeon = {
    max_input_bytes = 16777216
    max_numeric_literal_characters = 1024
    max_structured_comment_characters = 1048576
  }
  telex = {
    max_input_bytes = 67108864
    max_line_bytes = 1048576
    max_fields_per_event = 64
    max_decoded_payload_bytes = 33554432
  }
}

transport = {
  max_frame_bytes = 16777216
  max_buffer_bytes = 33554432
  max_header_bytes = 65536
}
```

`limits_id` identifies the named limit-set line. `limits_version` identifies an
exact immutable revision of that line. Changing any effective value requires a
new version. A consumer should expose both values with its normalized effective
configuration and in limit-exhaustion diagnostics.

### 1.1 Limit values

Every limit accepts a non-negative integer or one of two distinct custom null
values. Omitting a field has separate inheritance behavior:

| Value | Meaning |
| --- | --- |
| field omitted | inherit the value from the underlying consumer-selected limits file or overlay |
| non-negative integer | explicit inclusive limit; `0` permits zero and never means unbounded |
| `!"unBound"` | explicit absence of a policy-level limit; an immutable implementation safety ceiling still applies |
| `!"useImplementation"` | ignore inherited configured values and use the implementation's documented default |

`!"unBound"` and `!"useImplementation"` are exact, case-sensitive custom null
reasons. They are not aliases and implementations must not normalize either
one into the other. Negative integer sentinels such as `-1` are invalid.

A published cross-language limit set should avoid `!"useImplementation"`
because its effective value may differ between implementations. It should use
an explicit integer or `!"unBound"` for every applicable field. The reset value
remains useful in consumer-local configurations that deliberately restore
runtime behavior.

## 2. Shared structural limits

Structural limits describe one logical structure and therefore use one value
across every ingress that can create or accept it. AEON compilation, direct AES
validation, Telex decoding, and Film decoding must not silently apply different
values to the same counter.

| Limit | Counter |
| --- | --- |
| `max_attribute_depth` | nested attribute address-space depth |
| `max_generic_depth` | recursive datatype generic depth |
| `max_generic_arguments` | length of one datatype descriptor's `generics` array |
| `max_clarifier_values` | number of ordered clarifier values on one datatype descriptor |
| `max_datatype_components` | aggregate descriptors, numeric generic arguments, and clarifier values in one recursive datatype |
| `max_value_nesting_depth` | nested logical container depth |
| `max_path_depth` | canonical address structural-step depth |
| `max_string_codepoints` | decoded string literal length in Unicode code points |
| `max_key_segment_codepoints` | decoded key-segment length in Unicode code points |
| `max_list_items` | direct item count in one list |
| `max_tuple_items` | direct item count in one tuple |
| `max_path_characters` | canonical or reference path length in Unicode code points |

The generic-depth convention remains: `list<int>` has depth `0`,
`list<list<int>>` has depth `1`, and `list<list<list<int>>>` has depth `2`.
Exhaustion always rejects explicitly and never truncates the represented
structure.

`max_clarifier_values` is the canonical cross-implementation name. Existing
AEON APIs named `maxSeparatorDepth` or `max_separator_depth` map to it during
migration; they do not define a second counter.

Generic depth, generic-array length, clarifier-array length, and total datatype
components are independent counters. Depth bounds recursion; the two array
limits bound local breadth; the component limit bounds aggregate recursive
size. The published values are depth `1`, generic arguments `32`, clarifier
values `1`, and total components `64`.

Path depth and path character length are also independent. The root `$` has
depth zero. Each member or index adds one; an attribute transition plus its key
counts as one structural step. Node-head expansion therefore consumes a step
separately from its content index. Quoting does not alter depth.

## 3. Shared processing limits

Processing limits apply to logical AES work rather than a particular byte
encoding.

| Limit | Counter |
| --- | --- |
| `max_events` | accepted or produced assignment-event count |
| `max_reference_depth` | reference-resolution chain depth |
| `max_materialized_weight` | cumulative value weight materialized by clone/reference expansion |

An implementation may enforce a lower immutable safety ceiling where its
runtime requires one. It must reject a requested value above that ceiling when
loading configuration rather than silently substituting another value.

## 4. Format-local limits

Byte, line, frame, and buffering limits remain scoped to their physical format
or transport. The size of an AEON source is not the size of its Telex encoding,
and neither determines the size of a future Film representation.

AEON format limits include `max_input_bytes`, raw numeric-literal character
length, and structured-comment payload character length. The latter two are
source-format counters; normalized AES values do not retain AEON lexical
spelling or structured comments.

Telex format limits include:

- `max_input_bytes`;
- `max_line_bytes`;
- `max_fields_per_event`;
- `max_decoded_payload_bytes`.

Telex event count, path depth, generic depth, clarifier count, value nesting,
and datatype-component limits come from the shared sections. Film will add its
own byte-, frame-, table-, and buffering-specific fields without redefining
those shared counters.

Transport framing that is independent of an encoding uses its own sibling
section. The published values are `16 MiB` for a frame payload, `32 MiB` for
buffered transport bytes, and `64 KiB` for header inspection. A Telex stream
may span multiple transport frames.

## 5. Schema boundary

Schema definition and schema evaluation limits are deliberately absent. AEOS
budgets such as rule count, schema depth, alternatives, selector expansion,
reference-resolution steps, string length, and container children belong to a
separate versioned schema limits contract.

Likewise, SANSA query and mutation budgets remain owned by SANSA. A host may
load several limit files into one application configuration, but merging those
files does not make their counters part of this contract.

## 6. Selection and resolution

The consumer selects a semantic profile and limits file independently. A
developer-facing runtime configuration may place both references together for
convenience:

```aeon
profile = "aes.complete.v0"
limits = {
  id = "altopelago.aeonic-limits.v1"
  version = "1.0.0"
}
```

This association is host configuration, not AES stream metadata. An API may
also accept trusted per-call overrides. The effective value is subject to the
implementation's hard safety ceiling, and the implementation must make its
normalized effective configuration inspectable.

Unknown fields fail when loading a v1 limits file unless this contract later
defines an extension mechanism. Limit exhaustion is an error, never implicit
truncation, partial success, or a change of semantic profile.

When resolving overlays, an omitted field inherits the next consumer-selected
value. An explicit integer or `!"unBound"` replaces that value.
`!"useImplementation"` stops inheritance for the field and selects the
implementation's documented default.

## 7. Bootstrap loading

An AEON-encoded limits file cannot control the parser resources required to
read itself. Implementations therefore load it with a small, fixed bootstrap
policy before applying its contents. The bootstrap parser performs no reference
resolution, schema loading, network access, or document-selected profile
activation.

The fixed bootstrap ceilings are:

| Limit | Value |
| --- | ---: |
| input bytes | `65536` |
| projected events | `256` |
| canonical path depth | `8` |
| value nesting depth | `8` |
| attribute depth | `0` |
| generic depth | `0` |
| generic arguments | `0` |
| clarifier values | `0` |
| datatype components | `1` |

The TypeScript, Rust, and Python loaders must use these exact ceilings.
Bootstrap loading uses AEON transport mode, rejects headers, disables reference
and clone resolution, and performs no schema loading or network access.

## 8. Current implementation inventory

The following controls exist today and must be mapped before an implementation
claims support for the concrete common set.

| Area | Existing controls and defaults |
| --- | --- |
| AEON Core | attribute `1`, separator/clarifier `1`, generic `1`, value nesting `256`; input bytes and events are optional/unbounded when omitted |
| AEON WASM | input bytes `1 MiB`; attribute, separator/clarifier, and generic depth `1` |
| AES datatype codec | generic depth `1`; datatype components `4096`; JavaScript accepts overrides while Rust currently fixes both values |
| Finalization | materialized weight and reference depth are optional/unbounded when omitted |
| TypeScript framing transport | frame `16 MiB`, buffer `32 MiB`, inspected header `64 KiB` |
| Telex | generic and datatype-component guards exist; the other required syntax resource bounds are not yet consistently implemented |

`max_generic_depth` is narrow in the current AEON parsers: it controls only
recursive datatype annotations wherever a datatype may occur. It does not
limit generic-array length, clarifier-array length, datatype component count,
value nesting, paths, or payload length. TypeScript and Python canonicalization
currently hard-code generic and clarifier capability values of `8` instead of
using the consumer's effective limits.

AEON v1 already promises portability floors for decoded string length, key
segment length, numeric literal lexical length, container nesting, list/tuple
item count, canonical/reference path length, structured-comment length, and an
event budget when exposed. Several are not consistently enforced as public
processor options. The concrete set adopts those established floors while
retaining the existing default value-nesting limit of `256`.

Before implementations ship support for this limit set, the audit must also
find hard-coded allocation, recursion, collection, and input guards that are
not currently public options. Every discovered guard is either mapped to this
contract, documented as a runtime safety ceiling, or explicitly classified as
belonging to another limits contract.

## 9. Required implementation behavior

Every AltoPelago implementation consuming this format should:

1. load and validate `limits_id` and `limits_version` before processing input;
2. reject unknown identifiers, unsupported versions, invalid integers, and
   values above its supported safety ceiling;
3. derive its language-native options from the same normalized counters;
4. expose the effective identifier, version, and values for diagnostics and
   operational inspection;
5. report the canonical counter name, configured value, and observed value on
   exhaustion; and
6. maintain shared boundary vectors at, below, and above each limit.
