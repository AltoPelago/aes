# Non-AEON ingress limits audit

Status: implementation inventory complete, 2026-09-08; portable ingress and
integrity/transaction selection are complete, with provenance operational
counters deferred to a separate contract.

Scope: public and hard-coded resource guards reached without first compiling
AEON source, across the TypeScript, Rust, Python, and PHP ecosystems. This is
an implementation-status record for `altopelago.aeonic-limits.v1` version
`1.0.0`, not another limits contract.

## Boundary rule

The guard belongs at the earliest boundary that can measure the counter:

- Telex decoding and encoding enforce Telex byte, line, field, and decoded
  payload limits.
- Direct portable AES record validation enforces event-model structure and
  event count. It must not invent a byte limit for a carrier that has already
  decoded the records.
- Finalization additionally enforces reference depth and materialized weight.
- AEOS schema evaluation, SANSA query/mutation, ASP storage and transaction,
  HTTP, webhook, and log framing budgets remain separate operational
  contracts. They do not replace validation of an incoming AES event stream.

Implementation-native `AssignmentEvent` collections are same-process objects,
not a portable encoding. Compiler-produced collections inherit the limits of
their producer. A public API that accepts caller-constructed native events must
either document that trusted precondition or adapt and validate them at the
portable boundary; it must not imply that native object construction is a
limits-file ingress.

## Portable boundary inventory

| Ecosystem | Primary portable boundary | Delegating consumers | Current limits-file route |
| --- | --- | --- | --- |
| TypeScript | `packages/aes` Telex parser, encoder, and `validateTelexRecords` | Core export, SDK, runtime, portable finalizer, AEOS, SANSA, SO/ASP, CLI, and the Rust/WASM facade | Core exposes `telexLimits()`; the Telex CLI, SDK, and runtime accept the selected common configuration |
| Rust | `aes-telex` parser, encoder, and `validate_telex_records_*` | Core export, SDK, portable finalizer, AEOS, CLI, integrity, transactions, and WASM | Core exposes `telex_limits()`; the Telex CLI, SDK, and WASM accept the selected common configuration; integrity and transaction APIs expose explicit `_with_limits` companions |
| Python | `aeon.telex` parser, encoder, and `validate_telex_records` | Portable finalizer, AEOS, API, and CLI | `telex_limits()` maps the selected common file; CLI Telex operations expose `--limits-file` |
| PHP | `aeon\\aes` Telex parser, encoder, and `AesValidator::validateRecords` | Core export, portable JSON finalizer, and AEOS | `AeonicLimits::telexOptions()` maps the selected common file; no standalone Telex CLI ingress currently exists |

The JavaScript codec in the AES repository is the executable reference behind
the TypeScript behavior. It has the same counter coverage as the TypeScript
package and is included in shared CTS runs, but it is not counted as a fifth
AltoPelago language implementation here.

## Counter coverage at direct AES/Telex ingress

| Published counter | Current status in all four implementations | Required action |
| --- | --- | --- |
| `max_events` | enforced by decode, encode, and direct record validation | retain one shared counter; AEOS's additional schema-policy event budget remains separate |
| `max_path_depth` | enforced for event/header addresses and reference payloads | none |
| `max_path_characters` | enforced for event/header addresses and reference payloads | none |
| `max_generic_depth` | enforced by wire datatype parsing and logical descriptor validation | none |
| `max_generic_arguments` | enforced by wire datatype parsing and logical descriptor validation | none |
| `max_clarifier_values` | enforced by wire datatype parsing and logical descriptor validation | none |
| `max_datatype_components` | enforced by wire datatype parsing and logical descriptor validation | none |
| `max_attribute_depth` | enforced by all four direct portable validators | longest uninterrupted attribute-transition run; members and indexes reset the run |
| `max_value_nesting_depth` | enforced by all four direct portable validators | represented logical container ancestry; partial streams provide a lower bound |
| `max_string_codepoints` | enforced by all four direct portable validators | `StringLiteral` values and string datatype clarifiers use Unicode scalar counts |
| `max_key_segment_codepoints` | enforced by all four direct portable validators | decoded member/attribute segments and node-head tags are counted |
| `max_list_items` | enforced by all four direct portable validators | represented direct indexed children; partial streams provide a lower bound |
| `max_tuple_items` | enforced by all four direct portable validators | represented direct indexed children; partial streams provide a lower bound |
| `max_reference_depth` | enforced during portable finalization in TypeScript, Rust, Python, and PHP | ensure the selected common file reaches each entry point |
| `max_materialized_weight` | enforced during portable finalization in TypeScript, Rust, Python, and PHP | ensure the selected common file reaches each entry point |
| Telex `max_input_bytes` | enforced by all four codecs | format-local; never apply to direct record arrays |
| Telex `max_line_bytes` | enforced by all four codecs | format-local |
| Telex `max_fields_per_event` | enforced by all four codecs | format-local |
| Telex `max_decoded_payload_bytes` | enforced by all four codecs | format-local |

The shared structural checks now use one contract across all four validators.
Complete streams produce exact aggregate counts. Partial streams enforce lower
bounds over admitted records and require the completing or applying consumer to
revalidate the completed structure. The mutable Telex v0 candidate carries
at-limit and one-over vectors for all six counters; the released snapshot is
unchanged.

## Public call-path findings

- TypeScript, Rust, and Python portable finalizers validate records before
  materialization. PHP's newer `PortableJson` implementation does the same and
  supersedes the earlier finding that PHP finalization did not exist.
- All four AEOS Telex adapters parse and validate portable AES before converting
  to their implementation-native schema event shape. Their schema recursion,
  rule, alternative, selector, and container budgets remain AEOS-owned.
- SANSA and SO/ASP portable adapters use the TypeScript portable validator.
  Their operation counts, mutation nesting, host request bytes, scan sizes,
  and storage/log guards remain consumer-owned.
- JavaScript integrity and transaction functions pass `options.limits` through
  to direct portable record validation. Rust retains every default API and adds
  `_with_limits` companions for integrity encode/digest/verify and transaction
  body validation, digest/verify, envelope validation, and inspection (including
  source-backed inspection). Tests prove a value admitted by defaults is
  rejected at both boundaries by a stricter caller-selected Telex limit.
- Provenance audit APIs accept record arrays and exact source artifacts without
  their own record, artifact-count, per-artifact byte, or aggregate-byte
  controls. `max_events` can come from portable validation; source-artifact
  byte/allocation counters are not defined by v1 and require a separate
  provenance-operational decision rather than reuse of Telex bytes.
- Rust/WASM Telex calls expose direct numeric JSON options and accept trusted
  AEON limits source text as `limitsSource`. The latter is the WASM equivalent
  of common-file selection without assuming host filesystem access, and JSON
  result operations expose the selected effective configuration.
- Rust, TypeScript, and Python Telex CLI commands accept `--limits-file`; PHP has
  no corresponding standalone Telex command. TypeScript and Rust route both the
  Telex and finalization subsets, with explicit per-call finalization overrides
  retaining priority.

## Hard-coded guards classified outside this contract

- AEOS resource-policy constants are schema-evaluation budgets. Its duplicate
  default event ceiling is defense in depth after portable validation, not a
  second meaning for `max_events`.
- SANSA operation, selector, and mutation budgets are SANSA policy.
- ASP transaction-operation, scan, record, HTTP-body, webhook, codec, log, and
  storage bounds are ASP/host policy.
- Transport framing byte limits apply only in the framing implementation. The
  current TypeScript framing route maps the v1 transport section; absent
  framing implementations have no counter to claim.
- AEON WASM's hard-coded source byte and parser-depth defaults are an AEON
  source-ingress issue already recorded separately, not Telex behavior.

## Exact remaining rollout

1. Define provenance-artifact operational counters separately. Do not silently
   reinterpret AEON or Telex input-byte limits as retained-source budgets.

The selected identity, version, copied profile claims, normalized Telex values,
and normalized finalization values are now available through language-native
effective-configuration APIs in TypeScript, Rust, Python, and PHP. These are
inspection surfaces rather than AES wire fields.

Common-limit selection is now available through the relevant Telex SDK/runtime
routes. Rust retains `aeon_to_telex` and adds the configured
`aeon_to_telex_with_limits` companion. PHP Core owns
`TelexDocument::fromTelex`, preserving the Core-to-AES dependency direction.
