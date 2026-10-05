// Build the sole RC003 Helper and stage its locked runtime beside the app.
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const root = path.resolve(__dirname, "..");
const helperRoot = path.join(root, "hardware", "RC003", "helper");
const vendor = path.join(helperRoot, "vendor");
execFileSync("cargo", ["build", "--release", "--locked", "--manifest-path", path.join(helperRoot, "Cargo.toml")], {
  cwd: root, stdio: "inherit",
});
execFileSync("python", [path.join(vendor, "fetch_frida_gadget.py")], { cwd: root, stdio: "inherit" });
for (const [source, destination] of [
  [path.join(helperRoot, "target", "release", "sayall-helper.exe"), "sayall-helper.exe"],
  [path.join(vendor, "frida-gadget.dll"), "frida-gadget.dll"],
  [path.join(vendor, "Frida-COPYING.txt"), "licenses/Frida-COPYING.txt"],
  [path.join(root, "ATTRIBUTION.md"), "licenses/ATTRIBUTION.md"],
]) {
  const target = path.join(root, "src-tauri", destination);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(source, target);
  console.log(`[stage] ${destination}`);
}
