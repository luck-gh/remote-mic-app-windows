import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(resolve(process.cwd(), path), "utf8");
const repository = "luck-gh/remote-mic-app-windows";

describe("personal updater release contract", () => {
  it("uses only the personal stable endpoint and an independent signing identity", () => {
    const config = JSON.parse(read("src-tauri/tauri.conf.json"));
    expect(config.plugins.updater.endpoints).toEqual([
      `https://github.com/${repository}/releases/latest/download/latest.json`,
    ]);
    const decoded = Buffer.from(config.plugins.updater.pubkey, "base64").toString("utf8");
    expect(decoded).not.toContain("72EF4D508D580248");
  });

  it("keeps preview discovery and asset validation within the personal repository", () => {
    const updater = read("src-tauri/src/updater.rs").split("#[cfg(test)]")[0]!;
    expect(updater).toContain(`https://github.com/${repository}/releases.atom`);
    expect(updater).not.toContain("GetSayAll/remote-mic-app-windows");
    expect(updater).toContain("validate_download_source(&update.download_url)");
    expect(updater).toContain("#[cfg(debug_assertions)]");
  });

  it("generates release download links from the explicitly selected personal repository", () => {
    const manifest = read("scripts/generate-updater-manifest.ps1");
    const workflow = read(".github/workflows/windows-release.yml");
    expect(manifest).not.toContain('= "GetSayAll/remote-mic-app-windows"');
    expect(manifest).toContain('= "luck-gh/remote-mic-app-windows"');
    expect(workflow).toContain('-Repository $env:GITHUB_REPOSITORY');
    expect(workflow).toContain('verify-updater-signing');
  });

  it("keeps merged input diagnostics free of device identities and typed key codes", () => {
    const raw = read("crates/sayall-windows/src/raw_input_windows.rs");
    for (const field of ["selected_path=", "previous_path=", "rebound_path=", "lost_path=", "kept_path=", "bound_path=", "event_path="]) {
      expect(raw).not.toContain(field);
    }
    const host = read("src-tauri/src/lib.rs");
    const observation = host.slice(host.indexOf("fn end_key_observation("), host.indexOf("async fn preview_template_import("));
    expect(observation).not.toContain("last_vk=");
    expect(observation).not.toContain("keys=");
  });

  it("retains reconnect without system-wide radio or PnP reset side effects", () => {
    const radio = read("crates/sayall-windows/src/bluetooth_radio.rs");
    expect(radio).not.toContain("SetStateAsync");
    expect(radio).not.toContain("pnputil");
    const ble = read("crates/sayall-windows/src/ble.rs");
    expect(ble).not.toContain("cycle_bluetooth_radio");
    expect(ble).toContain("schedule_reconnect(");
    expect(read("src-tauri/src/lib.rs")).not.toContain("prepare_bluetooth_radio_recovery");
  });
});
