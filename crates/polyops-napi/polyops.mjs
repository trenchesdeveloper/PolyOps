// ESM entry point. Re-exports the CommonJS implementation (polyops.js) as
// real, statically-analyzable named exports, so `import { union } from
// 'polyops'` works — and matches polyops.d.ts. `import polyops from
// 'polyops'` (default) and `import * as polyops` work too.

import impl from './polyops.js';

export const {
	intersection,
	union,
	diff,
	xor,
	pack,
	unpack,
	intersectionFlat,
	unionFlat,
	diffFlat,
	xorFlat,
} = impl;

export default impl;
