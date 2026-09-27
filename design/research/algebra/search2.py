import random, sys
from model import *
rng = random.Random(7); RULE = sys.argv[2] if len(sys.argv) > 2 else 'keepmerges'
N = int(sys.argv[1])
res = {}
def record(law, ok, ex=None):
    r = res.setdefault(law, [0, 0, None]); r[0] += 1
    if not ok:
        r[1] += 1
        if r[2] is None or len(repr(ex)) < len(repr(r[2])): r[2] = ex
COMPS = ['F1', 'F2', 'ROOT']
def touches(H, n, pre):
    c = H[n]; t = restrict(c.tree, pre)
    return any(restrict(H[p].tree, pre) != t for p in c.parents) or (not c.parents and t)
for it in range(N):
    H = rand_hist(rng, n=rng.randint(2, 7), merge_p=rng.choice([0, 0.3, 0.5]))
    H = canon({n: C(c.parents, frozenset((('ROOT/' + p) if '/' not in p else p, v) for p, v in c.tree), c.event) for n, c in H.items()})
    parts = {k: filt(H, k, RULE) for k in COMPS}
    try:
        K = compose(parts)
    except ComposeError as e:
        record('compose.filter defined', False, (H, str(e))); continue
    record('compose.filter defined', True)
    byev = {c.event: n for n, c in K.items()}
    ok = True
    for pre, P in parts.items():
        for n, c in P.items():
            if restrict(K[byev[c.event]].tree, pre) != c.tree: ok = False
    record('component slice of composed commit = component commit', ok, (H, K))
    # causal precondition: every parent edge of H shares a component that both touch
    causal = all(any(touches(H, n, pre) and touches(H, p, pre) for pre in COMPS) for n in H for p in H[n].parents)
    evs = {c.event for c in K.values()}
    if all(c.event in evs for c in H.values()) and event_order(H) == event_order(K):
        kt = {c.event: c.tree for c in K.values()}; ht = {c.event: c.tree for c in H.values()}
        record('compose.filter trees=id | same event order & no commit dropped', kt == ht, (H, K))
    record('compose.filter order subset of original order', event_order(K) <= event_order(H), (H, K))
    # filter.compose round trip on filtered parts, always consistent by construction
    back = {pre: filt(K, pre, RULE) for pre in COMPS}
    record('filter.compose=id on parts that came from one F', all(canon(back[p]) == canon(parts[p]) for p in COMPS), (H, K))
for law, (n, bad, ex) in res.items():
    print('%-60s tested=%5d failures=%5d' % (law, n, bad))
for law, (n, bad, ex) in res.items():
    if bad:
        print('\n=====', law)
        for i, x in enumerate(ex):
            print(show(x, 'part%d' % i) if isinstance(x, dict) else x)
