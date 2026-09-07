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

## Executed lanes

| Repository/lane | Result |
| --- | --- |
| AltoPelago AES JavaScript, shared immutable CTS plus integrity, provenance, and AET candidate lanes | full suite passed, including all 38 AES Events, 50 Telex, 14 integrity, 11 provenance, and 14 AET vectors |
| AltoPelago AES Rust, shared immutable CTS plus integrity and AET candidate lanes | all conformance, completeness, limits, integrity, and transaction tests passed; Clippy is warning-free |
| AEON TypeScript AES Events CTS | 38 vectors passed |
| AEON TypeScript Telex CTS | 50 vectors passed |
| AEON TypeScript provenance projection checkpoint | all 24 workspace projects typechecked; all 23 non-fuzz unit-test projects passed; focused lexer 127, parser 167, AES 200, and Core 131 tests passed |
| AEON Python Telex/AES tests under the required bundled Python runtime | 12 tests passed |
| AEON PHP AES suite | 91 tests and 256 assertions passed |
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
additional Unicode span fixtures, all header shorthand/conflict combinations,
and mixed historical record-version readers. Those additions should advance a
development CTS and later publish a new immutable snapshot; they do not mutate
snapshot 0.1.

Public AET ingress and encryption, the remaining non-TypeScript provenance
projection and offset audits, index lifecycle closure, and head-aware ASP
storage remain separate unchecked requirements. Base event-stream integrity belongs to
`aes.integrity.v0`, while
`aes.transaction.v0` supplies the support-gated logical carrier, transaction
digest, and source-backed preparation gate. Neither completion changes this
earlier read/export and supported-application acceptance run.
