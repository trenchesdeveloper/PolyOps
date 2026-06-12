/**
 * Head-to-head performance: `martinez-polygon-clipping@0.8.1` vs the
 * `polyops` napi binding, on the SAME vendored fixtures the Rust
 * criterion suite uses (`crates/polyops/benches/fixtures/`). Mirrors
 * upstream `bench/martinez.bench.ts` (three `union` workloads). See
 * PERFORMANCE_PLAN.md §5.2.
 *
 * Prereq — build the napi binding first:
 *   cd ../crates/polyops-napi && npm ci && npm run build
 *
 * Run:  npm run bench
 *
 * The polyops-napi number (3) ÷ martinez (1) is the user-facing speedup;
 * compare it to the pure-Rust criterion numbers (2) to see the napi
 * marshalling overhead.
 */

import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { Bench } from 'tinybench';
import * as martinez from 'martinez-polygon-clipping';

const HERE = dirname(fileURLToPath(import.meta.url));
const FIXTURES = resolve(HERE, '..', 'crates', 'polyops', 'benches', 'fixtures');
const NAPI = resolve(HERE, '..', 'crates', 'polyops-napi', 'index.js');

if (!existsSync(NAPI)) {
	console.error('napi binding not built. Run:');
	console.error('  cd ../crates/polyops-napi && npm ci && npm run build');
	process.exit(1);
}

const polyops = (await import(NAPI)) as { union: (a: unknown, b: unknown) => unknown };
const M = martinez as any;

const load = (name: string): any => JSON.parse(readFileSync(resolve(FIXTURES, name), 'utf-8'));

const holeHole = load('hole_hole.geojson');
const asia = load('asia.geojson');
const asiaClip = load('asia_unionPoly.geojson');
const states = load('states_source.geojson');

interface Scenario {
	name: string;
	subject: unknown;
	clipping: unknown;
	/** per-task budget (ms) — large workloads get more so a few iterations land. */
	time: number;
}

const scenarios: Scenario[] = [
	{
		name: 'hole_hole',
		subject: holeHole.features[0].geometry.coordinates,
		clipping: holeHole.features[1].geometry.coordinates,
		time: 1000,
	},
	{
		// subject from a FeatureCollection; clip is a bare Feature's geometry.
		name: 'asia_union',
		subject: asia.features[0].geometry.coordinates,
		clipping: asiaClip.geometry.coordinates,
		time: 3000,
	},
	{
		name: 'states_clip',
		subject: states.features[0].geometry.coordinates,
		clipping: states.features[1].geometry.coordinates,
		time: 2000,
	},
];

/* Sanity first: both engines should agree on the output polygon count. */
console.log('Sanity (output polygon counts must match):');
for (const s of scenarios) {
	const m = M.union(s.subject, s.clipping);
	const p = polyops.union(s.subject, s.clipping) as unknown;
	const mc = Array.isArray(m) ? m.length : 0;
	const pc = Array.isArray(p) ? (p as unknown[]).length : 0;
	console.log(`  ${s.name.padEnd(12)} martinez=${mc} polyops=${pc} ${mc === pc ? '✓' : '⚠ DIFFER'}`);
}

console.log('\nHead-to-head (union; higher ops/s = faster):');
const rows: Record<string, string>[] = [];
for (const s of scenarios) {
	const bench = new Bench({ time: s.time });
	bench.add('martinez', () => {
		M.union(s.subject, s.clipping);
	});
	bench.add('polyops', () => {
		polyops.union(s.subject, s.clipping);
	});
	await bench.run();

	const hz = (name: string): number =>
		bench.tasks.find((t) => t.name === name)!.result!.throughput.mean;
	const mHz = hz('martinez');
	const pHz = hz('polyops');
	const fmt = (n: number) => (n < 100 ? n.toFixed(2) : Math.round(n).toLocaleString());

	rows.push({
		scenario: s.name,
		'martinez (ops/s)': fmt(mHz),
		'polyops-napi (ops/s)': fmt(pHz),
		speedup: `${(pHz / mHz).toFixed(2)}×`,
	});
}
console.table(rows);
console.log(
	'\nNote: compare polyops-napi ops/s against the pure-Rust criterion numbers' +
		'\n(`cargo bench -p polyops`) to gauge the napi marshalling overhead.',
);
