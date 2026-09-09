# Security Policy

AES includes parsers, canonical encoders, integrity inputs, provenance checks,
transaction inspection, and resource-limit enforcement. Security-sensitive
reports should be handled privately where possible.

## Reporting a vulnerability

- do not open a public GitHub issue for a suspected vulnerability
- prefer GitHub private vulnerability reporting for this repository when it is
  available
- include a minimal reproduction, affected implementation surface, expected
  impact, and any known workarounds

## Good report content

- affected JavaScript or Rust API
- exact Telex input or portable AES record sequence that triggers the issue
- whether the issue affects parsing, canonical bytes, completeness,
  provenance, integrity, signatures, transactions, or resource limits
- whether the behavior is specification, CTS, or implementation specific

## Scope examples

Security-relevant reports may include:

- canonicalization mismatches with integrity or signature impact
- accepting malformed or incomplete streams as valid
- provenance range or source-identity validation bypasses
- transaction evidence or trust-boundary violations
- parser denial-of-service vectors such as pathological inputs or unbounded work
- discrepancies between the JavaScript and Rust implementations that weaken a
  published validation rule

## Disclosure

Please allow time for triage and mitigation before public disclosure. Once a
fix or mitigation exists, public documentation can follow in the normal
repository history.
