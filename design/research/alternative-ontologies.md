# CloneX: alternative ontologies, replayed against hard episodes

Status: research note. Independent of the event-ID hypothesis
(`clonex.md` gist, "composable Git histories"), which is evaluated here as
ontology (a) alongside five others.

## 0. Framing

Every CloneX design has to answer four questions. The ontologies below differ
in their answers, and those answers are what change terminal behaviour.

1. **What is the truth of a composition?** Is its history *derived* from
   component histories (a pure function), or *recorded* (its own commits that
   refer to component states)?
2. **What makes two commits in different repos "the same change"?** An
   assigned identity (event ID, change ID, patch hash), or content (trees)
   plus ancestry?
3. **Is that identity correctness-bearing or UX-bearing?** If it is lost
   (squash merge, UI cherry-pick, force-push by some other tool), does sync
   become wrong, or merely less informative?
4. **Where does the change graph live?** In each Git repo, in a shared hidden
   store, or in one local client?

Notation used in DAGs:

```text
t0, t1 ...   trunc commits          d0, d1 ...  dotsync commits
u0, u1 ...   umbrella U commits     v0, v1 ...  composition V commits
h0 ...       agent-harness commits  [E]         carries event/change id E
```

U = `{tools/trunc, tools/dotsync, crates/ (U-owned), ...}`.
V = `{dotsync, agent-harness}` (paths chosen by V, e.g. `dotsync/`, `harness/`).

---

## 1. The ontologies

### (a) Event-ID sub-history algebra (the existing hypothesis)

- Truth: each repo's Git history. A composition's history is, in principle,
  the *composition* of its components' filtered sub-DAGs plus its own events;
  filter and compose are inverses.
- Identity: opaque `CX-Event: E` trailer on each commit; an event may be split
  into several commits in one repo and fused into one commit in another.
- Identity is **correctness-bearing**: composition fuses commits by event ID;
  a commit without an ID is an orphan event.
- No pointers: the composition carries the histories.

### (b) Client-owned change graph; repos are projections

- Truth: a local JJ-style graph of changes (stable change ids, op log,
  working-copy-as-commit) spanning all components. Each Git repo (tool,
  umbrella, V) is a bidirectional projection of that graph (Josh/Copybara-like,
  but change-id aware).
- Identity: change id (JJ-style), commit id per projection.
- Where the graph lives: in one client. With no privileged canonical store,
  two clients' graphs are reconciled only *through* the Git projections.

### (c) Patch-theoretic (Pijul/Darcs)

- Truth: a set of patches with dependencies; independent patches commute.
  A component is the path-scoped subset of patches; a composition is a union
  of patch sets, with path prefixes applied by a mount morphism.
- Identity: patch hash. Git is an import/export format.

### (d) State-based / content-addressed

- Truth: trees. No event identity. Correspondence is derived: "U commit u's
  `tools/trunc/` subtree hash equals trunc commit X's root tree".
- A composition's history is just its own commits, whose subtrees happen to
  equal component trees. Sync is a pure function of trees plus ancestry search.

### (e) Subtree-with-provenance, equality invariant

- Truth: composition commits embed complete component trees *and* a record
  "occurrence `tools/trunc` = trunc commit X" (trailer or tree file).
- Invariant: `tree(u):tools/trunc == tree(X)`. A mismatch is an invalid
  (dirty) state that must be repaired before the commit is well formed.
- Component commit objects are not necessarily present in U.

### (f) Anchored subtrees with an evolution overlay (proposed)

This is the design argued for below. Its parts:

1. **Content is authoritative.** A composition commit's tree *is* the state
   of every occurrence, like a vendored monorepo. Plain Git sees an ordinary
   repository with ordinary directories.
2. **Anchors, not pointers.** An in-tree manifest (`.clonex/mounts.toml`,
   one entry per *occurrence*, keyed by path) records, per occurrence:
   component name, remote(s), **anchor commit** (the component commit this
   occurrence was last derived from), and tracking policy.
   The anchor is a **base**, not an equality claim:
   `delta(occurrence) = tree(u):path − tree(anchor)`. Delta = empty means
   "in sync"; delta non-empty means "unpublished change relative to anchor".
   A non-empty delta is a valid state, not an error.
3. **Correspondence is derived from content + anchors.** Publishing a delta
   creates a component commit whose parent is the anchor and whose tree is
   the occurrence's tree. Absorbing upstream is a 3-way merge with base
   `tree(anchor)`. Re-anchoring after squash or rewrite uses tree equality
   against the new upstream history, which works because the anchor gives the
   search a starting point.
4. **Change identity is an overlay.** A JJ-style change id travels on every
   commit CloneX creates (trailer `Change-Id:` plus Git commit header), and an
   append-only **evolution log** (obsolescence markers `old -> new`, Mercurial
   evolve-style, stored under `refs/clonex/evolve/*`) links rewritten or
   superseded commits. Losing either degrades display and auto-evolution. It
   never makes sync wrong, because sync reads only trees and anchors.
5. **Local layer is (b).** Locally, all repos share one object store and one
   cross-repo op log; every operation is an atomic set of ref updates across
   repos, so undo restores all of them together. Agents program against
   this layer.
6. **Closure.** Component commits named by anchors are kept reachable in the
   composition's remote under `refs/clonex/anchors/<path>/<sha>` (not as
   extra parents; see §4, distinction D4). The manifest also records the
   remote URL, so a plain-Git user can `git fetch <url> <sha>`.

(f) is not a renaming of (e). (e) makes the anchor an equality invariant, and
that one bit decides whether a plain-Git edit, a partial publication or a
provisional push is an invalid state (e) or a pending delta (f). (f) is not
(d) either: (d) has to *guess* which commit is the base, while (f) records it.

### Rejected or folded candidates

- **Submodules** are the null hypothesis. Content is absent, the pointer is
  the only truth, and every edit needs a detached HEAD. (f) keeps what
  submodules get right (an explicit per-occurrence base) and discards the
  rest.
- **(c) as foundation** is rejected on Git-interop grounds (§2). Commutation
  is retained as a *local algorithm* inside (f)'s rebase (§5).
- **(b) as foundation** is rejected because it needs a canonical store, or
  else falls back to (d)/(e) correspondence the moment a second client or a
  GitHub UI action appears. It is retained as the local layer of (f).

---

## 2. Episode replays

### Episode 1 — one commit from U touches trunc, dotsync and U's crates/

Goal: trunc and dotsync each get one ordinary commit, U gets one commit, and
V (dotsync + agent-harness) shows the dotsync part.

**(a) events.** The author commits `u1 [E]` in U. Filtering produces:

```text
trunc:   t0 ── t1[E]          (tree: + CLAUDE.md)
dotsync: d0 ── d1[E]
U:       u0 ── u1[E]          (tools/trunc, tools/dotsync, crates/check)
V:       v0 ── v1[E]          (dotsync/ only; composed from d1)
```

This works as advertised. V's commit is *synthesised* by composition, and its
message and ID are copied from E. Terminal: `git log` in V shows "Add
CLAUDE.md beside AGENTS.md" with `CX-Event: E`. Subtle point: V now holds an
event E whose other parts (trunc, crates) are absent. That is fine, but
`clonex show E` in V must say "partial: 1 of 3 components present".

**(b) client graph.** Change `C` spans three paths. Exports: t1, d1, u1, v1,
each with change-id C. Identical to (a) on one machine. On a second machine,
C is known only through the exported trailers.

**(c) patches.** Patch P = {trunc/CLAUDE.md, dotsync/CLAUDE.md, crates/check}.
Scoped subsets P|trunc and P|dotsync are distinct patches with a "split-of"
relation, or one patch with a multi-repo footprint. Export to Git linearises
each. It works, but only via the bridge.

**(d) state.** U commit u1. Sync finds trunc commit t0 whose tree equals
`u0:tools/trunc`, creates t1 (parent t0, tree `u1:tools/trunc`), and copies
the message. Same for d1. V: its `dotsync/` tree equals d0's tree, dotsync is
now at d1, so V gets `v1` "sync dotsync to d1". The original change message is
only copied if CloneX chooses to, and grouping is not recorded anywhere.
Terminal: V's log says *what state* arrived, not *which change*.

**(e) provenance-equality.** Same objects as (d). u1's record says
`tools/trunc = t1, tools/dotsync = d1`, which is only valid *after* t1 and d1
exist. So u1 cannot be committed until the component commits are created.
Fine locally; see ep. 5 for the publication consequence.

**(f) anchored.** The author commits `u1` with change-id C. Its manifest is
unchanged (anchors t0, d0), so the deltas for tools/trunc and tools/dotsync
are non-empty. `clonex publish` materialises `t1 = commit(tree=u1:tools/trunc,
parent=t0, Change-Id C)` and d1 likewise, then records `u2` (or amends
unpublished u1) re-anchoring to t1 and d1:

```text
trunc:   t0 ── t1[C]
dotsync: d0 ── d1[C]
U:       u0 ── u1[C]            manifest anchors: t1, d1   (amended before push)
V:       v0 ── v1               "adopt dotsync d1: Add CLAUDE.md…" [C]
                                anchor dotsync = d1
```

V adopts on its own schedule (policy `track = main` does it automatically).
With change-id C, V's adoption commit carries C's message, so the terminal
matches (a). Without it, V shows "adopt dotsync d0..d1" plus the expanded
component log (like `git log --submodule=log`).

### Episode 2 — outsider's plain-Git PR to trunc, squash-merged on GitHub

Squash produces `s` on trunc main. `s` has no CX metadata. GitHub's default
squash message concatenates the PR's commit messages, so any trailers from
PR commits end up *in the body*, possibly several of them.

**(a) events.** `s` has no event ID, so CloneX must mint one. It cannot
rewrite `s`, so the mapping `s -> E_s` lives out of band (git notes or a
side ref). This is now a *second identity mechanism*, and every consumer must
check both. Worse: if the PR had been opened *from* U's exported commits
`t1[E1], t2[E2]`, the squash `s` carries `CX-Event: E1` and `CX-Event: E2` in
its body. It is one commit claiming two events whose other parts (in dotsync,
U) are two separate commits. Composition then either fuses E1 and E2 in U
(rewriting U's already published u1, u2) or treats `s` as a third thing.
Terminal: surprise, and possibly a forced rewrite of U.

**(b) client graph.** The projection of trunc diverged. Import `s` as a new
change. If the client had exported t1 (change C), it must decide whether `s`
obsoletes C. That needs content comparison, i.e. (d). The client graph also
never learns what other clients know.

**(c) patches.** The bridge imports `s` as a patch P_s. If P_s duplicates
patches the bridge already has (PR from U), Pijul sees two different patches
that make the same edit, which is a conflict or duplication. Duplicate
detection by content is (d) again.

**(d) state.** U's `tools/trunc` tree = `tree(t0)`, trunc main is at `s`, so
U gets u2 with `tools/trunc := tree(s)`. If the PR came from U, U's tree
already equals `tree(s)` (when there was no concurrent change) and there is
nothing to do. If U has local edits too, the base must be *guessed* (search
trunc history for a tree equal to U's last synced state). Reverts and
ledger-only commits make that tree-to-commit map non-injective.

**(e) provenance-equality.** U's record says `t0`. Adopt `s`: u2 with tree and
record `= s`. If U had authored t1 (squashed into s): record t1 ≠ s but
`tree(t1) == tree(s)` → recognise and re-record. If U has an unpublished edit
in tools/trunc at the same time, that edit is an *invalid state* under
equality. It must be published or moved aside before adopting.

**(f) anchored.**

```text
trunc:  t0 ── s                     (squash of outsider PR p1..p3)
U:      u1 (anchor t0, delta Δ or empty) ── u2 (anchor s)
        tree(u2):tools/trunc = merge3(base=tree(t0), ours=tree(u1):tools/trunc, theirs=tree(s))
```

When Δ is empty this is a clean adoption. When Δ is non-empty it is an
ordinary 3-way merge with a *recorded* base, and Δ survives as the new delta
relative to `s`. If `s` squashed U's own exported t1 (change C), then after
adoption the delta vanishes by tree equality and CloneX writes an evolution
marker `t1 -> s` (it can also read GitHub's "PR head t1 merged as s").
Terminal: `clonex log` shows "C landed upstream as s (squashed)". Nothing is
rewritten and no second identity mechanism is needed.

### Episode 3 — trunc occurs twice; security fix to both; only latest advances

```text
trunc:  v0.4.5 ── … ── t40 (main)
U:      tools/trunc            anchor t40,     track = main
        fixtures/trunc-0.4.5   anchor v0.4.5,  track = frozen (patch branch: release/0.4.5-cx)
```

One U commit `u5` fixes both occurrences.

**(a) events.** As the gist describes, u5[E] splits into two trunc commits:
`x[E]` on top of v0.4.5 and `y[E]` on top of t40. The gist does not say
*where x lives on GitHub* (a branch must be chosen, and that is policy, not
algebra). Later only `tools/trunc` advances: t41, t42 compose into U. Correct.
The surprise: `clonex show E` in trunc lists two commits, and a plain-Git
contributor looking at trunc sees two unrelated-looking commits with one
trailer value.

**(b)** Same as (a), with two projections of C into one repo. The graph needs
"change C has two commits in the trunc projection", which JJ calls a
divergent change and treats as an error state. So a new concept is needed:
multi-site changes.

**(c) patches.** The fix against v0.4.5 and the fix against t40 have
different contexts. If the fix depends on patches absent in 0.4.5, it is
necessarily a *different patch* (a backport). Patch theory has no "one change
at two revisions of one file", because a file identity has one state per
repo. So the two occurrences need separate namespaces, and the link between
the two patches is unrepresented. Terminal: two unrelated patches.

**(d) state.** Each occurrence is independent. Publishing fixtures/trunc-0.4.5
needs "which commit is the base?". Search trunc for `tree == u4:fixtures/…`
and find v0.4.5 (only if the fixture was never edited). Where to put the new
commit is policy. Later advancement needs a tracking policy per path, which
(d) must store *somewhere*. That is already an anchor in disguise.

**(e)** Two records into one trunc DAG. Works, but the equality invariant
forces both fixes to be published before u5 is well formed.

**(f) anchored.** u5 has two non-empty deltas. Publish:

```text
trunc:  v0.4.5 ── x[C]                 → release/0.4.5-cx (policy from manifest)
        t40 ── y[C]                    → PR branch → main
U:      u5 ── u6 (anchors: tools/trunc=y, fixtures/trunc-0.4.5=x)
later:  t40─y─t41─t42 ;  U adopts tools/trunc → t42 ; fixture anchor stays x
```

One change id on two commits is *allowed* here, because change identity is an
overlay: "C touched trunc at two sites". `clonex status` shows
"fixtures/trunc-0.4.5: frozen at x (v0.4.5 + C), 43 commits behind
tools/trunc". Tracking policy is naturally a manifest field keyed by
occurrence path, and both occurrences share one trunc object DAG.

### Episode 4 — trunc history rewritten (ledger repair, force-push)

The repair (see umbrella AGENTS.md) replays with `git commit-tree`, keeps
messages, authors and trailers, substitutes only edited ledger blobs in the
affected commits, and leaves the **tip tree byte-identical**. Old tip `t50`,
new tip `t50'`, `tree(t50) == tree(t50')`. Intermediate `t30'…t49'` differ
from `t30…t49` only in `.test-status.json`.

**(a) events.** Trailers are preserved, so every event now has *two*
representations in trunc's world: the old one (inside U's composed history)
and the new one (on trunc). They share event IDs and have different content
in `tools/trunc/.test-status.json` at intermediate events. The composition
invariant "U filtered at tools/trunc == trunc" is now false. Options:
(1) rewrite U, and transitively every composition containing trunc, so that
**one force-push cascades into force-pushes of every composition**; or
(2) accept permanent inconsistency, which the algebra has no word for.
Event ID alone cannot distinguish "same event, newer version", so (a) needs a
version/evolution notion (that is, change-id vs commit-id), which it does not
have.

**(b)** The client sees remote trunc rewritten. It imports new commits,
matches them by change-id, and marks the old ones obsolete. Works on this
machine only. Other clients rediscover the same thing independently.

**(c)** New patches for each changed ledger snapshot. Old patches must be
unrecorded everywhere, and patch sets diverge across compositions until each
applies the unrecord.

**(d) state.** U's `tools/trunc` equals `tree(t50')` already, so there is
nothing to do at the tip. Historical U commits have subtrees matching no
current trunc commit (for intermediate ones), which is harmless unless
someone asks "which trunc commit was U at in March?".

**(e)** U's record says t50, which is gone from the remote. Re-record to t50'
by tree equality. Historical records dangle unless the objects are kept.

**(f) anchored.**

```text
trunc (remote, after force-push):  … t29 ── t30' ── … ── t50'
U:  u70 (anchor t50) ── u71 (anchor t50')     tree unchanged; manifest-only commit
refs/clonex/anchors/tools/trunc/t50  keeps old objects reachable in U's remote
refs/clonex/evolve: t30->t30', …, t50->t50'   (derived: same message+author+date, tree diff ⊆ ledger)
```

No cascade: U is recorded, not derived, so U never needs rewriting. Old anchors
in U's history stay resolvable (closure refs plus the `backup/…` tag).
`clonex log tools/trunc` in U can present the rewritten line and hide
obsolete commits via the evolution log. If the rewrite removed a *secret*,
the closure refs let U keep it alive. So (f) needs a deliberate
`clonex purge <sha>` that drops closure refs. This is a real cost, and it
matches the preserve-by-default rule.

### Episode 5 — concurrent cross-component changes, different orders, partial publish failure

Dev A: change CA touching trunc and dotsync. Dev B: change CB, same two tools.
A publishes trunc first, B publishes dotsync first. Then A's dotsync push is
rejected (remote moved: d1 by B).

**(a) events.** Suppose the natural recovery (rebase, or GitHub's
rebase-merge / linear-history protection) gives:

```text
trunc:   t0 ── tA[EA] ── tB[EB]
dotsync: d0 ── dB[EB] ── dA[EA]
```

Composing U requires EA before EB (trunc) and EB before EA (dotsync). That is
a **cycle, so no DAG exists**. The algebra must then split an event in U,
which breaks the atomicity it promised, or refuse. Merges instead of rebases
avoid the cycle (`t0→tA, t0→tB, Mt`; `d0→dA, d0→dB, Md`), but `Mt` and `Md`
are two unrelated events, so U needs a synthetic merge commit of `Mt` and
`Md` that belongs to no event. The partial publication (tA accepted, dA
rejected) leaves EA half-published: its trunc part exists, its dotsync part
does not. The event is visible in trunc and missing in dotsync, and any
composition built now sees EA "split" by a failure, not by design.

**(b)** Each client has its own graph and exports. Client A must import B's
dB into its graph (as a foreign change with change-id CB, if the trailer
survived) and rebase dA. Correct, but the two clients never share a graph.

**(c)** Patches commute, so both orders yield the same patch set. This is
patch theory's best episode *inside* the model. The Git bridge still has to
map two different linearisations back to the same patches, which needs
identity on Git commits (trailers lost on squash).

**(d)** Each tool is simply its own history; U just takes trees. No cycle is
possible because nothing orders events across repos. Recovery of dA is an
ordinary rebase/merge in dotsync.

**(e)** U cannot commit a record for dotsync until dA exists on the remote.
A's U commit is blocked. It is correct but rigid (it reproduces "tool PRs
land before umbrella pointer").

**(f) anchored.**

```text
trunc:   t0 ── tA[CA] ── tB[CB]              (B merged later, as merge or linear)
dotsync: d0 ── dB[CB] ── m(dB, dA[CA])       (A merges, per "prefer merge commits")
U (A's): u0 ── uA   anchors: trunc=tA, dotsync=d0 + delta ΔA   ← valid, pushable
             ── uA' anchors: trunc=tA, dotsync=m              (after retry)
U (B's): u0 ── uB ; later U merge: mU(uA', uB) anchors = common descendants (tB, m)
```

The partial failure leaves a *valid* U state: trunc anchored at tA, dotsync
anchored at d0 with pending delta ΔA. `clonex status`: "dotsync: 1 change
unpublished (remote moved; merge or rebase needed)". U's own merge of A and B
needs, per occurrence, an anchor containing both lines. That is the
workspace's existing rule, now enforced mechanically: if no such component
commit exists, the merged occurrence carries a delta until one is published.
No cycle is possible because U's history is its own.

A CI check "main has no pending deltas" restores the strict AGENTS.md
ordering for protected branches without making it an ontological constraint.

### Episode 6 — tdd-ratchet: failing test, CI bot commit, then implementation, per tool

For each tool the required published shape is:

```text
tool PR branch:  base ── T (test, red) ── B (ledger bot: pending) ── I (impl) ── B2 (bot: passing)
```

The change is made from U as a stack of two changes: c1 (tests in trunc and
dotsync, plus U's own crates/ check), c2 (implementation).

**(a) events.** c1 → E1 and c2 → E2 filter into both tools. Bot commits B_t and
B_d have no event IDs: mint E_Bt and E_Bd out of band (see ep. 2's second
mechanism). U's composed history must now include E_Bt and E_Bd as U commits
that touch `tools/*/.test-status.json`, and E2 must depend on them in each
tool. Publication must be phased (push E1 parts, wait for bots, recompose,
push E2 parts), and phasing is outside the algebra. Composition is also
defined as deterministic, so U's commits for E_Bt and E_Bd are synthesised
bot-authored commits in U that no bot made.

**(b)** Works locally: the exported T is pushed, B arrives remotely, is
imported as a foreign change, and I is rebased. Everything depends on this
one client's orchestration.

**(c)** B is a patch depending on T. Export must place I after B. Patch order
is not a property the patch set stores, so ratchet's history check (which is
defined on the *Git* linearisation) is fed by the bridge's choice of order.

**(d)** Works: each tool's history is its own; U just takes trees. The base
for I is found by tree search (B's tree is unique here).

**(e)** U's c1 commit cannot record tools until T is published *and* B has
landed. Otherwise the record is stale immediately (remote head = B ≠ T).

**(f) anchored.** Per-component commits are the truth, so foreign commits are
not special:

```text
1. publish c1 deltas → trunc PR: t_base ── T_t ; dotsync PR: d_base ── T_d
2. dispatch ledger bots → remote heads become B_t, B_d
3. fetch: U's anchors (T_t, T_d) are ancestors of the remote PR heads → adopt
   U: uc1 (anchors T_t,T_d) ── uadopt (anchors B_t,B_d; tree gains ledger blobs)
   c2 (local, unpublished) auto-rebases onto uadopt (op-log recorded)
4. publish c2 deltas → I_t on B_t, I_d on B_d; dispatch bots → B2_t, B2_d
5. merge PRs (merge commits M_t, M_d); U adopts M_t, M_d
```

The phases are a *publication recipe* (`clonex publish --phase`), and the
ontology already admits foreign commits (bot, outsider) because anchors
advance over any ancestry. U's own ledger, for its crates/ check, follows the
same recipe on U's own PR. U's commits are real, and none is a synthesised
bot commit.

### Episode 7 — agent transaction across compositions: insert N between X and Y, move auth hunks from working copy into N, rebase descendants; then undo

Setup: local, partly unpublished stack in U: `X ── Y ── Z ── @` (working
copy), where X and Y touch `tools/agent-harness` and `crates/`, and V has
locally adopted Y's agent-harness commit.

**(a) events.** Create event E_N and filter it into agent-harness and U.
Rebase Y, Z in U and in agent-harness, and recompose V. Y keeps its event ID
while its commits change, which is fine only if the system distinguishes
event versions (it does not natively). Undo needs an op log across repos,
which (a) does not define.

**(b) client graph.** This is its native territory:
`jj new --insert-between X Y; jj squash --from @ --into N <auth paths>`.
Descendants rebase automatically, and projections re-export. `jj undo`
restores. The only problem is published projections (below).

**(c)** Insert patch N (auth hunks). If Y and Z don't depend on those hunks,
"rebase descendants" is a no-op because patches commute. Undo = unrecord N and
re-record the hunks into the working copy. Elegant, but only inside the patch
store.

**(d)** No change ids, so "between X and Y" and "descendants" have to be
recomputed from the graph each time, and undo depends on whatever reflog each
repo has. Weak.

**(e)** Rewrites must re-record provenance in every affected U commit, and
each re-record must point at a component commit that already exists. That is
a lot of eager component commits.

**(f) anchored + local (b) layer.** One operation O17 in the cross-repo op log:

```text
before O17:  U: X ── Y ── Z ── @        harness(local): hX ── hY ── hZ
             V: v3 (anchor harness=hY, local)
after  O17:  U: X ── N ── Y' ── Z' ── @'   harness: hX ── hN ── hY' ── hZ'
             V: v3' (anchor harness=hY')     ← auto-evolved via marker hY -> hY'
             evolve markers: Y->Y', Z->Z', hY->hY', hZ->hZ'
op log O17 = {U: refs…, harness: refs…, V: refs…} applied atomically
```

Unpublished deltas need no component commits at all: X, Y, Z can be U-only
commits whose harness deltas are materialised at publish time. So the rewrite
touches U, plus component commits only where they already exist locally.
`clonex undo` restores every ref in O17's before-state across U, harness and
V. If hY had already been pushed, undo restores local refs and reports:
"harness hY' was never pushed; hY remains on origin; V's remote adoption of
hY unaffected". Undo never un-pushes.

Agents program against the local layer: a transformation is a function over
a composition snapshot (`tree -> tree`, or change-graph edits), and CloneX
lifts the result into per-occurrence deltas. An agent never has to reason
about event IDs or component parentage, because both are derived.

### Episode 8 — plain `git clone` of U, no CloneX

**(a)** They see U's composed history: every component event as a U commit,
with trailers. `tools/trunc` is a plain directory, so building, editing,
`blame` and `bisect` at U granularity all work well. They contribute a plain
PR to U. The PR has no event ID, so on absorption CloneX mints one (the
out-of-band mechanism again) and filters the commit into trunc. If the PR is
squash-merged in U's UI, trailers from any CloneX-authored commits in the PR
body are mashed together (ep. 2 in U).

**(b)** They see whatever U projection was exported. Contributions are
imported into *someone's* client graph. The contributor has no access to the
real model.

**(c)** They see a linearised export. Their commits are imported as patches.

**(d)** They see a plain monorepo. Component histories are absent. They can
edit anything, and CloneX users later absorb by tree diff (base guessed).

**(e)** Plain monorepo plus trailers or a manifest naming SHAs they cannot
fetch. If they edit `tools/trunc`, their commit breaks the equality invariant.
It is well defined only once a CloneX user repairs it.

**(f)** A plain monorepo plus `.clonex/mounts.toml`. The manifest is readable
(`tools/trunc: git@github.com:maxeonyx/trunc.git @ t40, track main`), and
`git fetch <url> t40` works. Editing `tools/trunc/src/x.rs` and opening a PR
to U simply creates a delta relative to a recorded base. When a CloneX user
(or a CI job) runs `clonex publish`, the delta becomes a trunc commit
authored by the contributor (author preserved) with parent t40. The only
thing they must not do is hand-edit anchors, and CI verifies that each
anchor's tree + delta = the occurrence's tree. Squash-merging U's PR in the
GitHub UI is harmless, because the manifest is a *file*, and files survive
squash, cherry-pick and rebase, unlike trailers.

History visibility is (f)'s weak spot here. `git log -- tools/trunc` in a
plain clone shows U's adoption commits ("adopt trunc t40..t47"), not the 7
authored trunc commits, and `git blame` lands on adoption commits. Two
mitigations exist, both opt-in per composition:
- **expanded adoption**: replay each adopted component commit as its own U
  commit (one U commit per trunc commit, each with its own anchor). This
  reproduces (a)'s U history exactly, with SHA anchors instead of event IDs.
- **closure-by-parents**: make component commits extra parents of adoption
  commits. `git log` then walks trunc history (subtree-merge style), but
  GitHub's commit list and non-`--first-parent` log fill with root-layout
  trunc commits. Default off.

---

## 3. Scores

Scale 1–5, 5 = best. "Mechanisms" is 5 for fewest, "invalid states" is 5 for
fewest possible, and "hidden reconciliation" is 5 for least.

| | (a) events | (b) client graph | (c) patches | (d) state | (e) prov.-equality | (f) anchored |
|---|---|---|---|---|---|---|
| Naturalness of ordinary work | 3 | 5 (one machine) | 2 | 4 | 3 | 4 |
| Expressiveness | 4 | 5 | 4 | 2 | 3 | 4 |
| Understandability of surprises | 2 (cycles, cascades) | 3 | 2 | 4 | 4 | 4 |
| Git interop sanity | 3 | 2 | 1 | 5 | 4 | 5 |
| Information preservation | 4 | 3 | 3 | 1 | 3 | 4 |
| Recoverability | 2 | 4 (local) | 4 | 3 | 3 | 5 |
| Implementability of guarantees | 1 | 2 | 1 | 5 | 4 | 4 |
| Number of mechanisms | 3 (trailer + out-of-band IDs + composer) | 2 (graph + N bridges) | 1 (VCS + bridge) | 5 | 4 | 3 (manifest, overlay, op log) |
| Invalid states possible | 2 (cycles, split-by-failure, ID collisions) | 3 (divergent projections) | 4 | 5 | 2 (dirty = invalid) | 5 |
| Hidden reconciliation | 2 (recompose, cascade) | 1 (projection sync) | 2 (bridge dedup) | 3 (base guessing) | 4 | 4 |

Per-episode verdicts (✓ natural, ~ works with a caveat, ✗ breaks or needs an
extra mechanism):

| Episode | (a) | (b) | (c) | (d) | (e) | (f) |
|---|---|---|---|---|---|---|
| 1 cross-component commit | ✓ | ✓ | ~ | ~ (grouping lost) | ✓ | ✓ |
| 2 squash-merged outsider PR | ✗ (2nd ID mech, multi-trailer squash) | ~ | ✗ | ✓ | ~ | ✓ |
| 3 twice-present component | ~ (where does x live) | ✗ (divergent change) | ✗ | ~ | ~ | ✓ |
| 4 upstream force-push | ✗ (cascade) | ~ | ✗ | ✓ | ~ | ✓ |
| 5 concurrent + partial publish | ✗ (cycle) | ~ | ✓ in-model, ✗ bridge | ✓ | ~ (blocked) | ✓ |
| 6 ratchet phases + bot commits | ~ (synthesised bot commits in U) | ~ | ~ | ✓ | ~ | ✓ |
| 7 agent transaction + undo | ~ (no op log) | ✓ | ✓ in-model | ✗ | ~ | ✓ |
| 8 plain clone of U | ✓ (best history) | ~ | ~ | ~ | ~ | ~ (log/blame shows adoptions) |

---

## 4. Distinctions that actually change terminal behaviour

These are the design bits that produce different output, errors, or pushes.
Everything else is naming.

- **D1: Derived vs recorded composition history.** Derived (a, and (b)/(c)
  exports) means an upstream force-push cascades into composition
  force-pushes (ep. 4). Concurrent cross-repo orderings can make the
  composition uncomposable, because of the cycle (ep. 5). Bot and outsider
  commits then become synthesised commits in compositions (ep. 6). Recorded
  (d, e, f) means compositions never rewrite because someone else did, at the
  cost of explicit adoption commits.
- **D2: Correctness-bearing vs overlay identity.** Squash merges, GitHub UI
  cherry-picks, bot commits and outsiders all drop or mangle assigned IDs.
  If sync depends on them (a, b, c-bridge), each such action needs a second
  identity path. If identity is an overlay (f), losing it only degrades
  `clonex log` text.
- **D3: Anchor-as-base vs anchor-as-equality.** This one bit decides whether
  a plain-Git edit, a partial publication, or a provisional push is an error
  (e) or a pending delta (f). Visible as: blocked commit/push vs
  `status: 1 change unpublished`.
- **D4: How component objects reach a composition's clones: absent, closure
  refs, or extra parents.** This decides whether `git log`/GitHub's commit list
  in U is readable, whether a plain clone can push a trunc commit, and whether
  a rewritten-away secret survives in U.
- **D5: Adoption granularity, compressed or expanded.** Decides U's
  `git log`/`bisect` granularity: one adoption commit, or one U commit per
  component commit. This is a per-composition policy, not an ontology.
- **D6: Op-log scope: per-repo or cross-repo local.** Decides whether ep. 7's
  undo is one command or N.
- **D7: Where tracking policy lives.** Keying it per occurrence path is
  required by ep. 3 in every ontology. It is not a distinguishing choice, but
  (a) and (d) currently have nowhere to put it.

Non-distinctions: "event" vs "change id" naming; trailer vs header (except for
survivability, which is D2); "filter and compose are inverses" as a framing
(it holds exactly only in the absence of D1's failure cases); "pointer vs
contained history" as the gist frames it. The submodule problem was never
the pointer. It was that the pointer was the *only* truth and content was
absent. (f) keeps content authoritative and demotes the pointer to a base.

## 5. The integrating idea

> **Content is the only identity that survives every Git operation other
> people perform; so correctness must rest on trees plus a recorded base per
> occurrence, and every assigned identity must be an overlay.**

From this one decision (anchored, content-authoritative occurrences) several
episodes fall out together:

- ep. 2 and ep. 4: squash and force-push are absorbed by 3-way merge from a
  recorded base plus tree-equality re-anchoring. No second ID mechanism, no
  cascade.
- ep. 3: occurrences are paths with their own anchor and policy, and share
  one component DAG.
- ep. 5 and ep. 6: partial publication, foreign bot commits and concurrent
  orders are just anchors advancing over ordinary ancestry. No cycle is
  possible.
- ep. 8: a plain-Git edit is a delta against a recorded base. It is
  publishable later with authorship preserved.

The rest of the requirements (stable change ids, undo, stacked changes,
agent transactions: ep. 7) belong to a **local** JJ-style layer whose
outputs are ordinary commits and anchors. Patch commutation can be used
inside that layer's rebase, to skip no-op rebases of commuting changes, without
making patches the shared truth.

## 6. Recommendation

Adopt **(f) anchored subtrees with an evolution overlay**, with the (b)
client graph and cross-repo op log as the local UX layer. Keep the event-ID
idea from (a), but demote it to the change-id overlay: it names
"these edits were made together" and never decides sync.

Concrete commitments this implies:

1. `.clonex/mounts.toml` in the composition tree, keyed by occurrence path:
   `component`, `remotes`, `anchor`, `track` (`<ref>` | `frozen`),
   `patch-branch`, `adoption = compressed|expanded`.
2. `delta(path) = tree(path) − tree(anchor)`. It is a first-class, valid,
   pushable state. Protected branches may require zero deltas (CI check),
   which reproduces today's "tool PRs land first" rule as policy.
3. Publish = materialise the delta as a component commit (parent = anchor,
   author = original authors, `Change-Id` trailer and header) and re-anchor.
4. Adopt = 3-way merge from `tree(anchor)`. Re-anchor after rewrite/squash by
   tree equality on the new upstream line; an ambiguous match is surfaced to
   the user and never guessed silently.
5. Closure refs `refs/clonex/anchors/…` on each composition remote, plus an
   explicit `purge`.
6. Evolution markers under `refs/clonex/evolve/…`, derivable where possible
   (tree equality, GitHub "merged as", commit-tree replay signatures).
7. Local: one object store, one cross-repo op log, change ids, and
   auto-rebase/auto-evolve of dependent compositions.

## 7. Strongest objection to the recommendation

**Component history is second-class inside a composition.** (f) gives up the
gist's central promise that "an ordinary clone of F contains the history
necessary to derive F1". With compressed adoption, `git log -- tools/trunc`
and `git blame` in a plain clone of U show adoption commits, not the authored
trunc commits, and deriving trunc's history from U alone is impossible,
because U holds states and anchors, not trunc's DAG. The mitigations each
cost something:

- expanded adoption brings back (a)'s per-event U history, and with it U
  commit-count growth, plus ordering choices for concurrently adopted
  components (though never a cycle, since U's order is its own);
- closure-by-parents gives full history at the price of GitHub/`git log`
  noise and root-layout commits in bisect;
- closure refs keep the objects but need CloneX, or a refspec, to view them.

A second, lesser objection: manifest anchors conflict on concurrent U
branches that advance the same occurrence. The textual conflict resolves
mechanically: pick a component commit containing both anchors, or leave a
delta. But it is still the gitlink-merge situation the workspace rules
already warn about. (f) makes it resolvable without choosing a side blindly,
but does not make it disappear.

## 8. Open questions for design

- Where do frozen-occurrence patch branches live on the component remote
  (`release/*`, `clonex/<composition>/<path>`)? The component owner has to
  agree to receive them.
- Should expanded adoption be the default for the umbrella, where plain-Git
  readability matters, and compressed for ephemeral CI/agent views?
- Can evolution markers be shared across clients without a canonical store?
  Pushing `refs/clonex/evolve/*` to every affected remote is the obvious
  answer; conflicting markers (two successors) are a JJ-style divergence to
  surface, not a sync error.
- Author/committer policy for publishing a plain-Git contributor's delta from
  U into trunc (preserve author, CloneX as committer, `Co-Authored-By`?).
