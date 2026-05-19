/**
 * Optional sanity check: load the freshly built napi binding from
 * crates/polyops-napi/index.js and confirm it produces the same output
 * as the committed goldens. Useful once the algorithm is wired up; for
 * now it just smoke-tests that the module loads and exposes the four
 * operations.
 *
 * Run: `npm run verify` (requires `cd ../crates/polyops-napi && npm install && npm run build:debug` first)
 */

import { resolve, dirname } from 'node:path';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const NAPI_INDEX = resolve(HERE, '..', 'crates', 'polyops-napi', 'index.js');

if (!existsSync(NAPI_INDEX)) {
	console.error(`napi binding not built. Run:`);
	console.error(`  cd ../crates/polyops-napi && npm install && npm run build:debug`);
	process.exit(1);
}

const mod = await import(NAPI_INDEX);
for (const name of ['intersection', 'union', 'diff', 'xor'] as const) {
	if (typeof (mod as any)[name] !== 'function') {
		console.error(`Missing export: ${name}`);
		process.exit(1);
	}
}
console.log('napi binding loaded; all four operations are exported.');
