---
id: adr-0003-destructive-operation-policy
type: decision
title: Destructive operation policy
status: active
owner: engineering
created: 2026-06-28
updated: 2026-06-28
tags: [agents, safety, destructive-ops, rollback]
related: [adr-0002-agent-implementation-policy, adr-0006-evidence-before-claims]
---

## Context

Agents can delete data, rewrite history, and run irreversible shell commands in seconds. [ADR-0002](./ADR-0002-agent-implementation-policy.md) requires human approval for destructive operations but does not define what counts as destructive, what is safe by default, or how to recover when things go wrong.

## Decision

### Requires explicit human approval

Stop and ask before:

| Category | Examples |
|----------|----------|
| **Data loss** | `rm`, `trash`, emptying directories, dropping tables, deleting branches |
| **History rewrite** | `git push --force`, rebase that rewrites shared history, amend after push |
| **Mass mutation** | Bulk renames, repo-wide search-replace, moving project roots |
| **Privilege / system** | `sudo`, modifying launchd/cron, firewall, keychain, system paths |
| **Secrets exposure** | Committing `.env`, keys, tokens; printing secrets to logs |
| **Production touch** | Deploy, migrate prod DB, change live DNS or billing |
| **Ambiguous scope** | “Clean up,” “optimize,” or “refactor” without a bounded file list |

When in doubt, treat the operation as destructive.

### Always safe (without extra approval)

| Category | Examples |
|----------|----------|
| **Read-only inspection** | `git status`, `git diff`, `git log`, listing dirs, reading files |
| **Local additive edits** | New files in scoped task, small targeted edits to named files |
| **Reversible local git** | Commits on unpushed branches, stash, new branches |
| **Non-destructive checks** | Linters, formatters, dry-run flags that do not write |
| **Vault/docs** | Adding or updating Markdown in `agent-wiki/` per ADR schema |

Safe does **not** mean silent—still report what was read or changed.

### Backup requirements

Before approved destructive work:

1. Confirm what will be lost if the step fails.
2. Prefer **copy over move**; prefer **git-tracked revert** over filesystem delete.
3. For non-git or bulk deletes: ensure a recoverable copy exists (branch, stash, archive, or explicit backup path) and name it in the report.
4. Never assume Trash or reflog will save the operation—state the recovery path.

### Rollback expectations

Every destructive change plan must include:

- **Rollback trigger** — what failure signal stops execution.
- **Rollback steps** — exact commands or git operations to undo.
- **Verification** — how to confirm rollback succeeded.

If rollback is unknown or expensive, escalate before proceeding.

### Dry-run-first policy

When a tool supports dry-run, preview, or `--dry-run` / `-n` / `--check`:

1. Run dry-run first and show output.
2. Proceed to live run only after human approval for destructive categories, or when the live run is explicitly in scope for a non-destructive task.

Applies to: deletes, deploy scripts, migrations, package upgrades with post-install hooks, and filesystem mutators.

## Rationale

- Explicit categories reduce “helpful” agent damage.
- Safe defaults speed routine work without lowering the bar on irreversible ops.
- Backup + rollback + dry-run turn approval from a checkbox into a recoverable plan.

## Tradeoffs

| Upside | Downside |
|--------|----------|
| Fewer accidental data losses and force-pushes | More pause points on ambiguous tasks |
| Clear escalation path for agents | Humans must respond to approval requests |
| Reusable across all repos in the policy set | Edge cases may still need judgment |

## Agent workflow

1. Classify the planned operation: **safe**, **destructive**, or **ambiguous**.
2. If destructive or ambiguous: stop, state impact, backup plan, rollback plan, dry-run output.
3. Wait for explicit human approval—silence is not approval.
4. Execute minimally; report evidence per [ADR-0006](./ADR-0006-evidence-before-claims.md).

## Consequences

- Positive: Shared safety bar for Cursor, Codex, AGY, Claude, and local agents across all projects.
- Negative: Legitimate cleanup takes an extra message round-trip.
- Follow-ups: Repo-specific destructive lists may live in `30-runbooks/`; they extend but do not weaken this ADR.

## Review

<!-- TODO: Review after any destructive incident or tooling change. -->
