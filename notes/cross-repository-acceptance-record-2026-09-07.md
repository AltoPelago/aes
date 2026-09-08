# Cross-repository AES/Telex acceptance record

Date: 2026-09-07; incremental acceptance rerun 2026-09-09

## Result

The existing AES/Telex implementation and consumer acceptance lanes pass. One
new ASP integration vector closes the previously unrepresented supported path:

```text
AEON source
  -> portable aes.events.v0 encoded as telex.aes=0
  -> named SO scalar-replacement application
  -> ASP v0 / durable AES-DB
  -> strict portable aes.events.v0 view
  -> canonical telex.aes=0
```

The fixture derives the mutation address and expected kind from the source
Telex record, verifies the initial AES-DB Telex view is equivalent, performs the
already-supported named scalar mutation, and verifies that path, identity,
datatype, and record order survive the returned Telex stream. It does not add a
Telex writer or make a bare event stream actionable.

The existing node-head negative vector remains authoritative: direct node-head
replacement returns `AES_COMPAT_NODE_HEAD_STORAGE_REQUIRED`, writes nothing,
and leaves the journal empty. This is an accepted fail-closed result, not an
acceptance gap.

The incremental rerun also closes two limit-boundary defects found only when
the complete ecosystem was exercised together. All four Telex encoders now
enforce shared AES structural counters before serialization. Separately, the
ASP v0 read adapter names and bounds its legacy recursive-attribute
compatibility allowance while leaving fresh Telex export subject to the
caller's selected limits. Historical ASP state can therefore remain readable
without redefining the portable ingress defaults.

The later logical AET carrier lane also passes without opening transport
ingress. JavaScript and Rust agree on all 14 candidate transaction vectors, and
the ASP bridge admits only an independently validated, supported,
integrity-verified scalar AET into the existing named SO application path.
Generic inspection remains non-actionable.

The exact-source provenance lane adds 11 shared JavaScript/Rust candidate
vectors. Source-backed preparation now blocks readiness unless every payload
record has a verified origin and every exact artifact/span audit completes;
ordinary source-unavailable events remain locally valid and unverified. The
registered scalar application does not select this preparation and its mutation
behavior is unchanged.

The TypeScript projection checkpoint now retains an independent node-head
range, names its native coordinates as UTF-16 code units, and converts them to
UTF-8 byte spans only when exact source bytes are supplied. The opt-in path is
available through the named compatibility adapter and Core Telex export. Its
regressions cover ASCII, precomposed and combining Unicode, astral scalars,
BOM, CRLF, shorthand headers, invalid UTF-8, and scalar-splitting ranges.

The Python projection checkpoint now retains independent node-head and
attribute-entry ranges, names native offsets as Unicode code points, and keeps
an accepted BOM in that coordinate space. Its named adapter and public Telex
exports optionally derive a lowercase SHA-256 origin from exact source bytes
and convert retained ranges to UTF-8 byte spans. The shared TypeScript fixture
produces the same origin and spans; invalid UTF-8 and invalid native ranges
fail closed.

The Rust projection checkpoint now retains the exact compiled source and
independent node-head and attribute-entry byte ranges. Lexer offsets are UTF-8
bytes while columns count Unicode scalars. The named adapter and Core/SDK Telex
exports optionally derive the same lowercase SHA-256 origin as TypeScript and
Python, retain only proven ranges, and fail closed on invalid UTF-8, invalid
ranges, or a mismatched source artifact. Anonymous indexed occurrences remain
origin-only because the v0 flattened event shape does not retain an independent
range for them; the adapter reports that omission rather than asserting the
owner's range as occurrence provenance.

The PHP projection checkpoint establishes lexer offsets as UTF-8 bytes and
columns as Unicode-scalar coordinates, while retaining an accepted BOM in the
exact source coordinate space. Independent node-head and attribute-entry
ranges now survive event emission. The named adapter and Core Telex exports
optionally derive the same lowercase SHA-256 origin as TypeScript, Python, and
Rust, retain proven byte spans for all PHP source occurrences, and fail closed
on invalid UTF-8, invalid or scalar-splitting ranges, and a mismatched source
artifact.

The AEON specification and shared CTS checkpoint removes the last ambiguous
"character offset" language from the span appendix, the normative AEOS input
contract, and the CTS protocol. Spans are now uniformly defined as half-open,
zero-based UTF-8 byte ranges into the exact, unnormalised source artifact, with
BOM, CRLF, scalar-boundary, empty diagnostic-span, and absent-source behavior
spelled out. The AES projection 0.3 snapshot adds six common projection
vectors for ASCII, precomposed non-ASCII, combining sequences, astral scalars,
BOM plus CRLF, and source absence. Six additional header-projection fixtures
cover structured and shorthand normalization, fail-closed mixed-form conflict,
the default body-only projection, and an explicitly selected empty document
projection, plus same-name quoted `"aeon:*"` payload/header disambiguation. The
complete 82-vector snapshot passes in TypeScript, Rust, Python, and PHP and is
prepared locally as the new immutable, content-hash-pinned
`aes-cts-v1-snapshot-0.3`; the historical 0.2 snapshot was not modified. The
snapshot, canonical conformance pointers, and deterministic website lock are
recorded in signed local commits `3dad21e`, `4d8c539`, and `8a0ab96`. Each
CLI also exposes
exact-source provenance explicitly through `inspect --portable-aes
--source-provenance` and `inspect --telex --source-provenance`. Plain portable
projection omits a local span that has no immutable origin.

## Executed lanes

| Repository/lane | Result |
| --- | --- |
| AltoPelago AES JavaScript, shared immutable CTS plus integrity, provenance, and AET candidate lanes | all 254 tests passed, including all 38 AES Events, 50 Telex, 14 integrity, 11 provenance, and 14 AET vectors |
| AltoPelago AES Rust, shared immutable CTS plus integrity and AET candidate lanes | all conformance, completeness, limits, integrity, and transaction tests passed; Clippy is warning-free |
| AEON TypeScript AES Events CTS | 38 vectors passed |
| AEON TypeScript Telex CTS | 50 vectors passed |
| AEON TypeScript current workspace checkpoint | all 23 non-fuzz package/tool test projects passed; focused AES package passed 207 tests; canonical CTS passed 46, transport-limit CTS passed 8, SANSA CTS passed 46, and annotation CTS passed 14 |
| AEON Python current workspace checkpoint | all 333 unit tests passed; the focused Telex codec suite passed 15 tests, including encode-side structural-limit enforcement |
| AEON Rust provenance projection checkpoint | all 555 workspace unit tests passed; the immutable AES projection 0.3 lane passed 82/82; Clippy with warnings denied and formatting passed |
| AEON PHP current workspace checkpoint | all 901 repository tests and 2,734 assertions passed, including encode-side structural-limit enforcement |
| AEON cross-language AES projection CTS | all 82 vectors in immutable snapshot 0.3 passed independently in TypeScript, Rust, Python, and PHP; every referenced suite is content-hash pinned |
| AEON TypeScript aggregate CTS | the complete local aggregate passed with the immutable AES 0.3 target: AEOS 118, Core 271, AES projection 82, AES Events 38, Telex 50, canonical 46, finalization limits 4, transport limits 8, SANSA 46, and annotations 14 |
| Telex/JSON performance sanity check | on the local Apple M4 Pro with Node 24.16, the limits-aware benchmark completed at 100, 10,000, and 100,000 events; at 100,000 events Telex encoded in 199.084 ms versus 146.698 ms for validate-plus-JSON and produced 26.48% fewer bytes; raw `JSON.stringify` remains a separate 8.949 ms engine baseline |
| SANSA full suite | 257 tests passed |
| ASP full suite, including the source-derived round trip, hardened AET scalar bridge, and portable subtree/index lifecycle | 1,112 tests passed; 62 conformance cases passed; the focused portable mutation set passed 130 tests |
| AEON TypeScript integrity package | 14 tests passed |
| AEON TypeScript CLI | 97 tests passed |
| AEON Rust CLI | 123 tests passed; `cargo fmt --check` passed |
| Aeon Tonics focused legacy-boundary tests | `aes-diff` 30 tests and `aeon-edit` 80 tests passed |

The first sandboxed ASP full-suite run could not bind loopback test servers and
reported `listen EPERM` failures. The same suite was rerun with loopback access
and passed all 1,112 tests; these were environment restrictions, not
implementation failures.

The 2026-09-09 promotion preparation also advances the mutable AES projection
manifest to snapshot id 0.4, moves TypeScript/Rust/Python default claims to the
immutable 0.3 path, adds matching released/legacy/next PHP projection commands,
and updates the canonical specification pointers and website projection
fixtures. CTS repository validation, claim validation, all four 82-vector
implementation runs, and the deterministic website publication test pass. The
website lock now identifies specification revision
`4d8c5398ec0570fe54acc9aa65e5c8ccca16742c` and source digest
`ecfa1ddb9cf1b7f9152d56efdf11a1707d35bf6a056bf4bab984e4c8ae13cf7f`.
No repository was pushed and no website was deployed.

## Coverage confirmed by the existing lanes

- complete versus partial profile behavior, ancestry, duplicate addresses, and
  event-local validity;
- expanded node-head paths, node/reference translation, and separation of
  structural identity from path identity;
- flattened attribute ancestry and quoted attribute paths;
- expanded datatype structure and compact Telex descriptor canonicalization;
- WTC lexical preservation, anchor/reference forms, and authority separation;
- header-plane opt-in, header-before-body ordering, and header/body address
  separation;
- origin/span dependency and canonical origin validation;
- record-order preservation and format/resource-limit boundaries;
- SANSA selection/navigation, scalar Telex mutation, identity preservation, and
  rejection of ambiguous or partial portable mutations;
- ASP strict read views, Telex export, historical reads, replay, checkpoint,
  compaction floor, backup, restore, and point-in-time recovery;
- supported scalar and contained-scalar SO dispatch plus stable rejection of
  node-head storage expansion.

## Still-open work (not failures of this run)

The more exhaustive matrix in the primary checklist remains useful future CTS
work: all structural-identity locations across every source implementation,
all header shorthand/conflict combinations, and mixed historical
record-version readers. Those additions should advance a
development CTS and later publish a new immutable snapshot; they do not mutate
snapshot 0.1.

Public AET ingress and encryption, Rust independent anonymous-occurrence
ranges, and head-aware ASP storage remain separate unchecked requirements.
Base event-stream integrity belongs to
`aes.integrity.v0`, while
`aes.transaction.v0` supplies the support-gated logical carrier, transaction
digest, and source-backed preparation gate. Neither completion changes this
earlier read/export and supported-application acceptance run.
