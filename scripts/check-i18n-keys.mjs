import fs from "node:fs";
import path from "node:path";

const rootDir = process.cwd();
const includeUnused = process.argv.includes("--unused");
const localeDir = path.join(rootDir, "src", "locales");
const scanDirs = ["src", "tests"];
const scanExts = new Set([".vue", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"]);
const ignoredDirs = new Set([
  ".git",
  "dist",
  "node_modules",
  "src-tauri",
  "target",
]);

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function flattenKeys(value, prefix = "", output = new Set()) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    if (prefix) output.add(prefix);
    return output;
  }

  for (const [key, child] of Object.entries(value)) {
    const nextPrefix = prefix ? `${prefix}.${key}` : key;
    flattenKeys(child, nextPrefix, output);
  }

  return output;
}

function walk(dirPath, output = []) {
  if (!fs.existsSync(dirPath)) return output;

  for (const entry of fs.readdirSync(dirPath, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!ignoredDirs.has(entry.name)) {
        walk(path.join(dirPath, entry.name), output);
      }
      continue;
    }

    if (entry.isFile() && scanExts.has(path.extname(entry.name))) {
      output.push(path.join(dirPath, entry.name));
    }
  }

  return output;
}

function collectUsedKeys(filePath) {
  const source = fs.readFileSync(filePath, "utf8");
  const patterns = [
    /\b(?:t|te|tm|rt)\(\s*(['"`])([^'"`${}]+)\1/g,
    /\$t\(\s*(['"`])([^'"`${}]+)\1/g,
    /\bi18n\.global\.(?:t|te|tm|rt)\(\s*(['"`])([^'"`${}]+)\1/g,
  ];
  const keys = [];

  for (const pattern of patterns) {
    for (const match of source.matchAll(pattern)) {
      keys.push({
        key: match[2],
        file: path.relative(rootDir, filePath),
      });
    }
  }

  return keys;
}

function printSection(title, lines) {
  console.log(`\n${title}`);
  console.log("=".repeat(title.length));
  if (lines.length === 0) {
    console.log("OK");
    return;
  }
  for (const line of lines) console.log(line);
}

const localeFiles = fs
  .readdirSync(localeDir)
  .filter((fileName) => fileName.endsWith(".json"))
  .sort();

const localeKeys = new Map();
for (const fileName of localeFiles) {
  const localeName = path.basename(fileName, ".json");
  localeKeys.set(localeName, flattenKeys(readJson(path.join(localeDir, fileName))));
}

const usageByKey = new Map();
for (const dirName of scanDirs) {
  for (const filePath of walk(path.join(rootDir, dirName))) {
    for (const usage of collectUsedKeys(filePath)) {
      if (!usageByKey.has(usage.key)) usageByKey.set(usage.key, new Set());
      usageByKey.get(usage.key).add(usage.file);
    }
  }
}

const usedKeys = [...usageByKey.keys()].sort();
const allLocaleKeys = new Set([...localeKeys.values()].flatMap((keys) => [...keys]));

const missingUsedLines = [];
for (const key of usedKeys) {
  const missingLocales = [...localeKeys.entries()]
    .filter(([, keys]) => !keys.has(key))
    .map(([localeName]) => localeName);

  if (missingLocales.length > 0) {
    missingUsedLines.push(
      `${key} | missing: ${missingLocales.join(", ")} | used in: ${[
        ...usageByKey.get(key),
      ].join(", ")}`,
    );
  }
}

const localeMismatchLines = [];
for (const [localeName, keys] of localeKeys.entries()) {
  const missing = [...allLocaleKeys].filter((key) => !keys.has(key)).sort();
  for (const key of missing) {
    localeMismatchLines.push(`${localeName} missing ${key}`);
  }
}

const unusedLocaleLines = [...allLocaleKeys]
  .filter((key) => !usageByKey.has(key))
  .sort()
  .map((key) => key);

printSection("Used keys missing from locales", missingUsedLines);
printSection("Locale shape mismatches", localeMismatchLines);
if (includeUnused) {
  printSection("Locale keys not found in static t() calls", unusedLocaleLines);
} else {
  console.log(
    `\nLocale keys not found in static t() calls: ${unusedLocaleLines.length} (run with --unused to list)`,
  );
}

if (missingUsedLines.length > 0 || localeMismatchLines.length > 0) {
  process.exitCode = 1;
}
