# PHP Aeonic limits audit

Status: AEON compiler rollout complete, 2026-09-06; interchange status updated
2026-09-08.

Scope: `altopelago/aeon-php` against `altopelago.aeonic-limits.v1` version
`1.0.0`. This is an implementation-status record, not a second limits contract.

## Summary

PHP now implements the closed common limits file, normalized effective
configuration (including selected identity and claims), canonical option names,
and all 16 AEON parsing/compilation counters. The legacy clarifier and
value-nesting option names remain explicit migration aliases. Since the
original audit, PHP has added Telex, direct portable AES validation, AEOS
ingress, and portable JSON finalization. `Compiler::compileToTelex` accepts a
trusted loaded limits document through `aeonicLimits`, applies its compiler and
Telex subsets, and returns the effective view. Framing transport still does not
exist.

PHP generic depth now uses the shared convention: `list<int>` has depth `0`
and `list<list<int>>` has depth `1`. Generic argument count and total datatype
component count are enforced independently.

The PHP fixture harness follows the consolidated next manifest and respects its
explicit test exclusions. Both protocol targets pass through `bin/aeon-php`:
`core-limits-cts.v1.next.json` passes 32/32 and the complete consolidated
`core-cts.v1.next.json` snapshot passes 265/265. Closing the full target also
aligned standardized diagnostic phase labels and parser-owned paths, made the
JSON envelope robust to malformed UTF-8 diagnostics, and corrected structured
header recognition after a leading shebang.

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
| `max_reference_depth` | enforced | portable JSON finalization; `AeonicLimits::finalizationOptions()` maps the common file |
| `max_materialized_weight` | enforced | portable JSON finalization; `AeonicLimits::finalizationOptions()` maps the common file |
| AEON `max_input_bytes` | enforced | `maxInputBytes`, default `16 MiB`; `maxBytes` remains an alias |
| AEON `max_numeric_literal_characters` | enforced | `maxNumericLiteralCharacters`, default `1024` |
| AEON `max_structured_comment_characters` | enforced | `maxStructuredCommentCharacters`, default `1048576` |
| Telex format counters | enforced | PHP Telex parse/encode and `AeonicLimits::telexOptions()` |
| transport counters | not applicable yet | no PHP framing/header-inspection transport package |

## Hard-coded and separate limits

- SANSA position-index guards belong to SANSA, not this limits contract.
- AEOS schema regex, rule, path, alternative, event, and container budgets are
  schema-validation limits and remain deliberately separate.
- PHP byte offsets and `strlen`-based input accounting operate on source bytes,
  while the new decoded string/key/path counters use deterministic UTF-8 scalar
  counting. Portable span offset-unit alignment remains a separate audit item.

## Remaining rollout

1. Add framing counters if a PHP transport framing surface is introduced;
   absence must not be advertised as conformance.
2. Add a Core-owned Telex import/materialization convenience route, or define an
   equivalent option convention that does not introduce an AES-to-Core package
   dependency cycle.
