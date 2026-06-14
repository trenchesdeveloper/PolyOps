/**
 * ESM-surface tests: the .mjs entry exposes real named exports + a default
 * export, and they behave identically. (Package-level `import 'polyops'`
 * resolution via the exports map is exercised by the tarball-install check
 * in CI / release verification.)
 *
 * Run with: `node --test __test__`
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';

import polyops, {
	union,
	intersection,
	diff,
	xor,
	pack,
	unpack,
	unionFlat,
} from '../polyops.mjs';

const A = [[[0, 0], [2, 0], [2, 2], [0, 2], [0, 0]]];
const B = [[[1, 1], [3, 1], [3, 3], [1, 3], [1, 1]]];

test('named + default ESM exports are all present', () => {
	for (const [n, f] of Object.entries({ union, intersection, diff, xor, pack, unpack, unionFlat })) {
		assert.equal(typeof f, 'function', `${n} should be a function`);
	}
	assert.equal(typeof polyops.union, 'function', 'default export carries the API');
});

test('named import runs and matches the default export', () => {
	const viaNamed = union(A, B);
	const viaDefault = polyops.union(A, B);
	assert.ok(Array.isArray(viaNamed) && viaNamed.length === 1);
	assert.deepEqual(viaNamed, viaDefault);
});
