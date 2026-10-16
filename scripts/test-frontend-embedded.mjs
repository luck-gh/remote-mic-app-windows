import assert from 'node:assert/strict';
import { test } from 'node:test';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { brotliCompressSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { frontendInputFingerprint, verifyEmbeddedFrontend, writeFrontendReceipt } from './verify-frontend-embedded.mjs';

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const topic = path.join(repository, 'target/dev/onboarding-skip-exit');
fs.mkdirSync(topic, { recursive: true });
const fixtures = fs.mkdtempSync(path.join(topic, 'frontend-verification-'));

for (const kind of ['stale-directory', 'wrong-entry-content']) {
  test(`rejects ${kind} even when all resource path strings are present`, () => {
    const root = path.join(fixtures, kind);
    const dist = path.join(root, kind === 'stale-directory' ? 'dist-build' : 'dist');
    fs.mkdirSync(path.join(dist, 'assets'), { recursive: true });
    fs.writeFileSync(path.join(dist, 'index.html'), '<script type="module" src="/assets/current.js"></script>');
    fs.writeFileSync(path.join(dist, 'assets/current.js'), 'console.log("current")');
    const executable = path.join(root, 'app.exe');
    fs.writeFileSync(executable, Buffer.concat([
      Buffer.from('index.html assets/current.js '),
      brotliCompressSync(Buffer.from('<script type="module" src="/assets/old.js"></script>')),
      brotliCompressSync(Buffer.from('console.log("old")')),
    ]));
    const result = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(repository, 'scripts/verify-frontend-embedded.ps1'), '-Executable', executable, '-FrontendDirectory', dist], { encoding: 'utf8' });
    assert.equal(result.error, undefined);
    assert.notEqual(result.status, 0, 'Resource paths alone cannot prove current HTML/JS content');
  });
}

function fixture(name) {
  const root = path.join(fixtures, name);
  const dist = path.join(root, 'dist');
  const executable = path.join(root, 'target/release/app.exe');
  fs.mkdirSync(path.join(root, 'src'), { recursive: true });
  fs.mkdirSync(path.join(dist, 'assets'), { recursive: true });
  fs.mkdirSync(path.dirname(executable), { recursive: true });
  fs.writeFileSync(path.join(root, 'src/main.ts'), 'const current = true;');
  fs.writeFileSync(path.join(dist, 'index.html'), '<script type="module" src="/assets/current.js"></script><link rel="stylesheet" href="/assets/current.css">');
  fs.writeFileSync(path.join(dist, 'assets/current.js'), 'import("./dynamic.js");console.log("current");');
  fs.writeFileSync(path.join(dist, 'assets/dynamic.js'), 'export const current = true;');
  fs.writeFileSync(path.join(dist, 'assets/current.css'), 'body { color: green; }');
  const receipt = writeFrontendReceipt(root, [{ fileName: 'assets/current.js', references: ['assets/dynamic.js'] }]);
  const codegen = path.join(root, 'target/release/build/app/out/tauri-codegen-assets');
  fs.mkdirSync(codegen, { recursive: true });
  const compressedFiles = receipt.assets.map(asset => {
    const bytes = brotliCompressSync(fs.readFileSync(path.join(dist, asset.path)));
    const file = path.join(codegen, createHash('sha256').update(bytes).digest('hex') + path.extname(asset.path));
    fs.writeFileSync(file, bytes);
    return file;
  });
  const dependencies = [...receipt.assets.map(asset => path.join(dist, asset.path)), ...compressedFiles];
  const escape = value => value.replaceAll(' ', '\\ ');
  fs.writeFileSync(executable.replace(/\.exe$/, '.d'), `${escape(executable)}: ${dependencies.map(escape).join(' ')}\n`);
  const writeExe = () => fs.writeFileSync(executable, Buffer.concat([Buffer.from(receipt.assets.map(asset => asset.path).join(' ')), ...compressedFiles.map(file => fs.readFileSync(file))]));
  writeExe();
  return { root, dist, executable, receipt, compressedFiles, writeExe, verify: () => verifyEmbeddedFrontend({ root, executable }) };
}

test('accepts current HTML, entry CSS and transitive JS with compressed bytes in the actual EXE', () => {
  const current = fixture('positive with spaces');
  assert.equal(current.verify().passed, true);
  assert.equal(current.verify().assetCount, 4);
  assert.deepEqual(current.verify().entryScripts, ['assets/current.js']);
});

test('rejects a stale build receipt after a frontend input changes', () => {
  const current = fixture('stale-input');
  fs.writeFileSync(path.join(current.root, 'src/main.ts'), 'const current = false;');
  assert.throws(current.verify, /frontend_receipt_stale/);
});

test('rejects an old embedded HTML entry even if the new JS is also present', () => {
  const current = fixture('old-html');
  const index = current.receipt.assets.findIndex(asset => asset.path === 'index.html');
  fs.writeFileSync(current.compressedFiles[index], brotliCompressSync(Buffer.from('<script src="/assets/old.js"></script>')));
  current.writeExe();
  assert.throws(current.verify, /frontend_embedded_content_mismatch:index.html/);
});

test('rejects different embedded JS bytes with the same resource name', () => {
  const current = fixture('same-name-old-js');
  const index = current.receipt.assets.findIndex(asset => asset.path === 'assets/current.js');
  fs.writeFileSync(current.compressedFiles[index], brotliCompressSync(Buffer.from('console.log("old");')));
  current.writeExe();
  assert.throws(current.verify, /frontend_embedded_content_mismatch:assets\/current.js/);
});

test('does not use a matching codegen cache if its compressed bytes are absent from the actual EXE', () => {
  const current = fixture('cache-not-exe');
  fs.writeFileSync(current.executable, current.receipt.assets.map(asset => asset.path).join(' ') + ' runtime_frame terminal_context');
  assert.throws(current.verify, /frontend_embedded_content_mismatch/);
});

test('rejects a caller-selected old output directory', () => {
  const current = fixture('wrong-directory');
  assert.throws(() => verifyEmbeddedFrontend({ root: current.root, executable: current.executable, frontendDirectory: path.join(current.root, 'dist-build') }), /frontend_directory_must_be_current_dist/);
});

test('rejects output bytes changed after the current build receipt', () => {
  const current = fixture('output-changed');
  fs.writeFileSync(path.join(current.dist, 'assets/current.js'), 'console.log("changed")');
  assert.throws(current.verify, /frontend_asset_changed/);
});

test('fails closed when the compiler dependency receipt is absent', () => {
  const current = fixture('missing-depfile');
  // Rename this generated fixture rather than deleting it, preserving the negative evidence.
  fs.renameSync(current.executable.replace(/\.exe$/, '.d'), current.executable + '.missing-depfile');
  assert.throws(current.verify, /ENOENT/);
});

test('rejects compiler inputs selected from an old frontend directory', () => {
  const current = fixture('wrong-compiler-input');
  const depfile = current.executable.replace(/\.exe$/, '.d');
  fs.writeFileSync(depfile, fs.readFileSync(depfile, 'utf8').replaceAll(path.join(current.dist, 'index.html'), path.join(current.root, 'dist-build/index.html')));
  assert.throws(current.verify, /frontend_compiler_input_mismatch:index.html/);
});

test('refuses to issue a build receipt after inputs change during the build', () => {
  const current = fixture('input-changed-during-build');
  const started = frontendInputFingerprint(current.root);
  fs.writeFileSync(path.join(current.root, 'src/main.ts'), 'const changedDuringBuild = true;');
  assert.throws(() => writeFrontendReceipt(current.root, [], started), /frontend_inputs_changed_during_build/);
});
