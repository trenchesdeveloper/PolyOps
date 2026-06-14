'use strict';

/**
 * polyops/flat — the typed-array fast path.
 *
 * Same GeoJSON-shaped API as the default `polyops` export, but coordinates
 * are packed into Float64Array/Uint32Array buffers so only flat data
 * crosses the N-API boundary. The nested-array boundary conversion is
 * >half the cost on large inputs and dominates on small ones; routing
 * through buffers makes polyops faster than martinez-polygon-clipping at
 * every size (see BENCHMARKS.md).
 *
 *   const polyops = require('polyops/flat');
 *   polyops.union(subjectCoords, clippingCoords);   // GeoJSON in/out, fast
 *
 * For pipelines that keep geometry in buffer form across many calls, use
 * the raw `*Flat` ops + `pack`/`unpack` to avoid repacking each call.
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
	pack,
	unpack,
	// GeoJSON-shaped — drop-in compatible with the default export, but faster.
	intersection: wrap(native.intersectionFlat),
	union: wrap(native.unionFlat),
	diff: wrap(native.diffFlat),
	xor: wrap(native.xorFlat),
	// Raw buffer ops, for consumers that keep geometry flat across calls.
	intersectionFlat: native.intersectionFlat,
	unionFlat: native.unionFlat,
	diffFlat: native.diffFlat,
	xorFlat: native.xorFlat,
};
