# Architecture

Layout and metadata conventions for this vault.

## Folder map

| Folder | Purpose |
|--------|---------|
| `00-inbox/` | Unsorted captures. File or delete within a reasonable window. |
| `10-projects/` | Per-project context: scope, stack, paths, contacts, open questions. |
| `20-systems/` | Shared infrastructure: hosts, services, repos, env patterns. |
| `30-runbooks/` | Repeatable procedures with prerequisites and verification steps. |
| `40-agents/` | Agent lanes, tool rules, handoff notes. |
| `50-decisions/` | Decision records (context, choice, consequences). |
| `60-references/` | Pointers to external docs, APIs, specs — not full copies. |
| `90-archive/` | Deprecated or completed entries moved here with status updated. |
| `99-meta/` | Vault meta: templates, schemas, maintenance notes. |

Number prefixes enforce sort order. Do not renumber without updating `.agyrc` and agent rules.

## Entry format

Each entry is a single Markdown file with YAML frontmatter:

```yaml
---
id: unique-slug
type: project | system | runbook | agent | decision | reference | inbox
title: Human-readable title
status: draft | active | deprecated | archived
owner: handle or team
created: YYYY-MM-DD
updated: YYYY-MM-DD
tags: []
related: []
---
```

### Field rules

| Field | Required | Notes |
|-------|----------|-------|
| `id` | yes | Stable slug; matches filename stem when possible |
| `type` | yes | Must match folder intent (see table above) |
| `title` | yes | Short display name |
| `status` | yes | Drives whether agents should trust the content |
| `owner` | yes | Who maintains the entry |
| `created` | yes | First publication date |
| `updated` | yes | Last meaningful edit |
| `tags` | no | Free-form labels for search/filter |
| `related` | no | List of `id` values or relative paths to other entries |

Body structure is flexible. Templates in `99-meta/templates/` suggest minimal sections per type.

## Linking

- **Within vault:** Markdown links or `related` frontmatter entries.
- **Outside vault:** URLs in body or `60-references/` entries; avoid embedding secrets.

## Lifecycle

```
00-inbox → typed folder (draft) → active → deprecated → 90-archive/
```

When archiving, set `status: archived`, update `updated`, and move the file. Do not delete history without explicit approval.

## What not to store

- Credentials, tokens, or private keys
- Large generated dumps or logs
- Binary blobs (link out instead)

## Agent read order (suggested)

1. Task-relevant file(s) by `id` or tag
2. `related` entries one hop deep
3. Parent project or system doc if the task touches a repo or host

Stop when context is sufficient; do not load the entire vault.
