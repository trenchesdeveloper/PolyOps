#!/usr/bin/env node

import { mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawn } from "node:child_process";

const DEFAULT_COUNT = 20000;
const DEFAULT_CONCURRENCY = 4;
const DEFAULT_PACKAGE = "polyops@latest";
const DEFAULT_RUNTIMES = ["node"];
const ALLOWED_RUNTIMES = new Set(["node", "bun"]);

function parseArgs(argv) {
  const options = {
    count: DEFAULT_COUNT,
    concurrency: DEFAULT_CONCURRENCY,
    packageName: DEFAULT_PACKAGE,
    root: join(tmpdir(), `polyops-spin-${Date.now()}`),
    keep: false,
    sharedCache: false,
    runtimes: DEFAULT_RUNTIMES,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    const next = () => {
      i += 1;
      if (i >= argv.length) {
        throw new Error(`${arg} requires a value`);
      }
      return argv[i];
    };

    if (arg === "--count") {
      options.count = Number.parseInt(next(), 10);
    } else if (arg === "--concurrency") {
      options.concurrency = Number.parseInt(next(), 10);
    } else if (arg === "--package") {
      options.packageName = next();
    } else if (arg === "--runtimes") {
      options.runtimes = next()
        .split(",")
        .map((runtime) => runtime.trim())
        .filter(Boolean);
    } else if (arg === "--root") {
      options.root = resolve(next());
    } else if (arg === "--keep") {
      options.keep = true;
    } else if (arg === "--shared-cache") {
      options.sharedCache = true;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`Unknown option: ${arg}`);
    }
  }

  if (!Number.isInteger(options.count) || options.count < 1) {
    throw new Error("--count must be a positive integer");
  }
  if (!Number.isInteger(options.concurrency) || options.concurrency < 1) {
    throw new Error("--concurrency must be a positive integer");
  }
  if (options.runtimes.length === 0) {
    throw new Error("--runtimes must include at least one runtime");
  }
  for (const runtime of options.runtimes) {
    if (!ALLOWED_RUNTIMES.has(runtime)) {
      throw new Error(`unsupported runtime "${runtime}", expected node or bun`);
    }
  }

  return options;
}

function printHelp() {
  console.log(`Usage: node scripts/spin-polyops-apps.mjs [options]

Creates isolated throwaway apps, installs polyops in each app's own node_modules,
and runs a fresh runtime process per app to use the package.

Options:
  --count <n>          Number of apps to create. Default: ${DEFAULT_COUNT}
  --concurrency <n>    Number of apps to process in parallel. Default: ${DEFAULT_CONCURRENCY}
  --package <spec>     npm package spec to install. Default: ${DEFAULT_PACKAGE}
  --runtimes <list>    Comma-separated runtimes: node,bun. Default: node
  --root <dir>         Directory for generated apps. Default: OS temp dir
  --keep               Keep successful app directories. Failed apps are always kept.
  --shared-cache       Use one package-manager cache under <root> instead of one cache per app.
  -h, --help           Show this help.

Examples:
  node scripts/spin-polyops-apps.mjs --count 20000 --runtimes node,bun
  node scripts/spin-polyops-apps.mjs --count 10 --runtimes node,bun --concurrency 2
`);
}

function run(command, args, cwd, env) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd,
      env: { ...process.env, ...env },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolvePromise({ stdout, stderr });
      } else {
        const err = new Error(
          `${command} ${args.join(" ")} failed with exit code ${code}`,
        );
        err.stdout = stdout;
        err.stderr = stderr;
        reject(err);
      }
    });
  });
}

function runtimeForIndex(index, runtimes) {
  return runtimes[(index - 1) % runtimes.length];
}

function appName(index, runtime) {
  return `polyops-${runtime}-spin-${String(index).padStart(5, "0")}`;
}

function packageJson(index, runtime) {
  return `${JSON.stringify(
    {
      name: appName(index, runtime),
      version: "1.0.0",
      private: true,
      type: "commonjs",
    },
    null,
    2,
  )}
`;
}

function usageScript(index, runtime) {
  return `const polyops = require("polyops");

const expected = ["intersection", "union", "diff", "xor"];
for (const name of expected) {
  if (typeof polyops[name] !== "function") {
    throw new Error(\`missing export: \${name}\`);
  }
}

const subject = [[[0, 0], [2, 0], [2, 2], [0, 2], [0, 0]]];
const clipping = [[[1, 1], [3, 1], [3, 3], [1, 3], [1, 1]]];
const result = polyops.intersection(subject, clipping);

if (!Array.isArray(result)) {
  throw new Error("intersection did not return a MultiPolygon array");
}

console.log(JSON.stringify({ app: ${index}, runtime: "${runtime}", exports: expected, polygons: result.length }));
`;
}

function installCommand(runtime, packageName, cacheDir) {
  if (runtime === "node") {
    return {
      command: "npm",
      args: [
        "install",
        packageName,
        "--no-audit",
        "--no-fund",
        "--foreground-scripts",
        "--cache",
        cacheDir,
      ],
    };
  }

  return {
    command: "bun",
    args: [
      "add",
      packageName,
      "--no-summary",
      "--cache-dir",
      cacheDir,
    ],
  };
}

function runCommand(runtime) {
  if (runtime === "node") {
    return { command: "node", args: ["use-polyops.cjs"] };
  }
  return { command: "bun", args: ["use-polyops.cjs"] };
}

async function createAndRunApp(index, options) {
  const runtime = runtimeForIndex(index, options.runtimes);
  const dir = join(options.root, appName(index, runtime));
  const cacheDir = options.sharedCache
    ? join(options.root, `.${runtime}-cache`)
    : join(dir, `.${runtime}-cache`);

  await mkdir(dir, { recursive: true });
  await writeFile(join(dir, "package.json"), packageJson(index, runtime));
  await writeFile(join(dir, "use-polyops.cjs"), usageScript(index, runtime));

  const install = installCommand(runtime, options.packageName, cacheDir);
  await run(install.command, install.args, dir);

  const command = runCommand(runtime);
  const check = await run(command.command, command.args, dir);

  if (!options.keep) {
    await rm(dir, { recursive: true, force: true });
  }

  return check.stdout.trim();
}

async function runPool(options) {
  await mkdir(options.root, { recursive: true });

  let nextIndex = 1;
  let completed = 0;
  const runtimeCounts = Object.fromEntries(options.runtimes.map((runtime) => [runtime, 0]));
  const failures = [];

  async function worker(workerId) {
    while (true) {
      const index = nextIndex;
      nextIndex += 1;
      if (index > options.count) {
        return;
      }

      try {
        const output = await createAndRunApp(index, options);
        const { runtime } = JSON.parse(output);
        runtimeCounts[runtime] += 1;
        completed += 1;
        if (completed === 1 || completed % 25 === 0 || completed === options.count) {
          console.log(`[${completed}/${options.count}] worker=${workerId} ${output}`);
        }
      } catch (error) {
        failures.push({
          index,
          message: error.message,
          stderr: error.stderr,
          stdout: error.stdout,
        });
        console.error(`[failed app ${index}] ${error.message}`);
        if (error.stderr) {
          console.error(error.stderr);
        }
      }
    }
  }

  const workers = Array.from(
    { length: Math.min(options.concurrency, options.count) },
    (_, i) => worker(i + 1),
  );

  await Promise.all(workers);

  if (failures.length > 0) {
    console.error(`\n${failures.length} app(s) failed. Root kept at: ${options.root}`);
    process.exitCode = 1;
    return;
  }

  const runtimeSummary = Object.entries(runtimeCounts)
    .map(([runtime, count]) => `${runtime}=${count}`)
    .join(", ");
  console.log(`\nAll ${options.count} apps installed and used ${options.packageName}.`);
  console.log(`Runtime distribution: ${runtimeSummary}`);
  if (options.keep) {
    console.log(`Apps kept at: ${options.root}`);
  } else {
    console.log(`Successful app directories were removed. Root: ${options.root}`);
  }
}

try {
  const options = parseArgs(process.argv.slice(2));
  await runPool(options);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
