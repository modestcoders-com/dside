#!/usr/bin/env node
/**
 * Guards the public chronicle.
 *
 * This repository is public and `PROGRESS.md` is published with it. Two kinds of text must not
 * reach it: paths from a contributor's machine, and names that belong to a person or an
 * unpublished project. The second kind cannot be detected without knowing the names, so they
 * live in a gitignored list (`.private-names`) — outside the repository, never published.
 *
 *   node scripts/check-private.mjs            # report and exit 1 on a hit
 *   node scripts/check-private.mjs --warn     # report, exit 0 (for the build)
 *
 * Machine paths are checked everywhere in the versioned text; names only in the chronicle.
 */
import { readFileSync, readdirSync, existsSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const warnOnly = process.argv.includes("--warn");

/** Directories that are never part of the published text. */
const SKIP = new Set([".git", "node_modules", "target", "dist", "src-tauri", "_handover", ".kilo"]);

/** Files whose whole point is prose that a reader sees. */
const TEXT = /\.(md|markdown)$/i;

function walk(dir, out = []) {
  for (const entry of readdirSync(dir)) {
    if (SKIP.has(entry)) continue;
    const path = join(dir, entry);
    const stats = statSync(path);
    if (stats.isDirectory()) walk(path, out);
    else if (TEXT.test(entry)) out.push(path);
  }
  return out;
}

const namesPath = join(root, ".private-names");
const names = existsSync(namesPath)
  ? readFileSync(namesPath, "utf8")
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#"))
  : [];

const findings = [];
for (const file of walk(root)) {
  const text = readFileSync(file, "utf8");
  const where = relative(root, file);
  text.split("\n").forEach((line, index) => {
    const at = `${where}:${index + 1}`;
    // /home/<user>/ and /Users/<user>/ — the path says which machine it was.
    const machine = line.match(/\/(?:home|Users)\/[^/\s"`')]+/);
    if (machine) findings.push({ at, kind: "path", hit: machine[0], line: line.trim().slice(0, 100) });
    for (const name of names) {
      if (line.toLowerCase().includes(name.toLowerCase())) {
        findings.push({ at, kind: "name", hit: name, line: line.trim().slice(0, 100) });
      }
    }
  });
}

if (findings.length === 0) {
  console.log(`private-check: clean (${names.length} name(s) watched)`);
  process.exit(0);
}

console.error(`private-check: ${findings.length} finding(s) in published text`);
for (const finding of findings) {
  console.error(`  ${finding.at} [${finding.kind}: ${finding.hit}]`);
  console.error(`    ${finding.line}`);
}
process.exit(warnOnly ? 0 : 1);
