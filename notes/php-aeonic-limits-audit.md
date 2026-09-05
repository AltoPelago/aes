# PHP Aeonic limits audit

Status: AEON compiler rollout complete, 2026-09-06.

Scope: `altopelago/aeon-php` against `altopelago.aeonic-limits.v1` version
`1.0.0`. This is an implementation-status record, not a second limits contract.

## Summary

PHP now implements the closed common limits file, normalized effective
configuration, canonical option names, and all 16 AEON parsing/compilation
counters. The legacy clarifier and value-nesting option names remain explicit
migration aliases. PHP finalization, Telex, and framing transport surfaces do
not yet exist, so their counters remain inapplicable rather than claimed.

PHP generic depth now uses the shared convention: `list<int>` has depth `0`
and `list<list<int>>` has depth `1`. Generic argument count and total datatype
component count are enforced independently.

The PHP fixture harness follows the consolidated next manifest and respects its
explicit test exclusions. The protocol-level `core-limits-cts.v1.next.json`
lane passes 32/32 through `bin/aeon-php`. PHP must not claim the complete next
Core snapshot yet: the full protocol runner still exposes pre-existing event
projection and diagnostic-normalization differences outside this limits work.

## Counter inventory

| Published counter | PHP status | Current surface or finding |
| --- | --- | --- |
| `max_attribute_depth` | enforced | `maxAttributeDepth`, default `1` |
| `max_generic_depth` | enforced | `maxGenericDepth`, default `1`; counts nested generic edges |
| `max_generic_arguments` | enforced | `maxGenericArguments`, default `32` |
| `max_clarifier_values` | enforced | `maxClarifierValues`, default `1`; `maxSeparatorDepth` remains an alias |
| `max_datatype_components` | enforced | `maxDatatypeComponents`, default `64` |
| `max_value_nesting_depth` | enforced | `maxValueNestingDepth`, default `256`; `maxNestingDepth` remains an alias |
| `max_path_depth` | enforced | `maxPathDepth`, default `1024`, covering source references and resolved event paths |
| `max_string_codepoints` | enforced | `maxStringCodepoints`, default `1048576`, counted as Unicode scalars |
| `max_key_segment_codepoints` | enforced | `maxKeySegmentCodepoints`, default `1024` |
| `max_list_items` | enforced | `maxListItems`, default `65536` |
| `max_tuple_items` | enforced | `maxTupleItems`, default `65536` |
| `max_path_characters` | enforced | `maxPathCharacters`, default `8192` |
| `max_events` | enforced | `maxEvents`, default `100000` |
| `max_reference_depth` | missing | PHP currently has no shared finalization/materialization implementation |
| `max_materialized_weight` | missing | PHP currently has no shared finalization/materialization implementation |
| AEON `max_input_bytes` | enforced | `maxInputBytes`, default `16 MiB`; `maxBytes` remains an alias |
| AEON `max_numeric_literal_characters` | enforced | `maxNumericLiteralCharacters`, default `1024` |
| AEON `max_structured_comment_characters` | enforced | `maxStructuredCommentCharacters`, default `1048576` |
| Telex format counters | not applicable yet | no PHP Telex codec |
| transport counters | not applicable yet | no PHP framing/header-inspection transport package |

## Hard-coded and separate limits

- SANSA position-index guards belong to SANSA, not this limits contract.
- AEOS schema regex, rule, path, alternative, event, and container budgets are
  schema-validation limits and remain deliberately separate.
- PHP byte offsets and `strlen`-based input accounting operate on source bytes,
  while the new decoded string/key/path counters use deterministic UTF-8 scalar
  counting. Portable span offset-unit alignment remains a separate audit item.

## Remaining rollout

1. Resolve the existing PHP event-projection and diagnostic-envelope differences
   before claiming the complete consolidated `core-cts.v1.next.json` target.
2. Add finalization, Telex, and transport counters when those PHP surfaces
   exist; absence must not be advertised as conformance.
