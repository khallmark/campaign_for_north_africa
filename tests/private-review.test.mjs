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
  await writeFile(join(root, 'index.html'), '<!doctype html><title>Synthetic route fixture</title><div class="startup"></div>');
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
  assert.match(await (await fetch(new URL('/', base))).text(), /class="startup-overview".*src="\/tiles\/map\/0\/0_0.png"/);
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
  assert.doesNotMatch(await (await fetch(`http://127.0.0.1:${server.httpServer.address().port}/`)).text(), /startup-overview|\/tiles\/map\//);

  // Even with the opt-in configured, a real production build/preview must not serve source bytes.
  await writeFile(join(root, '.env.test'), `CNA_PRIVATE_REVIEW_TILES=${root}/tiles\n`);
  await build({ ...config, ...reviewConfig({ command: 'build', mode: 'test' }) });
  const production = await preview({ ...config, ...reviewConfig({ command: 'serve', mode: 'test' }), preview: { host: '127.0.0.1', port: 0 } });
  servers.push(production);
  assert.equal((await fetch(`http://127.0.0.1:${production.httpServer.address().port}${path}`, request)).status, 404);
  assert.match(await readFile(join(root, 'dist/index.html'), 'utf8'), /Synthetic route fixture/);
  assert.doesNotMatch(await readFile(join(root, 'dist/index.html'), 'utf8'), /startup-overview|\/tiles\/map\//);
});

test('private map tiles revalidate cached bytes and never cache missing files', async t => {
  const { root, assets, servers } = await fixture(t);
  await mkdir(join(root, 'tiles/0'), { recursive: true });
  const file = join(root, 'tiles/0/0_0.png');
  const bytes = Object.values(assets)[0];
  await writeFile(file, bytes);
  const options = reviewConfig({ command: 'serve', mode: 'test' });
  const server = await createServer({
    ...options, root, configFile: false, logLevel: 'silent',
    server: { ...options.server, host: '127.0.0.1', port: 0 },
  });
  servers.push(server);
  await server.listen();
  const url = `http://127.0.0.1:${server.httpServer.address().port}/tiles/map/0/0_0.png`;
  const first = await fetch(url);
  assert.equal(first.status, 200);
  assert.equal(first.headers.get('cache-control'), 'private, max-age=0, must-revalidate');
  const etag = first.headers.get('etag');
  assert.ok(etag);
  assert.deepEqual(Buffer.from(await first.arrayBuffer()), bytes);
  const cached = await fetch(url, { headers: { 'if-none-match': etag } });
  assert.equal(cached.status, 304);
  assert.equal((await cached.arrayBuffer()).byteLength, 0);
  const updated = Buffer.concat([bytes, Buffer.from('changed')]);
  await writeFile(file, updated);
  const changed = await fetch(url, { headers: { 'if-none-match': etag } });
  assert.equal(changed.status, 200);
  assert.notEqual(changed.headers.get('etag'), etag);
  assert.deepEqual(Buffer.from(await changed.arrayBuffer()), updated);
  await rm(file);
  const missing = await fetch(url, { headers: { 'if-none-match': changed.headers.get('etag') } });
  assert.equal(missing.status, 404);
  assert.equal(missing.headers.get('cache-control'), 'private, no-store');
});

for (const enabled of [false, true]) {
  test(`private SQLite is denied through Vite files and transforms (review=${enabled})`, async t => {
    const { root, assets, servers } = await fixture(t, enabled);
    await mkdir(join(root, 'data'));
    await mkdir(join(root, 'public/data'), { recursive: true });
    const privateBytes = Buffer.from('SQLite format 3\0PRIVATE_DATABASE_FIXTURE');
    for (const suffix of ['', '-wal', '-shm', '-journal']) {
      await writeFile(join(root, `data/catalog.sqlite${suffix}`), privateBytes);
    }
    await writeFile(join(root, 'public/data/safe.json'), '{"safe":true}');
    await writeFile(join(root, '.env'), 'PRIVATE_ENV_FIXTURE=1');
    await writeFile(join(root, 'secret.pem'), 'PRIVATE_CERT_FIXTURE');
    await mkdir(join(root, 'tiles'), { recursive: true });
    await writeFile(join(root, 'tiles/manifest.json'), '{"review":true}');
    const options = reviewConfig({ command: 'serve', mode: 'test' });
    const server = await createServer({
      ...options, root, configFile: false, logLevel: 'silent',
      server: { ...options.server, host: '127.0.0.1', port: 0 },
    });
    servers.push(server);
    await server.listen();
    const base = `http://127.0.0.1:${server.httpServer.address().port}`;
    const paths = [
      '/data/catalog.sqlite', `/@fs/${root}/data/catalog.sqlite`,
      '/data/%63atalog.sqlite', '/%64ata/catalog.sqlite', '/data%2Fcatalog.sqlite',
      `/@fs/${root}/data/%63atalog.sqlite`, `/@fs/${encodeURIComponent(root + '/data/catalog.sqlite')}`,
      '/data%252Fcatalog.sqlite', '/data/catalog%2esqlite',
      '/data/catalog.sqlite?raw', '/data/catalog.sqlite?url', '/data/catalog.sqlite?import',
      `/@fs/${root}/data/catalog.sqlite?raw`,
      '/data/catalog.sqlite-wal', '/data/catalog.sqlite-shm', '/data/catalog.sqlite-journal',
      '/.env', '/secret.pem',
    ];
    for (const path of paths) {
      for (const method of ['GET', 'HEAD']) {
        const response = await fetch(base + path, { method, headers: { accept: 'application/octet-stream' } });
        const bytes = Buffer.from(await response.arrayBuffer());
        assert.ok([403, 404].includes(response.status), `${method} ${path}: ${response.status}`);
        assert.equal(bytes.includes(privateBytes), false, path);
      }
    }
    // Encoded separators are not file routes in Vite. HTML navigation may use
    // the SPA fallback, but it must return the fixture page, never DB content.
    for (const path of ['/data%2Fcatalog.sqlite', '/data%252Fcatalog.sqlite', `/@fs/${encodeURIComponent(root + '/data/catalog.sqlite')}`]) {
      const response = await fetch(base + path, { headers: { accept: 'text/html' } });
      assert.match(response.headers.get('content-type'), /^text\/html/);
      const text = await response.text();
      assert.match(text, /Synthetic route fixture/);
      assert.equal(text.includes('PRIVATE_DATABASE_FIXTURE'), false);
    }
    assert.match(await (await fetch(base + '/')).text(), /Synthetic route fixture/);
    assert.deepEqual(await (await fetch(base + '/data/safe.json')).json(), { safe: true });
    assert.equal((await fetch(base + '/@vite/client')).status, 200);
    const token = await fetch(base + '/__source-tokens/' + encodeURIComponent(Object.keys(assets)[0]), { headers: { accept: 'image/png' } });
    assert.equal(token.status, enabled ? 200 : 404);
    const tiles = await fetch(base + '/tiles/map/manifest.json', { headers: { accept: 'application/json' } });
    assert.equal(tiles.status, enabled ? 200 : 404);
    if (enabled) assert.deepEqual(await tiles.json(), { review: true });
  });
}
