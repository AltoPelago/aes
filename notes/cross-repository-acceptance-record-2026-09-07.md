# Cross-repository AES/Telex acceptance record

Date: 2026-09-07

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
spelled out. The mutable AES development snapshot adds six common projection
vectors for ASCII, precomposed non-ASCII, combining sequences, astral scalars,
BOM plus CRLF, and source absence. Six additional header-projection fixtures
cover structured and shorthand normalization, fail-closed mixed-form conflict,
the default body-only projection, and an explicitly selected empty document
projection, plus quoted `"aeon:*"` payload disambiguation. The resulting 79
vectors pass in TypeScript, Rust, Python, and PHP; no released snapshot was
modified. Each CLI also exposes
exact-source provenance explicitly through `inspect --portable-aes
--source-provenance` and `inspect --telex --source-provenance`. Plain portable
projection omits a local span that has no immutable origin.

## Executed lanes

| Repository/lane | Result |
| --- | --- |
| AltoPelago AES JavaScript, shared immutable CTS plus integrity, provenance, and AET candidate lanes | full suite passed, including all 38 AES Events, 50 Telex, 14 integrity, 11 provenance, and 14 AET vectors |
| AltoPelago AES Rust, shared immutable CTS plus integrity and AET candidate lanes | all conformance, completeness, limits, integrity, and transaction tests passed; Clippy is warning-free |
| AEON TypeScript AES Events CTS | 38 vectors passed |
| AEON TypeScript Telex CTS | 50 vectors passed |
| AEON TypeScript provenance projection checkpoint | all 24 workspace projects typechecked; all 23 non-fuzz unit-test projects passed; focused lexer 127, parser 167, AES 203, and Core 132 tests passed |
| AEON Python provenance projection checkpoint | all 323 unit tests passed; all consolidated CTS lanes passed, including Core 265, AES 79, canonicalization 27, finalization 18, inspect 6, map 3, SANSA 46, annotations 14, and AEOS 118 cases |
| AEON Rust provenance projection checkpoint | all 554 workspace unit tests passed; all consolidated CTS lanes passed, including Core 265, AES 79, canonicalization 27, finalization 14, finalization limits 4, inspect 6, map 3, SANSA 9, annotations 14, and AEOS 118 cases; Clippy with warnings denied and formatting passed |
| AEON PHP provenance projection checkpoint | all 873 repository tests and 2,658 assertions passed; Core CTS 265/265, limits 32/32, and AES 91 tests/256 assertions passed; Composer manifest validation completed with only pre-existing metadata/constraint warnings |
| AEON cross-language development AES projection CTS | all 79 vectors passed independently in TypeScript, Rust, Python, and PHP; six exact-source span and six header-projection vectors were added only to the mutable next manifest |
| SANSA full suite | 257 tests passed |
| ASP full suite, including the source-derived round trip, hardened AET scalar bridge, and portable subtree/index lifecycle | 1,110 tests passed; 62 conformance cases passed |
| AEON TypeScript integrity package | 14 tests passed |
| AEON TypeScript CLI | 96 tests passed |
| AEON Rust CLI | 123 tests passed; `cargo fmt --check` passed |
| Aeon Tonics focused legacy-boundary tests | `aes-diff` 30 tests and `aeon-edit` 80 tests passed |

The first sandboxed ASP full-suite run could not bind loopback test servers and
reported eight `listen EPERM` failures. The same unmodified suite was rerun with
loopback access and passed all 1,109 tests; these were environment restrictions,
not implementation failures.

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
ranges, index lifecycle closure, and head-aware ASP storage remain separate
unchecked requirements.
Base event-stream integrity belongs to
`aes.integrity.v0`, while
`aes.transaction.v0` supplies the support-gated logical carrier, transaction
digest, and source-backed preparation gate. Neither completion changes this
earlier read/export and supported-application acceptance run.
