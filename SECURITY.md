<!--
  Title: Minibench
  Author: Christopher Lester (christopher@miniforge.ai)
  Copyright 2025-2026 Christopher Lester. Licensed under Apache 2.0.
-->

# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| latest  | ✅        |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

To report a security vulnerability, email **security@miniforge.ai** with:

- A description of the vulnerability and its potential impact
- Steps to reproduce or proof-of-concept code
- Any suggested mitigations you are aware of

You should receive an acknowledgement within 48 hours. We will keep you informed as we investigate and address the
report.

## Scope

Minibench hosts and renders state-validation snapshots that product adapters emit. It is a local-first application: the
data plane binds loopback only, and the kernel reads snapshot JSON. Reports in scope:

- Deserialization flaws reachable from snapshot or registry JSON — a malformed or hostile snapshot causing memory
  unsafety, unbounded allocation, or a panic that a caller cannot contain
- Path traversal or arbitrary file read through `MINIBENCH_SNAPSHOT_DIR`, the CLI's directory arguments, or the
  corrections directory
- The data plane binding a non-loopback interface, or serving snapshots the caller should not reach
- Evidence-validation or regression-gate bypasses — a snapshot that should fail `minibench validate` or `minibench diff`
  but passes, since those gates are what downstream consumers trust

## Out of Scope

- Issues that require physical access to a machine
- Social engineering attacks
- Vulnerabilities in third-party dependencies (please report those upstream)
- The content of a product's snapshot: Minibench validates a snapshot against its registry, but it does not and cannot
  verify that the adapter measured the product honestly. Adapter correctness is the product's responsibility.
- Fixtures under `fixtures/` — these are synthetic demo data, not a security boundary

## Preferred Languages

We prefer reports in English.
