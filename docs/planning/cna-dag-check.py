#!/usr/bin/env python3
"""Validate the four CNA plans and docs/planning/cna-dag.manifest.json as one global DAG.

Checks: required fields and S/M leaf sizes; local and cross-project keys exist; cross-project edges
match each leaf's scope text; acyclicity including parent/child completion and inherited container
blockers; stage order (no backward milestone or cross-project edges); single file ownership; and that
verification commands only use harnesses that exist or are produced by a prerequisite.

Usage: python3 docs/planning/cna-dag-check.py [--manifest PATH] [--json]
Exit status 0 when every check passes, 1 otherwise.
"""
import argparse, fnmatch, json, os, re, sys
from collections import defaultdict, deque

ap = argparse.ArgumentParser()
ap.add_argument("--manifest", default="docs/planning/cna-dag.manifest.json")
ap.add_argument("--json", action="store_true")
args = ap.parse_args()
root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.chdir(root)
M = json.load(open(args.manifest))
errors, nodes, kind, parent, epic_of = [], {}, {}, {}, {}
deps = defaultdict(set)          # node -> nodes it depends on
stage = {}

def err(msg):
    errors.append(msg)

LEAF_FIELDS = ["title", "goal", "scope", "acceptance_criteria", "verification", "size"]
for p in M["projects"]:
    slug = p["slug"]
    plan = json.load(open(p["plan"]))
    if plan["meta"]["slug"] != slug:
        err(f"{p['plan']}: meta.slug {plan['meta']['slug']} != {slug}")
    for e in plan["epics"]:
        if e.get("removed"):
            continue
        ek = f"{slug}:{e['key']}"
        nodes[ek], kind[ek], epic_of[ek] = e, "epic", ek
        if e["key"] not in p["epics"]:
            err(f"{ek}: missing from manifest stages")
        else:
            stage[ek] = p["epics"][e["key"]]["stage"]
        if not e.get("issues"):
            err(f"{ek}: epic has no issues")
        for i in e.get("issues", []):
            if i.get("removed"):
                continue
            ik = f"{slug}:{i['key']}"
            nodes[ik], kind[ik], parent[ik], epic_of[ik] = i, "issue", ek, ek
            if not i["key"].startswith(e["key"] + "."):
                err(f"{ik}: not under {ek}")
            if not i.get("subissues"):
                err(f"{ik}: issue has no subissues")
            for l in i.get("subissues", []):
                if l.get("removed"):
                    continue
                lk = f"{slug}:{l['key']}"
                if lk in nodes:
                    err(f"{lk}: duplicate key")
                nodes[lk], kind[lk], parent[lk], epic_of[lk] = l, "leaf", ik, ek
                if not l["key"].startswith(i["key"] + "."):
                    err(f"{lk}: not under {ik}")
                for f_ in LEAF_FIELDS:
                    if not l.get(f_):
                        err(f"{lk}: empty {f_}")
                if l.get("size") not in ("S", "M"):
                    err(f"{lk}: size {l.get('size')} is not S/M")
                if "non_goals" not in l:
                    err(f"{lk}: missing non_goals")

def slug_of(n):
    return n.split(":")[0]

# Local blocked_by edges and container completion edges.
for n, obj in nodes.items():
    s = slug_of(n)
    for b in obj.get("blocked_by", []):
        t = f"{s}:{b}"
        if t not in nodes:
            err(f"{n}: unknown blocked_by {b}")
        else:
            deps[n].add(t)
    if n in parent:
        deps[parent[n]].add(n)    # container completes after child

# Cross-project edges.
xmap = defaultdict(set)
for x in M["cross_project_edges"]:
    c, p_ = x["consumer"], x["producer"]
    for k in (c, p_):
        if k not in nodes:
            err(f"cross edge endpoint missing: {k}")
    if slug_of(c) == slug_of(p_):
        err(f"cross edge within one project: {c} -> {p_}")
    if c in nodes and p_ in nodes:
        deps[c].add(p_)
        xmap[c].add(p_)
# Scope text must list exactly the manifest cross edges.
for n, obj in nodes.items():
    if kind[n] != "leaf":
        continue
    listed = set()
    for line in obj["scope"]:
        if line.startswith("Cross-project prerequisites"):
            listed |= set(re.findall(r"(cna-[a-z-]+:E\d+\.I\d+\.S\d+)", line))
    if listed != xmap.get(n, set()):
        err(f"{n}: scope cross-project list {sorted(listed)} != manifest {sorted(xmap.get(n, set()))}")

# Inherited start constraints: a node cannot start before its containers' blockers.
def inherited(n):
    out, p_ = set(), parent.get(n)
    while p_:
        out |= {d for d in deps[p_] if parent.get(d) != p_}   # exclude completion edges to own children
        p_ = parent.get(p_)
    return out
start_deps = {n: set(deps[n]) | inherited(n) for n in nodes}

# Cycle check (Kahn) over start_deps.
indeg = {n: 0 for n in nodes}
rev = defaultdict(set)
for n, ds in start_deps.items():
    for d in ds:
        if d in nodes:
            rev[d].add(n)
            indeg[n] += 1
q = deque(sorted(n for n in nodes if indeg[n] == 0))
order = []
while q:
    n = q.popleft()
    order.append(n)
    for m in sorted(rev[n]):
        indeg[m] -= 1
        if indeg[m] == 0:
            q.append(m)
if len(order) != len(nodes):
    stuck = {n for n in nodes if indeg[n] > 0}
    # Walk dependencies inside the stuck set until a node repeats: that is a real cycle.
    path, pos, n = [], {}, min(stuck)
    while n not in pos:
        pos[n] = len(path)
        path.append(n)
        n = min(d for d in start_deps[n] if d in stuck)
    cyc = path[pos[n]:] + [n]
    err(f"cycle: {' -> '.join(cyc)}")

# Stage and milestone order.
def enum(n):
    return int(epic_of[n].split(":")[1][1:])
for n, ds in deps.items():
    for d in ds:
        if d not in nodes or n not in nodes:
            continue
        en, ed = epic_of[n], epic_of[d]
        if en == ed:
            continue
        if stage.get(ed, 0) > stage.get(en, 0):
            err(f"backward stage edge: {n} (stage {stage.get(en)}) depends on {d} (stage {stage.get(ed)})")
        if slug_of(n) == slug_of(d) and enum(d) > enum(n):
            err(f"backward milestone edge: {n} depends on later epic {d}")

# Transitive closure for tool checks.
memo = {}
def closure(n):
    if n in memo:
        return memo[n]
    seen, stack = set(), [n]
    while stack:
        x = stack.pop()
        for d in start_deps.get(x, ()):
            if d not in seen:
                seen.add(d)
                stack.append(d)
    memo[n] = seen
    return seen

def produced_by(n, path):
    """path exists, or n or a prerequisite lists it (or a parent directory) in files_hint."""
    if os.path.exists(path):
        return True
    for k in closure(n) | {n}:
        if kind.get(k) != "leaf":
            continue
        for h in nodes[k].get("files_hint", []):
            if h == path or (h.endswith("/") and path.startswith(h)):
                return True
    return False

TP = [(re.compile(t["pattern"]), t["producer"]) for t in M["tool_prerequisites"]]
for n, obj in nodes.items():
    if kind[n] != "leaf":
        continue
    for cmd in obj["verification"]:
        for rx, prod in TP:
            if rx.search(cmd) and n != prod and prod not in closure(n):
                err(f"{n}: '{cmd}' needs {prod} as a prerequisite")
        crate = re.search(r"-p (cna-[a-z]+)", cmd)
        cdir = {"cna-sim": "crates/cna-sim", "cna-map": "crates/cna-map", "cna-catalog": "crates/cna-catalog", "cna-frontend": "crates/frontend"}.get(crate.group(1)) if crate else None
        for b in re.findall(r"--bin ([\w-]+)", cmd):
            if cdir and not produced_by(n, f"{cdir}/src/bin/{b}.rs"):
                err(f"{n}: bin {b} not produced by self or prerequisite")
        for t in re.findall(r"--test ([\w-]+)", cmd):
            if cdir and not produced_by(n, f"{cdir}/tests/{t}.rs"):
                err(f"{n}: test target {t} not produced by self or prerequisite")
        m = re.search(r"--schemafile (\S+) (\S+)", cmd)
        if m:
            for pth in m.groups():
                if not produced_by(n, pth):
                    err(f"{n}: {pth} not produced by self or prerequisite")

# Ownership.
owners = [(o["owner"], pat) for o in M["ownership"] for pat in o["patterns"]]
exc = {e["key"]: set(e["paths"]) for e in M.get("scaffold_exceptions", [])}
def owner_of(path):
    hits = {o for o, pat in owners if fnmatch.fnmatch(path, pat) or fnmatch.fnmatch(path.rstrip("/") + "/x", pat)}
    return hits
for n, obj in nodes.items():
    if kind[n] != "leaf":
        continue
    for h in obj.get("files_hint", []):
        own = owner_of(h)
        if len(own) > 1:
            err(f"{n}: {h} has multiple owners {sorted(own)}")
        elif not own:
            err(f"{n}: {h} has no owner")
        elif slug_of(n) not in own and h not in exc.get(n, set()):
            err(f"{n}: edits {h} owned by {sorted(own)[0]}")

# Report.
leaves = [n for n in nodes if kind[n] == "leaf"]
depth = {}
for n in order:
    depth[n] = 1 + max((depth[d] for d in start_deps[n] if d in depth), default=0) if kind[n] == "leaf" or True else 0
per = defaultdict(lambda: defaultdict(int))
for n in nodes:
    per[slug_of(n)][kind[n]] += 1
for n in leaves:
    per[slug_of(n)]["size_" + nodes[n]["size"]] += 1
summary = {"ok": not errors, "errors": errors, "counts": {k: dict(v) for k, v in per.items()},
           "leaves": len(leaves), "cross_project_edges": len(M["cross_project_edges"]),
           "longest_chain_nodes": max(depth.values()) if depth else 0}
if args.json:
    print(json.dumps(summary, indent=2))
else:
    for k, v in sorted(summary["counts"].items()):
        print(f"{k}: {v}")
    print(f"leaves={summary['leaves']} cross_project_edges={summary['cross_project_edges']} longest_chain={summary['longest_chain_nodes']}")
    for e in errors:
        print("ERROR", e)
    print("OK" if not errors else f"{len(errors)} error(s)")
sys.exit(0 if not errors else 1)
