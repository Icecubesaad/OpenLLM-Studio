---
id: adr-0006-evidence-before-claims
type: decision
title: Evidence before claims
status: active
owner: engineering
created: 2026-06-28
updated: 2026-06-28
tags: [agents, evidence, trust, validation]
related: [adr-0002-agent-implementation-policy, adr-0005-agent-reporting-standard]
---

## Context

Agents often state conclusions as facts: “build passed,” “tests green,” “no linter errors”—without running commands. That erodes trust and causes bad merges. [ADR-0002](./ADR-0002-agent-implementation-policy.md) forbids inventing results; this ADR defines *how* to report truthfully.

## Decision

### Core rule

**No evidence, no claim.** If you did not run it, did not read it, or did not observe it directly, you must not assert it as fact.

### Observation vs inference

| Label | Use when |
|-------|----------|
| **Observation** | Direct tool output you ran or file content you read—cite command or path. |
| **Inference** | Logical conclusion from observations—mark explicitly (“likely,” “probably,” “based on X”). |
| **Unknown** | Cannot verify in this session—say so; do not guess. |

Example:

- Observation: `npm test` exited 0; 42 tests passed (command run at repo root).
- Inference: Production deploy is probably safe because CI mirrors local test script—**not verified here**.

### Forbidden claims without evidence

- Build / compile succeeded
- Tests passed or failed
- Linter clean
- App runs correctly in browser
- API returned expected response
- Git state (clean, pushed, up to date) without running git
- Performance or security properties without measurement or audit

Allowed alternative: “I did not run tests; recommend: `npm test`.”

### Citing validation

In **Validation** (per [ADR-0005](./ADR-0005-agent-reporting-standard.md)), include:

1. **Command** — exact invocation and working directory.
2. **Result** — exit code and salient output (summarize; do not dump walls of logs).
3. **Scope** — what was and was not exercised.

If a command failed or was skipped, report that explicitly—failure evidence is still evidence.

### Uncertainty

When confidence is below certainty:

- State what is unknown and why (missing tool, no network, ambiguous spec).
- List the **smallest check** that would resolve uncertainty.
- Do not soften unknowns into false confidence.

### Dry-run and destructive ops

Preview output is evidence for *what would happen*—not proof that live execution succeeded. See [ADR-0003](./ADR-0003-destructive-operation-policy.md).

## Rationale

- Trust comes from reproducible checks, not authoritative tone.
- Separating inference prevents the next agent from treating guesses as ground truth.
- Cited commands let humans replay validation in one paste.

## Tradeoffs

| Upside | Downside |
|--------|----------|
| Higher-quality agent output; fewer false “done” | Agents must run tools or admit gaps |
| Easier human audit and CI alignment | Some tasks stay “unverified” until human runs checks |
| Directly improves merge safety | Slightly longer Validation sections |

## Agent workflow

1. Before claiming success, run the relevant check or label the claim as inference/unknown.
2. In reports, use Observation / Inference / Unknown labels where ambiguity exists.
3. Prefer running a quick command over speculating.
4. Never fabricate test, lint, or build results.

## Consequences

- Positive: This ADR is the trust layer for the whole policy core—implementation, reporting, and destructive ops all depend on honest evidence.
- Negative: Agents expose limits more often—that is desired behavior.
- Follow-ups: Repo-specific validation commands belong in `30-runbooks/` or `10-projects/`, not duplicated here.

## Review

<!-- TODO: Review when validation tooling or CI patterns change. -->
