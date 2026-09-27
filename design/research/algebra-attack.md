# Adversarial review of the CloneX filter/compose algebra

Target: `clonex.md` gist revision `a2b844ee`.
Method: hand-built counterexamples, plus a small executable model
(`algebra/model.py`) with random search (`algebra/search.py`,
`algebra/search2.py`) and named scenarios (`algebra/scenarios.py`). The
`hypothesis` package is not installed, so a seeded random generator of ≤7-commit
DAGs with merges, deletions, empty commits and a root-level `README` is used
instead. Run `python3 search2.py 8000 keepmerges` to reproduce.

Notation: `x ─→ y` means x is a parent of y. `[E]` is the CX event on a commit.
`F1/`, `F2/` are component paths, `ROOT` is the files the parent owns.

---

## 0. Definitions the gist leaves open, made precise

A history `H` is a finite DAG of commits `c = (parents, tree, event)`. A tree is
a map from path to blob.

**filter_P(H)**, processed in topological order:

1. `t(c) = restrict(tree(c), P)` with the prefix `P/` stripped.
2. `cand(c)` = the union over the parents `p` of `near(p)`, reduced to its
   maximal elements (transitive reduction).
3. Survival rule. The gist says only "removes commits with no remaining effect".
   There are three plausible readings, and they behave differently (see F2):
   - **R-all**: drop `c` if `t(c)` equals the tree of *some* reconnected parent.
   - **R-first**: drop `c` if `t(c)` equals the first reconnected parent's tree.
   - **R-keepmerges**: keep `c` if `|cand(c)| ≥ 2`; otherwise drop it if its
     tree equals its only parent's. A root survives if its tree is non-empty.
4. If `c` survives, `near(c) = {c'}`, where `c'` is the new commit
   `(cand(c), t(c), event(c))`. Otherwise `near(c) = cand(c)`.

**compose(D_1..D_k)**, with one occurrence per component and no split events:

- The event set is the union. `pred(e)` is the union of `event(parent)` over
  every manifestation of `e`. The result must be acyclic.
- A composed commit for `e` has as parents the transitive reduction of `pred(e)`.
- Its tree places, for each component `i`, either the manifestation of `e` in
  `D_i`, or the manifestation in `D_i` of the unique maximal ancestor event of
  `e` that has one. If that maximum is not unique, the tree is **undefined**.

This is the most permissive definition consistent with the gist. Any
alternative has to add inputs, and those extra inputs are what this review is
about.

---

## Findings

### F1. compose is not the inverse of filter: independent events become concurrent. The gist's own example has no current state

Counterexample (automated, minimal):
```
F:        E0 {F1/b=2} ─→ E1 {F1/b=2, F2/b=0}      (E1 touches only F2)
filter_F1 = E0          filter_F2 = E1 (root)
compose  = E0   E1      two roots, two heads
```
The composed `E1` tree lacks `F1/b`. No composed commit contains both changes.

The gist's own example does the same. Composing `F1 = α→χ→γ` with
`F2 = β→χ→δ` gives two heads, `γ` (with `F2` at χ) and `δ` (with `F1` at χ)
(`scenarios.py` S1). No commit's tree contains both γ and δ, so the composed
`F` has no state a `main` ref could name.

compose computes the causal order: the transitive closure of the per-component
orders. That is Mazurkiewicz-trace style. Everything filter throws away is lost:
- the order between events that share no component. In `F` that was α then β,
  or γ then δ;
- the tree of `F` at each commit, outside the components that commit touched;
- merge commits whose effect was nil in every component;
- empty commits, and commits that touch only paths outside every component;
- per-representation messages, authors and dates, once fusion has to choose one.

Measured over 6000 random `F` histories, `compose∘filter = F` failed in 83% of
them, even with `ROOT` included as a component.
Breaks: "Composition performs the inverse operation", and "compose … with
history [the F picture]".
Severity: **requires extra stored information**, or a redefinition. "Inverse"
has to become "inverse up to causal weakening", or `F` has to be kept as its own
history.
Minimal repair: compose adds deterministic *join* commits with no event over
concurrent heads. Filtering drops these automatically under R-keepmerges,
because a join reduces to one parent in each component. For fidelity to one
particular `F`, you need `F`'s own residual history, meaning its order edges
and its root files. `F` is then not free to be reconstructed "with no
historical F".

### F2. Filtering composes only under one of the three survival rules

Counterexample for R-all (automated, reduced):
```
E0 {F1/sub/c=0}
E0 ─→ E1 {sub/c deleted, F2/a=0}
E0 ─→ E2 {F1/b=2}                      (sub/c=0 unchanged)
E1,E2 ─→ E4 (merge, restores sub/c=0, F1 tree == E2's F1 tree)
```
`filter_F1` drops `E4`, because it equals parent `E2`. `filter_sub` of that
result therefore has no `E4`. `filter_F1/sub` applied directly keeps `E4`:
`E2` collapses into `E0 < E1`, so `E4`'s only parent is `E1`, and against `E1`
it restores `c`. The two results differ. R-first fails the same way.
Measured over 6000 random histories: R-all failed 10 times, R-first 7 times,
R-keepmerges 0 times.
Breaks: "a file has a filtered history DAG; directories recursively compose the
DAGs beneath them". That claim needs `filter_q ∘ filter_p = filter_{p/q}`.
Severity: **requires surfacing an ambiguity**. Specify R-keepmerges, which
matches git-filter-repo's treatment of merges.

### F3. filter drops causal facts that compose later needs, so compose∘filter can be undefined even under R-keepmerges

Counterexample (automated, 1 in 8000):
```
E0 {F1/b=1}
E0 ─→ E2 {F1/b=2, F2/a=2}
E0 ─→ E1 {F2/a=0, README=0}
E2 ─→ E3 {F1/b=1, F1/sub/c=2, README=2}
E1,E2 ─→ E4  merge: takes F2/a=2 from E2, README=0 from E1
E3,E4 ─→ E5  merge: README=2
```
In `filter_ROOT`, `E4` is empty, so the link from `E5` to `E4` is cut and
`E5 = merge(E1, E3)`. In the composed DAG, `E5`'s ancestors include the `F2`
events `E1` and `E2`, which are concurrent. The event that resolved them, `E4`,
is not an ancestor of `E5` there. The `F2` state at `E5` is therefore
undefined. In `F` it was `a=2`.

This is not exotic. A criss-cross merge whose resolution in one component is
recorded only by a merge that is empty in another component produces it.
Breaks: "F's own history already … determines its state at every revision" as
seen *after* filtering, and the claim that compose is deterministic.
Severity: **requires extra stored information**.
Minimal repair: each event representation records its causal frontier across
components, for example `CX-Base: <component-or-occurrence> <event>` for
components it did not touch. Alternatively, reconnect through events rather than
through the component. Either way this is a per-commit statement of which
revision each other component was at. That is the submodule pointer the gist
disclaims, moved into commit metadata.

### F4. Concurrent cross-component events can be linearised in opposite orders, giving a cycle or spurious ancestry

Case (a), cycle (`scenarios.py` S2). Alice and Bob each make a cross-component
event, `E` and `E'`. `F1`'s remote receives `E` first, and Bob rebases `E'` onto
it. `F2`'s remote receives `E'` first, and Alice rebases `E` onto it.
```
F1: r1 ─→ E ─→ E'        F2: r2 ─→ E' ─→ E
compose: cycle through E
```
Case (b), no cycle but still wrong (S3). `F1` merges `E ∥ E'` instead of
rebasing. `F2` rebases to `E' → E`. compose succeeds, but the composed `E` has
`E'` as an ancestor while its `F1` slice is `E`'s own `F1` tree, which lacks
`E'`'s `F1` change. The composed history shows `E` silently reverting `E'` in
`F1`. `filter_F1(compose)` adds the edge `E' → E`, which `F1` never had. The
same shape turns up in random search: `filter∘compose ≠ id` in 400 of 5991
random event-sharing input pairs, and 9 were cyclic.

Breaks: "changes may be contributed from any repository … and
deterministically appear in other compatible compositions". "Compatible" is
undefined, and the ordinary rebase workflow on two remotes produces
incompatible histories.
Severity: **fatal to the claimed "any repository, any time" contribution
model** as stated. It becomes a hard precondition that needs coordination.
Minimal repair: an order-consistency check at push and fetch time. The order
restricted to shared events must agree across components, so a cross-component
event must be rebased atomically across all the components it touches. compose
must reject violations with a diagnostic, and must never pick an order. This is
a distributed-consensus requirement, so the model needs a policy for it, such
as "the first component to accept an event fixes its predecessors", or
cross-component merge events that are required to be the same event
everywhere.

### F5. compose is not monotone: learning more rewrites composed history

Case (S5). `F1 = a→E→g`, and `F2 = b` has not yet received `E`.
- Composed now: `E` has no `F2` parent and no `F2` change. It is "half
  present".
- After `F2` fetches `E`, composed `E` becomes a merge of `a` and `b`, and every
  descendant gets a new Git ID (`g: 3800bd09 → e405bfd2`).

Any composed repository that someone has already pushed or cloned is then
non-fast-forwarded.
Breaks: determinism "from the histories", and the implicit claim that Git
commit IDs in composed repositories are stable.
Severity: **requires extra stored information**.
Minimal repair: each representation carries an *event manifest*, the set of
components or occurrences the event touches, ideally with a content digest per
manifestation. compose then refuses to materialise, or blocks at, an event whose
manifest is incomplete. Without a manifest, compose cannot tell "E only touched
F1" apart from "E's F2 part hasn't arrived".

### F6. compose needs inputs the gist says do not exist

compose, as defined in section 0, only works when each component occurs once,
at a fixed path, and always sits at its newest revision. A real `M` needs:

| Input | Why | Counterexample |
|---|---|---|
| placement: occurrence path → component, per revision | paths belong to the parent, and renames happen | `F1/` renamed to `lib/F1/`: filtering at either path cuts the history in two |
| a cut per occurrence (which revisions are included) | `M` may pin `old/` at `b` while `F1` advances to `d` | composing `F1 = a─b─c─d` alone puts both occurrences at `d` |
| which manifestation goes to which occurrence | split events | `F1` has `x[E]` off `b` and `y[E]` off `d`; which one is `old/`? |
| a canonical metadata choice for fused commits | the messages or authors on the `F1` and `F2` copies of `χ` may differ | amend `χ`'s message in `F1` only |
| a residual (parent-owned) history | `README` edits in the same event as `F1` | F10 |

The first three together are, per revision, "occurrence P = component X at
commit Y". That is the pointer model. It is already derivable from `M`'s own
trees when `M` itself is available, but not from the components. So the claim
"No historical F needs to have existed" holds only for the degenerate case
(single occurrences, all at head, fixed paths, no parent-owned files, a
consistent order). The claim "only additional information required is optional
transport configuration" is false.
Severity: **requires extra stored information**, and it challenges the "no
pointers" thesis directly. A minimal honest statement: pointers are *derivable
from `M`'s content history*, and are *required input* when composing `M` from
components.

### F7. Repeated occurrences: "one logical F1 DAG" needs an occurrence-identity map that is not stored anywhere

Take `M` with `old/ = F1@b` and `new/ = F1@d`.
- `filter_old(M)` and `filter_new(M)` are each well defined. The gist's picture,
  with `x[E]` under `b` and `y[E]` under `d`, is the **union** of the two,
  fused where manifestations are identical. That requires knowing that `old/`
  and `new/` are the same component. A shared event does not establish this:
  `filter_F1(F)` and `filter_F2(F)` share `χ` but are different components.
  Tree-hash equality does not establish it either: `old/` may have been created
  by `cp -r`, and a tree can match a revert as well as the original. The
  occurrence set is therefore semantic configuration. If it lives out of band,
  two clones of `M` with different configuration derive different `F1`
  histories.
- The parents in the picture exist only if `M`'s history contains `a` and `b`
  as commits touching `old/`. If `old/` was vendored in a single commit `V`,
  then `filter_old(M) = V → x[E]`, and `x`'s parent is `V`, not `b`. Placing
  it under `b` needs a recorded "V ≡ F1@b".
- **Same revision, same edit.** `old/` and `new/` are both at `b`, and `E`
  applies the same diff to both. The two manifestations have the same parent,
  tree and metadata, so they are one `F1` commit. That is fine, but only
  because the metadata is copied byte for byte.
- **Same revision, different edits.** You get two sibling `F1` commits with the
  same event and the same parent. `F1` gets an extra head with no ref.
  Transport has to invent refs for it, or it is garbage-collected, after which
  `filter(M)` is permanently "ahead" of the `F1` remote. Recomposing into any
  repository with a single `F1` occurrence fails (the S4 error).
- **An ordinary Git user edits one of two identical occurrences** (commit `U`,
  touching `new/` only). Filtering is fine. Recomposing `M` from `F1` cannot
  tell which occurrence got `U`.
- **Advancing a pin.** `M` moves `old/` from `x` (that is, `b` plus `E`) to
  "`d` plus `E`". In `M` this is one commit `V`. `filter_old(M)` gives `V`
  with parent `x`, because `c` and `d` never touched `old/`, and a tree that
  contains `c` and `d`. `V` is effectively a squash of `c` and `d` rather than
  a merge of `x` and `d`. Later merges in `F1` duplicate or conflict on `c` and
  `d`. The alternative, replaying `c` and `d` onto `old/` as `M` commits
  carrying events `c` and `d`, puts the same event on two nodes of `M`'s own
  DAG, in the order `c_new < E < c_old`. Fusion then produces a cycle.

Breaks: "Repeated histories are shared", "There is one logical F1 DAG", and
"no global … identity required".
Severity: **requires extra stored information**. That means an occurrence map
that is versioned *in* the history, not in `.git`, plus per-occurrence base
records. Advancing a pin needs an explicit `CX-Base: old/ F1:<d-event>`, so
that filtering can emit `V` as a merge with parents `x` and `d`.

### F8. Event identity under ordinary Git editing

The trailer lives in the commit message, so Git copies it in exactly the wrong
places.

- **cherry-pick** copies `CX-Event: E`. `F1` then has `E` on two branches
  (`a─E` and `a─f─E'`). compose with `F2`, which holds `E` once, errors
  (`scenarios.py` S4: "event E split within F1"). In a repository with two
  occurrences it would instead be silently read as a split and *fused*, which
  joins a feature branch's copy with the original. This is wrong fusion that
  nothing detects.
- **rebase or amend** also copy the trailer. The old commit survives in other
  clones, and in any `M` that already composed it. After fetching, `F1` holds
  `E@p` and `E@q`, which is indistinguishable from a split event.
- **squash** merges two events into one commit, whose message then carries two
  trailers. Say `F1` has `S{E1,E2}` and `F2` has `E1 → G → E2`. `S` must come
  after `G`, via `E2`, and before `G`, via `E1`: a cycle. Even without a `G`,
  the composed commit fuses `E1` and `E2`, and `filter_F2(compose)` loses
  `F2`'s intermediate `E1` state.
- **split** (one commit becomes two): either both halves carry `E`, which is a
  false split, or only one does, and the other gets an unrelated fresh event
  that breaks atomicity with `F2`'s `E`.
- **revert in one component only** gives a new event. That is harmless,
  though it means an "atomic" event has been partially undone.
- **eventless commits** made by users without CloneX: the event ID must be
  derived, for example as the hash of the commit. When that commit is routed
  through `M` and filtered back, the trailer has to be *stripped*, or `F1` sees
  a different commit for the same event. The rule is to strip it and check that
  the result hashes to the trailer value.

Breaks: "the event ID identifies that these changes happened together", and
fusion by ID.
Severity: **requires extra stored information**. At minimum, identity has to
bind to content. For example, `E = H(nonce ‖ manifest)`, where the manifest
lists each (occurrence, base tree, result tree), so a copy made by cherry-pick
or rebase is detectably a *different* representation. Tooling must mint new IDs
on cherry-pick, rebase and split, and must refuse multi-event commits unless
the other components agree. A "same event, different content" state must
surface as a conflict, never as a split.

### F9. Trust

The ID is opaque and unauthenticated. Anyone who can push to `F1` can write
`CX-Event: E`, where `E` is a reviewed cross-component event in `F2`. Every
composition then fuses the attacker's change into the reviewed atomic commit,
under the reviewed commit's metadata. An attacker can also:
- create cycles or false splits deliberately, to deny composition;
- supersede a legitimate `E` by publishing it first under a different
  predecessor (see F4).

Random-ID collisions are negligible. Deliberate reuse costs nothing.
Severity: **requires extra stored information**. That means a manifest
commitment (F8) plus signatures, or a policy that an event's representation in
component `X` is accepted only from `X`'s authority. Composition must verify
every manifestation against the event's manifest.

### F10. Parent-owned files

`F = {README, F1/, F2/}`, with event `K` changing `README` and `F1/x`.
`filter_F1` keeps only the `F1` half. `compose(filter_F1, filter_F2)` has no
`README` at all. With the residual `ROOT` treated as a component, the
round-trip laws hold as far as they hold anywhere, which random search
confirmed. But `ROOT` is `F`'s own history, and it has no standalone meaning.
Severity: **requires surfacing an ambiguity**. The model should say that the
complement of the components is itself a component that belongs to the parent,
and that `F` is recoverable only together with it.

### F11. Smaller issues (cosmetic, or a rule is needed)

- A component path that is a file, or that is deleted: the filtered root is a
  blob or empty. Git needs a tree root, so define the empty tree, and reject
  files as components.
- Git cannot represent an empty directory, so a component that becomes empty
  vanishes from the parent's tree. Its placement is then lost unless it is
  recorded.
- Determinism of Git IDs across implementations needs a canonical filtered
  commit serialisation: the message with its trailers, author, committer,
  dates, and encoding.
- The gist's picture of `F` shows `γ` and `δ` as a fork. That is presumably a
  causal diagram rather than a Git history anyone would have (see F1).

---

## Laws that DO hold

These were checked by the executable model over thousands of random histories
(≤7 commits, with merges, deletions and empty commits) under R-keepmerges.
Preconditions are stated explicitly.

1. **filter is idempotent.** `filter_P(filter_P(H)) = filter_P(H)`, when both
   passes keep paths. Holds under all three survival rules (0 failures in
   18,000 checks).
2. **Nested commutation.** `filter_q(filter_p(H)) = filter_{p/q}(H)`.
   *Precondition*: R-keepmerges (0 failures in 6000). It fails under R-all and
   R-first (F2).
3. **Ancestry is preserved and reflected.** For surviving events `e1` and
   `e2`, `e1 < e2` in `filter_P(H)` iff `e1 < e2` in `H`. This holds through
   merges and under every rule, because reconnection is by reachability
   (0 failures in 6000). Note that it says nothing about ancestry through
   *dropped* commits (F3).
4. **Slice law.** If `K = compose(filter_{P_i}(H))` is defined, then for every
   component `i` and every event `e` with a manifestation in component `i`,
   `restrict(tree(K[e]), P_i) = tree(filter_{P_i}(H)[e])` (0 failures in 8000).
5. **compose∘filter only weakens.** If defined, `order(K) ⊆ order(H)`
   (0 failures in 8000). *If* in addition no commit of `H` is dropped by every
   filter, and `order(K) = order(H)`, then every event's tree is equal
   (0 failures in 1865 qualifying cases). Informally, `compose∘filter` is the
   identity exactly on histories that are already their own causal reduction.
6. **filter∘compose = id on consistent inputs.** If the `D_i` are
   `filter_{P_i}(H)` for a single `H` and a partition `{P_i}` (including
   `ROOT`) under R-keepmerges, and compose is defined, then
   `filter_{P_i}(compose(D)) = D_i` (0 failures in 8000). For arbitrary
   event-sharing `D_i` it fails (F4). The general precondition is *order
   consistency*: the transitive closure of the union of the component orders,
   restricted to the events of each `D_i`, equals `D_i`'s own order. The `D_i`
   must also be filter-normal: transitively reduced, with no empty non-merge
   commits and no event repeated inside one `D_i`.
7. **compose is a function of the set of inputs.** When it is defined it does
   not depend on input order. *Precondition*: a canonical metadata rule for
   fused commits.
8. **compose is defined** iff the union order is acyclic, no event is repeated
   within a component (the single-occurrence case), and for every event, the
   latest manifestation of each untouched component among its ancestors is
   unique.

Laws that do **not** hold, even with every precondition above: compose is not
monotone in its inputs (F5). compose is not total on histories that
independent contributors could produce (F4, F8). `compose∘filter` is not the
identity (F1). And one `F1` DAG over several occurrences cannot be derived from
Git content alone (F7).
