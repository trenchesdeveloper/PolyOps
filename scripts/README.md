# scripts/

## `spin-polyops-apps.mjs` — install + scale/compat tester

Creates N isolated throwaway apps, installs `polyops` into **each app's own
`node_modules`**, runs a fresh runtime process per app that `require`s it and
runs the four ops, then cleans up. Useful for validating that polyops
installs + loads its native prebuilt reliably across many independent
projects and runtimes.

```bash
# start small!
node scripts/spin-polyops-apps.mjs --count 10 --runtimes node,bun --concurrency 2

# larger scale, sharing one cache per runtime (much faster, fewer downloads)
node scripts/spin-polyops-apps.mjs --count 1000 --runtimes node,bun --shared-cache
```

Options (`--help` for all): `--count` (default **20000**), `--concurrency`
(4), `--package` (`polyops@latest`), `--runtimes` (`node`,`bun`), `--root`,
`--keep`, `--shared-cache`. `node` installs via `npm`, `bun` via `bun add`;
each app uses its own cache unless `--shared-cache`.

> ⚠️ **The default `--count` is 20,000.** Without `--shared-cache`, each app
> does a *real, cold* `polyops` download — high counts mean lots of disk,
> time, and **real npm-registry traffic** (which also nudges the package's
> download stats). Use a small `--count` to smoke-test, `--shared-cache` for
> scale, and only go large deliberately.

### What we've measured (Apple M1 Pro)

- **10,000** runs (shared install, process-scale): **0 failures**; the four
  ops are within noise (~0.33 ms p50); per-process **cold import ~10 ms**
  dominates.
- **1,000** distinct **cold, no-cache** installs: **0 failures**; every app
  installed, resolved the `polyops-darwin-arm64` prebuilt, and ran. Cold
  install ≈ **bun ~5.3 s** vs **pnpm ~6.3 s** (pnpm also validated by hand;
  this script covers `node`/`bun`).
- polyops loads + runs under **Bun, npm, and pnpm** — notably,
  `martinez-polygon-clipping@0.8.1` fails to load under Bun (a `tinyqueue`
  CJS-interop error) while polyops works.

(pnpm's strict `node_modules` doesn't hoist the optional platform package to
the project root, but polyops resolves it internally — load+run still
succeeds.)
