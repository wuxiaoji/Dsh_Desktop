// Wrapper for the tauri CLI (registered as the "tauri" npm script).
// - "npm run tauri build"  -> bumps the patch version first, then builds the installer
// - "npm run tauri build -- minor" -> bumps the minor version (major also supported)
// - every other invocation (dev, icon, ...) passes through untouched
import { spawn } from "node:child_process";
import { bumpVersion } from "./bump-version.mjs";

const args = process.argv.slice(2);

if (args[0] === "build") {
  const level = ["major", "minor", "patch"].includes(args[1]) ? args[1] : "patch";
  const next = bumpVersion(level);
  console.log("[bump] version -> v" + next);
}

const child = spawn("npx", ["tauri", ...args], {
  stdio: "inherit",
  shell: process.platform === "win32",
});
child.on("error", (err) => {
  console.error("[tauri wrapper] failed to start tauri:", err);
  process.exit(1);
});
child.on("exit", (code) => process.exit(code ?? 0));
