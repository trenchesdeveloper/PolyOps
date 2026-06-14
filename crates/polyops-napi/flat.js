'use strict';

/**
 * polyops/flat — alias of the default entry.
 *
 * As of the default-fast-path consolidation, the main `polyops` export
 * already routes through the typed-array fast path, so this subpath is
 * just an alias kept for discoverability and back-compat. Prefer
 * `require('polyops')`.
 */

module.exports = require('./polyops.js');
