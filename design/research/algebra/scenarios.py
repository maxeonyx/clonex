from model import *
def comp(spec):  # component history from spec with plain paths
    H, n = build(spec); return H, n
def heads(H):
    ps = {p for c in H.values() for p in c.parents}; return [H[n].event for n in H if n not in ps]
def run(title, comps):
    print('\n###', title)
    try:
        K = compose(comps); print(show(K)); print('  heads:', heads(K)); return K
    except ComposeError as e:
        print('  ComposeError:', e)

# S1: the gist's own example
F1, _ = comp([('α', [], {'x': 'α'}, 'α'), ('χ', ['α'], {'x': 'χ'}, 'χ'), ('γ', ['χ'], {'x': 'γ'}, 'γ')])
F2, _ = comp([('β', [], {'y': 'β'}, 'β'), ('χ', ['β'], {'y': 'χ'}, 'χ'), ('δ', ['χ'], {'y': 'δ'}, 'δ')])
run('S1 gist example compose(F1,F2)', {'F1': F1, 'F2': F2})

# S2: concurrent cross-component events linearised in opposite orders
F1, _ = comp([('r', [], {'x': '0'}, 'r1'), ('E', ['r'], {'x': 'E'}, 'E'), ('E2', ['E'], {'x': "E'"}, "E'")])
F2, _ = comp([('r', [], {'y': '0'}, 'r2'), ('E2', ['r'], {'y': "E'"}, "E'"), ('E', ['E2'], {'y': 'E'}, 'E')])
run('S2 E<E\' in F1, E\'<E in F2', {'F1': F1, 'F2': F2})

# S3: same, but F1 merged instead of rebasing (E || E' then merge m1 with its own event)
F1, _ = comp([('r', [], {'x': '0'}, 'r1'), ('E', ['r'], {'x': 'E', 'x2': '0'}, 'E'), ('E2', ['r'], {'x2': "E'"}, "E'"), ('m', ['E', 'E2'], {}, 'm1')])
F2, _ = comp([('r', [], {'y': '0'}, 'r2'), ('E2', ['r'], {'y': "E'"}, "E'"), ('E', ['E2'], {'y': 'E'}, 'E'), ('z', ['E'], {'y': 'z'}, 'z')])
run('S3 F1 merged (m1), F2 rebased: spurious order leaks into F1', {'F1': F1, 'F2': F2})

# S4: cherry-pick copies CX-Event trailer onto a second F1 commit
F1, _ = comp([('a', [], {'x': 'a'}, 'a'), ('E', ['a'], {'x': 'E'}, 'E'), ('f', ['a'], {'z': 'f'}, 'f'), ('Ep', ['f'], {'x': 'E', 'z': 'f'}, 'E')])
F2, _ = comp([('b', [], {'y': 'b'}, 'b'), ('E', ['b'], {'y': 'E'}, 'E')])
run('S4 cherry-picked E on two F1 branches', {'F1': F1, 'F2': F2})

# S5: partial knowledge: F2 has not yet received E; later it does. Descendant commit ids change.
F1, _ = comp([('a', [], {'x': 'a'}, 'a'), ('E', ['a'], {'x': 'E'}, 'E'), ('g', ['E'], {'x': 'g'}, 'g')])
F2old, _ = comp([('b', [], {'y': 'b'}, 'b')])
F2new, _ = comp([('b', [], {'y': 'b'}, 'b'), ('E', ['b'], {'y': 'E'}, 'E')])
K1 = run('S5a F2 lacks E', {'F1': F1, 'F2': F2old})
K2 = run('S5b F2 has E', {'F1': F1, 'F2': F2new})
g1 = [n for n, c in K1.items() if c.event == 'g']; g2 = [n for n, c in K2.items() if c.event == 'g']
print('  composed id of g changes:', g1, '->', g2)
