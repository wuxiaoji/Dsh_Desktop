// Increments the project version (major/minor/patch) and keeps every version
// field in sync: package.json, package-lock.json, src-tauri/tauri.conf.json.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const LEVELS = ["major", "minor", "patch"];

function increment(version, level) {
  const parts = String(version ?? "0.0.0").split(".").map((n) => parseInt(n, 10) || 0);
  while (parts.length < 3) parts.push(0);
  const idx = LEVELS.indexOf(level);
  const i = idx === -1 ? 2 : idx; // default: patch
  parts[i] += 1;
  for (let k = i + 1; k < 3; k++) parts[k] = 0;
  return parts.slice(0, 3).join(".");
}

export function bumpVersion(level = "patch", rootDir = ROOT) {
  const pkgPath = path.join(rootDir, "package.json");
  const lockPath = path.join(rootDir, "package-lock.json");
  const confPath = path.join(rootDir, "src-tauri", "tauri.conf.json");

  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
  const next = increment(pkg.version, level);
  pkg.version = next;
  fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

  if (fs.existsSync(lockPath)) {
    const lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
    if (lock.version) lock.version = next;
    if (lock.packages && lock.packages[""]) lock.packages[""].version = next;
    fs.writeFileSync(lockPath, JSON.stringify(lock, null, 2) + "\n");
  }

  const conf = JSON.parse(fs.readFileSync(confPath, "utf8"));
  conf.version = next;
  fs.writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

  const cargoPath = path.join(rootDir, "src-tauri", "Cargo.toml");
  if (fs.existsSync(cargoPath)) {
    let cargo = fs.readFileSync(cargoPath, "utf8");
    cargo = cargo.replace(/^version\s*=\s*"[^"]*"/m, 'version = "' + next + '"');
    fs.writeFileSync(cargoPath, cargo);
  }

  return next;
}

// Standalone CLI: node scripts/bump-version.mjs [patch|minor|major]
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const level = process.argv[2] || "patch";
  console.log("version -> " + bumpVersion(level));
}
