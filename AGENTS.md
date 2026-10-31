# AGENTS.md

Instructions for any agent working in this repository. Read this before editing.

## The one rule that gets broken

`docs/` here is a **mirror of the Obsidian vault**. Any change to a file in
`docs/` must be written to the vault in the same change. Any change made in the
vault must be copied back into `docs/`. The two copies are never allowed to
differ.

- Vault (canonical, has the graph and the canvas view):
  `/home/saptodeep/Projects/AgentsMemory/Projects/Newton/`
- Repository mirror: `docs/`

| Repo | Vault |
|---|---|
| `docs/PRD.md` | `PRD.md` |
| `docs/Project Brief.md` | `Project Brief.md` |
| `docs/Security Audit.md` | `Security Audit.md` |
| `docs/Newton Harness.md` | `Newton Harness.md` |
| `docs/Working Flow.canvas` | `Working Flow.canvas` |
| `docs/Memory/Memory Graph Creation.canvas` | `Memory/Memory Graph Creation.canvas` |
| `docs/UI/*` | `UI/*` |

Sync both ways after any doc edit:

```sh
# repo -> vault
cd docs && cp -r --parents * /home/saptodeep/Projects/AgentsMemory/Projects/Newton/

# vault -> repo
cd /home/saptodeep/Projects/AgentsMemory/Projects/Newton && cp -r --parents * /home/saptodeep/Projects/Newton/docs/
```

Then check that nothing diverged: `diff -r docs /home/saptodeep/Projects/AgentsMemory/Projects/Newton`.

Why this exists: the vault is where the user reads and links notes, the repo is
where contributors read them. Two copies that drift silently are worse than one
copy that is hard to find.

## Vault conventions that also apply in `docs/`

- **No frontmatter, no tags.** Ever. Not in `docs/`, not in the vault.
- **Wikilinks** (`[[Visual Spec]]`) are the linking style. On GitHub they render
  as plain text; that is expected and not a bug to "fix" by rewriting them as
  Markdown links.
- **A wikilink must point at a note that exists.** If you add `[[Foo]]`, create
  `Foo` or drop the link.
- **`.canvas` files are JSON.** They render in Obsidian, not on GitHub. Read
  them as JSON, edit the `text` fields, keep the node ids stable.
- **Date every research claim** with `Checked YYYY-MM-DD.` at the end.
- **New doc?** Put it where the vault taxonomy puts it, then copy it here. Do
  not create a doc in `docs/` only.

## Code rules

- Build and test before claiming anything works: `./scripts/check.sh`.
  It runs exactly what CI runs. Green locally, green in CI, or it is not done.
- CI must pass on all three platforms. If a change only builds on Linux, it is
  not merged.
- `src/Theme.qml` and `OrbState::name()` share the five state strings. Renaming
  one without the other is caught by `tst_orbstate`, but do not rely on the test
  to tell you the design changed: the design lives in `docs/UI/Visual Spec.md`.
- Do not add a web view, a JS runtime or a telemetry SDK. The product promise is
  a local, quiet binary; see `docs/Security Audit.md`.
- Existing code stays unless the task is to change it. Add tests, do not delete
  working files.

## Where the truth lives

- What Newton is: `docs/Project Brief.md`
- What it must do: `docs/PRD.md`
- What it must look like: `docs/UI/Visual Spec.md` (measured from the renders)
- What it must not become: `docs/Security Audit.md`
- How a turn runs: `docs/Newton Harness.md` and `docs/Working Flow.canvas`
