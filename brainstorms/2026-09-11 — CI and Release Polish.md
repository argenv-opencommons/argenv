# 2026-09-11 — CI and Release Polish

## Starting point

`argenv` (under `argenv-opencommons`, a separate org from `sync-dot-mesh`)
arrived as a substantial, real implementation — ~3,365 lines across
`src/`/`tests/`, six dedicated test files, dual `MIT OR Apache-2.0`
licensing already correct for a general-purpose crate other projects
will depend on, three working CI jobs (including a genuinely clever
one: validating the emitted JSON Schema against an independent Python
`jsonschema` engine, so the schema is checked by something other than
the code that produced it). Its `dev/flake.nix` deliberately lives
outside the repo root specifically so the crate's real packaging
(`cargo`) never gets confused with dev tooling — a smart distinction
worth learning from, not "fixing."

What was actually missing, confirmed by reading the live repo rather
than assumed: no conventional-commit enforcement (real history mixed
`fix:`/`refactor:` with plain `init`, `Initial commit`, `major:
reshape`), no branch protection at all, no changelog automation tied
to CI, no backlog/brainstorm docs convention.

## Decisions made

- **`git-cliff` + `release-plz` on top of the existing CI**, not a
  from-scratch release pipeline — same proven `cliff.toml` template as
  `sync-dot-mesh/core`, adapted to this project's real commit-group
  shape. Replaces the old manual tag-triggered `publish-crate.yml`
  with an automated `release-plz.yml` that maintains a standing
  release PR (changelog + version bump) on every push to `main`.
  Nothing else in the org depends on `argenv` yet, so nothing breaks
  by making this change now rather than later.
- **`release-plz.toml`**: workspace-level default publish behavior,
  not a per-crate override — `argenv` (the library) publishes to
  crates.io by default, matching the "plug and play" goal; `argenv-cli`
  already opts out via its own `Cargo.toml` `publish = false`, so no
  redundant override needed in `release-plz.toml` itself.
- **PR-title scopes specific to this project's real shape** — `model`,
  `cli`, `contract`, `lint`, `api`, `ci`, `deps`, `release` — not
  copy-pasted from `sync-dot-mesh/core`'s scopes, which describe a
  different project's modules entirely.
- **This backlog folder**, same convention as `sync-dot-mesh` — dated
  entries, including corrected mistakes, not just clean wins.
- **`CARGO_REGISTRY_TOKEN` not added yet** — flagged explicitly rather
  than added silently, since it's what actually triggers a real
  crates.io publish once a release PR merges. Same for GitHub Pages
  for `publish-api.yml` — already present from before this pass,
  left as-is.

## Why this polish precedes `atpret` depending on it

Explicit sequencing, not an oversight: `argenv` gets fully polished and
published to crates.io first, so `atpret` can pull it in as an
ordinary external dependency (`cargo add argenv`) like any other
crate, rather than a path dependency to a sibling repo. Real usage
inside `atpret` afterward is expected to surface friction in
`argenv`'s design that a pure unit-test suite wouldn't — that feedback
loop is deliberate, not a symptom of shipping too early.
