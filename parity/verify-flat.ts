/**
 * Verify the typed-array fast path (`polyops/flat`) matches the committed
 * parity goldens — i.e. produces the same output as
 * `martinez-polygon-clipping@0.8.1` across the full corpus. This is the
 * correctness gate for the flat API (the speed claims are in BENCHMARKS.md).
 *
 * Prereq — build the binding:
 *   cd ../crates/polyops-napi && npm ci && npm run build
 *
 * Run:  npm run verify-flat
 */

import { readFileSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const GOLDENS = resolve(HERE, '..', 'crates', 'polyops', 'tests', 'goldens');

const flatMod = await import('../crates/polyops-napi/flat.js');
const flat = (flatMod as any).default ?? flatMod;

/** golden directory name -> flat.js method (martinez naming). */
const OPS: Record<string, 'intersection' | 'union' | 'diff' | 'xor'> = {
	intersection: 'intersection',
	union: 'union',
	difference: 'diff',
	xor: 'xor',
};

const close = (a: any, b: any, eps = 1e-10): boolean => {
	if (a === null || b === null) return a === b;
	if (!Array.isArray(a) || !Array.isArray(b)) return Math.abs(a - b) <= eps;
	if (a.length !== b.length) return false;
	return a.every((x, i) => close(x, b[i], eps));
};

let pass = 0;
const failures: string[] = [];

for (const [dir, op] of Object.entries(OPS)) {
	const opDir = resolve(GOLDENS, dir);
	for (const file of readdirSync(opDir)) {
		if (!file.endsWith('.json')) continue;
		const { subject, clipping, expected } = JSON.parse(readFileSync(resolve(opDir, file), 'utf-8'));
		const got = flat[op](subject, clipping);
		if (close(got, expected)) pass++;
		else failures.push(`${dir}/${file}`);
	}
}

console.log(`polyops/flat vs goldens: ${pass} passed, ${failures.length} failed`);
if (failures.length) {
	console.error('FAILURES:\n  ' + failures.join('\n  '));
	process.exit(1);
}
