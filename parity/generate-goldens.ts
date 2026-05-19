/**
 * Generate parity goldens for PolyOps.
 *
 * Reads the upstream w8r/martinez repository (path via MARTINEZ_REPO env
 * var, default: ../../martinez relative to the repo root), runs
 * `martinez-polygon-clipping@0.8.1` against:
 *
 *   1. test/genericTestCases/*.geojson    — N-feature FeatureCollections
 *      where features[0]=subject, features[1]=clipping, features[2..]
 *      describe expected results per-operation via properties.operation.
 *
 *   2. test/featureTypes/                  — 4 subjects × 1 clipping ×
 *      4 ops = 16 cases.
 *
 * For each case we write a golden JSON file under
 *   crates/polyops/tests/goldens/{operation}/{name}.json
 *
 * Shape (matches `tests/parity.rs::Golden`):
 *   { subject: <coords>, clipping: <coords>, expected: <multipolygon|null> }
 */

import { mkdirSync, readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs';
import { join, dirname, basename, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import * as martinez from 'martinez-polygon-clipping';

/**********************************************************************
 * Paths.
 **********************************************************************/
const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, '..');
const MARTINEZ_REPO = resolve(
	process.env.MARTINEZ_REPO ?? resolve(REPO_ROOT, '..', 'martinez'),
);
const GOLDENS_ROOT = resolve(REPO_ROOT, 'crates/polyops/tests/goldens');

/**********************************************************************
 * Operation table — maps upstream operation strings to our directory
 * names and the corresponding martinez function. Upstream uses "diff"
 * and "diff_ba"; we normalize to "difference".
 **********************************************************************/
type OpName = 'intersection' | 'union' | 'difference' | 'xor';

interface OpEntry {
	dir: OpName;
	run: (a: unknown, b: unknown) => unknown;
}

const OPS: Record<string, OpEntry> = {
	intersection: { dir: 'intersection', run: (a, b) => (martinez as any).intersection(a, b) },
	union:        { dir: 'union',        run: (a, b) => (martinez as any).union(a, b) },
	diff:         { dir: 'difference',   run: (a, b) => (martinez as any).diff(a, b) },
	diff_ba:      { dir: 'difference',   run: (a, b) => (martinez as any).diff(b, a) },
	xor:          { dir: 'xor',          run: (a, b) => (martinez as any).xor(a, b) },
};

/**********************************************************************
 * Helpers.
 **********************************************************************/
function ensureDir(path: string): void {
	mkdirSync(path, { recursive: true });
}

function readJson<T = unknown>(path: string): T {
	return JSON.parse(readFileSync(path, 'utf-8')) as T;
}

function geometryCoords(geom: { type: string; coordinates: unknown }): unknown {
	/** Both Polygon and MultiPolygon are accepted by martinez as-is. */
	return geom.coordinates;
}

function writeGolden(op: OpName, name: string, subject: unknown, clipping: unknown, expected: unknown): void {
	const dir = join(GOLDENS_ROOT, op);
	ensureDir(dir);
	const file = join(dir, `${name}.json`);
	writeFileSync(file, JSON.stringify({ subject, clipping, expected }) + '\n');
}

/**********************************************************************
 * Generators.
 **********************************************************************/
function generateFromGenericTestCases(): number {
	const dir = join(MARTINEZ_REPO, 'test/genericTestCases');
	if (!existsSync(dir)) {
		console.warn(`skip: ${dir} not found`);
		return 0;
	}
	let count = 0;
	for (const file of readdirSync(dir)) {
		if (!file.endsWith('.geojson') || file.startsWith('_')) continue;
		const data = readJson<{ features: any[] }>(join(dir, file));
		if (!data.features || data.features.length < 2) continue;

		const subject = geometryCoords(data.features[0].geometry);
		const clipping = geometryCoords(data.features[1].geometry);
		const stem = basename(file, '.geojson');

		/**
		 * Some upstream files only carry the subject+clipping; in that
		 * case we still emit goldens for all four ops by running them
		 * directly. Suffix the filename with the op letter if the file
		 * declares its own expected ops to avoid name clashes.
		 */
		const declaredOps = data.features.slice(2)
			.map((f: any) => f.properties?.operation as string | undefined)
			.filter((s): s is string => typeof s === 'string');

		if (declaredOps.length === 0) {
			for (const opKey of ['intersection', 'union', 'diff', 'xor'] as const) {
				const op = OPS[opKey];
				const result = op.run(subject, clipping);
				writeGolden(op.dir, stem, subject, clipping, result ?? null);
				count++;
			}
			continue;
		}

		const usedNames = new Set<string>();
		for (const opName of declaredOps) {
			const op = OPS[opName];
			if (!op) {
				console.warn(`unknown op ${opName} in ${file}, skipping`);
				continue;
			}
			/** Disambiguate diff vs diff_ba within the same fixture. */
			let name = stem;
			if (opName === 'diff_ba') name = `${stem}__ba`;
			while (usedNames.has(`${op.dir}/${name}`)) name = `${name}_`;
			usedNames.add(`${op.dir}/${name}`);

			const result = op.run(subject, clipping);
			writeGolden(op.dir, name, subject, clipping, result ?? null);
			count++;
		}
	}
	return count;
}

function generateFromFeatureTypes(): number {
	const ftDir = join(MARTINEZ_REPO, 'test/featureTypes');
	if (!existsSync(ftDir)) {
		console.warn(`skip: ${ftDir} not found`);
		return 0;
	}
	const clipping = geometryCoords(readJson<any>(join(ftDir, 'clippingPoly.geojson')).geometry);
	const subjects = ['poly', 'polyWithHole', 'multiPoly', 'multiPolyWithHole'] as const;

	let count = 0;
	for (const subjectName of subjects) {
		const subject = geometryCoords(readJson<any>(join(ftDir, `${subjectName}.geojson`)).geometry);
		const caseName = `${subjectName}ToClipping`;

		for (const opKey of ['intersection', 'union', 'diff', 'xor'] as const) {
			const op = OPS[opKey];
			const result = op.run(subject, clipping);
			writeGolden(op.dir, `featureTypes_${caseName}`, subject, clipping, result ?? null);
			count++;
		}
	}
	return count;
}

/**********************************************************************
 * Entry.
 **********************************************************************/
console.log(`Reading upstream from: ${MARTINEZ_REPO}`);
console.log(`Writing goldens to:    ${GOLDENS_ROOT}`);

if (!existsSync(MARTINEZ_REPO)) {
	console.error(`MARTINEZ_REPO does not exist: ${MARTINEZ_REPO}`);
	console.error(`Set MARTINEZ_REPO env var or clone w8r/martinez to ../martinez relative to this repo.`);
	process.exit(1);
}

ensureDir(GOLDENS_ROOT);

const generic = generateFromGenericTestCases();
const featureTypes = generateFromFeatureTypes();

console.log(`Goldens generated: ${generic} from genericTestCases, ${featureTypes} from featureTypes. Total: ${generic + featureTypes}.`);
