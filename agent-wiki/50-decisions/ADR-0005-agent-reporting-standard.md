---
id: adr-0005-agent-reporting-standard
type: decision
title: Agent reporting standard
status: active
owner: engineering
created: 2026-06-28
updated: 2026-06-28
tags: [agents, reporting, cursor, codex, agy]
related: [adr-0002-agent-implementation-policy, adr-0006-evidence-before-claims]
---

## Context

Agents finish tasks with inconsistent reports—some omit risks, some claim success without tests, some bury the file list. Humans and follow-up sessions need a predictable structure to review work and resume context.

## Decision

Every **implementation agent** (any agent that changes code, config, or operational state) must end with a report using these sections **in order**. Omit a section only when truly not applicable; say `N/A` with one-line reason.

### Mandatory report structure

| # | Section | Content |
|---|---------|---------|
| 1 | **Summary** | What was requested, what was done, outcome in 2–4 sentences. |
| 2 | **Architecture** | How the change fits existing structure; patterns followed or extended. |
| 3 | **Risks** | What could break, who is affected, severity. |
| 4 | **Assumptions** | What was inferred without confirmation; dependencies on environment or prior state. |
| 5 | **Proposed Plan** | What you intended before implementation (or post-hoc if exploratory). |
| 6 | **Files To Change** | Planned or actual paths; mark added / modified / untouched intentionally. |
| 7 | **Implementation** | What was actually built; notable deviations from the plan. |
| 8 | **Validation** | Commands run, outputs observed, tests added—per [ADR-0006](./ADR-0006-evidence-before-claims.md). |
| 9 | **Remaining Issues** | Open bugs, follow-ups, blocked items, deferred scope. |

### Scope rules

- **Small tasks** (single-file fix, typo): compress sections 2–5 but keep all nine headings.
- **Read-only audits**: sections 6–7 may be `N/A`; Validation cites inspection commands only.
- **Blocked tasks**: stop after Assumptions or Proposed Plan; explain blocker in Remaining Issues.

Reports are part of the deliverable—not optional commentary.

## Rationale

- Fixed section order lets humans skim and lets agents parse prior session output.
- Separating **Assumptions** from **Risks** reduces hidden inference.
- **Files To Change** vs **Implementation** catches scope creep and undocumented edits.
- **Validation** and **Remaining Issues** pair with evidence policy—no silent “done.”

## Tradeoffs

| Upside | Downside |
|--------|----------|
| Reviewable, resumable handoffs across Cursor, Codex, AGY, Claude | Verbose for trivial changes |
| Easier to spot missing tests or unlisted files | Agents may pad sections—keep each concise |
| Aligns with consolidation: one report shape everywhere | Requires discipline on every task |

## Agent workflow

1. Before coding: draft **Proposed Plan** and **Files To Change** mentally or in reply.
2. After coding: fill **Implementation** and **Validation** from actual work—not intent.
3. Always list **what was not touched** when scope boundaries matter.
4. Link relevant ADRs (e.g. language choice → ADR-0001).

## Consequences

- Positive: Reporting becomes a reusable operational standard across all repos inheriting this vault.
- Negative: Short tasks feel heavier—use compressed subsections, not skipped headings.
- Follow-ups: Optional template in `99-meta/templates/` if agents need a copy-paste skeleton.

## Review

<!-- TODO: Review when report format needs tightening. -->
