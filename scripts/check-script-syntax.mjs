import { readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

// Parse authored scripts without importing them, writing bytecode or running
// importers. Rust semantic lint and formatting remain Clippy/rustfmt's job.
const files = ['vite.config.js', '.agents/setup',
  ...['scripts', 'tests'].flatMap(dir => readdirSync(dir).map(name => `${dir}/${name}`))];
for (const file of files) {
  let command;
  if (/\.(mjs|js)$/.test(file)) command = ['node', '--check', file];
  else if (file.endsWith('.sh') || file === '.agents/setup') command = ['bash', '-n', file];
  else if (file.endsWith('.py')) command = ['python3', '-c',
    'import ast, pathlib, sys; ast.parse(pathlib.Path(sys.argv[1]).read_text(), filename=sys.argv[1])', file];
  else continue;
  const result = spawnSync(command[0], command.slice(1), { stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
console.log('Script syntax checks passed (JavaScript, Bash, Python).');
