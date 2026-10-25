import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, realpath, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { build, createServer, preview } from 'vite';
import reviewConfig from '../vite.config.js';

async function fixture(t, enabled = true) {
  const cwd = process.cwd();
  const previous = process.env.CNA_PRIVATE_REVIEW_TILES;
  delete process.env.CNA_PRIVATE_REVIEW_TILES;
  const root = await realpath(await mkdtemp(join(tmpdir(), 'cna-private-route-')));
  const servers = [];
  t.after(async () => {
    await Promise.all(servers.map(server => server.close()));
    process.chdir(cwd);
    if (previous === undefined) delete process.env.CNA_PRIVATE_REVIEW_TILES;
    else process.env.CNA_PRIVATE_REVIEW_TILES = previous;
    await rm(root, { recursive: true, force: true });
  });
  process.chdir(root);
  const images = join(root, 'docs/CNAv2.1.0/images');
  await mkdir(images, { recursive: true });
  await writeFile(join(root, 'index.html'), '<!doctype html><title>Synthetic route fixture</title>');
  if (enabled) await writeFile(join(root, '.env.test'), `CNA_PRIVATE_REVIEW_TILES=${root}/tiles\n`);
  const assets = {
    'Token (1) #2.png': Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j1ioAAAAASUVORK5CYII=', 'base64'),
    'Token + 3.svg': Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>'),
  };
  for (const [name, bytes] of Object.entries(assets)) await writeFile(join(images, name), bytes);
  await writeFile(join(root, 'outside.svg'), '<svg>outside allowed directory</svg>');
  await symlink(join(root, 'outside.svg'), join(images, 'escape.svg'));
  return { root, assets, servers };
}

test('private source route serves only opted-in image basenames over GET/HEAD', async t => {
  const { root, assets, servers } = await fixture(t);
  const server = await createServer({
    ...reviewConfig({ command: 'serve', mode: 'test' }), root, configFile: false,
    logLevel: 'silent', server: { host: '127.0.0.1', port: 0 },
  });
  servers.push(server);
  await server.listen();
  const base = `http://127.0.0.1:${server.httpServer.address().port}/__source-tokens/`;
  for (const [name, bytes] of Object.entries(assets)) {
    const url = base + encodeURIComponent(name);
    const response = await fetch(url);
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('content-type'), name.endsWith('.svg') ? 'image/svg+xml' : 'image/png');
    assert.equal(response.headers.get('cache-control'), 'private, no-store');
    assert.equal(response.headers.get('x-content-type-options'), 'nosniff');
    assert.match(response.headers.get('content-security-policy'), /default-src 'none'.*sandbox/);
    assert.deepEqual(Buffer.from(await response.arrayBuffer()), bytes);
    const head = await fetch(url, { method: 'HEAD' });
    assert.equal(head.status, 200);
    assert.equal((await head.arrayBuffer()).byteLength, 0);
    const post = await fetch(url, { method: 'POST' });
    assert.equal(post.status, 405);
    assert.equal(post.headers.get('allow'), 'GET, HEAD');
  }
  for (const path of ['..%2Foutside.svg', 'folder%2Ftoken.png', 'folder%5Ctoken.png', '%00.png', '%ZZ.png', 'index.html', 'missing.png', 'escape.svg']) {
    assert.equal((await fetch(base + path)).status, 404, path);
  }
});

test('private source route is absent without opt-in and from production build/preview', async t => {
  const { root, assets, servers } = await fixture(t, false);
  const config = { root, configFile: false, mode: 'test', logLevel: 'silent' };
  const server = await createServer({ ...config, ...reviewConfig({ command: 'serve', mode: 'test' }), server: { host: '127.0.0.1', port: 0 } });
  servers.push(server);
  await server.listen();
  const path = '/__source-tokens/' + encodeURIComponent(Object.keys(assets)[0]);
  const request = { headers: { accept: 'image/png' } };
  assert.equal((await fetch(`http://127.0.0.1:${server.httpServer.address().port}${path}`, request)).status, 404);

  // Even with the opt-in configured, a real production build/preview must not serve source bytes.
  await writeFile(join(root, '.env.test'), `CNA_PRIVATE_REVIEW_TILES=${root}/tiles\n`);
  await build({ ...config, ...reviewConfig({ command: 'build', mode: 'test' }) });
  const production = await preview({ ...config, ...reviewConfig({ command: 'serve', mode: 'test' }), preview: { host: '127.0.0.1', port: 0 } });
  servers.push(production);
  assert.equal((await fetch(`http://127.0.0.1:${production.httpServer.address().port}${path}`, request)).status, 404);
  assert.match(await readFile(join(root, 'dist/index.html'), 'utf8'), /Synthetic route fixture/);
});
