// Preserve compiler/test output and expose each useful tail within GitHub's annotation limit.
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
  if (process.env.GITHUB_ACTIONS) {
    for (const [name, output] of [
      ["tests", run.stdout],
      ["compiler", run.stderr],
      ["execution", run.error?.message],
    ]) {
      if (!output) continue;
      const tail = output.replace(/\u001b\[[0-9;]*m/g, "").slice(-3400);
      console.log(
        `::error title=Rust ${name} failed::${tail.replace(/%/g, "%25").replace(/\r/g, "%0D").replace(/\n/g, "%0A")}`,
      );
    }
  }
  process.exit(run.status && run.status > 0 && run.status < 256 ? run.status : 1);
}
