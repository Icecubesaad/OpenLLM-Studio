---
id: adr-0004-knowledge-lifecycle-policy
type: decision
title: Knowledge lifecycle policy
status: active
owner: engineering
created: 2026-06-28
updated: 2026-06-28
tags: [agent-wiki, knowledge, lifecycle, archive]
related: [adr-0002-agent-implementation-policy]
---

## Context

The vault holds both **facts** (projects, systems, references) and **rules** (ADRs in `50-decisions/`). Without lifecycle rules, agents file content in the wrong folder, duplicate runbooks as project notes, or leave stale docs marked `active`.

## Decision

### Where knowledge belongs

| Location | Holds | Does not hold |
|----------|-------|----------------|
| **`00-inbox/`** | Unsorted captures, quick agent notes, items awaiting classification | Long-lived policy or procedures |
| **`10-projects/`** | Per-repo context: scope, stack, paths, contacts, open questions | Generic engineering rules |
| **`20-systems/`** | Shared infrastructure: hosts, services, env patterns across projects | One-off task notes |
| **`30-runbooks/`** | Repeatable procedures: prerequisites, steps, verification, rollback | Why we chose an approach (→ ADR) |
| **`50-decisions/`** | ADRs: durable **rules** and architectural choices | Project status updates |
| **`60-references/`** | Pointers to external specs, APIs, upstream docs | Full copies of large manuals |
| **`90-archive/`** | Deprecated or completed entries with `status: archived` | Current operational truth |

**Policy vs facts:** ADRs govern *how* work is done; project/system/runbook entries describe *what exists*. Policy is reusable across OpenLLM Studio, PiperDesk, LockIn, Plum OS, and future repos.

### Promotion paths

```
00-inbox → (classify) → 10-projects | 20-systems | 30-runbooks | 60-references
                              ↓
                    stable choice worth recording
                              ↓
                         50-decisions (ADR)
                              ↓
                         superseded or done
                              ↓
                         90-archive/
```

| From | To | When |
|------|-----|------|
| Inbox | Project | Content is tied to one repo/product and will stay relevant |
| Inbox | System | Content describes shared infra used by multiple projects |
| Inbox | Runbook | Steps are repeatable and verified at least once |
| Inbox | Reference | Value is a durable external link or citation |
| Project / practice | ADR | A **rule or choice** should bind future humans and agents |
| Any folder | Archive | Superseded, wrong, or completed—update `status` and move |

Do **not** promote inbox items directly to ADR without filing context in project or runbook first, unless the note is already a complete decision record.

### Demotion and archive

- Set `status: deprecated` when replaced; link to the successor in `related`.
- Move to `90-archive/` when no agent should treat it as active truth.
- Never delete history without [ADR-0003](./ADR-0003-destructive-operation-policy.md) approval.

### Agent filing rules

1. New session notes → `00-inbox/` unless the target folder is obvious.
2. Do not duplicate ADR text into project files—link to `50-decisions/`.
3. One topic per file; use `related` for peers.
4. Bump `updated` on every substantive edit.

## Rationale

- Separating **rules** (ADRs) from **facts** (projects, systems) lets policy travel across repos without copy-paste.
- Inbox absorbs friction so agents can capture fast and classify later.
- Archive preserves audit trail without polluting active context loads.

## Tradeoffs

| Upside | Downside |
|--------|----------|
| Agents load smaller, relevant slices | Requires periodic human filing of inbox |
| Stable policy core (target 8–12 ADRs) | Mis-filed notes until someone promotes them |
| Clear promotion triggers | Judgment calls at inbox → ADR boundary |

## Consequences

- Positive: Vault scales as an engineering policy repository, not an ever-growing notes pile.
- Negative: Inbox can backlog if nobody files it—schedule occasional triage.
- Follow-ups: Stop expanding general docs; grow `50-decisions/` until the policy core is stable.

## Review

<!-- TODO: Review when vault structure or folder count changes. -->
