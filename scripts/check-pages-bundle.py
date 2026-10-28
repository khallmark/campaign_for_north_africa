"""Fail closed on non-release files before uploading dist to public Pages.

Run alongside map-census --release-check, which rejects locked source artwork
even when renamed. Pages never includes private review tiles, even with a grant.
"""
from pathlib import Path
import re
import sys

root = Path(sys.argv[1])
allowed = {
    "assets": {".js", ".css"},
    "art": {".png", ".webp", ".json"},
    "data": {".json"},
    "sprites": {".png"},
    "wasm": {".js", ".wasm", ".ts"},
}
secrets = re.compile(
    rb"-----BEGIN (?:[A-Z ]*PRIVATE KEY|OPENSSH PRIVATE KEY)-----"
    rb"|(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})"
    rb"|AKIA[0-9A-Z]{16}"
)
count = 0
for path in sorted(root.rglob("*")):
    relative = path.relative_to(root)
    if path.is_symlink():
        sys.exit(f"Pages rejects symlinks: {relative}")
    if any(part in {"tiles", "__source-tokens", "CNAv2.1.0"} for part in relative.parts):
        sys.exit(f"Pages rejects private review path: {relative}")
    if path.is_dir():
        if relative.parts[0] not in allowed or any(p.startswith('.') for p in relative.parts):
            sys.exit(f"Pages rejects directory: {relative}")
        continue
    if not path.is_file() or path.stat().st_nlink != 1:
        sys.exit(f"Pages requires ordinary, unlinked files: {relative}")
    if any(p.startswith('.') for p in relative.parts) or not (
        str(relative) in {"index.html", "favicon.svg", "deployment.json"}
        or (len(relative.parts) > 1 and path.suffix in allowed.get(relative.parts[0], set()))
    ):
        sys.exit(f"Pages rejects file: {relative}")
    payload = path.read_bytes()
    if payload.startswith((b"SQLite format 3\0", b"%PDF-", b"version https://git-lfs.github.com/spec/v1")):
        sys.exit(f"Pages rejects source/database payload: {relative}")
    if secrets.search(payload):
        sys.exit(f"Pages rejects credential marker (content withheld): {relative}")
    count += 1
for required in ("index.html", "wasm/cna_bg.wasm", "deployment.json"):
    if not (root / required).is_file():
        sys.exit(f"Pages bundle is incomplete: missing {required}")
print(f"Pages bundle check passed: {count} release files; no private paths, databases, PDFs, links or credential markers.")
