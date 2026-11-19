// Preserve compiler/test output and expose the useful tail in GitHub check annotations.
import { spawnSync } from "node:child_process";
const run = spawnSync(
  "cargo",
  ["test", "--locked", "--manifest-path", "src-tauri/Cargo.toml", "--lib"],
  { encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
);
process.stdout.write(run.stdout ?? "");
process.stderr.write(run.stderr ?? "");
if (run.error) console.error(run.error.message);
if (run.status !== 0) {
  const tail = [run.stdout, run.stderr, run.error?.message]
    .filter(Boolean)
    .join("\n")
    .split("\n")
    .slice(-100)
    .join("\n")
    .slice(-14000);
  if (process.env.GITHUB_ACTIONS)
    console.log(
      `::error title=Rust validation failed::${tail.replace(/%/g, "%25").replace(/\r/g, "%0D").replace(/\n/g, "%0A")}`,
    );
  process.exit(run.status || 1);
}
