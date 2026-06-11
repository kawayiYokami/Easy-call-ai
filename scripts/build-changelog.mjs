import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const rootChangelogPath = path.join(repoRoot, "CHANGELOG.md");
const changelogRootDir = path.join(repoRoot, "docs", "changelog");
const releasesDir = path.join(changelogRootDir, "releases");
const latestPath = path.join(changelogRootDir, "latest.md");
const remotePath = path.join(changelogRootDir, "remote.md");
const indexPath = path.join(changelogRootDir, "index.json");
const recentReleaseCount = 12;

function parseVersionParts(version) {
  return version
    .replace(/^v/i, "")
    .split(/[.-]/)
    .map((part) => Number.parseInt(part, 10))
    .map((part) => (Number.isFinite(part) ? part : 0));
}

function compareVersionsDesc(left, right) {
  const a = parseVersionParts(left);
  const b = parseVersionParts(right);
  const length = Math.max(a.length, b.length);
  for (let index = 0; index < length; index += 1) {
    const diff = (b[index] ?? 0) - (a[index] ?? 0);
    if (diff !== 0) {
      return diff;
    }
  }
  return right.localeCompare(left, "en");
}

function normalizeMarkdown(markdown) {
  return markdown.replace(/\r\n/g, "\n").trim();
}

function parseRootSections(markdown) {
  const normalized = `${normalizeMarkdown(markdown)}\n`;
  const lines = normalized.split("\n");
  const releases = [];
  let currentVersion = "";
  let currentLines = [];

  function flushCurrent() {
    if (!currentVersion) return;
    const body = currentLines.join("\n").trim();
    releases.push({
      version: currentVersion,
      body,
    });
  }

  for (const line of lines) {
    const match = line.match(/^##\s+发布：\s*(v[^\s]+)\s*$/i);
    if (match) {
      flushCurrent();
      currentVersion = match[1];
      currentLines = [];
      continue;
    }
    if (currentVersion) {
      currentLines.push(line);
    }
  }
  flushCurrent();
  return releases;
}

function parseReleaseMarkdown(markdown, fallbackVersion) {
  const normalized = normalizeMarkdown(markdown);
  const lines = normalized.split("\n");
  const firstLine = lines[0] ?? "";
  const match = firstLine.match(/^#\s+发布：\s*(v[^\s]+)\s*$/i);
  const version = match?.[1] ?? fallbackVersion;
  const body = lines.slice(1).join("\n").trim();
  return { version, body };
}

function buildReleaseMarkdown(version, body) {
  const trimmedBody = body.trim();
  return `# 发布：${version}\n\n${trimmedBody}\n`;
}

function extractSummary(body) {
  const firstBullet = body
    .split("\n")
    .map((line) => line.trim())
    .find((line) => line.startsWith("- "));
  return firstBullet ? firstBullet.slice(2).trim() : "";
}

async function ensureDir(dir) {
  await mkdir(dir, { recursive: true });
}

async function migrateFromRoot() {
  const currentRoot = await readFile(rootChangelogPath, "utf8");
  const releases = parseRootSections(currentRoot);
  if (releases.length === 0) {
    throw new Error("现有 CHANGELOG.md 中没有解析到任何“## 发布：vX.Y.Z”节");
  }
  await ensureDir(releasesDir);
  for (const release of releases) {
    const filePath = path.join(releasesDir, `${release.version}.md`);
    const content = buildReleaseMarkdown(release.version, release.body);
    await writeFile(filePath, content, "utf8");
  }
}

async function loadReleaseEntries() {
  await ensureDir(releasesDir);
  const files = await readdir(releasesDir);
  const entries = [];
  for (const file of files) {
    if (!file.toLowerCase().endsWith(".md")) continue;
    const filePath = path.join(releasesDir, file);
    const fileStat = await stat(filePath);
    if (!fileStat.isFile()) continue;
    const fallbackVersion = path.basename(file, ".md");
    const markdown = await readFile(filePath, "utf8");
    const parsed = parseReleaseMarkdown(markdown, fallbackVersion);
    entries.push({
      version: parsed.version,
      body: parsed.body,
      fileName: file,
      relativePath: `docs/changelog/releases/${file}`,
      summary: extractSummary(parsed.body),
    });
  }
  entries.sort((left, right) => compareVersionsDesc(left.version, right.version));
  return entries;
}

function buildRootChangelog(entries) {
  const lines = [
    "# 变更日志",
    "",
    "> 此文件由 `pnpm changelog:build` 自动生成，请不要手改。",
    "> 详细版本说明拆分在 `docs/changelog/releases/`，应用内远程查看使用 `docs/changelog/remote.md`。",
    "",
    "## 最近版本",
    "",
  ];
  for (const entry of entries) {
    const summarySuffix = entry.summary ? ` - ${entry.summary}` : "";
    lines.push(`- [${entry.version}](${entry.relativePath})${summarySuffix}`);
  }
  lines.push("");
  return `${lines.join("\n")}\n`;
}

function buildLatestMarkdown(entry) {
  return `# 变更日志\n\n## 发布：${entry.version}\n\n${entry.body.trim()}\n`;
}

function buildRemoteMarkdown(entries) {
  const lines = [
    "# 变更日志",
    "",
    "> 此文件由 `pnpm changelog:build` 自动生成，展示最近版本的完整说明。",
    "",
  ];
  for (const entry of entries) {
    lines.push(`## 发布：${entry.version}`);
    lines.push("");
    if (entry.body.trim()) {
      lines.push(entry.body.trim());
      lines.push("");
    }
  }
  return `${lines.join("\n").trim()}\n`;
}

function buildIndexJson(entries) {
  return JSON.stringify(
    {
      generatedAt: new Date().toISOString(),
      latestVersion: entries[0]?.version ?? "",
      releases: entries.map((entry) => ({
        version: entry.version,
        path: entry.relativePath,
        summary: entry.summary,
      })),
    },
    null,
    2,
  );
}

async function main() {
  const args = new Set(process.argv.slice(2));
  if (args.has("--migrate-from-root")) {
    await migrateFromRoot();
  }

  const entries = await loadReleaseEntries();
  if (entries.length === 0) {
    throw new Error("docs/changelog/releases/ 下没有可用的发布说明文件");
  }

  await ensureDir(changelogRootDir);
  await writeFile(rootChangelogPath, buildRootChangelog(entries), "utf8");
  await writeFile(latestPath, buildLatestMarkdown(entries[0]), "utf8");
  await writeFile(
    remotePath,
    buildRemoteMarkdown(entries.slice(0, recentReleaseCount)),
    "utf8",
  );
  await writeFile(indexPath, `${buildIndexJson(entries)}\n`, "utf8");
}

main().catch((error) => {
  console.error(`[changelog] ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
});
