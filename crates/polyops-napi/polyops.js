'use strict';

/**
 * polyops — default entry point.
 *
 * `intersection`/`union`/`diff`/`xor` take and return GeoJSON-shaped
 * coordinate arrays (drop-in compatible with `martinez-polygon-clipping`),
 * but route through the typed-array fast path: coordinates are packed into
 * Float64Array/Uint32Array buffers so only flat data crosses the N-API
 * boundary. That boundary conversion was the dominant cost of the naive
 * nested-array binding (>half the time on large inputs, and enough to lose
 * to in-process JS on tiny ones); routing through buffers makes polyops
 * faster than martinez at every size. See BENCHMARKS.md.
 *
 *   const polyops = require('polyops');
 *   polyops.union(subjectCoords, clippingCoords);   // GeoJSON in/out, fast
 *
 * `pack`/`unpack` and the raw `*Flat` ops are also exported for pipelines
 * that keep geometry in buffer form across many calls (skips repacking).
 * The unwrapped nested-array native functions remain available via the
 * generated loader at `polyops/index.js` if ever needed.
 */

const native = require('./index.js');

/** GeoJSON Polygon (number[][][]) or MultiPolygon (number[][][][]) -> flat buffers. */
function pack(geom) {
	const isMulti = Array.isArray(geom?.[0]?.[0]?.[0]);
	const polys = isMulti ? geom : [geom];

	let nCoords = 0;
	let nRings = 0;
	for (const poly of polys) {
		nRings += poly.length;
		for (const ring of poly) nCoords += ring.length * 2;
	}

	const coords = new Float64Array(nCoords);
	const ringLengths = new Uint32Array(nRings);
	const polyRingCounts = new Uint32Array(polys.length);

	let ci = 0;
	let ri = 0;
	let pi = 0;
	for (const poly of polys) {
		polyRingCounts[pi++] = poly.length;
		for (const ring of poly) {
			ringLengths[ri++] = ring.length;
			for (let k = 0; k < ring.length; k++) {
				coords[ci++] = ring[k][0];
				coords[ci++] = ring[k][1];
			}
		}
	}
	return { coords, ringLengths, polyRingCounts };
}

/** Flat buffers -> GeoJSON MultiPolygon (number[][][][]), or null. */
function unpack(fp) {
	if (!fp) return null;
	const { coords, ringLengths, polyRingCounts } = fp;
	const out = [];
	let ci = 0;
	let ri = 0;
	for (let p = 0; p < polyRingCounts.length; p++) {
		const nrings = polyRingCounts[p];
		const poly = [];
		for (let r = 0; r < nrings; r++) {
			const n = ringLengths[ri++];
			const ring = new Array(n);
			for (let k = 0; k < n; k++) {
				ring[k] = [coords[ci++], coords[ci++]];
			}
			poly.push(ring);
		}
		out.push(poly);
	}
	return out;
}

const wrap = (flatFn) => (subject, clipping) => unpack(flatFn(pack(subject), pack(clipping)));

module.exports = {
	// GeoJSON in/out — drop-in compatible with martinez, fast by default.
	intersection: wrap(native.intersectionFlat),
	union: wrap(native.unionFlat),
	diff: wrap(native.diffFlat),
	xor: wrap(native.xorFlat),
	// Helpers + raw buffer ops, for consumers that keep geometry flat.
	pack,
	unpack,
	intersectionFlat: native.intersectionFlat,
	unionFlat: native.unionFlat,
	diffFlat: native.diffFlat,
	xorFlat: native.xorFlat,
};
