#!/usr/bin/env node
/**
 * The numbers a release note quotes, measured instead of remembered.
 *
 * A chronicle that says "146 tests, 556 keys" is a claim like any other, and a claim typed by
 * hand goes stale the moment someone adds a test. A review of a sibling project found exactly
 * that: dates and counts in the progress file drifting away from what the code did. This script
 * is the cure — the numbers come from here, and the text quotes its output instead of guessing.
 *
 *   pnpm status
 */
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const env = { ...process.env };
// The AppImage this session may run inside sets ARGV0; rustup then mistakes itself for a proxy.
delete env.ARGV0;

function run(command, args, options = {}) {
  return execFileSync(command, args, { encoding: "utf8", env, ...options });
}

const results = {};

try {
  const out = run("cargo", ["test", "--lib", "--manifest-path", "src-tauri/Cargo.toml"], {
    cwd: root,
  });
  const match = out.match(/test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/);
  results.tests = match
    ? { passed: Number(match[1]), failed: Number(match[2]), ignored: Number(match[3]) }
    : { raw: "unparseable test output" };
} catch (error) {
  // A failing suite is the one thing this script must report loudly.
  const out = `${error.stdout ?? ""}${error.stderr ?? ""}`;
  const match = out.match(/test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/);
  results.tests = match
    ? { passed: Number(match[1]), failed: Number(match[2]), ignored: Number(match[3]) }
    : { raw: "the test run could not be read" };
}

try {
  const out = run("node", ["scripts/check-i18n.mjs"], { cwd: root });
  const match = out.match(/(\d+) keys, (\d+) catalogs/);
  results.i18n = match ? { keys: Number(match[1]), catalogs: Number(match[2]) } : { raw: out.trim() };
} catch (error) {
  results.i18n = { raw: `${error.stdout ?? ""}${error.stderr ?? ""}`.trim() || "check failed" };
}

results.version = {
  package: JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version,
  cargo: /^version\s*=\s*"([^"]*)"/m.exec(
    readFileSync(join(root, "src-tauri/Cargo.toml"), "utf8"),
  )?.[1],
  tauri: JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8")).version,
};
results.tag = run("git", ["describe", "--tags", "--always"], { cwd: root }).trim();
results.commits = Number(run("git", ["rev-list", "--count", "HEAD"], { cwd: root }).trim());

const tests = results.tests;
const summary = tests.passed === undefined
  ? "tests: could not be measured"
  : `tests: ${tests.passed} passed, ${tests.failed} failed, ${tests.ignored} ignored`;

console.log(JSON.stringify(results, null, 2));
console.log();
console.log(summary);
if (results.i18n.keys) console.log(`i18n: ${results.i18n.keys} keys in ${results.i18n.catalogs} catalogs`);
console.log(`version ${results.version.package} (cargo ${results.version.cargo}, tauri ${results.version.tauri}), tag ${results.tag}, ${results.commits} commits`);

// Non-zero when something is red, so it can gate a script as well as inform a note.
if (tests.failed > 0) {
  console.error(`\n${tests.failed} failing test(s) — the numbers above are not release-ready.`);
  process.exitCode = 1;
}
