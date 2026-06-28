# Contributing

How to add and maintain vault entries.

## 1. Choose a type and folder

| Type | Folder | Template |
|------|--------|----------|
| project | `10-projects/` | `99-meta/templates/project.md` |
| system | `20-systems/` | extend project template or freeform |
| runbook | `30-runbooks/` | `99-meta/templates/runbook.md` |
| agent | `40-agents/` | freeform with standard frontmatter |
| decision | `50-decisions/` | `99-meta/templates/decision.md` |
| reference | `60-references/` | freeform with standard frontmatter |
| inbox | `00-inbox/` | frontmatter only until filed |

## 2. Create the file

1. Copy the matching template from `99-meta/templates/`.
2. Set `id` to a unique, stable slug (e.g. `openllm-studio`).
3. Set `created` and `updated` to today (ISO date).
4. Set `owner` to your handle or team.
5. Fill body sections; delete unused section headings.
6. Save as `<id>.md` in the target folder.

Example:

```bash
cp 99-meta/templates/project.md 10-projects/my-project.md
# edit frontmatter and body
```

## 3. Link related entries

Add peer `id` values or paths to `related:` in frontmatter. Add inline links in the body where helpful.

## 4. Inbox workflow

- Quick captures → `00-inbox/<short-slug>.md` with `type: inbox`.
- Within one session or sprint: move to the proper folder, change `type`, set `status: draft` or `active`.

## 5. Update existing entries

- Bump `updated` on every substantive edit.
- Change `status` when deprecating; move to `90-archive/` when done.
- Do not rewrite `id` after others may link to it; create a new entry and link instead.

## 6. Quality bar

- One topic per file.
- Imperative voice in runbooks; factual tone elsewhere.
- No placeholder lorem or fake project names.
- Prefer tables and bullet lists over long prose.

## 7. Validation checklist

- [ ] Frontmatter has all required fields
- [ ] `type` matches folder
- [ ] Filename matches `id` (recommended)
- [ ] No secrets in body or frontmatter
- [ ] `related` entries exist or are marked TODO in body
