#!/usr/bin/env node
// Generates website/src/pages/changelog.md from the repository CHANGELOG.md.
// CHANGELOG.md is maintained automatically by the pre-release workflow, so the
// website page stays in sync by running this script (pnpm run changelog).
// Each release is rendered as a collapsible card, newest first, with the
// latest release expanded by default.
// Usage: node scripts/sync-changelog.mjs

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const SOURCE = join(ROOT, "CHANGELOG.md");
const TARGET = join(ROOT, "website", "src", "pages", "changelog.md");
const REPO = "MalpenZibo/ashell";

const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

function formatDate(iso) {
  const [year, month, day] = iso.split("-").map(Number);
  if (!year || !month || !day) {
    return iso;
  }
  return `${MONTHS[month - 1]} ${day}, ${year}`;
}

let content = readFileSync(SOURCE, "utf8");

// MDX treats <https://...> autolinks as JSX elements; convert them to
// regular markdown links so the page renders correctly.
content = content.replace(/<(https?:\/\/[^>\s]+)>/g, "[$1]($1)");

const headingRegex = /^## \[([\w.\-]+)\](?: - (\d{4}-\d{2}-\d{2}))?$/gm;
const releases = [];
let match;
while ((match = headingRegex.exec(content)) !== null) {
  releases.push({
    tag: match[1],
    date: match[2] ?? "",
    start: match.index,
    end: headingRegex.lastIndex,
  });
}

const sections = releases.map((release, index) => {
  const next = releases[index + 1];
  const body = content.slice(release.end, next ? next.start : undefined).trim();
  const open = index === 0 ? " open" : "";
  return [
    `<details className="changelogRelease"${open}>`,
    `<summary><span className="changelogVersion">${release.tag}</span><span className="changelogDate">${formatDate(release.date)}</span></summary>`,
    "",
    body,
    "",
    `[View on GitHub](https://github.com/${REPO}/releases/tag/${release.tag})`,
    "",
    "</details>",
    "",
  ].join("\n");
});

const page = [
  "---",
  "title: Changelog",
  "description: Release notes for ashell",
  "hide_table_of_contents: true",
  "---",
  "",
  "# Changelog",
  "",
  "Release notes for ashell.",
  "",
  ...sections,
].join("\n");

writeFileSync(TARGET, page);
console.log(
  `Synced CHANGELOG.md -> website/src/pages/changelog.md (${releases.length} releases)`,
);
