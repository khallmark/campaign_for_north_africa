#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
python3 - <<'PY'
import hashlib
import json
from pathlib import Path
import subprocess
import sys

paths = subprocess.check_output(['git', 'ls-files', '-z']).split(b'\0')[:-1]
attrs = subprocess.check_output(
    ['git', 'check-attr', '-z', '--stdin', 'filter'], input=b'\0'.join(paths) + b'\0'
).split(b'\0')[:-1]
sources = {p.decode() for p in paths if p.startswith(b'docs/CNAv2.1.0/')}
sources.update(attrs[i].decode() for i in range(0, len(attrs), 3) if attrs[i + 2] == b'lfs')
lock = json.loads(Path('docs/sources.lock.json').read_text())['files']
errors = []
if sources != set(lock):
    errors.append('Source inventory changed; review and regenerate docs/sources.lock.json.')
for name in sorted(sources | set(lock)):
    path = Path(name)
    if path.is_symlink() or not path.is_file():
        errors.append(f'{name}: missing source or unexpected symlink')
        continue
    with path.open('rb') as stream:
        if stream.read(128).startswith(b'version https://git-lfs'):
            errors.append(f'{name}: LFS pointer; run git lfs pull')
            continue
        stream.seek(0)
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    if name in lock and (path.stat().st_size != lock[name]['size'] or digest != lock[name]['sha256']):
        errors.append(f'{name}: size/SHA256 differs from source lock')
for error in errors:
    print(error, file=sys.stderr)
if errors:
    sys.exit(1)
print(f'Source check: {len(sources)} files match locked size/SHA256; no LFS pointers.')
PY
