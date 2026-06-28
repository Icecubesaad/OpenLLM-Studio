---
id: adr-0002-agent-implementation-policy
type: decision
title: Agent implementation policy
status: active
owner: engineering
created: 2026-06-28
updated: 2026-06-28
tags: [agents, engineering, policy, cursor, codex, agy]
related:
  - adr-0001-zsh-python-rust-tool-boundaries
  - adr-0003-destructive-operation-policy
  - adr-0005-agent-reporting-standard
  - adr-0006-evidence-before-claims
---

## Context

Multiple agents—Cursor, Codex, AGY CLI, Claude, and local models—work across the same repos. Without shared policy, implementations diverge: unnecessary rewrites, dependency sprawl, silent destructive edits, and changes that cannot be reviewed or reversed.

## Decision

Every agent-led implementation follows this policy:

| Rule | Requirement |
|------|-------------|
| **Audit first** | Every task starts with understanding scope, existing code, and constraints before writing. |
| **Modify in place** | Default to changing existing code; do not replace working systems without justification. |
| **Minimize dependencies** | Add libraries or tools only when the stdlib or repo stack cannot reasonably solve the problem. |
| **Small reversible changes** | Prefer minimal diffs that can be rolled back in one step. |
| **Tests before merge** | Add or run tests that cover changed behavior; do not claim success without evidence—see [ADR-0006](./ADR-0006-evidence-before-claims.md). |
| **Explain every changed file** | Report what changed, why, and what was intentionally left untouched. |
| **Human approval for destructive ops** | No deletes, force pushes, mass renames, or irreversible actions without explicit approval—see [ADR-0003](./ADR-0003-destructive-operation-policy.md). |

Language and tooling choices defer to [ADR-0001](./ADR-0001-zsh-python-rust-tool-boundaries.md).

## Rationale

- **Audit first** prevents agents from “fixing” the wrong layer or duplicating existing utilities.
- **Modify in place** reduces churn and preserves working context agents and humans already rely on.
- **Minimize dependencies** keeps supply chains and upgrade surfaces small—aligned with removing complexity before adding it.
- **Small reversible changes** make review tractable and rollback safe when an agent misjudges scope.
- **Tests before merge** turns assertions into evidence; agents must not invent pass/fail results.
- **Explain every changed file** supports human review and future agent sessions loading partial context.
- **Human approval for destructive ops** is the hard stop for data loss and repo damage.

## Tradeoffs

| Upside | Downside |
|--------|----------|
| Consistent behavior across Cursor, Codex, AGY, Claude, and local agents | Slower than unconstrained “just rewrite it” agents |
| Easier review and handoff between sessions | Requires agents to read ADRs before acting |
| Lower regression and dependency risk | May defer “clean” refactors that do not change operational requirements |

## Agent workflow

1. Load relevant ADRs and project context from `agent-wiki/`.
2. Audit: list affected paths, existing patterns, and constraints.
3. Propose the smallest change that satisfies the request.
4. Implement, test, and report per [ADR-0005](./ADR-0005-agent-reporting-standard.md).
5. Escalate to the human before any destructive or ambiguous operation.

## Consequences

- Positive: The wiki becomes an operational handbook—not just notes—that any agent can load to match expected engineering behavior.
- Negative: Agents that skip context reads may feel “slow”; that is intentional.
- Follow-ups: Link project-specific runbooks in `30-runbooks/` when a repo needs exceptions to this policy.

## Review

<!-- TODO: Review when agent tooling or merge workflow changes. -->
