import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

test('Pages accepts release assets and rejects private files, renamed payloads and links', async t => {
  const root = await mkdtemp(join(tmpdir(), 'cna-pages-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  for (const dir of ['wasm', 'art', 'data']) await mkdir(join(root, dir));
  for (const [file, bytes] of Object.entries({
    'index.html': '<html></html>', 'wasm/cna_bg.wasm': '\0asm',
    'deployment.json': '{"sha":"test"}', 'art/ground.webp': 'generated-art',
  })) await writeFile(join(root, file), bytes);
  const check = () => spawnSync('python3', ['scripts/check-pages-bundle.py', root], { encoding: 'utf8' });
  assert.equal(check().status, 0);
  for (const [file, bytes] of [
    ['data/catalog.sqlite', 'db'], ['data/rules.pdf', 'pdf'], ['.env', 'secret'],
    ['data/renamed.json', 'SQLite format 3\0db'], ['art/renamed.webp', '%PDF-document'],
    ['data/key.json', '-----BEGIN PRIVATE KEY-----'],
    ['data/token.json', `ghp_${'x'.repeat(36)}`],
    ['data/lfs.json', 'version https://git-lfs.github.com/spec/v1\noid sha256:test'],
  ]) {
    await writeFile(join(root, file), bytes);
    assert.notEqual(check().status, 0, file);
    await rm(join(root, file));
  }
  for (const dir of ['tiles', '__source-tokens', 'docs', 'data/.private', 'art/tiles']) {
    await mkdir(join(root, dir));
    assert.notEqual(check().status, 0, dir);
    await rm(join(root, dir), { recursive: true });
  }
  await symlink(join(root, 'deployment.json'), join(root, 'data/link.json'));
  assert.notEqual(check().status, 0);
  await rm(join(root, 'data/link.json'));
  assert.equal(check().status, 0);
});
