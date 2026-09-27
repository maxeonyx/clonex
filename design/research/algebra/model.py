"""Tiny executable model of CloneX filter/compose, used to hunt counterexamples.

A history H is dict id -> Commit(parents: tuple, tree: frozenset of (path, content), event: str|None).
Trees are flat: path strings like 'F1/foo'. Commit identity in a filtered history is
derived structurally (event, sorted parent ids, tree) so equal manifestations coincide,
mimicking Git content addressing when metadata is copied verbatim.
"""
import hashlib, random, itertools
from dataclasses import dataclass

@dataclass(frozen=True)
class C:
    parents: tuple
    tree: frozenset
    event: object

def cid(parents, tree, event):
    h = hashlib.sha1(repr((sorted(parents), sorted(tree), event)).encode()).hexdigest()[:8]
    return h

def topo(H):
    seen, out = set(), []
    def visit(n):
        if n in seen: return
        seen.add(n)
        for p in H[n].parents: visit(p)
        out.append(n)
    for n in sorted(H): visit(n)
    return out

def ancestors(H, n):
    st, res = list(H[n].parents), set()
    while st:
        x = st.pop()
        if x in res: continue
        res.add(x); st.extend(H[x].parents)
    return res

def restrict(tree, prefix):
    pre = prefix.rstrip('/') + '/'
    return frozenset((p[len(pre):], c) for p, c in tree if p.startswith(pre))

def restrict_set(tree, prefixes):
    """Filter to a set of prefixes, keeping full paths (used for commutation tests)."""
    return frozenset((p, c) for p, c in tree if any(p.startswith(q.rstrip('/') + '/') for q in prefixes))

def filt(H, prefix, rule='all', keep_paths=False):
    """filter_prefix(H). rule: 'all' = drop iff tree equals SOME reconnected parent's tree
    (i.e. survive only if it differs from all); 'first' = drop iff equals first parent;
    root commits survive iff restricted tree nonempty."""
    order = topo(H)
    newid = {}         # old id -> new id or None
    near = {}          # old id -> frozenset of nearest surviving new ids at-or-below
    out = {}
    for n in order:
        c = H[n]
        t = restrict_set(c.tree, [prefix]) if keep_paths else restrict(c.tree, prefix)
        cand = set()
        for p in c.parents: cand |= near[p]
        # transitive reduction among candidates
        cand = {x for x in cand if not any(x in anc_new(out, y) for y in cand if y != x)}
        ps = tuple(sorted(cand))
        ptrees = [out[p].tree for p in ps]
        if not ps:
            survive = len(t) > 0
        elif rule == 'keepmerges':
            survive = len(ps) >= 2 or t != ptrees[0]
        elif rule == 'all':
            survive = all(t != pt for pt in ptrees)
        else:
            survive = t != ptrees[0]
        if survive:
            i = cid(ps, t, c.event)
            out[i] = C(ps, t, c.event)
            newid[n] = i; near[n] = frozenset([i])
        else:
            newid[n] = None; near[n] = frozenset(cand)
    return out

_anc_cache = {}
def anc_new(H, n):
    return ancestors(H, n)

def canon(H):
    """Canonical form: relabel structurally so that isomorphic histories compare equal."""
    order = topo(H); m = {}; out = {}
    for n in order:
        c = H[n]; ps = tuple(sorted(m[p] for p in c.parents))
        i = cid(ps, c.tree, c.event); m[n] = i; out[i] = C(ps, c.tree, c.event)
    return out

def event_order(H):
    """Set of (e1,e2) with e1 strictly before e2 (by ancestry) among evented commits."""
    rel = set()
    for n in H:
        for a in ancestors(H, n):
            if H[a].event and H[n].event: rel.add((H[a].event, H[n].event))
    return rel

class ComposeError(Exception): pass

def compose(comps):
    """comps: dict prefix -> history (one occurrence per component, no split events).
    Returns composed history, or raises ComposeError. Every commit must carry an event."""
    man = {}   # event -> {prefix: commit id}
    for pre, H in comps.items():
        for n, c in H.items():
            if c.event is None: raise ComposeError('eventless commit %s in %s' % (n, pre))
            d = man.setdefault(c.event, {})
            if pre in d: raise ComposeError('event %s split within %s: needs placement info' % (c.event, pre))
            d[pre] = n
    # event DAG = union of component parent edges
    preds = {e: set() for e in man}
    for pre, H in comps.items():
        for n, c in H.items():
            for p in c.parents: preds[c.event].add(H[p].event)
    # cycle check / topo
    order, state = [], {}
    def visit(e, stack):
        if state.get(e) == 1: raise ComposeError('cycle through %s' % e)
        if state.get(e) == 2: return
        state[e] = 1
        for p in sorted(preds[e]): visit(p, stack)
        state[e] = 2; order.append(e)
    for e in sorted(man): visit(e, [])
    anc = {}
    for e in order:
        a = set()
        for p in preds[e]: a |= anc[p] | {p}
        anc[e] = a
    out, cmap = {}, {}
    for e in order:
        red = {p for p in preds[e] if not any(p in anc[q] for q in preds[e] if q != p)}
        tree = set()
        for pre, H in comps.items():
            if pre in man[e]:
                n = man[e][pre]
            else:
                # latest manifestation among composed ancestors, must be unique maximum
                cands = [x for x in anc[e] if pre in man[x]]
                maxes = [x for x in cands if not any(x in anc[y] for y in cands if y != x)]
                if not maxes: continue
                if len(maxes) > 1:
                    raise ComposeError('state of %s at %s undefined: concurrent %s' % (pre, e, sorted(maxes)))
                n = man[maxes[0]][pre]
            tree |= {(pre + '/' + p, c) for p, c in H[n].tree}
        ps = tuple(sorted(cmap[p] for p in red))
        i = cid(ps, frozenset(tree), e); cmap[e] = i
        out[i] = C(ps, frozenset(tree), e)
    return out

# ---------- builders ----------
def build(spec):
    """spec: list of (name, parents, changes:{path:content|None}, event). Trees accumulate from
    first parent then merged 'theirs' of other parents path-wise for untouched paths (simple)."""
    H, names = {}, {}
    for name, parents, changes, event in spec:
        base = {}
        for p in parents:
            for k, v in H[names[p]].tree: base.setdefault(k, v)
        if parents:
            base = dict(H[names[parents[0]]].tree) | {k: v for p in parents[1:] for k, v in H[names[p]].tree if k not in dict(H[names[parents[0]]].tree)}
        for k, v in changes.items():
            if v is None: base.pop(k, None)
            else: base[k] = v
        ps = tuple(names[p] for p in parents)
        t = frozenset(base.items())
        i = cid(ps, t, event); names[name] = i; H[i] = C(ps, t, event)
    return H, names

def show(H, label=''):
    lines = [label] if label else []
    ev = {n: H[n].event for n in H}
    for n in topo(H):
        c = H[n]
        lines.append('  %s [%s] parents=%s tree=%s' % (n, c.event, [ev[p] for p in c.parents], sorted(c.tree)))
    return '\n'.join(lines)

# ---------- random generation ----------
PATHS = ['F1/a', 'F1/b', 'F1/sub/c', 'F2/a', 'F2/b', 'README']

def rand_hist(rng, n=7, merge_p=0.3, paths=PATHS, empty_p=0.1):
    spec, names = [], []
    for i in range(n):
        if names and rng.random() < merge_p and len(names) >= 2:
            parents = rng.sample(names, 2)
        elif names:
            parents = [rng.choice(names)]
        else:
            parents = []
        ch = {}
        if rng.random() > empty_p:
            for p in rng.sample(paths, rng.randint(1, min(3, len(paths)))):
                ch[p] = str(rng.randint(0, 2)) if rng.random() > 0.15 else None
        nm = 'c%d' % i
        spec.append((nm, parents, ch, 'E%d' % i)); names.append(nm)
    H, _ = build(spec)
    return H
