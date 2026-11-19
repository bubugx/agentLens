import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
const json = (path) => JSON.parse(readFileSync(path, "utf8"));
const pkg = json("package.json");
const tauri = json("src-tauri/tauri.conf.json");
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const lock = json("package-lock.json");
const cargoLock = readFileSync("src-tauri/Cargo.lock", "utf8");
const cargoLockVersion = cargoLock.match(
  /\[\[package\]\]\nname = "agentlens"\nversion = "([^"]+)"/,
)?.[1];
const [command = "--check", value] = process.argv.slice(2);
const versionPattern =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$/;
const valid = (version) =>
  versionPattern.test(version ?? "") &&
  version
    .split("-")
    .slice(1)
    .join("-")
    .split(".")
    .every((part) => !/^0[0-9]+$/.test(part));
if (command === "--set") {
  if (!valid(value)) throw new Error("Use a version such as 0.2.0 or 0.2.0-beta.1");
  pkg.version = tauri.version = lock.version = lock.packages[""].version = value;
  for (const [path, data] of [
    ["package.json", pkg],
    ["package-lock.json", lock],
    ["src-tauri/tauri.conf.json", tauri],
  ])
    writeFileSync(path, JSON.stringify(data, null, 2) + "\n");
  writeFileSync(
    "src-tauri/Cargo.toml",
    cargo.replace(/^version\s*=\s*"[^"]+"/m, `version = "${value}"`),
  );
  writeFileSync(
    "src-tauri/Cargo.lock",
    cargoLock.replace(
      /(\[\[package\]\]\nname = "agentlens"\nversion = ")[^"]+("\n)/,
      (_, before, after) => `${before}${value}${after}`,
    ),
  );
  execFileSync(process.execPath, [fileURLToPath(import.meta.url), "--check"], {
    stdio: "inherit",
  });
} else if (command === "--check") {
  const versions = {
    package: pkg.version,
    npmLock: lock.version,
    npmRoot: lock.packages[""].version,
    tauri: tauri.version,
    cargo: cargoVersion,
    cargoLock: cargoLockVersion,
  };
  if (!valid(pkg.version) || Object.values(versions).some((v) => v !== pkg.version))
    throw new Error(`Version mismatch: ${JSON.stringify(versions)}`);
  if (value && value !== `v${pkg.version}`) throw new Error(`Tag ${value} must be v${pkg.version}`);
  console.log(
    `Version ${pkg.version}: all six version fields agree${value ? ` with ${value}` : ""}.`,
  );
} else throw new Error("Usage: npm run version:check -- [v0.1.0] | npm run version:set -- 0.2.0");
