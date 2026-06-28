---
id: adr-0001-zsh-python-rust-tool-boundaries
type: decision
title: Zsh, Python, and Rust tool boundaries
status: active
owner: engineering
created: 2026-06-28
updated: 2026-06-28
tags: [tooling, zsh, python, rust, cli, agents]
related: [adr-0002-agent-implementation-policy]
---

## Context

This project spans operator workflows, ad-hoc analysis, and tools that may need to ship or run on multiple machines. Without boundaries, scripts accumulate in the wrong language—slow shell, brittle Python CLIs, or premature Rust for one-off tasks.

## Decision

Use each language by **lifecycle and distribution needs**:

| Tool | Use when |
|------|----------|
| **Zsh** | Glue commands, aliases, path jumps, one-machine workflows, “do this now” automation |
| **Python** | Quick scanners, reports, file parsing, dashboard generation, experiments that may change shape |
| **Rust** | Repeatable CLIs, distributed tools, large filesystem scans, security/audit tools, anything we want to trust, ship, or run everywhere |

Default to the lightest option that fits. Promote upward only when requirements outgrow the current tier.

## Rationale

- **Zsh** is already the shell environment; zero install, instant iteration, ideal for personal operator glue.
- **Python** balances speed of writing with libraries for parsing, reporting, and prototyping—acceptable when correctness and performance are “good enough” and scope is still moving.
- **Rust** pays off when the tool must be **fast**, **portable**, **deterministic**, or **maintained as a product**—not when the problem is still being discovered.

## Tradeoffs

| Choice | Upside | Downside |
|--------|--------|----------|
| Zsh | Fastest to write; no deps | Hard to test, share, or run on non-macOS shells; error-prone at scale |
| Python | Rich stdlib/ecosystem; quick reports | Slower on huge trees; packaging/version drift across machines |
| Rust | Fast, single binary, strong guarantees | Higher upfront cost; overkill for throwaway glue |

**Rule of thumb:** optimize for **time-to-clarity** early, **time-to-trust** when the workflow stabilizes.

## When to promote Zsh → Python

- Logic exceeds ~30 lines or needs structured data (JSON, CSV, YAML).
- Output becomes a report others will re-run or diff.
- You need tests, argparse, or reusable modules.
- The script runs on a schedule or in CI—not only interactively.

## When to promote Python → Rust

- Full-tree scans take noticeable time or memory at production scale.
- The tool must ship as a **binary** without a Python runtime on target hosts.
- Wrong answers or crashes have **security/audit** impact (permissions, secrets scanning, integrity checks).
- Multiple operators or agents depend on **stable CLI flags and exit codes**.
- You are re-running the same script weekly and maintenance cost exceeds a one-time Rust port.

## Never promote because

- Rust is faster (in theory).
- The language is fashionable.
- A rewrite feels cleaner.

Promote only because **operational requirements changed**—distribution, reliability, scale, or audit scope—not aesthetics or preference.

## Agent workflow

Agents (Cursor, Codex, AGY, Claude, local models) must not choose implementation languages arbitrarily.

Default process:

1. **Audit** the problem—scope, callers, runtime constraints, and existing code.
2. **Recommend** the lightest suitable language per this ADR.
3. **Justify** any promotion with concrete operational triggers from the sections above.
4. **Avoid** rewriting a working implementation solely to change languages.

When in doubt, stay at the current tier and file a note in `00-inbox/` if requirements are unclear.

## Consequences

- Positive: Clear lane per task; less duplicate tooling; easier agent context (“is this glue, experiment, or product?”).
- Negative: Promotion steps add brief friction; requires discipline not to skip straight to Rust or stay in shell too long.
- Follow-ups: Add project-specific examples under `10-projects/` or runbooks under `30-runbooks/` when a promoted tool lands.

## Review

<!-- TODO: Review when promotion triggers prove wrong in practice. -->
