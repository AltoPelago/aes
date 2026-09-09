# Governance

This repository is maintained under a single-maintainer model.

## Maintainer model

- the `aes` reference implementation and working surface are maintained by one
  primary maintainer
- roadmap, release, and merge authority stay with the maintainer
- public visibility does not imply shared governance

## Contribution model

- the source may be published publicly
- outside code contributions are not part of the default workflow
- pull requests should not be assumed to be accepted or reviewed as a normal
  governance path
- changes are maintained directly by the maintainer unless an invited
  collaboration is made explicit
- forks, independent implementations, downstream tooling, and ecosystem
  experiments are encouraged

## Authority boundaries

- reference implementation authority lives in this repository
- normative specification authority lives in `aeonite-org/aeonite-specs`
- immutable shared conformance authority lives in `aeonite-org/aeonite-cts`
- AEON, SANSA, AEOS, ASP, and other consumers own their integration behavior
- consumer-selected policies, schemas, resource limits, and trust decisions do
  not become AES semantics merely because an implementation supports them

See [AUTHORITY.md](./AUTHORITY.md) for the complete publication boundary.

## Practical expectation

The public `aes` repository should be readable, stable, independently testable,
and honest about its maintenance model. It should not imply an open
contribution process or normative authority that the project does not intend
to provide here.
