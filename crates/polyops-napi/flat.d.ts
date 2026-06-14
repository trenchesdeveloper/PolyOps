/* The typed-array fast path. See flat.js. */

export type Position = [number, number];
export type Ring = Position[];
export type Polygon = Ring[];
export type MultiPolygon = Polygon[];

/** Buffer-encoded MultiPolygon (see flat.js for the layout). */
export interface FlatPolys {
	coords: Float64Array;
	ringLengths: Uint32Array;
	polyRingCounts: Uint32Array;
}

/** Pack GeoJSON-shaped coordinates into flat buffers. */
export function pack(geom: Polygon | MultiPolygon): FlatPolys;
/** Unpack flat buffers back into a GeoJSON MultiPolygon (or null). */
export function unpack(fp: FlatPolys | null): MultiPolygon | null;

/* GeoJSON in/out — drop-in compatible with the default export, but faster. */
export function intersection(subject: Polygon | MultiPolygon, clipping: Polygon | MultiPolygon): MultiPolygon | null;
export function union(subject: Polygon | MultiPolygon, clipping: Polygon | MultiPolygon): MultiPolygon | null;
export function diff(subject: Polygon | MultiPolygon, clipping: Polygon | MultiPolygon): MultiPolygon | null;
export function xor(subject: Polygon | MultiPolygon, clipping: Polygon | MultiPolygon): MultiPolygon | null;

/* Raw buffer ops — for pipelines that keep geometry flat across calls. */
export function intersectionFlat(subject: FlatPolys, clipping: FlatPolys): FlatPolys | null;
export function unionFlat(subject: FlatPolys, clipping: FlatPolys): FlatPolys | null;
export function diffFlat(subject: FlatPolys, clipping: FlatPolys): FlatPolys | null;
export function xorFlat(subject: FlatPolys, clipping: FlatPolys): FlatPolys | null;
