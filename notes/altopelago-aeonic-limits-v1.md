# AltoPelago Aeonic Limits v1

Status: informative AES-adjacent implementation contract.

Limits identifier: `altopelago.aeonic-limits.v1`

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

The initial file shape is:

```aeon
limits_id = "altopelago.aeonic-limits.v1"
limits_version = "1.0.0"

profile_claims = [
  "aeon.gp.profile.v1"
  "aes.complete.v0"
]

structure = {
  max_attribute_depth = 1
  max_generic_depth = 1
  max_clarifier_values = 1
  max_value_nesting_depth = 256
}

processing = {
  max_events = 100000
  max_path_depth = 256
  max_reference_depth = 64
  max_materialized_weight = 100000
  max_datatype_components = 4096
}

formats = {
  aeon = {
    max_input_bytes = 16777216
  }
  telex = {
    max_input_bytes = 33554432
    max_line_bytes = 1048576
    max_fields_per_event = 64
    max_decoded_payload_bytes = 16777216
  }
}
```

The numbers above illustrate the shape; they are not the released AltoPelago
default set. A concrete limits file becomes an implementation target only when
its exact values, version, and lifecycle have been approved and published.

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
| `max_clarifier_values` | number of ordered clarifier values on one datatype descriptor |
| `max_value_nesting_depth` | nested logical container depth |

The generic-depth convention remains: `list<int>` has depth `0`,
`list<list<int>>` has depth `1`, and `list<list<list<int>>>` has depth `2`.
Exhaustion always rejects explicitly and never truncates the represented
structure.

`max_clarifier_values` is the canonical cross-implementation name. Existing
AEON APIs named `maxSeparatorDepth` or `max_separator_depth` map to it during
migration; they do not define a second counter.

## 3. Shared processing limits

Processing limits apply to logical AES work rather than a particular byte
encoding.

| Limit | Counter |
| --- | --- |
| `max_events` | accepted or produced assignment-event count |
| `max_path_depth` | canonical address segment depth |
| `max_reference_depth` | reference-resolution chain depth |
| `max_materialized_weight` | cumulative value weight materialized by clone/reference expansion |
| `max_datatype_components` | descriptor, generic-argument, and clarifier components in one datatype |

An implementation may enforce a lower immutable safety ceiling where its
runtime requires one. It must reject a requested value above that ceiling when
loading configuration rather than silently substituting another value.

## 4. Format-local limits

Byte, line, frame, and buffering limits remain scoped to their physical format
or transport. The size of an AEON source is not the size of its Telex encoding,
and neither determines the size of a future Film representation.

AEON format limits currently include `max_input_bytes`.

Telex format limits include:

- `max_input_bytes`;
- `max_line_bytes`;
- `max_fields_per_event`;
- `max_decoded_payload_bytes`.

Telex event count, path depth, generic depth, clarifier count, value nesting,
and datatype-component limits come from the shared sections. Film will add its
own byte-, frame-, table-, and buffering-specific fields without redefining
those shared counters.

Transport framing that is independent of an encoding may use its own sibling
section for `max_frame_bytes`, `max_buffer_bytes`, and `max_header_bytes`.

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

The exact bootstrap byte and structural ceilings must be shared by the
TypeScript, Rust, and Python loaders and documented alongside the first
published concrete limits file.

## 8. Current implementation inventory

The following controls exist today and must be mapped before a concrete common
set is released.

| Area | Existing controls and defaults |
| --- | --- |
| AEON Core | attribute `1`, separator/clarifier `1`, generic `1`, value nesting `256`; input bytes and events are optional/unbounded when omitted |
| AEON WASM | input bytes `1 MiB`; attribute, separator/clarifier, and generic depth `1` |
| AES datatype codec | generic depth `1`; datatype components `4096`; JavaScript accepts overrides while Rust currently fixes both values |
| Finalization | materialized weight and reference depth are optional/unbounded when omitted |
| TypeScript framing transport | frame `16 MiB`, buffer `32 MiB`, inspected header `64 KiB` |
| Telex | generic and datatype-component guards exist; the other required syntax resource bounds are not yet consistently implemented |

Before release, the implementation audit must also find hard-coded allocation,
recursion, collection, and input guards that are not currently public options.
Every discovered guard is either mapped to this contract, documented as a
runtime safety ceiling, or explicitly classified as belonging to another
limits contract.

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
