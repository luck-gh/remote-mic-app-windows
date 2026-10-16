import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { brotliDecompressSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const repository = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const receiptName = '.frontend-build.json';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => path.resolve(value).toLowerCase();

function filesUnder(directory) {
  if (!fs.existsSync(directory)) return [];
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const full = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error('frontend_input_symlink_unsupported');
    return entry.isDirectory() ? filesUnder(full) : [full];
  });
}

export function frontendInputFingerprint(root) {
  const inputs = [
    ...filesUnder(path.join(root, 'src')).filter(file => !/\.test\.[^.]+$/.test(file)),
    ...filesUnder(path.join(root, 'public')),
    ...['index.html', 'vite.config.ts', 'package.json', 'pnpm-lock.yaml', 'tsconfig.json', 'scripts/verify-frontend-embedded.mjs']
      .map(file => path.join(root, file)).filter(file => fs.existsSync(file)),
  ];
  if (!inputs.length) throw new Error('frontend_inputs_missing');
  return hash(inputs.map(file => `${path.relative(root, file).replaceAll('\\', '/')}\n${hash(fs.readFileSync(file))}`).sort().join('\n'));
}

function assetPath(dist, relative) {
  if (typeof relative !== 'string' || !relative || relative.includes('\\') || relative.startsWith('/')) throw new Error('invalid_frontend_asset_path');
  const resolved = path.resolve(dist, relative);
  if (!resolved.startsWith(path.resolve(dist) + path.sep)) throw new Error('frontend_asset_path_escaped');
  return resolved;
}

function htmlReferences(html) {
  const references = [...html.matchAll(/\b(?:src|href)=["']([^"']+)["']/g)].map(match => match[1]);
  return references.filter(value => !/^(?:[a-z]+:|\/\/|#)/i.test(value)).map(value => value.replace(/^\//, '').split(/[?#]/)[0]);
}

export function writeFrontendReceipt(root, chunks = [], expectedFingerprint = frontendInputFingerprint(root)) {
  const dist = path.join(root, 'dist');
  const html = fs.readFileSync(path.join(dist, 'index.html'), 'utf8');
  const graph = new Map(chunks.map(chunk => [chunk.fileName, chunk.references]));
  const referenced = new Set(['index.html']);
  const visit = relative => {
    if (referenced.has(relative)) return;
    assetPath(dist, relative);
    referenced.add(relative);
    for (const next of graph.get(relative) ?? []) visit(next);
  };
  for (const relative of htmlReferences(html)) visit(relative);
  if (![...referenced].some(file => file.endsWith('.js'))) throw new Error('frontend_entry_js_missing');
  const assets = [...referenced].sort().map(file => ({ path: file, sha256: hash(fs.readFileSync(assetPath(dist, file))) }));
  if (frontendInputFingerprint(root) !== expectedFingerprint) throw new Error('frontend_inputs_changed_during_build');
  const receipt = { schema: 1, inputFingerprint: expectedFingerprint, runtimeSimulation: process.env.VITE_SAYALL_RUNTIME_SIMULATION === '1', assets };
  fs.writeFileSync(path.join(dist, receiptName), JSON.stringify(receipt, null, 2));
  return receipt;
}

function currentFrontend(root, frontendDirectory) {
  const dist = path.join(root, 'dist');
  if (canonical(frontendDirectory) !== canonical(dist)) throw new Error('frontend_directory_must_be_current_dist');
  const receipt = JSON.parse(fs.readFileSync(path.join(dist, receiptName), 'utf8'));
  if (receipt.schema !== 1 || !Array.isArray(receipt.assets) || receipt.assets.length < 2) throw new Error('frontend_receipt_invalid');
  if (receipt.inputFingerprint !== frontendInputFingerprint(root)) throw new Error('frontend_receipt_stale');
  const assets = new Map();
  for (const asset of receipt.assets) {
    if (assets.has(asset.path) || !/^[a-f0-9]{64}$/.test(asset.sha256)) throw new Error('frontend_receipt_invalid');
    const bytes = fs.readFileSync(assetPath(dist, asset.path));
    if (hash(bytes) !== asset.sha256) throw new Error(`frontend_asset_changed:${asset.path}`);
    assets.set(asset.path, bytes);
  }
  const html = assets.get('index.html');
  if (!html) throw new Error('frontend_html_missing');
  const references = htmlReferences(html.toString('utf8'));
  if (!references.some(file => file.endsWith('.js'))) throw new Error('frontend_entry_js_missing');
  for (const relative of references) if (!assets.has(relative)) throw new Error('frontend_reference_not_in_receipt');
  return { dist, receipt, assets, references };
}

function executableDependencies(executable) {
  const dependencyFile = executable.replace(/\.exe$/i, '') + '.d';
  const first = fs.readFileSync(dependencyFile, 'utf8').replace(/\\\r?\n/g, '').split(/\r?\n/)[0];
  const separator = first.indexOf(': ');
  if (separator < 0) throw new Error('executable_dependency_file_invalid');
  const unescape = value => value.replace(/\\([ #])/g, '$1');
  if (canonical(unescape(first.slice(0, separator))) !== canonical(executable)) throw new Error('dependency_target_mismatch');
  const tokens = first.slice(separator + 2).match(/(?:\\[ #]|[^\s])+/g) ?? [];
  return tokens.map(token => path.resolve(unescape(token)));
}

export function verifyEmbeddedFrontend({ root = repository, executable = path.join(root, 'target/release/sayall-windows-app.exe'), frontendDirectory = path.join(root, 'dist') } = {}) {
  const current = currentFrontend(root, frontendDirectory);
  const dependencies = executableDependencies(executable);
  const dependencySet = new Set(dependencies.map(canonical));
  const compressedPaths = dependencies.filter(file => file.split(/[\\/]/).includes('tauri-codegen-assets'));
  const outputDirectories = new Set(compressedPaths.map(file => canonical(path.dirname(file))));
  if (outputDirectories.size !== 1) throw new Error('codegen_dependency_set_missing_or_ambiguous');
  const binary = fs.readFileSync(executable);
  const embedded = new Map();
  for (const file of compressedPaths) {
    const compressed = fs.readFileSync(file);
    if (!compressed.length || binary.indexOf(compressed) < 0) continue;
    let bytes;
    try { bytes = brotliDecompressSync(compressed); }
    catch { throw new Error('codegen_brotli_content_invalid'); }
    embedded.set(hash(bytes), bytes);
  }
  for (const [relative, expected] of current.assets) {
    if (!dependencySet.has(canonical(assetPath(current.dist, relative)))) throw new Error(`frontend_compiler_input_mismatch:${relative}`);
    if (binary.indexOf(Buffer.from(relative)) < 0) throw new Error(`frontend_resource_key_missing:${relative}`);
    const actual = embedded.get(hash(expected));
    if (!actual || !actual.equals(expected)) throw new Error(`frontend_embedded_content_mismatch:${relative}`);
  }
  // index.html itself is exact, so its startup script/style references belong to this receipt.
  return {
    passed: true,
    assetCount: current.assets.size,
    inputFingerprint: current.receipt.inputFingerprint,
    htmlSha256: hash(current.assets.get('index.html')),
    entryScripts: current.references.filter(file => file.endsWith('.js')),
    runtimeSimulation: current.receipt.runtimeSimulation,
  };
}

if (process.argv[1] && canonical(process.argv[1]) === canonical(fileURLToPath(import.meta.url))) {
  try {
    const [action, ...args] = process.argv.slice(2);
    let result;
    if (action === 'fingerprint') result = frontendInputFingerprint(repository);
    else if (action === 'write-receipt') {
      const graph = JSON.parse(fs.readFileSync(0, 'utf8'));
      result = writeFrontendReceipt(repository, graph, args[0]);
    } else if (action === 'verify') result = verifyEmbeddedFrontend({ executable: args[0], frontendDirectory: args[1] });
    else throw new Error('frontend_verification_action_invalid');
    console.log(JSON.stringify(result));
  } catch (error) {
    console.error(`frontend_verification result=failed reason=${error.code ?? error.message}`);
    process.exitCode = 1;
  }
}
