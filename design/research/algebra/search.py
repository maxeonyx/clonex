import random, sys
from model import *

rng = random.Random(1)
RULE = sys.argv[2] if len(sys.argv) > 2 else 'all'
N = int(sys.argv[1]) if len(sys.argv) > 1 else 3000
res = {}
def record(law, ok, ex=None):
    r = res.setdefault(law, [0, 0, None])
    r[0] += 1
    if not ok:
        r[1] += 1
        if r[2] is None or len(ex[0]) < len(r[2][0]): r[2] = ex

for it in range(N):
    H = rand_hist(rng, n=rng.randint(2, 7), merge_p=rng.choice([0, 0.3, 0.5]))
    for rule in ['all', 'first', 'keepmerges']:
        f1 = filt(H, 'F1', rule)
        # idempotence (re-filter a filtered history at a prefix that contains everything)
        f1k = filt(H, 'F1', rule, keep_paths=True)
        f1kk = filt(f1k, 'F1', rule, keep_paths=True)
        record('idempotent[%s]' % rule, canon(f1k) == canon(f1kk), (H, f1k, f1kk))
        # nested commutation: filter F1/sub via F1 vs directly
        direct = filt(H, 'F1/sub', rule)
        via = filt(f1, 'sub', rule)
        record('nested-commute[%s]' % rule, canon(direct) == canon(via), (H, direct, via))
        # set-filter commute: {F1,F2} then {F1} vs {F1}
        both = filt_set = None
    # ancestry preservation (reflect + preserve) for surviving events
    f1 = filt(H, 'F1', RULE)
    ordH = event_order(H); ordF = event_order(f1)
    ev = {c.event for c in f1.values()}
    pres = all(((a, b) in ordF) == ((a, b) in ordH) for a in ev for b in ev if a != b)
    record('ancestry-preserved-and-reflected', pres, (H, f1, None))
    # compose . filter = id ?
    parts = {'F1': filt(H, 'F1'), 'F2': filt(H, 'F2'), 'ROOT': None}
    # represent README via a 'ROOT' component: move README under ROOT/ for the test
    H2 = {n: C(c.parents, frozenset((('ROOT/' + p) if '/' not in p else p, v) for p, v in c.tree), c.event) for n, c in H.items()}
    H2 = canon(H2)
    parts = {k: filt(H2, k, RULE) for k in ['F1', 'F2', 'ROOT']}
    try:
        K = compose(parts)
        ok = canon(K) == canon(H2)
        record('compose.filter=id (partition incl. root)', ok, (H2, K, None))
        # weaker: equal up to event order and trees at each event
        kt = {c.event: c.tree for c in K.values()}; ht = {c.event: c.tree for c in H2.values() if any(c.event == d.event for d in K.values())}
        record('compose.filter preserves tree at each surviving event', kt == ht, (H2, K, None))
    except ComposeError as e:
        record('compose.filter defined', False, (H2, str(e), None))
    else:
        record('compose.filter defined', True)
    # filter . compose = id ? build two random component histories with shared events
    A = rand_hist(rng, n=rng.randint(2, 5), paths=['x', 'y'], merge_p=0.2, empty_p=0)
    B = rand_hist(rng, n=rng.randint(2, 5), paths=['x', 'y'], merge_p=0.2, empty_p=0)
    # rename events in B partially to share with A
    evA = [c.event for c in A.values()]
    mp = {}
    Bn = {}
    for n in topo(B):
        pass
    try:
        A = filt({n: C(c.parents, frozenset(('R/' + p, v) for p, v in c.tree), c.event) for n, c in A.items()}, 'R')
        B = filt({n: C(c.parents, frozenset(('R/' + p, v) for p, v in c.tree), 'B' + c.event if rng.random() < 0.5 else c.event) for n, c in B.items()}, 'R')
        K = compose({'F1': A, 'F2': B})
        ok = canon(filt(K, 'F1', RULE)) == canon(A) and canon(filt(K, 'F2', RULE)) == canon(B)
        record('filter.compose=id (when compose defined)', ok, (A, B, K))
    except ComposeError as e:
        record('compose defined on arbitrary event-sharing inputs', False, (A, B, str(e)))
    else:
        record('compose defined on arbitrary event-sharing inputs', True)

for law, (n, bad, ex) in res.items():
    print('%-55s tested=%5d failures=%5d' % (law, n, bad))

for law in [ 'compose.filter preserves tree at each surviving event', 'compose.filter defined', 'filter.compose=id (when compose defined)']:
    ex = res[law][2]
    print('\n=====', law)
    for i, x in enumerate(ex):
        if isinstance(x, dict): print(show(x, 'part%d' % i))
        else: print('part%d' % i, x)
