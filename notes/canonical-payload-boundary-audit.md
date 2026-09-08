# Portable AES canonical-payload boundary audit

Status: completed implementation audit, 2026-09-08

## Outcome

The reviewed ASP, AEOS, integrity, and CTS paths now preserve the portable
`aes.events.v0` representation kind and canonical string payload as separate
semantic inputs. Source-only spelling is normalized before the portable
boundary and is neither transported nor admitted to portable integrity input.

No change to `asp.read-result.v0` storage is required. Its implementation-native
value objects remain a named legacy source contract; the strict compatibility
adapter is the boundary that expands them into flat portable records.

## Boundary inventory

| Surface | Input representation | Portable rule | Verification |
| --- | --- | --- | --- |
| AEON TypeScript, Rust, Python, and PHP projectors | implementation-native parser values | emit normative `kind` plus normalized string `value`; containers become flat value-less records | the immutable AES projection 0.3 target passes 82/82 in all four implementations, including the 16-family scalar matrix, numeric spelling normalization, and distinct clone/pointer references |
| ASP strict read view | versioned `asp.read-result.v0` values | normalize legacy fields, synthesize `NodeHead`, flatten nested values, translate references, and omit source `raw` | the value-family matrix covers every ASP scalar family; equal `4_2` and `42` source spellings now produce equal portable records and content fingerprints |
| AEOS direct Telex ingress | validated portable records | consume `kind`, canonical `value`, split datatype components, flat paths, and identity metadata without reparsing AEON | TypeScript, Rust, Python, and PHP validate portable records before adaptation; PHP now preserves the explicit portable temporal kind instead of re-inferring it from a shared runtime class or `&` payload |
| `aes.integrity.v0` | validated `aes.events.v0` records | cover `kind` and `value` independently in structural logical bytes | candidate vectors 15 and 16 give different digests for the same `2026-09-07` payload as `StringLiteral` and `DateLiteral`; vector 17 rejects a source-only `raw` field |
| Shared CTS | released v0 snapshots plus immutable AEON projection snapshot 0.3 | released targets remain immutable; later payload assertions land only in the 0.4 `.next` target | `aes-cts.v1.snapshot-0.3.json` includes `08-portable-value-payloads.json` with every suite hash-pinned; the source-lane normalizer exposes canonical portable `value` without changing older targets |

## Representation rules confirmed

- `kind` is the normative AEON representation-kind token. Runtime class names
  are implementation details and cannot replace it at a portable boundary.
- Every portable scalar and reference payload is a string. Boolean, numeric,
  null, temporal, and reference consumers interpret that string only after AES
  validation and retain the declared kind while doing so.
- `ObjectNode`, `ListNode`, `TupleLiteral`, and `NodeLiteral` do not carry a
  `value`; descendants are independent flat events. `NodeHead` carries only the
  canonical tag payload at its explicit child address.
- AEON delimiters, sigils, trimtick mechanics, numeric separators, parser
  objects, and nested AST trees are source representation. They are not
  portable event fields.
- Integrity validates the selected AES event context before encoding. Equal
  payload text under different recognized kinds remains distinct; alternate
  source spellings normalized to the same portable record remain equal under
  semantic integrity.

## Snapshot handling

The released `aes-events-cts-v0-snapshot-0.1` and
`telex-cts-v0-snapshot-0.1` files were not modified. The complete 82-vector
projection target was promoted by minting the new immutable,
content-hash-pinned `aes-cts-v1-snapshot-0.3`; the historical 0.2 target remains
unchanged. The mutable `.next` manifest has advanced to snapshot id 0.4.

## Remaining work

There is no remaining implementation blocker under this audit. New value
families, deeper payload grammar validation, or Film-specific binary payload
layouts require their own versioned contract work; they do not reopen the
portable v0 boundary.
