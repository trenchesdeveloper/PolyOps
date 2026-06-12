/**
 * Refresh the vendored benchmark fixtures from a local `w8r/martinez`
 * clone into `crates/polyops/benches/fixtures/`. Same `MARTINEZ_REPO`
 * convention as `generate-goldens.ts`. See PERFORMANCE_PLAN.md §3.
 *
 * Run:  MARTINEZ_REPO=../../martinez npm run copy-bench-fixtures
 */

import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, '..');
const MARTINEZ_REPO = resolve(process.env.MARTINEZ_REPO ?? resolve(REPO_ROOT, '..', 'martinez'));
const SRC = resolve(MARTINEZ_REPO, 'test', 'fixtures');
const DEST = resolve(REPO_ROOT, 'crates/polyops/benches/fixtures');

const FILES = ['hole_hole.geojson', 'asia.geojson', 'asia_unionPoly.geojson', 'states_source.geojson'];

if (!existsSync(SRC)) {
	console.error(`upstream fixtures not found: ${SRC}`);
	console.error('Set MARTINEZ_REPO or clone w8r/martinez to ../martinez relative to the repo.');
	process.exit(1);
}

mkdirSync(DEST, { recursive: true });
for (const f of FILES) {
	const from = resolve(SRC, f);
	if (!existsSync(from)) {
		console.warn(`skip (missing upstream): ${f}`);
		continue;
	}
	copyFileSync(from, resolve(DEST, f));
	console.log(`copied ${f}`);
}
console.log(`\nDone. Fixtures refreshed into ${DEST}`);
