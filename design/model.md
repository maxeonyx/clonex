# CloneX semantic model — design search and selection

Status: **selected model, implemented as a vertical slice; confidence
moderate-high for the core (tested laws, real-umbrella trial), low for the
client/transaction surface.** §8 records how implementation and two
independent attacks changed §3 after it was first written — read it before
trusting any sentence above it. Inputs: `episodes.md` (lead pass, ~130
episodes), `research/blind-episodes.md` (134 episodes by an agent that never
saw the model), `research/algebra-attack.md` (+ executable search in
`research/algebra/`), `research/alternative-ontologies.md`,
`research/git-interop-matrix.md` (empirical, 5 carriers × ~40 operations).

## 1. What the search established (the facts that constrain everything)

1. **Compose is not the inverse of filter when compose means "interleave
   events by identity".** Automated search: `compose∘filter = F` fails in 83%
   of random histories. Order between unrelated events, the parent's own
   files, merges, and "which component revision was present" are all lost.
   Ordinary rebasing in two remotes produces cycles (algebra F4). An event
   that has reached one component but not another makes every composed
   descendant unstable (F5). Hidden inputs — which revision each occurrence
   holds, which split copy belongs to which occurrence — cannot be derived
   from the components (F6, F7). *Four independent lines of investigation
   reached this.*
2. **Every metadata carrier dies somewhere.** Headers: dropped by every
   rewrite, including jj's. Trailers: survive rebase/cherry-pick/amend, get
   mangled by squash, get *copied* by cherry-pick (which makes identity look
   like two events). Notes and custom refs: not transferred by default.
   Tree files: lost by subdirectory filtering. GitHub merge buttons recreate
   commits (UNVERIFIED on GitHub itself, but consistent with the local
   results). **So correctness cannot depend on event identity metadata.**
3. **Identity of states is robust; identity of changes is not.** Tree
   hashes and commit hashes survive everything that doesn't change content.
   Git's 3-way merge already handles "the same change arrived by two routes"
   when it can find a correct merge base.
4. **The real environment needs exact SHAs of component commits** to
   survive a round trip: commit statuses (`latest-ci-green`), tags,
   GitHub's signatures, `state.json` attestations, and tdd-ratchet's ledger
   are all keyed by SHA. The ledger rewrite (umbrella 8e3a81f) showed what
   losing SHAs costs.
5. **Per-occurrence "this subtree is based on component commit X" is
   essential historical information** (my C9, C12, C14, E6, K24; algebra
   F3/F6; ontology agent's anchor). Without it, filtering a composition
   produces a history that disagrees with the real component, an upgrade
   of an occurrence looks like a synthetic jump commit, and signed
   commits can't round-trip.
6. **Real component rules constrain intermediate history**: tdd-ratchet
   needs red and green commits pushed separately, with a CI bot commit in
   between, on each tool's PR branch. Remote actors add commits *under*
   local unpublished work (K14, blind X1/X10).

## 2. Candidate systems

These differ in foundations, not names. Each was replayed against the hard
episodes: B3 (cross-cutting change), K14 (ratchet bots), E7/F4 (concurrent
opposite-order publication), F7/K15 (component force-push), C3/C12
(repeated occurrences, upgrade), D2/D5/D6 (editing published work), G9
(GitHub merge commits), F14 (plain clone), H4 (ephemeral CI composition),
J8/K12 (agent transaction + undo).

### C-EV: event-interleaving algebra (the original design document)
Compositions store fused events; components are derived by filtering;
compositions are derived by composing on event IDs.
- B3 ✓ elegant. E7 ✗ cycle, no valid history. F7 ✗ component rewrite forces
  every composition to rewrite or fork. K14 ~ bot commits have no IDs, so they
  need a second derived-identity mechanism. G9 ✗ each GitHub merge commit is a
  new event to be interleaved. C12 ✗ an upgrade becomes a squash or a
  cycle. Correctness depends on trailers surviving (fact 2).
- Verdict: **rejected as the storage model**; kept as a *view*: an
  interleaved "logical log" can be computed on demand (§6).

### C-SUB: submodules with a better client
Composition stores pointers; content lives elsewhere. Rejected by the
owner's premise (plain clone lacks content; pointer bumps remain), and
episodes F14/G11 (archives, plain clones) confirm the premise.

### C-STORE: CloneX owns a private change graph; every Git repo is an export
(jj-like store spanning all histories; Josh/Copybara-like bidirectional
export).
- Strong local UX. But: the store is a privileged place (T3), exports must
  round-trip exact SHAs (fact 4) so the store must also absorb foreign
  commits verbatim, and G9/F7 still need reconciliation of everything other
  people do on GitHub. Plain clone of U has no CloneX meaning unless the
  store is published too. Every problem C-ANCHOR solves still has to be
  solved, and a private graph has to be kept in sync with the Git repos on
  top of that.
- Verdict: **rejected** — a second source of truth with no terminal-behaviour
  advantage once C-ANCHOR exists.

### C-STATE: pure trees, no recorded provenance
Correspondence is inferred from tree hashes. Fails on C12 (upgrade with a
local delta: no tree equals any component tree), E6, and any occurrence
carrying unpublished work. Verdict: **rejected**, but its core insight
(content identity is the robust identity) is adopted.

### C-ANCHOR: anchored occurrences (selected)
Described in §3. Composition history records *states*, including
"occurrence P is based on component commit X", as real commit-graph edges.
Components keep their own real histories, which the composition contains
verbatim. Filtering is a lens, not an inverse.
- B3 ✓ one U commit; each tool gets one derived commit. K14 ✓ the bot
  commit is adopted under the local stack; a derived green commit is based
  on it. E7 ✓ no linearization is ever needed: U adopts each component's
  actual result. F7 ✓ adoption of rewritten main is an ordinary 3-way merge;
  U's history is never rewritten. G9 ✓ a GitHub merge commit is adopted with
  no content change (fast-forward in T-space). C3/C12 ✓ split is natural; an
  upgrade is a real fast-forward of the occurrence's base. D6 ✓ reordering
  events that don't touch T leaves T's derived commits byte-identical. F14 ✓
  a plain clone has content *and* every component's real commits. H4 ✓ an
  ephemeral composition is one deterministic adoption commit. J8/K12: all
  cross-history local editing is ordinary editing of one repository (U), so
  jj's op log and transactions already cover it.

## 3. The selected model

### Objects (all ordinary Git)
- **History**: a Git commit DAG. No global name. Two occurrences refer to
  "the same history" when their bases live in one connected DAG. A fork is
  just another branch.
- **Composition**: any Git repo with a `.clonex.toml` manifest at its root.
  A repo without a manifest is a composition with no occurrences, so every
  repo is one and nothing is privileged.
- **Occurrence** (manifest entry): `path`, a composition-local `name`, an
  optional `remote` and `branch` for publication, and a `follow` intent
  (`branch` | `none`). Declarations only; no revisions.
- **Base** of an occurrence at composition commit `c`: a set of component
  commits (usually one). A commit that sets a base carries a trailer
  `Clonex-Base: <path> <sha>`, and each named sha must be reachable from `c`.
  Adoption makes the sha a parent. Otherwise, base(c, P) = the maximal
  elements of the union of the parents' bases. Bases therefore live in the
  commit graph, can't name something the repository doesn't contain, and
  merge without textual conflicts.
- **Delta**: the difference between `tree(c)[P]` and what its base implies.
  An empty delta means "this occurrence equals component commit X". A
  non-empty delta is the component work the composition holds that the
  component doesn't have yet: unpublished, unlanded, or a deliberate local
  patch (a vendored fixture).

### The lens: `get_P` (filter) and `put_P` (adopt)
`get_P(c)` maps a composition commit to a component commit, which may be
virtual (computed, not pushed). It is memoized and processed in topological
order:
1. If `c` sets a base for P to `{b}` and `tree(c)[P] == tree(b)`, then
   `get_P(c) = b` exactly. **This is the round-trip law, and it preserves
   SHAs, signatures and statuses.**
2. Otherwise `cand` = the union of `get_P(p)` over the parents, plus any base
   `c` sets, reduced to maximal elements in component ancestry. If `cand` is
   a single commit `k` and `tree(c)[P] == tree(k)`, then `get_P(c) = k`
   (no effect). Otherwise `get_P(c)` = a new commit with tree `tree(c)[P]`,
   parents `cand`, `c`'s author, **committer := author** (so the commit is
   stable across rebases, which rewrite committer dates), and `c`'s message
   with a `Clonex-Change: <id>` trailer ensured. It is keep-merges:
   survival is decided by git-filter-repo's rule, the only one under which
   nested filtering composes (algebra F2).
3. If `c` has no P, or P isn't declared at `c`, it maps to none.

`put_P(c, t)` (adopt component commit `t` into occurrence P of head `c`)
creates a commit with parents `[c, t]`, trailer `Clonex-Base: P t`, and
`tree(c)` with P replaced by `merge3(base = mergebase(get_P(c), t), ours =
tree(c)[P], theirs = tree(t))`. When everything the composition had has
landed in `t`, the merge base is `get_P(c)` and the result is exactly
`tree(t)`: a fast-forward in component space.

**Publish** = compute `get_P(head)` and push it to the occurrence's
remote/branch. The derived commits are the events since the base, split
per occurrence. **Sync** = fetch the followed branch and `put_P`. Local
unpublished composition commits may be rebased onto the adoption (jj does
this automatically), so remote-originated commits land *under* local work
(K14).

### Change identity (overlay, not load-bearing)
`Clonex-Change: <id>` goes on derived component commits, set from the
composition commit's jj change-id (or derived from its SHA when made by
plain Git). It is used for:
- replacing a published representation when its source is rewritten (a PR
  branch force-update);
- "where does this change appear" queries, and grouping in logs;
- recognising squash merges.

Losing it degrades only these conveniences. Merges, publication and
filtering stay correct, because they are content + base.

## 4. Laws (to be property-tested; §7 lists what is tested)

- **L1 PutGet** (round trip): `get_P(put_P(c, t)) = t` whenever the delta
  after adoption is empty. The SHA is identical.
- **L2 GetPut**: `put_P(c, get_P(c))` leaves `tree` unchanged.
- **L3 Determinism**: `get_P` is a pure function of commit objects. Two
  machines produce identical SHAs, with no clock or identity input.
- **L4 Locality**: rebasing or reordering composition commits that don't
  touch P, and rebases that only change committer metadata, leave `get_P`
  unchanged.
- **L5 Nesting**: `get_Q ∘ get_P = get_{P/Q}` for occurrences nested through a
  sub-composition. This needs keep-merges.
- **L6 Idempotence**: `get_P` applied to a derived component history is the
  identity (filtering an already-filtered history).
- **L7 Convergence**: publish, then adopt the published head → empty delta,
  unchanged composition tree.
- **L8 Content fusion**: two occurrences with equal bases and equal edits
  derive the *same* component commit (C2). No identity rule is involved.
- **L9 Order-independence**: adopting components in any order gives the
  same composed tree (the commits differ).
- **Non-law (deliberate)**: a composition's own history is not recoverable
  from its components. The composition *is* its own history. This is the
  answer to "compose∘filter ≠ F".

## 5. What happened to the original design document's ideas

| Idea | Fate |
|---|---|
| Composition contains histories, not pointers | **Kept, strengthened**: content in the tree, component commits verbatim in the object graph, bases as graph edges |
| Opaque event ID as the connecting primitive | **Demoted** to a UX overlay; content + base is load-bearing |
| Filtering restricts, drops empties, reconnects | **Kept** as `get_P`, made precise (keep-merges), with the anchor base case |
| Composition as inverse of filtering | **Replaced** by adoption (`put_P`); interleaving is a derived view |
| Events split on filtering | **Kept**: falls out per occurrence |
| Events fuse on composition | **Replaced** by content fusion (L8) and view-level grouping |
| Paths belong to the parent | **Kept**: manifest paths are composition-local |
| Transport config out of band | **Partly kept**: remote/branch are declared in the manifest so collaborators share them; local overrides in `.git/config` [owner question Q3] |
| No privileged repo | **Kept**: every repo is a composition; no store outside Git |

## 6. Product boundary: CloneX does not need to own the client

Because every cross-history edit is an ordinary edit of *one* repository
(the composition), and component commits are derived lazily at
publication, the composition model needs no special working copy, staging,
rewrite machinery or op log. **jj (colocated) is the recommended everyday
client; plain Git works.** CloneX contributes only what no client has:
declare, adopt/sync, publish, lens queries (derived histories, "where does
change X appear", delta/unpublished status), and the interleaved logical-log
view.

The programmable-transaction interaction model (J8) is real and valuable,
but it is a **general VCS improvement orthogonal to composition**. It
belongs on jj-lib's transaction machinery, where it would also be recorded
as one jj operation and be undoable. It is deferred as a separate track,
not bundled into the composition engine. Evidence that could reopen this:
a composition workflow that needs a cross-*repository* atomic local edit.
None has been found, because the composition is one repository.

Known costs of this boundary:
- `git log` in a composition shows component commits (subtree-style); use
  `--first-parent` for the composition's own story.
- Plain `git rebase` (without `--rebase-merges`) of a composition branch that
  contains an adoption of *unmerged* component commits replays them at the
  root. The damage is visible and recoverable via reflog; jj and
  `--rebase-merges` are safe.
- `git blame` in the composition attributes adopted lines to the adoption
  merge. `clonex blame` can follow the lens.

## 7. Open questions and deferred areas

Engineering (future agents decide by experiment):
- Rewrite propagation into *immutable* published branches → always a new
  change. Into PR branches → replace (by change id) vs append (delta).
  Default: replace if the branch has no foreign commits, append otherwise
  (bot commits make "foreign commits present" common).
- Multi-base occurrences publish as component merge commits; conflict
  presentation is unexplored.
- Moves and occurrence removal are manifest-driven; how to recognise a
  plain-Git move (tree-equality heuristic) is unexplored.
- Partial-path occurrences (`R/src/ledger`, K17) need the component's full
  base commit so `get` can rebuild the full component tree: the tree is
  `base.tree` with the subpath replaced. Designed for, not yet built.
- Server opportunities: a change-id index across hosted repos, event-aware
  PR grouping, automatic adoption bots. None is required by the model.
- Security: `Clonex-Adopt` (originally `Clonex-Base`) claims are verifiable (the sha must be reachable
  and the tree must match), so they can't be forged into lies about
  content. `Clonex-Change` is unverified display data. Signing derived
  commits happens at publish time and conflicts with L3; the resolution
  (record the signed object as the base after adoption) follows from L1.

Owner value judgments (asked in the handoff): see `questions.md`.


## 8. What implementation and review changed (authoritative over §3–§6)

Sources: `research/concept-removal.md`, `research/anchor-attack.md` (19
scenarios over real Git, attacking §3 as first written), and friction met
while building (`src/`, `tests/`).

**Changed rules (as implemented):**
- **The lens is total.** `get_P` does not consult the manifest; any path
  of any commit has a derived history. Gating on declarations was *wrong*
  (it made extraction a special mode and made pre-declaration history
  vanish). The manifest now only drives sync/publish.
- **Adoption claim renamed** `Clonex-Adopt: <path> <sha>`; valid only if
  `sha` is a parent. Adoptions are *path-relative*: an adoption at `A`
  answers queries for `A/Q` through the adopted commit's own lens
  (`get_{A/Q}(c) = get_Q(adopted)`), and adoptions below a lens path are
  carried into the derived commit re-rooted (with the adopted commit as a
  parent). This is what makes nesting (L5) hold; found by a failing test.
- **Only the composition's own side is walked.** Adopted parents never feed
  candidates except through the rule above (attack A1: bases leaking from
  nested compositions, and a trust hole via outsider trailers).
- **`name` and the custom remote override are gone.** Path is the key;
  mirrors/local paths are Git's `url.<base>.insteadOf`.
- **No SHA-derived change ids.** A change id derived from a commit SHA
  changes on every plain rebase, which changes every derived commit (breaks
  L4). Change identity exists only when stable: jj's `change-id` header on
  the composition commit, carried as `Clonex-Change` on derived commits.
- **Adoption reapplies only the composition's own work** (attack A4, three
  real failures): own work = derived commits not reachable from anything
  adopted. None → the occurrence becomes the new component state (an
  upstream rewrite such as the ledger repair no longer conflicts). For each
  own commit whose change exists upstream (same change id, or identical
  patch via `git cherry`), apply only the difference from that version:
  a squash-landed-then-reverted change stays reverted; an amend after a
  reviewer's commit applies without conflict. Otherwise Git's 3-way merge.
- **Publication replaces a branch only if its tip is what this repository
  last published there** (`refs/clonex/published/…`, a lease); anyone
  else's commits (bot, reviewer, GitHub) must be adopted first.
- **Frozen occurrences are published only when named.**
- **Component tips live under `refs/remotes/clonex/<path>/<branch>`**, so jj
  imports them as untracked remote bookmarks, which jj makes immutable by
  default: `jj rebase -b` can no longer rewrite adopted component commits
  into composition-shaped commits (attack A6; verified with jj 0.44).

**Restated claim (attack item 9):** change identity is not needed for
*content* correctness (no silent loss, no wrong content) *except* where
Git itself would be wrong — re-merging a squash-landed-then-reverted change
— and there `git cherry` patch-id detection covers the single-squash case
without metadata. Identity *is* needed for merge-base selection after
rewrites (clean amends over published versions) and for detecting stale
foreign commits.

**Known open defects and gaps (with the attack's minimal cases):**
- *Stale adoptions* (A5, K14 with a red-commit edit after the bot ran): jj
  rebases the adoption merge onto the edited change and keeps the bot
  commit, whose ledger now describes a test state it never saw. Needed:
  detect adopted commits built on a superseded version of one of our
  changes (change id), report them, and offer "drop and republish" so the
  bot reruns. Not built.
- *Sync placement* (A9): `sync` adopts at HEAD, so green work made before
  syncing becomes a sibling of the bot commit (content right, shape wrong
  for ratchet-style repos). Needed: adopt at the last published commit and
  rebase unpublished work above it (jj `rebase -s`). Not built.
- *Occurrence moves* (A3): a `git mv` of an occurrence starts a new root.
  Planned: `clonex mv` that re-adopts the old path's component commit at
  the new path (no new rule, one existing mechanism).
- *Plain-Git hazards* (A7): `git rebase` of a composition branch that
  adopted unmerged component commits replays them at the root — **also with
  `--rebase-merges`**, silently (my §6 claim that it was safe was false).
  Squash-merging a composition PR on GitHub drops adoption parents; the
  claims then fail validation and derived history "jumps". Publish guards
  (refuse derived commits that revert upstream work or jump) are designed,
  not built.
- *Sub-path occurrences* (K17) are **unsupported**: an occurrence must hold a
  whole component tree; publishing a subdirectory-only history would delete
  the rest of the component.
- *Multi-base occurrences* publish as component merges; conflict
  presentation during adoption is plain Git conflict paths (first-class
  conflicts are jj's job once the adoption commit is made).

**Verified (tests):** `tests/episodes.rs` replays 17 episodes (A2, A3, A5,
B1, B2, B3, C2, C3, C12, D2, D6, D12, F6, F7, G6, K14, L5); `tests/laws.rs`
property-tests tree law, PutGet, no-lost-outsider-commits, append-only main,
convergence and cross-clone determinism over generated histories, and was
mutation-checked (removing the round-trip rule or using the committer
identity makes it fail). A trial on a copy of the real agent-tools umbrella
converted all seven submodules, published one cross-cutting commit as one
ordinary commit per tool, detected the real 12-commit dotsync pin lag, and
merged dotsync main into the feature branch by `sync`.
