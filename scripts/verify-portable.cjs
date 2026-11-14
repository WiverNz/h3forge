// Build-time inspection only; no Node runtime is shipped with H3Forge.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const { brotliDecompressSync } = require('node:zlib');
const { createHash } = require('node:crypto');
process.chdir(path.join(__dirname, '..'));
const exe = fs.readFileSync('.local/release/H3Forge.exe');
const hash = createHash('sha256').update(exe).digest('hex');
assert.equal(fs.readFileSync('.local/release/H3Forge.exe.sha256', 'utf8'), `${hash}  H3Forge.exe\n`);
const config = JSON.parse(fs.readFileSync('apps/h3forge-desktop/src-tauri/tauri.conf.json', 'utf8'));
assert.equal(config.bundle.active, false);
assert.deepEqual(config.bundle.resources, []);
assert.ok(!config.bundle.externalBin);

function files(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
    const name = path.join(dir, entry.name);
    return entry.isDirectory() ? files(name) : [name];
  });
}
const assets = files('web/dist');
const allowed = /^(index\.html|LICENSES\.txt|theme-init\.js|assets\/index-[\w-]+\.(js|css))$/;
assert.equal(assets.length, 5, 'Unexpected frontend asset count');
for (const name of assets) assert.match(path.relative('web/dist', name).replaceAll('\\', '/'), allowed);
const build = 'target/x86_64-pc-windows-msvc/release/build';
const payloads = fs.readdirSync(build).filter(name => name.startsWith('h3forge-desktop-'))
  .flatMap(name => files(path.join(build, name)))
  .filter(name => name.includes('tauri-codegen-assets'))
  .map(name => fs.readFileSync(name)).filter(bytes => exe.includes(bytes));
const embedded = [...new Map(payloads.map(bytes => [createHash('sha256').update(bytes).digest('hex'), bytes])).values()]
  .map(bytes => brotliDecompressSync(bytes));
assert.equal(embedded.length, 5, 'Expected all five compiled frontend assets in the EXE');
for (const name of assets.filter(name => !name.endsWith('index.html'))) {
  assert.ok(embedded.some(bytes => bytes.equals(fs.readFileSync(name))), `Missing embedded ${name}`);
}
assert.ok(embedded.some(bytes => bytes.includes(Buffer.from('<div id="root">')) && bytes.includes(Buffer.from('<title>H3Forge</title>'))));

// Inspect the PE resource directory, not just an icon filename in build logs.
const pe = exe.readUInt32LE(0x3c);
assert.equal(exe.toString('ascii', pe, pe + 4), 'PE\0\0');
assert.equal(exe.readUInt16LE(pe + 4), 0x8664, 'Expected Windows x64');
const optional = pe + 24;
assert.equal(exe.readUInt16LE(optional), 0x20b);
const sections = [];
for (let i = 0; i < exe.readUInt16LE(pe + 6); i++) {
  const offset = optional + exe.readUInt16LE(pe + 20) + i * 40;
  sections.push({ rva: exe.readUInt32LE(offset + 12), size: exe.readUInt32LE(offset + 16), raw: exe.readUInt32LE(offset + 20) });
}
function raw(rva) {
  const section = sections.find(section => rva >= section.rva && rva < section.rva + section.size);
  assert.ok(section, `Invalid PE RVA ${rva}`);
  return section.raw + rva - section.rva;
}
const resourceBase = raw(exe.readUInt32LE(optional + 112 + 2 * 8));
const resources = [];
function walk(relative, ids = []) {
  const offset = resourceBase + relative;
  const count = exe.readUInt16LE(offset + 12) + exe.readUInt16LE(offset + 14);
  for (let i = 0; i < count; i++) {
    const entry = offset + 16 + i * 8;
    const id = exe.readUInt32LE(entry);
    const child = exe.readUInt32LE(entry + 4);
    if (child & 0x80000000) walk(child & 0x7fffffff, [...ids, id]);
    else {
      const data = resourceBase + child;
      const start = raw(exe.readUInt32LE(data));
      resources.push({ ids: [...ids, id], bytes: exe.subarray(start, start + exe.readUInt32LE(data + 4)) });
    }
  }
}
walk(0);
const ico = fs.readFileSync('apps/h3forge-desktop/src-tauri/icons/icon.ico');
for (let i = 0; i < ico.readUInt16LE(4); i++) {
  const entry = 6 + i * 16;
  const start = ico.readUInt32LE(entry + 12);
  const bytes = ico.subarray(start, start + ico.readUInt32LE(entry + 8));
  // Only the 256 px frame may be PNG. Browser download lists and other shell
  // consumers fall back to the generic EXE icon when small frames are PNG.
  const isPng = bytes.readUInt32BE(0) === 0x89504e47;
  assert.equal(isPng, ico[entry] === 0, `Icon frame ${i} must be ${ico[entry] === 0 ? 'PNG' : 'BMP'}`);
  assert.ok(resources.some(resource => resource.ids[0] === 3 && resource.bytes.equals(bytes)), `Missing icon frame ${i}`);
}
assert.ok(resources.some(resource => resource.ids[0] === 14), 'Missing icon group');
const imports = [];
for (let offset = raw(exe.readUInt32LE(optional + 112 + 8)); exe.readUInt32LE(offset + 12); offset += 20) {
  const start = raw(exe.readUInt32LE(offset + 12));
  imports.push(exe.toString('ascii', start, exe.indexOf(0, start)));
}
assert.ok(!imports.some(name => /vcruntime|msvcp|webview2loader|python|node/i.test(name)), `Unexpected runtime DLL: ${imports}`);
for (const encoding of ['latin1', 'utf16le']) {
  const match = exe.toString(encoding).match(/[a-z]:[\\/](?:Users|Projects|Games|a)[\\/][^\x00-\x1f]{0,160}/i);
  assert.ok(!match, `Absolute development path in EXE: ${match?.[0]}`);
}
console.log(`Verified ${exe.length} bytes, SHA-256 ${hash}`);
console.log(`Embedded frontend: ${assets.length} assets; Windows icon: ${ico.readUInt16LE(4)} exact frames; imports: ${imports.join(', ')}`);
