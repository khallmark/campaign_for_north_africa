import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFile, mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

async function repository(t) {
  const root = await mkdtemp(join(tmpdir(), 'cna-guards-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, 'scripts'));
  for (const script of ['check-sources.sh', 'check-no-credentials.sh']) {
    await copyFile(new URL(`../scripts/${script}`, import.meta.url), join(root, 'scripts', script));
  }
  const git = (...args) => execFileSync('git', args, { cwd: root, stdio: 'pipe' });
  git('init', '--quiet');
  const check = name => spawnSync('bash', [`scripts/check-${name}.sh`], { cwd: root, encoding: 'utf8' });
  return { root, git, check };
}

test('credential guard rejects chunk-boundary secrets without logging values, including sparse files', async t => {
  const { root, git, check } = await repository(t);
  const path = join(root, 'fixture.txt');
  await writeFile(path, 'ordinary source');
  git('add', 'fixture.txt');
  assert.equal(check('no-credentials').status, 0);
  const secret = 'synthetic-do-not-log';
  // Split a credential key across the scanner chunk boundary.
  await writeFile(path, 'x'.repeat(65531) + '\npass' + 'word=' + secret);
  const rejected = check('no-credentials');
  assert.equal(rejected.status, 1);
  assert.match(rejected.stderr, /fixture.txt/);
  assert.ok(!rejected.stderr.includes(secret));
  git('add', 'fixture.txt');
  git('update-index', '--skip-worktree', 'fixture.txt');
  await rm(path);
  const sparse = check('no-credentials');
  assert.equal(sparse.status, 1, 'sparse exclusion must not bypass the credential scan');
  assert.ok(!sparse.stderr.includes(secret));
});

test('credential guard accepts clean sparse blobs but rejects missing files and symlinks', async t => {
  const { root, git, check } = await repository(t);
  const path = join(root, 'fixture.txt');
  await writeFile(path, 'ordinary source');
  git('add', 'fixture.txt');
  git('update-index', '--skip-worktree', 'fixture.txt');
  await rm(path);
  assert.equal(check('no-credentials').status, 0);
  git('update-index', '--no-skip-worktree', 'fixture.txt');
  assert.equal(check('no-credentials').status, 1);
  await symlink('outside', path);
  assert.equal(check('no-credentials').status, 1);
});

test('source guard checks inventory, hashes, missing sources and LFS hydration', async t => {
  const { root, git, check } = await repository(t);
  const name = 'docs/CNAv2.1.0/synthetic.txt';
  await mkdir(join(root, 'docs/CNAv2.1.0'), { recursive: true });
  const bytes = Buffer.from('synthetic source, no original artwork');
  await writeFile(join(root, name), bytes);
  await writeFile(join(root, 'docs/sources.lock.json'), JSON.stringify({ files: {
    [name]: { size: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') },
  } }));
  git('add', 'docs');
  assert.equal(check('sources').status, 0);
  await writeFile(join(root, name), Buffer.alloc(bytes.length, 120));
  assert.match(check('sources').stderr, /size\/SHA256 differs/);
  await rm(join(root, name));
  assert.match(check('sources').stderr, /missing source/);
  await writeFile(join(root, name), 'version https://git-lfs.github.com/spec/v1\n');
  assert.match(check('sources').stderr, /LFS pointer; run git lfs pull/);
  await writeFile(join(root, name), bytes);
  await writeFile(join(root, 'docs/CNAv2.1.0/extra.txt'), 'new source');
  git('add', 'docs/CNAv2.1.0/extra.txt');
  assert.match(check('sources').stderr, /Source inventory changed/);
});
