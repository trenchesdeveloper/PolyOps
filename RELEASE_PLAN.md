# Release Plan — `polyops` 0.0.2 (multi-platform npm + crates.io)

> **Status:** Proposed. Companion to [`PLAN.md`](PLAN.md) §10 (Distribution)
> and §13 Milestone 8. Generalizes to every future `vX.Y.Z` tag.
> **Last updated:** 2026-06-11.

---

## 1. Goal

Build a **tag-triggered** release workflow that publishes `polyops@0.0.2`
to **crates.io** and **npm**, where the npm package uses standard
`napi-rs` **optional per-platform binary packages** so installs resolve a
prebuilt native addon on every supported platform.

The public API is unchanged — `intersection`, `union`, `diff`, `xor`
(npm) / `intersection`, `union`, `difference`, `xor` (Rust).

`0.0.1` shipped a single-platform binary (manual publish, no tag).
`0.0.2` is the first release with the full prebuilt matrix and an
automated, reproducible pipeline.

---

## 2. Prerequisites & decisions (resolve before tagging)

| # | Item | Detail |
|---|------|--------|
| P1 | **npm scope for platform packages** | napi derives platform package names from `napi.package.name`. Today `package.json` `name` is **unscoped** (`polyops`), so platform packages default to `polyops-darwin-arm64`, `polyops-linux-x64-gnu`, … (unscoped). `PLAN.md` §10 envisioned scoped `@polyops/polyops-*`, which requires an npm **org named `polyops`** that the `NPM_TOKEN` can publish to. **Decision needed:** unscoped (zero setup) vs scoped (create the org first). This plan assumes **unscoped** unless the org exists. |
| P2 | **Secrets present** | `CRATES_IO_TOKEN`, `NPM_TOKEN` configured as repo secrets (see §6). |
| P3 | **crates.io name owned** | Already published `polyops@0.0.1` ✅ — the owner can publish `0.0.2`. |
| P4 | **npm name owned** | Already published `polyops@0.0.1` ✅. |

---

## 3. Repository changes (one PR, merged before tagging)

### 3.1 Version bump `0.0.1 → 0.0.2`

Exact locations — **all must change together** or the build breaks:

- [`Cargo.toml`](Cargo.toml) → `[workspace.package] version = "0.0.2"`.
- [`crates/polyops-napi/Cargo.toml`](crates/polyops-napi/Cargo.toml) →
  the path-dependency line `polyops = { path = "../polyops", version = "0.0.1" }`
  **must become `version = "0.0.2"`**. ⚠️ A `0.0.x` version requirement is
  exact under Cargo's caret rules (`^0.0.1` matches **only** `0.0.1`), so
  leaving this at `0.0.1` makes the workspace fail to resolve once
  `polyops` is `0.0.2`. This is the easiest bump to miss.
- [`crates/polyops-napi/package.json`](crates/polyops-napi/package.json) →
  `"version": "0.0.2"`.
- `crates/polyops-napi/package-lock.json` → the two `version` fields
  (run `npm install` to sync, don't hand-edit).
- `Cargo.lock` → run `cargo update -p polyops --precise 0.0.2` (or any
  `cargo build`) to sync.

> The `napi version` npm script can propagate the version into the
> generated `npm/*/package.json` files once they exist (§3.3).

### 3.2 `.gitignore` — track npm-dir metadata, keep binaries ignored

Current [`.gitignore`](.gitignore) ignores both the binaries **and** the
whole npm dir:

```
*.node
crates/polyops-napi/npm/
```

Change to: **keep `*.node` ignored, stop ignoring `npm/`** so the
per-platform `package.json` metadata is committed while the generated
`.node` binaries stay out of git:

```
*.node
*.wasm
# (remove the `crates/polyops-napi/npm/` line)
# binaries inside npm/*/ are still covered by the *.node rule above
```

### 3.3 Generate platform package directories

From `crates/polyops-napi/`:

```bash
npx napi create-npm-dir -t .          # napi-rs 2.x
```

This reads `napi.triples` (already set to the 7 targets below) and writes
`npm/<platform>/package.json` for each. **Commit those `package.json`
files** (the `.node` files they will hold remain gitignored).

### 3.4 Main `package.json` adjustments

- **Remove `*.node` from `files`.** For the multi-platform release the
  binary lives in the platform packages, not the main package:

  ```jsonc
  "files": ["index.d.ts", "index.js"]   // was: [..., "*.node"]
  ```

- **Make publishing workflow-controlled, not hook-controlled.** Today
  `prepublishOnly` runs `napi prepublish -t npm` (no `--skip-gh-release`),
  which would (a) re-run during the workflow's `npm publish` and (b) try
  to create a GitHub release needing `GITHUB_TOKEN`. The release workflow
  publishes the main package with **`npm publish --ignore-scripts`**, so
  the hook is bypassed there. For local-publish safety also update the
  hook:

  ```jsonc
  "prepublishOnly": "napi prepublish -t npm --skip-gh-release"
  ```

- `napi.triples` already lists all 7 targets — **no change needed**.

---

## 4. Release workflow — `.github/workflows/release.yml`

**Trigger:** tags matching `v[0-9]+.[0-9]+.[0-9]+` only (e.g. `v0.0.2`).
Never on branch push / PR.

```yaml
on:
  push:
    tags: ['v[0-9]+.[0-9]+.[0-9]+']
```

### Job DAG

```
validate ─┬─> publish-crate
          └─> build-npm-artifacts (matrix) ─> publish-npm ─> smoke-published
```

### 4.1 `validate`
- Assert the tag version (`${GITHUB_REF_NAME#v}`) equals the
  `[workspace.package] version` in `Cargo.toml` **and**
  `crates/polyops-napi/package.json` `version`. Fail fast on mismatch.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- In `crates/polyops-napi`: `npm ci && npm run build && npm test && npm pack --dry-run`

### 4.2 `publish-crate` (needs: validate)
- `cargo publish -p polyops --token "$CRATES_IO_TOKEN"`
  (`polyops-napi` has `publish = false` — never goes to crates.io).

### 4.3 `build-npm-artifacts` (needs: validate) — matrix
Targets (match `napi.triples`):

| Target | Runner | Notes |
|--------|--------|-------|
| `x86_64-apple-darwin` | `macos-latest` | native |
| `aarch64-apple-darwin` | `macos-latest` | native (Apple Silicon runner) |
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | native |
| `aarch64-unknown-linux-gnu` | `ubuntu-latest` | cross — Zig or `aarch64` linker |
| `x86_64-unknown-linux-musl` | `ubuntu-latest` | cross — Zig / musl toolchain |
| `aarch64-unknown-linux-musl` | `ubuntu-latest` | cross — Zig / musl toolchain |
| `x86_64-pc-windows-msvc` | `windows-latest` | native |

- `rustup target add <target>` as needed.
- `npx napi build --platform --release --target <target>` (add `--zig`
  for the Linux cross targets, or use the napi-rs docker images — pick one
  cross strategy and keep it consistent).
- Upload `polyops.*.node` as a per-target GitHub Actions artifact.

### 4.4 `publish-npm` (needs: all matrix builds)
- Download all artifacts into `crates/polyops-napi/artifacts/`.
- `napi artifacts -d artifacts --dist npm` — moves each `.node` into its
  `npm/<platform>/` dir.
- Write `crates/polyops-napi/.npmrc` with `//registry.npmjs.org/:_authToken=${NPM_TOKEN}`.
- `napi prepublish -t npm --skip-gh-release` — publishes each platform
  package and injects `optionalDependencies` into the main `package.json`.
- `npm publish --access public --ignore-scripts` — publishes the main
  package (`--ignore-scripts` so `prepublishOnly`/`prepack` don't re-run).

### 4.5 `smoke-published` (needs: publish-npm)
- Matrix over `ubuntu-latest`, `macos-latest`, `windows-latest`.
- Fresh `npm install polyops@0.0.2` in an empty dir (small retry loop —
  npm registry propagation can lag a minute or two).
- `node -e "const p=require('polyops'); const k=Object.keys(p); if(!['intersection','union','diff','xor'].every(f=>k.includes(f))) process.exit(1)"`
- Optionally run one real op (e.g. intersection of two unit squares) and
  assert a non-null result, to prove the native addon actually loaded.

---

## 5. Required secrets

| Secret | Used by | Scope |
|--------|---------|-------|
| `CRATES_IO_TOKEN` | `publish-crate` | crates.io publish token |
| `NPM_TOKEN` | `publish-npm` | npm **automation** token with publish rights (and access to the `@polyops` org if P1 chooses scoped packages) |
| `GITHUB_TOKEN` (default) | checkout, artifact up/download, optional release metadata | **never** used for npm/crates publishing |

---

## 6. Test plan

**Before tagging (locally / in the prep PR):**
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd crates/polyops-napi && npm ci && npm run build && npm test && npm pack --dry-run
```

**After tagging (observe the workflow):**
1. `polyops@0.0.2` appears on crates.io.
2. npm shows the **main package + all 7 platform packages**, and the main
   package's `optionalDependencies` lists all 7.
3. `smoke-published` is green on macOS, Linux, and Windows — fresh install
   resolves the right prebuilt and the four exports load.

---

## 7. Safety / rollback notes

- **Order matters:** crates.io and npm publishes are **irreversible**
  (npm unpublish is restricted; crates.io can only yank). The `validate`
  gate runs the full test + parity-adjacent checks before any publish.
- **Partial-failure recovery:** if `publish-npm` fails *after* some
  platform packages published, re-running the tag won't republish an
  existing version (npm/crates reject duplicates). Recovery is a patch
  bump (`0.0.3`) — never force-republish a version.
- **Dry-run first:** consider a `workflow_dispatch` input `dry_run` that
  runs everything except the two `publish`/`prepublish` steps, to exercise
  the full matrix on a throwaway run before the real tag.
- **GH release:** the `v0.0.1` tag exists; its GitHub Release is still
  pending manual approval. For `0.0.2`, decide whether `napi prepublish`
  creates the release (drop `--skip-gh-release`, needs `GITHUB_TOKEN`) or
  it's created in a separate step. This plan keeps `--skip-gh-release` and
  creates the release explicitly.

---

## 8. Open questions

- **Scoped vs unscoped platform packages** (P1) — blocks the exact
  package names and the `NPM_TOKEN` org scope. Recommend deciding now.
- **Cross-compile strategy** — Zig (`--zig`, simplest, one runner) vs
  napi-rs docker images (closer to upstream napi CI, heavier). Pick one.
- **musl coverage priority** — `PLAN.md` §10 flags `linux-*-musl` as
  critical for Alpine/`process-photo`. If cross-musl proves flaky, ship
  gnu-only in `0.0.2` and add musl in `0.0.3` rather than block the
  release. `log()` the gap if so.
