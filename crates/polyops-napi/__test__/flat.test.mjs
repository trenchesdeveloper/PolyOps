/**
 * Tests for the typed-array fast path (flat.js): it exposes the expected
 * surface and produces the same results as the default (nested) export.
 *
 * Run with: `node --test __test__`
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';

const loadFlat = async () => {
	const m = await import('../flat.js');
	return m.default ?? m;
};

const close = (a, b, eps = 1e-9) => {
	if (a === null || b === null) return a === b;
	if (!Array.isArray(a) || !Array.isArray(b)) return Math.abs(a - b) <= eps;
	if (a.length !== b.length) return false;
	return a.every((x, i) => close(x, b[i], eps));
};

test('polyops/flat exposes wrappers, raw flat ops, and pack/unpack', async () => {
	const flat = await loadFlat();
	for (const n of [
		'intersection', 'union', 'diff', 'xor',
		'intersectionFlat', 'unionFlat', 'diffFlat', 'xorFlat',
		'pack', 'unpack',
	]) {
		assert.equal(typeof flat[n], 'function', `${n} should be a function`);
	}
});

test('flat GeoJSON wrappers match the default export', async () => {
	const def = await import('../index.js');
	const flat = await loadFlat();

	// Polygon inputs (exercises the 1-polygon multipolygon encoding).
	const a = [[[0, 0], [2, 0], [2, 2], [0, 2], [0, 0]]];
	const b = [[[1, 1], [3, 1], [3, 3], [1, 3], [1, 1]]];

	for (const op of ['intersection', 'union', 'diff', 'xor']) {
		const expected = def[op](a, b);
		const got = flat[op](a, b);
		assert.ok(close(got, expected), `${op}: flat result should match default export`);
	}
});

test('pack/unpack round-trips a multipolygon', async () => {
	const flat = await loadFlat();
	const mp = [
		[[[0, 0], [4, 0], [4, 4], [0, 4], [0, 0]], [[1, 1], [2, 1], [2, 2], [1, 2], [1, 1]]],
		[[[5, 5], [6, 5], [6, 6], [5, 6], [5, 5]]],
	];
	assert.ok(close(flat.unpack(flat.pack(mp)), mp), 'pack then unpack should be identity');
});
