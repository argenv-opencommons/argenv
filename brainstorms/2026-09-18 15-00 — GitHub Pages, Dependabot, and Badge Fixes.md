# 2026-09-18 15:00 — GitHub Pages, Dependabot, and Badge Fixes

## GitHub Pages: was never enabled

The `Publish contract API` workflow had been failing on every push to
`main` since 2026-07-23, predating any work this session — the
workflow's own top comment already said why: `NEXT STEP: enable Pages
(Settings -> Pages -> Source: GitHub Actions)`. Confirmed directly
rather than assumed: `GET /repos/.../pages` returned `404 Not Found` —
no Pages site existed at all. Fixed via the API
(`POST /repos/.../pages` with `build_type: "workflow"`), then the
workflow was manually dispatched and watched to actual completion
rather than trusted from the settings change alone — it deployed
successfully, and the site itself was independently verified serving
(`HTTP 200` on `https://argenv-opencommons.github.io/argenv/`).

## Badges: crates.io and docs.rs needed no code fix

Checked directly rather than assumed broken: both badges in README
already pointed at the right URLs. They rendered as broken/red purely
because the crate wasn't published yet — `shields.io` now reports
`crates.io: v0.1.0` and `docs: passing` for both, confirmed by
fetching the badge SVGs directly. Nothing to change there; the
crates.io publish earlier this session already fixed them.

Added one real badge: a "contract API — live" badge linking to the
now-working Pages deployment, added identically to both README copies
(root `README.md` and `crates/argenv/README.md`, kept in sync by hand
— they are separate files, not a symlink, confirmed by diffing them).

## Dependabot

Added `.github/dependabot.yml`, same convention as `sync-mesh-core`
(read directly from that repo rather than reconstructed from memory):
weekly `cargo` updates with routine minor/patch bumps grouped into one
PR, security updates always kept separate; weekly `github-actions`
updates so the CI/release workflow's own actions
(`checkout`, `dtolnay/rust-toolchain`, `release-plz-action`, the
PR-title checker) stay current too.

## Outcome

Merged as [argenv PR #3](https://github.com/argenv-opencommons/argenv/pull/3)
— all four required checks green before merge. Confirmed on `main`
afterward, not just on the PR: both `CI` and `Release` completed
successfully post-merge.
