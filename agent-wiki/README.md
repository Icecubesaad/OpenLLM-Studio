# Agent Wiki

Shared Markdown/YAML knowledge vault for Cursor, Codex, AGY CLI, Claude, and local agents.

## Purpose

Store project context, runbooks, decisions, and references in plain files. No app, database, or web UI — just folders, frontmatter, and links.

## How agents should use this vault

1. **Read before acting.** Check `10-projects/` for active work, `30-runbooks/` for procedures, and `50-decisions/` for settled choices.
2. **Respect folder roles.** See [ARCHITECTURE.md](./ARCHITECTURE.md) for what belongs where.
3. **Follow metadata.** Every entry uses YAML frontmatter with `id`, `type`, `title`, `status`, `owner`, `created`, `updated`, `tags`, and `related`.
4. **Prefer links over duplication.** Use `related` and inline Markdown links; do not copy large blocks between files.
5. **Inbox first.** New notes go in `00-inbox/` until filed into the correct folder.
6. **Stay minimal.** Add only what future agents need; archive stale items to `90-archive/`.

## Quick paths

| Need | Folder |
|------|--------|
| Active project context | `10-projects/` |
| Infrastructure / tooling | `20-systems/` |
| Step-by-step procedures | `30-runbooks/` |
| Agent roles and rules | `40-agents/` |
| Architecture / product decisions | `50-decisions/` |
| External specs and links | `60-references/` |
| Templates | `99-meta/templates/` |

## Tool integration

### Cursor

- Add `agent-wiki/` to workspace or reference paths in `.cursor/rules` or project rules.
- Point agents at specific files: `@agent-wiki/10-projects/my-project.md`.
- Use `@agent-wiki/README.md` or `@agent-wiki/ARCHITECTURE.md` as stable context anchors.

### Codex / Claude / local agents

- Include the vault root or relevant subfolders in system/context prompts.
- Load frontmatter + body for the files that match the task; skip unrelated folders.
- When writing back, use templates from `99-meta/templates/` and follow [CONTRIBUTING.md](./CONTRIBUTING.md).

### AGY CLI

- `.agyrc` at the vault root maps AGY to key folders and templates.
- Run AGY from the repo root or set context to `agent-wiki/` so paths resolve.

## Adding content

See [CONTRIBUTING.md](./CONTRIBUTING.md). Copy a template, fill frontmatter, save in the correct folder, remove from inbox if applicable.

## Conventions

- Filenames: lowercase, hyphenated, `.md` extension (e.g. `deploy-staging.md`).
- Dates: ISO 8601 (`YYYY-MM-DD`).
- Status: `draft`, `active`, `deprecated`, `archived`.
- Types: `project`, `system`, `runbook`, `agent`, `decision`, `reference`, `inbox`.
