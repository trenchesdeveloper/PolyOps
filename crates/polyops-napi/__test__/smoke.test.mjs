/**
 * Smoke test — confirms the napi binding loads and exposes the four
 * operations once `napi build` has produced the native module.
 *
 * Run with: `node --test __test__`
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';

test('polyops exports the four operations', async () => {
	const mod = await import('../index.js');
	for (const name of ['intersection', 'union', 'diff', 'xor']) {
		assert.equal(typeof mod[name], 'function', `${name} should be a function`);
	}
});
