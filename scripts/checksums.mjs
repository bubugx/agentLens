import { createHash } from "node:crypto";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve, basename } from "node:path";
const dir = resolve(process.argv[2] ?? "artifacts/release");
const files = readdirSync(dir)
  .filter((f) => /\.(dmg|exe|deb|AppImage|mp4|srt|vtt)$/.test(f))
  .sort();
if (
  !files.some((f) => f.endsWith(".dmg")) ||
  !files.some((f) => f.endsWith(".exe")) ||
  !files.some((f) => f.endsWith(".deb")) ||
  !files.some((f) => f.endsWith(".AppImage"))
)
  throw new Error("Release is incomplete: require macOS, Windows, Debian and AppImage packages.");
const sums =
  files
    .map(
      (f) =>
        `${createHash("sha256")
          .update(readFileSync(resolve(dir, f)))
          .digest("hex")}  ${basename(f)}`,
    )
    .join("\n") + "\n";
writeFileSync(resolve(dir, "SHA256SUMS.txt"), sums);
console.log(`Checksummed ${files.length} release assets.`);
