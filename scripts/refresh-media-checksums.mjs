import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const checksumPath = process.argv[2];
if (!checksumPath) throw new Error("Pass the existing release SHA256SUMS.txt path.");
const names = ["agentlens-intro-zh.mp4", "agentlens-intro-zh.srt", "agentlens-intro-zh.vtt"];
const replacements = new Map(
  names.map((name) => [
    name,
    createHash("sha256")
      .update(readFileSync(resolve("docs/media", name)))
      .digest("hex"),
  ]),
);
const found = new Set();
const lines = readFileSync(checksumPath, "utf8")
  .trimEnd()
  .split(/\r?\n/)
  .map((line) => {
    const match = line.match(/^([a-f0-9]{64})  (.+)$/);
    if (!match) throw new Error("Invalid release checksum entry.");
    const name = match[2];
    if (!replacements.has(name)) return line;
    if (found.has(name)) throw new Error(`Duplicate media entry: ${name}`);
    found.add(name);
    return `${replacements.get(name)}  ${name}`;
  });
if (found.size !== names.length)
  throw new Error("Release must already contain all three media entries.");
writeFileSync(checksumPath, lines.join("\n") + "\n");
console.log("Updated video and subtitle hashes; installer entries preserved.");
