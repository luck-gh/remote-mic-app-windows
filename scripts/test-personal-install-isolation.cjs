const { readFileSync } = require("node:fs");
const { resolve } = require("node:path");
const assert = require("node:assert/strict");
const { test } = require("node:test");

const root = resolve(__dirname, "..");
const read = (path) => readFileSync(resolve(root, path), "utf8");

test("personal installation and configuration have an independent identity", () => {
  const config = JSON.parse(read("src-tauri/tauri.conf.json"));
  assert.equal(config.identifier, "app.luckgh.remote-mic.personal.windows");
  assert.equal(config.productName, "无线麦个人版");
  assert.equal(config.bundle.windows.nsis.startMenuFolder, "无线麦个人版");
  const installer = read("src-tauri/windows/installer.nsi");
  assert.ok(installer.includes('!define INSTALLIDENTITY "无线麦个人版"'));
  assert.ok(installer.includes('!define MANUKEY "Software\\LuckGH"'));
});

test("personal app and helper use matching isolated runtime resources", () => {
  for (const path of [
    "crates/sayall-windows/src/rc003_bridge.rs",
    "src-tauri/src/rc003_task.rs",
    "src-tauri/src/lib.rs",
    "hardware/RC003/helper/src/main.rs",
  ]) {
    const source = read(path);
    assert.ok(source.includes('.join("SayAllPersonal")'), path);
    assert.ok(!source.includes('.join("SayAll")'), path);
  }
  for (const path of [
    "crates/sayall-windows/src/rc003_bridge.rs",
    "hardware/RC003/helper/src/main.rs",
  ]) {
    const source = read(path);
    assert.ok(source.includes("SayAllPersonal.Rc003Bridge"), path);
    assert.ok(!source.includes("SayAll.Rc003Bridge"), path);
  }
  for (const path of ["src-tauri/src/rc003_task.rs", "hardware/RC003/helper/src/main.rs"]) {
    assert.ok(read(path).includes('"SayAll Personal RC003 Helper"'), path);
    assert.ok(!read(path).includes('"SayAll RC003 Helper"'), path);
  }
  assert.ok(read("hardware/RC003/helper/src/main.rs").includes("const DEFAULT_PORT: u16 = 47832;"));
});

test("transcript and audio archives use the personal runtime root", () => {
  for (const path of [
    "src-tauri/src/transcripts.rs",
    "crates/sayall-windows/src/voice_recording.rs",
  ]) {
    const source = read(path);
    assert.ok(source.includes('.join("SayAllPersonal")'), path);
    assert.ok(!source.includes('.join("SayAll")'), path);
  }
});

test("exit, activation, startup and installer cleanup do not target upstream", () => {
  for (const path of [
    "crates/sayall-windows/src/graceful_exit.rs",
    "src-tauri/windows/installer-hooks.nsh",
  ]) {
    assert.ok(read(path).includes("SayAllPersonal-GracefulExit"), path);
    assert.ok(!read(path).includes("SayAll-GracefulExit"), path);
  }
  assert.ok(read("crates/sayall-windows/src/instance_signal.rs").includes("SayAllPersonal-ShowMainWindow"));
  assert.ok(read("src-tauri/src/lib.rs").includes("SayAllPersonal.Windows.SingleInstance"));
  assert.ok(read("src-tauri/src/startup.rs").includes('const VALUE_NAME: &str = "SayAllPersonal";'));
  assert.ok(read("src-tauri/windows/verify-capture-cleanup.ps1").includes("'SayAllPersonal/rc003-capture-cleanup.json'"));
  assert.ok(!read("src-tauri/windows/installer.nsi").includes("$LOCALAPPDATA\\SayAll\""));
});
