# CloneX episode reservoir (lead designer's pass)

Written *before* choosing an architecture. Independent passes live in
`research/` (blind-episodes.md was produced by an agent that never saw the
event model). Each episode names the question(s) it exposes as `Q:`; the
questions are collected and answered in `model.md`.

Notation. Repos are capitals (`U` umbrella, `T` trunc, `D` dotsync, `R`
tdd-ratchet, `H` agent-harness, `B` tb). Commits are lower case; `x[E]` means
Git commit `x` represents logical event `E`. `X@p` means "history X placed at
path p inside a composition". `a─b─c` is a lineage, `╲` a branch. "GH" means a
GitHub remote with merge-commit-only settings (the real setting for the tool
repos, verified: `allow_merge_commit=true, squash=false, rebase=false`).

Real-world grounding (from `at-main`):

- 77 of 268 umbrella commits are pointer bumps ("Point dotsync at v0.8.0").
- A cross-cutting change today ("CLAUDE.md alias beside every AGENTS.md") is
  one umbrella concern commit + one PR per tool (trunc #11 …) + one pointer
  bump per tool + umbrella merge PR, serialized because the umbrella PR must
  not merge until every child commit is on its remote.
- tdd-ratchet enforces *per tool repo history*: a red (test) commit must be
  pushed and recorded by a CI bot commit on the PR before the green commit.
  The bot writes `.test-status.json` into the tool's PR branch.
- `latest-ci-green` reads a commit status posted on the pinned SHA; statuses,
  signatures and tags do not survive history rewrites; a ledger repair rewrite
  really happened (umbrella 8e3a81f).
- Some tools (dotsync) are actively developed standalone despite the
  workspace-routing concern; standalone development is real, not theoretical.

---

## A. Creation and composition

**A1. Brand-new repo.** Max runs "new history `cx`" inside U at `tools/cx`.
Before: U = `u1─u2`. After: U = `u1─u2─u3[N]` where u3 adds `tools/cx/…`.
No remote for cx exists yet. Later he wants `github.com/maxeonyx/clonex`.
Q: is `tools/cx` a *history* (a thing that can be exported) merely because it
is a directory, or because someone declared it? Does the exported cx repo's
root commit represent `N`, and what tree does it have? (Answer shapes A5.)

**A2. Import an existing independent history.** U today holds T as a
submodule pointer. Goal: U contains T's real history.
T = `t1─t2─…─t40` (with merges from PRs). After: `U` contains the 40 T events,
each represented by a U commit whose tree has `tools/trunc` = that T state,
joined to U by a merge. Plain `git log -- tools/trunc` in U shows T's
history. Q: does importing require *rewriting* T's commits into U commits (new
SHAs) or can U's history literally contain T's commit objects? (Git subtree
merge keeps T's objects as the second parent lineage, trees at T's root, not at
`tools/trunc` — then `git log -- tools/trunc` shows nothing useful. Josh
rewrites. This is a real fork: *physical sharing of objects* vs *rewritten
representations*.)

**A3. Import with existing submodule history.** U's 268 commits contain
gitlinks for `tools/*`. Converting: each old U commit that bumped a pointer
should become a U commit that *contains* T's events between old and new
pointer. Q: can history conversion be done without rewriting U (append-only:
one "convert" commit that replaces gitlinks by trees, and T's history merged
in as side lineages)? Which old U SHAs must be preserved (tags, statuses)? →
Append-only conversion is strongly preferred; old U commits stay as-is.

**A4. Compose a never-existed repository.** Max wants a "frontend" view `V` =
{D@`dotsync`, H@`harness`} for an agent working on UI of both. No V exists.
He asks for it. Q: must V become a Git repo with a history? If V only exists
for one afternoon, what is its history — the union of D and H events
interleaved? In what order (D and H share no events)? Is the interleaving a
choice CloneX makes, and must two people who compose V get identical SHAs?

**A5. Extract a history from an existing directory.** U has `crates/standards`
with 200 U commits touching it; Max wants to publish it as `crosscut` repo.
Filter U at `crates/standards` → C = 200 commits (minus empties). Some of those
U commits touched both `crates/standards` and `tools/trunc` (e.g. "Require a
CLAUDE.md alias", which also changed trunc). Those U commits already carry
events (or not, if made by plain git before CloneX). Q: events for pre-CloneX
commits: derive identity from the U commit (its SHA) so that C's commit and
T's commit for the same U commit are recognized as one event without any
metadata ever having been written? → strong candidate: **a commit with no
explicit event is its own event, identified by its SHA in the repo where it was
first seen.** But "first seen" is not globally knowable…

**A6. Compose histories that share events.** C (crosscut, extracted in A5)
and T both contain representations of U's "CLAUDE.md alias" event. Compose W =
{C@`crosscut`, T@`trunc`}. Fuse: W gets one commit for that event. Q: the
identity used to fuse must be the same in C and T even though they were
derived at different times by different people → identity must be derivable
deterministically from the source commit, or written explicitly.

**A7. Compose unrelated histories.** W = {T, B} share nothing. Composition
history = two independent lineages. Options: (a) two roots merged by a
composition commit; (b) a linear interleaving by date; (c) a "product" DAG.
Q: is interleaving order semantic? If W is ephemeral CI context, only the
*current state* matters; if W is a long-lived repo, its history should show
when each event entered W.

**A8. Nested composition.** U contains `tools/*`; a bigger `M` = {U@`at`,
personal-site@`site`}. An event E in M touching `at/tools/trunc` and `site/`.
Filter M→U→T must equal filter M→T at `at/tools/trunc`. Q: path composition
associativity; is T "inside U inside M" the same occurrence as "T inside M"?

**A9. Overlapping compositions.** U (all tools), V (D+H), Sec (security view:
`R`, `oc`, U's `crates/standards/src/concerns/`). The concerns dir is a
*subdirectory* of U's own files, not a separate repo. Q: can a composition
include a sub-path of another repo's own files as a history without that path
being declared anywhere? (If filtering is a pure function of path, yes.)

**A10. Several task views over the same ecosystem.** Agent A works in V,
agent B in U, simultaneously, both touching D. Q: they are two clones; how do
their D events meet? Only via some remote (D's GH, U's GH, V's GH, or a
CloneX store). Which remote is "the" place for D events? Is it OK that the
answer is "any of them, union"?

**A11. A composition with two revisions of one history.**
U = {T@`tools/trunc` (latest), T@`fixtures/trunc-0.4.5` (frozen, for compat
tests of the ratchet)}. T = `…─t30(v0.4.5)─…─t40`. Q: does U record that
`fixtures/trunc-0.4.5` *is T at t30* (so filtering yields a history ending at
t30 with t30's SHA), or only that its content equals t30's tree?

**A12. Composition where component is not at repo root.** Sec wants only
`tools/tdd-ratchet/src/ledger/` from R. Filter R at `src/ledger`. Q: histories
are path-scoped all the way down; a "component" is just (history, subpath).
Composition of subpaths → Sec's events on `ledger/` flow back to R. Q: is
writing back through a subpath filter well-defined? (Yes if filter is
per-path restriction: the R commit = R parent tree with `src/ledger` replaced.)

## B. Ordinary development

**B1. Change one component from its standalone repo.** Max in a plain clone
of D (with or without cx) makes `d41`, PR, merges on GH → D main =
`d40─d41─m[merge]`. Later someone in U syncs. Ideal: U gets U-commits
representing d41 and m (or just the merge result?), `tools/dotsync` updated,
no "Point dotsync" commit needed. Q: does U represent D's merge commit `m` as a
U merge, or flatten? Does U's representation keep D's authorship/date/message?

**B2. Change one component from U.** In U, edit `tools/dotsync/src/x.rs`,
commit `u9[E]`. Ideal: D gets `d42[E]` (tree = D state + change), U has
u9. Q: what is d42's parent — D's main tip known to U. If D's GH main moved
since (B1 happened), d42 is based on stale D state → publication must
rebase/merge (see F-section).

**B3. One logical change across several tools (the motivating case).**
"Alias AGENTS.md as CLAUDE.md" from U: touches every `tools/*/`,
`crates/standards/…`, root `AGENTS.md`. One U commit `u10[E]`. Ideal outcome:
each tool repo gets one commit `x_T[E]`, `x_D[E]`… on a branch/PR; U has one
commit. No pointer bumps. The umbrella's own CI sees all of them at once.
Q: per-tool PR creation, per-tool CI gating, and the umbrella PR ordering
rule ("umbrella must not merge until every child commit is on its remote") —
is that rule *still needed* when U contains the content? (If U contains the
content, U never references anything unavailable → the ordering rule
disappears. Big simplification.)

**B4. Component + composition-owned files together.** Same as B3; the U part
(`crates/standards/…`) has no separate remote. Filter U at `crates/` is just
U. Nothing to publish except U. Fine.

**B5. Several independent changes before publication.** In U: `u11[E1]`
touches T only; `u12[E2]` touches D only; `u13[E3]` touches T+D. Publication:
T gets `E1,E3`; D gets `E2,E3`. In T, E3's parent is E1 (T-relevant ancestor).
In D, E3's parent is E2. Q: this is the gist's filtering; confirm filtered
ancestry = nearest relevant ancestors. What if E1 and E3 are unrelated
semantically — does T now *require* E1 before E3? (Yes: ancestry is what it
is; to publish E3 alone you reorder in U first.)

**B6. Parallel developers.** Alice in U makes `ua[Ea]` (T+D); Bob in a plain
clone of T makes `tb1`. Both push. Alice's publication to T fails (non-FF).
Ideal: Alice's cx fetches `tb1`, rebases `Ea`'s T representation on it (and
U's `ua` onto the U-representation of `tb1`), republishes. Q: the rebase
happens *in U* (moving `ua` onto new state) — and Ea's identity survives.

**B7. Temporary local experiment.** Agent in U tries a change across 3 tools,
decides against it. It should never have touched any remote; abandoning is
local. Q: nothing is materialized into component repos until publish. So
filtered component commits are *derived lazily*, not eagerly written.

**B8. Abandon work already published to PR branches.** E published to T and
D PR branches; decide to abandon. Ideal: one intent "abandon E" closes both
PRs / deletes both branches. Q: publication state per (event, remote)
must be known to do this; is that state stored or re-derived from remotes?

**B9. Move changes between events.** Working copy has hunks in T and D;
Max wants T hunks in E1 and D hunks in E2 — ordinary JJ-style split, but
across histories. Nothing special: an event is just a U commit here.

**B10. Review a large cross-history event.** Reviewer wants one diff for E
across all tools + the per-tool PRs. Q: where is review done? One umbrella PR
with the full diff (GH shows it natively because U contains content!). Per-tool
PRs still exist if per-tool CI/ledger gates are required. Is duplicate review
acceptable, or does the umbrella PR become the review and tool PRs are
mechanical? [OWNER-DECISION candidate]

**B11. Bisect.** A regression in U CI. `git bisect` in U over U history:
with full component history inside U, bisect walks through tool commits at
fine grain. Every U state is a real state that existed? No — the U states
representing imported T commits have the *U tree at import base* + that T
state: a combination that may never have been built. Q: which composed states
are "real"? Is bisect over composed intermediate states meaningful? (It is
exactly as meaningful as a subtree-merge side lineage: mostly yes.)

**B12. Blame.** `git blame tools/trunc/src/lib.rs` in U must attribute lines
to the original T authors/commits. Requires U's representation of T events to
preserve author and message, and lines to not be re-attributed to a merge.

**B13. Restore an earlier state.** "Put D back to how it was at v0.8.0 in U
without touching other tools". New event: `tools/dotsync` tree := D@v0.8.0.
Filtered into D, this is a revert-like commit on top of D main. Fine.

**B14. Conflict resolution across histories.** Alice's `Ea` and Bob's `tb1`
conflict in T. Resolution happens in U (Alice's context). The resolved
content in T is a new T commit (rebased Ea representation). Q: first-class
conflicts (JJ) would let Alice publish nothing until resolved; conflicts
should be representable locally, never published.

## C. Repeated occurrences (deep)

Setup for all: T = `t1─t2─t3─t4`, B-files unrelated. U contains T at `L =
tools/trunc` and at `F = fixtures/trunc-old`.

**C1. Same revision twice.** L=t4, F=t4. Max edits `L/src/a.rs` only, commit
`u1[E]`. Filter at L: T gets `t5[E]` on t4. Filter at F: no change. Now L≠F.
Did F "fall behind", or was E simply not applied to F? Q: after this, is F
still "tracking" T? Is there any concept of tracking at all in U's history, or
only in a sync policy?

**C2. Same revision twice, identical edit to both.** Max does a search-replace
touching `L/x` and `F/x` identically, commit `u2[E]`. Filter at L: `t5 = t4 +
Δ`. Filter at F: `t5' = t4 + Δ`. Deterministic filtering with identical
parent, tree, author, message → **t5 == t5' as Git objects**. One commit in T.
Fusion by content addressing — no special rule. Q: this only works if the
filtered commit's metadata is a pure function of (event, parent, tree); any
per-occurrence metadata breaks it.

**C3. Different revisions, one event touching both** (the gist's case).
L=t4, F=t2. `u3[E]` edits both. Filter at L: `x = t4+Δ1 [E]`. Filter at F:
`y = t2+Δ2 [E]`. T now has two new heads. Publishing to T: `x` goes to `main`;
`y` goes… where? There is no T branch for "t2 + fixes". Q: an occurrence at
an old revision that receives edits *is a branch of T*. Publication must know
which T branch it corresponds to; otherwise `y` is an anonymous head that
exists only inside U. Is that OK? (It must be — this is vendoring with local
patches.) So an occurrence has an optional *published branch*, which is
transport config, not history.

**C4. One occurrence advances, the other doesn't.** T GH main gets `t5` (from
outsider). U syncs. L should get t5; F should not (it's a frozen fixture). Q:
the difference between L and F is *intent*, and it must be shared by everyone
who syncs U (otherwise Alice's sync advances F and Bob's doesn't). → Intent
must live in U's tree (versioned with U), e.g. `.clonex` manifest: `L follows
T:main`, `F` has no follow. That is *not* a revision pointer.

**C5. An event affects all occurrences on purpose** — CVE fix to be applied
to L (at t5) and F (at t2). One U event `u4[E]` = two T commits (x on t5, y
on t2). The "has the CVE fix been applied everywhere?" question is answered by
"which heads/branches contain a representation of E" — a query CloneX can
answer and Git can't. Backports become single events. Q: this is the
cherry-pick-with-identity case: E has two representations on one history,
which is *intended*, not divergence.

**C6. Event affects some occurrences.** As C1. Fine.

**C7. The composition records the distinction.** In C1, is it recorded that
F is *deliberately* not updated? Only via the intent manifest (C4). History
itself records only what happened.

**C8. Event touches repeated T and repeated D simultaneously.** 4 occurrence
representations, 1 U commit, filtered to 2 commits in T and 2 in D (or 1 each
if the occurrences were identical and edits identical — C2).

**C9. Add an occurrence.** Add `fixtures/trunc-0.4.5` = T@t30 to U. U commit
`u5[N]` adds the subtree. Filter at F from u5: first commit introducing F has
tree t30 — filtered history of F would be a *new root* with tree = t30, not
T's real history. Q: **introducing an occurrence must connect to T's existing
history**, otherwise the filtered F history is a disconnected copy. How does
U's history say "this subtree *is* t30"? Either (a) U's history contains a
lineage representing t1…t30 at F (a subtree-merge of T placed at F) joined by
merge into u5, or (b) u5 carries an annotation "F := T commit t30" (a
provenance record — *not* a live pointer, a historical fact), or (c) tree
equality lookup: F's subtree hash equals t30's root tree hash — derivable if
you have T. Q: which of these makes filter a pure function of U alone? Only
(a) and (b). (c) needs T.

**C10. Remove an occurrence.** Delete `fixtures/trunc-old`. Filter at F: a
commit that deletes everything? For T, "delete all files" is a real T commit
if published. It must *not* be published to T main. Q: removing an occurrence
is a composition-structure event, not a T event. Distinguishing
"delete all of T's files" from "stop containing T here" needs structural
knowledge → the manifest again. Plain git users deleting the dir would be
interpreted as structure change if the manifest is also changed; ambiguous
otherwise.

**C11. Occurrence moves path.** `git mv tools/trunc tools/text/trunc`. Filter
at old path: deletion; at new path: new root. With no structural knowledge,
T's history appears to end. Q: moves must be recognizable. Tree-hash
equality between the removed subtree and the added subtree recognizes a pure
move (same tree) deterministically; a move-with-edits is ambiguous unless the
manifest also moves.

**C12. Two occurrences become equivalent.** F at t2 gets upgraded to t5, same
as L. U event `u6` sets `F` tree = t5 tree. Filter at F: a commit on the
t2-branch whose tree = t5's tree — a *second copy* of t5's state with a
different history (fast-forwarding a branch is not a commit!). Q: upgrading
an occurrence should be a *fast-forward of its branch* (F's lineage now
includes t3,t4,t5), not a synthetic "jump" commit. So `u6` must record that
F's state is now T commit t5 (provenance again), or U's history must include a
merge from the t5 lineage into F's lineage. **This is the strongest argument
that per-occurrence provenance ("this subtree is history-commit X") is
essential historical information, not a cache.**

**C13. One occurrence becomes intentionally independent.** F is forked
("our patched trunc"). Its events go on F's own branch; publication goes to
a fork remote or nowhere. Its history remains connected to T's (shares
t1…t2). Q: a fork is not a new history; it is a branch of T that nobody
publishes to T main. Identity of "T" is just the connected event graph.

**C14. Reuse a historical state already contained elsewhere.** U adds
`fixtures/trunc-t4` where t4 is also in L's past. Provenance "F := t4" makes
filtering at F reproduce exactly T's t1…t4 SHAs. Q: with provenance, adding
an occurrence is O(1) metadata; without it, we'd have to replay history.

**C15. Plain git user edits only one of two identical occurrences.** (C1 via
plain git.) cx later sees `u7` with no event metadata, touching L. It must
assign an identity (derived from u7's SHA) — and filtered T commit is
deterministic. OK.

**C16. Plain git user copies a directory to create an occurrence.**
`cp -r tools/trunc fixtures/t` + commit. No manifest update. cx sees a new
subtree whose tree hash equals the tree at L (= t4). Heuristic: offer to
record it as an occurrence of T@t4. Never silently. Q: degradation is
"unrecognized copy until confirmed", which is fine.

## D. History editing

**D1. Amend an unpublished event.** `u8[E]` in U touches T and D; amend to
fix a typo in D part. New `u8'[E]` (same E). Nothing published → nothing
else to do. Filtered representations re-derived lazily.

**D2. Amend a published event.** E published to T PR branch (`x[E]`) and D
PR branch (`y[E]`). Amend only the D part. Now: U `u8'[E]`, D representation
`y'[E]` must replace `y` on the PR branch (force-push a PR branch — normal
GitHub PR workflow). T representation unchanged: `x` is still correct
(filtered u8' at T == x exactly? Only if deterministic filtering reproduces
x byte-for-byte: same parent, tree, message, author, *committer date*).
Q: **committer timestamps**: re-deriving `x` after amend must not change its
SHA, else every amend force-pushes every PR. → filtered commit metadata must
be a function of the event's stored metadata, not of "now".

**D3. Rewrite an event already merged into T main.** E is in T main; amend
in U → impossible to replace in T without force-pushing main. Ideal: cx
refuses to rewrite *published-immutable* representations and offers "new
event that fixes it". Q: immutability is a property of a representation in a
particular remote/branch (T main is immutable; T PR branch is not; U local is
not). JJ's `immutable_heads()` is per-repo; here it must be per history per
remote.

**D4. Squash two events.** In U: `E1` (T only) and `E2` (T+D) → squash E1 into
E2. Result: one U commit. Identity? JJ: result keeps the destination's change
id (E2); E1 is abandoned. Representations: T had `x1[E1]─x2[E2]` on a PR
branch → becomes `x2'[E2]`. D had `y2[E2]` → unchanged content? D's part of
E2 unchanged, and its parent unchanged → `y2` identical, no re-push. Good
property: **rewriting only affects representations whose content or ancestry
changed.**

**D5. Split an event.** `E` (T+D) split into `E_T` (T) and `E_D` (D). JJ:
one half keeps E, the other gets a new id. Representations: T's `x[E]` →
`x[E_T or E]`… If T's half kept `E` then T's rep is byte-identical → no
re-push. So split should keep identity on… both? Q: whichever half keeps E,
the other history's representation changes identity (new event id in message
→ new SHA). Unless identity is not in the message. **Insight: if event
identity is stored in the commit object (trailer/header), then *identity
changes* force re-publication even when content is unchanged.** A
content-only representation (identity out of band) avoids that.

**D6. Reorder.** `E1(T) ─ E2(D)` → `E2 ─ E1`. In T: E1's parent unchanged,
E2 not in T. In D: E2 unchanged. So filtered representations are unchanged!
Reordering independent-history events is a no-op for components. Beautiful
property of filtering; must be preserved by the implementation (no
per-composition data such as "U parent SHA" inside component reps).

**D7. Duplicate.** `jj duplicate E` → new event E' with same content. Filter:
if E' lands on the same T base as E with same content → byte-identical
unless identity differs. Duplicates must get distinct identity.

**D8. Abandon.** Abandon E in U → descendants rebased. Published reps of E
on PR branches: branches must be updated (drop commit). On T main: impossible
→ "revert event" instead.

**D9. Restore.** `jj restore --from X tools/dotsync` = new event whose D
representation sets tree to D-state-at-X. Fine.

**D10. Cherry-pick-like intention.** "Apply E (from T main) to T release
branch": in a U with occurrences L (main) and Rel (release-0.4 branch),
cherry-pick = add a representation of E at Rel. Should it keep E's identity
(C5 says yes, intended multi-representation) or create E' (JJ duplicate)?
Q: [OWNER-DECISION-ish, but mostly engineering] — identity must answer "is
this fix present on release-0.4?" → keep E, mark it as *the same event at a
second location*. But then compose of a U without Rel, from T repo containing
both reps, must pick the right one (the one in the occurrence's ancestry).
Fusion rule must be *ancestry-scoped*, not global.

**D11. Transplant into another ancestry.** Move E from being on top of T
main to on top of an older D? (Cross-history transplant meaningless — events
don't move across histories; they move within one history's DAG.)

**D12. Rewrite only one representation of a shared event.** Someone in plain
T amends `x[E]` (on their PR branch) — edits T part. Now T has `x'[E]` (if
trailer preserved) and U still has u8[E] with old T content. Two versions of
E disagree on T's content. Q: which wins? Needs *supersession*: "x' supersedes
x" (known because x' has E and x is its predecessor on a PR branch force-push?).
Git has no record of that except reflog. Detection: same event id, different
content, one on a branch that previously had the other. Resolution: U syncs,
sees T's E rep changed, updates U's E (rewrites u8 → u8'' with T part = x').
That's *distributed rewrite propagation*. Mercurial evolve does exactly this
with obsolescence markers. Q: do we need obsmarkers, or is "latest
representation on the tracked branch wins" enough? (Divergence when both
sides rewrote.)

**D13. Ordinary git rebase of CloneX commits in T.** T PR branch `x[E]─x2[E2]`
rebased by plain git onto new main. Trailers survive (message copied);
headers are dropped. SHAs change. U sync: recognizes E, E2 by trailer →
supersession by identity. With headers only → identity lost → U sees new
events `x'` `x2'` duplicating E, E2 content → conflicts/duplication. **Strong
evidence for trailers (or tree-level metadata) over headers for identity.**

**D14. Metadata lost.** GitHub web edit, `git commit --amend -m` rewriting the
message without the trailer. U sees a commit with no identity whose tree equals
the tree U expected for E's T rep. Recognizable by *tree equality on the
same base*. Degradation: "recovered by content match", explainable.

**D15. Semantically equivalent change recreated independently.** Two people
both fix the same typo in T. Different events, same tree. Fine: separate
events; merge is trivial.

**D16. Squash-merge on GH** (not enabled on these repos, but other hosts do).
`x1[E1]─x2[E2]` squash-merged → `s` with message containing both trailers.
Q: `s` represents {E1, E2} — **one commit, several events**. Must be
representable: event set per commit, not a single id. Composing U: U has
u(E1) and u(E2) separately; T main has s. U state after E2 == T state at s;
U doesn't need new commits; correspondence: s ~ (E1∘E2).

## E. Branches, concurrency, merges

**E1. Independent events on different components.** U: `E1(T)` and `E2(D)`
on parallel heads, merged in U: `m`. Filter T: E1 only; merge m has no T
effect beyond E1 → dropped (or collapsed). Fine.

**E2. Independent events on the same component, merged in U.** U: `E1(T)`,
`E2(T)` on parallel heads, merge `m(U)`. Filter T: `t─e1`, `t─e2`, merge
`m_T` (T tree = merged T). m_T has two parents in T. Real T merge. Identity
of `m`? It is an event (the merge resolution may contain content). Q: merge
commits are events too (they carry resolution). Filter keeps merges whose
filtered parents are distinct and not ancestor-related.

**E3. Component merge commits imported into U.** T main: `t─(pr)─m` merge
commits (GitHub merge button). U representation: U commits for each T
commit, including a U merge for m. Q: U's history contains T's merge
structure. Filter U at T must reproduce T's merge exactly (same parents
order) → requires deterministic mapping of parents.

**E4. Criss-cross.** Rare; ensure filter doesn't infinite-loop or produce
different results by traversal order. Property test.

**E5. Component with different branching than composition.** T has release
branches; U doesn't care. U only contains L=T main. T's release branch events
are not in U. Fine.

**E6. Event created with incomplete knowledge.** Alice's U hasn't fetched D
for a week. She makes E(T+D). Her D rep is based on stale D. Publication to D
must rebase E's D part onto D main (conflict possible). U must then also
reflect the rebased D part: U's `E` rewritten on top of the U-rep of new D
events. Q: **publication can require rewriting the local composition event**.
Fine locally (unpublished), but if E was already pushed to U's GH… U's E is
then immutable; the D rep differs from what U has. Then U has `E` (old D
base), and D has `E'` (rebased). Reconciliation = U merges D's new events and
E' in: U state after sync has D = D main incl. E'. U's history: `E` then a
merge bringing D's other events. Filter U at D: E (old base) and then merge
with D main... produces a *different* D history than D's actual. **Filter(U)
at D ≠ D.** Q: is that acceptable? It means U's view of D history is its
own, not D's. With provenance (C12), U's merge commit says "D := D commit
x'" and filtering at D *uses* D's actual commit → Filter(U) at D == D's
actual history. **Provenance makes filter agree with the real component.**

**E7. Two cross-component changes concurrently, opposite orders.** Alice:
E_A(T+D), Bob: E_B(T+D). T main gets A then B; D main gets B then A
(publication race, each rebased the other). Compose U from T and D: in T,
A<B; in D, B<A. No single U linearization respects both with A and B each
atomic. Options: (a) cycle → surface as conflict; (b) fuse A,B into one U
state change; (c) U history is a merge: A and B on parallel branches, merge
point m where T=[A,B] D=[B,A]. Parallel branches in U: branch 1 has A (T part
applied on T base, D part on D base), branch 2 has B. Merge m: T = A+B, D =
A+B. m's T tree = T main (A then B), m's D tree = D main. Filter U at T:
through branches: A_T', B_T' parallel then merge → a T history that doesn't
equal T's linear A─B. Again **Filter(U) at T ≠ T** unless provenance used. Q:
the model must accept that the composition's history is *a* consistent
history, and component histories are *their own*, related by
provenance/correspondence, not necessarily equal by filtering. OR the model
must forbid/prevent the race via ordering of publication. **Critical design
question: is "filter(composition) = component" a law, or is correspondence
weaker?**

**E8. Later discovery that independent events interact.** E1(T) and E2(D)
independent; composed U state after both fails CI (semantic conflict).
Normal software problem; fix with a new event. Nothing special.

**E9. Filtering through merges where only one side touches T.** U: main has
E1(T); side branch has E2(D); merge m. Filter T: e1 then m has T-tree = e1's
→ no effect → dropped. Good.

**E10. Same event through several ancestry paths.** In T, E's representation
x merged into both main and release (merge commits). Composition containing
both → x appears once in ancestry of both. Fine.

## F. Transport

**F1. Fetch from one component remote.** U syncs T from GH: new T commits
t41…t45 (incl. merges). U must create U reps (or reuse via provenance merge).
Q: does fetching T *change U's history*? It must produce new U commits (to
put them in U's tree). Who authors those? It's a sync — like "Point trunc at"
but with full history inside. **The pointer-bump commit does not disappear;
it becomes an automatic merge.** Unless U's history is virtual (G-section).
Q: can the "sync commit" be avoided entirely? Only if U's state is *derived*
(U's main = compose(U_root main, T main, D main…)) rather than stored.

**F2. Fetch from several remotes.** Same, for all tools; one sync = one
merge with many parents? or one merge per tool? Deterministic ordering.

**F3. Publish a cross-history event to several remotes.** E(T+D+U-root) →
push T branch, D branch, U branch. Order? No distributed transaction on GH.
If T push succeeds and D fails: T has E_T on a PR branch. Harmless: PR
branches are not main. For direct-to-main pushes (Max pushes to main of his
own tools): T main has E_T, D main doesn't. U main (with E) not yet pushed.
Retry later. Q: define publication as idempotent convergence: "make each
remote branch contain its representation of E"; retries are safe; partial
state is visible as "E: T ✓, D ✗ (rejected: non-FF), U pending". Order rule:
**publish components before compositions** (so compositions never reference
what's unavailable)? With content-containing compositions, U never
references anything — order doesn't matter for availability, only for "does
U main claim E while D main doesn't have it yet". Q: is that a problem? For
the umbrella's pinned-main-parity concern it would flag it. Harmless otherwise.

**F4. One remote rejects permanently** (permissions: Max can push T, not a
colleague's repo X). E(T+X): T part published, X part must go as a PR from a
fork. Q: publication target per history = (remote, branch, method: push /
PR / fork-PR).

**F5. Network failure mid-publish; retry.** Idempotent: re-check remote
state; push what's missing. No local "in-progress transaction" state needed
if publication is computed as desired-state vs remote-state diff.

**F6. Remote branch moved (someone pushed).** Non-FF → fetch, rebase E's rep
onto new tip (and U's E correspondingly), republish. Q: rebase of the
component rep vs of the composition event: they must stay in lockstep.

**F7. Force-push on a component remote (history rewrite, like the ledger
repair).** T main was `t1…t40`, becomes `t1…t20─t21'…t40'` (same trees
mostly, new SHAs). U contains reps of t21…t40 (with provenance → old SHAs).
Q: U sync sees T main not descending from what U has. Options: (a) treat
new commits as new events → duplication; (b) match by identity/tree →
supersession ("t21' supersedes t21"); U doesn't need content changes if trees
equal; only provenance must be updated (a new U commit? or a replacement
mapping?). **The real environment has done this.** The repair rewrote ledger
files (trees differ in `.test-status.json` only).

**F8. Partial publication, later retry after others built on it.** T main
got E_T; D didn't; Bob in plain D clone makes d50. Alice retries: E_D must
rebase on d50. Fine per F6.

**F9. Remote history rewriting of an event CloneX made.** Covered in D12.

**F10. Missing event metadata in remote.** Commits from plain git: see A5,
D14. Identity derived from SHA.

**F11. Several remotes with overlapping history.** D's history lives in D GH,
U GH (inside U), V GH (inside V). Alice fetches D events from V instead of D
GH. Q: any remote containing a history is a valid source of its events.
Discovery ("V contains D") via filter of V at its D path + identity match.

**F12. Discovering events from a larger composition.** Bob only has D. He
knows U contains D. `cx fetch --from U`: fetch U, filter at `tools/dotsync`,
get D events (including ones not yet on D GH, e.g. Alice's unpublished-to-D
but pushed-to-U event). Q: if Alice pushed U before D, Bob can get E_D from U.
Good redundancy; means "published" is per remote, and "available" is
"anywhere".

**F13. Offline work.** All local; publication later. Trivial if the model is
local-first.

**F14. Clone from one repo with no CloneX config.** `git clone U`: gets full
content and history (all tools' histories inside). Builds work. No need for
submodule init! (Real improvement: standalone_publishability and
workspace-routing concerns become simpler.)

**F15. Later adding CloneX config.** `cx init` in that plain clone: reads
manifest from U's tree (occurrences, follow intents), transport config
defaults suggested from manifest (history → remote URL). Q: remote URLs —
in-tree (shared) or `.git`-local? `.gitmodules` puts URLs in-tree and that's
useful for everyone. The gist puts transport out of band. [candidate
OWNER-DECISION; my lean: in-tree default URLs, local overrides.]

## G. Plain Git interoperability

**G1. Clone with plain git, never install cx.** U works as a monorepo. T
works as a normal repo.
**G2. Ordinary commits to U touching tools/trunc.** No trailer → derived
identity. cx later filters them into T. The plain user doesn't know T exists.
Q: someone must run the sync; CI could (U's CI publishes T reps?). That's a
bot with push rights to every tool repo. [OWNER-DECISION: is automatic
component publication from U's main acceptable?]
**G3. Plain push to T.** Imported by next U sync.
**G4. Merge CloneX history using git.** Merge commits are fine.
**G5. Rebase CloneX commits with git.** Trailers survive; SHAs change → D13.
**G6. Squash with git.** Trailers accumulate → D16 (multi-event commit).
**G7. Cherry-pick with git.** Trailer copied → *same event id on a second
commit in the same history* → looks like C5 (intended multi-rep) or D10.
Q: git cherry-pick copies the trailer; is that "same event, second
location" (desirable for backports!) or accidental duplication? Ancestry-
scoped fusion makes both harmless.
**G8. Edit trailer manually / delete.** D14.
**G9. GitHub merge UI.** Merge commits: no trailer, made by GitHub, SHA from
GitHub. It's an event (derived id). U imports it. For a PR whose head was a
CX rep of E, the merge commit m is a *new* event "merge E into main". In U,
E was already on U main (if made in U) → U's import of T main sees m whose T
tree == U's T tree already → no U content change; provenance update only.
Q: **every per-tool PR merged by GitHub generates a merge commit in T that
U must absorb as a no-content correspondence update.** If U records
provenance in U commits, each GitHub merge causes a U commit. If
correspondence is derived (tree equality), none.
**G10. PRs.** Per-tool PRs contain the tool rep of E. PR description links
the U PR and sibling PRs (event id).
**G11. Archive downloads.** Tree only; U's archive contains everything —
strictly better than submodules (GitHub archives omit submodules today).
**G12. Mirrors, forks.** Fork of T is a branch of T's history; forks keep
trailers. Fine.
**G13. `git log` in U.** Shows every tool commit interleaved (noisy but
honest). `git log --first-parent` shows U's own events + sync merges.

## H. Hosting and collaboration

**H1. PR across repos.** Event E → PRs on T, D, U. Links among them. Merging
order constraint: none for availability (content is in U). Per-tool CI must
pass per tool. U PR shows the whole diff.
**H2. CI on one component** (T's own CI, T GH Actions) — unchanged.
**H3. CI on a composition** (U CI) — sees all tools at once. Good.
**H4. CI on alternative compositions** (V: D+H) — V may be ephemeral: built
in CI from D and H remotes, never hosted. Requires deterministic compose from
components *without* V ever existing. Q: ephemeral compose needs no history
at all — just "tree at each occurrence" from given heads. **Compose of
current state is trivial; compose of history is where all the hardness
is.** Which consumers actually need composed *history*? bisect, blame, log in
the composition, filtering back out. Ephemeral CI needs only state.
**H5. Release tags.** T v0.4.13 tag on a T commit. In U, the tag could be
mirrored as `trunc/v0.4.13` on the U rep. Q: version files: U's
`docs/version.json` generated from tool versions — a derived file; with
content in U, it's derivable from U's tree.
**H6. Provenance.** "Which U state did CI test for release X of T?" — T's
release is built by T's CI from T's tree. U's CI tested U state s; the
correspondence says s contains T@x. Needs correspondence query.
**H7. Signed commits.** Filtered reps can't carry the original signature
unless byte-identical to the signed object. With provenance (U says "T :=
commit x") filter at T returns x itself (signed). Derived reps of U-made
events can be signed by the publisher at publish time (new object) → but
then determinism (D2) conflicts with signing (signature includes the object
bytes; deterministic object → deterministic signature bytes? No, signatures
may be nondeterministic). Q: signing happens at *publication*, and the
signed object becomes the canonical rep (recorded as provenance).
**H8. Permissions.** Colleague can push U but not T. Their event E(T) goes
into U; T rep published via PR by someone with rights, or by a bot. Fine.
**H9. Public/private mixture.** U public, contains private tool P? Then P's
content leaks via U. Q: a composition containing private histories must be
private; CloneX should warn when publishing a composition to a more public
remote than its members. Event *relationships* leak too: a public T commit
whose message has `CX-Event: E` where E also touched private P → leaks
nothing but an opaque id (fine) unless the message mentions P.
**H10. Outsider contributions** via plain Git to T. G3.

## I. Development contexts and CI

**I1. Full-project integration tests** in U CI — natural.
**I2. Frontend-only tests** in V (ephemeral). H4.
**I3. Security audit context:** Sec = {R/src/ledger, oc, U/crates/standards/…
concerns}. Read-only; ephemeral; needs history for audit (blame!). → needs
composed history, not just state. Deterministic, reproducible, so an audit
report can cite Sec commit SHAs? Only if compose is deterministic (same
inputs → same SHAs).
**I4. Agent context:** an agent task gets a composition containing exactly
the relevant histories (issue #4's "workspace of multiple repos"). Its
changes flow back to each history. **This is issue #4's workspace
lifecycle + CloneX composition = the same product.** The at-* clone directory
per task could be a composition checkout.
**I5. Release builds:** from component repo (standalone_publishability) —
unchanged.
**I6. Refactor/migration across tools:** B3.
**I7. Must compositions be hosted?** No: U hosted; V, Sec ephemeral. But
ephemeral compositions that receive edits must be able to publish those edits
to components (I4) without ever being hosted.

## J. Client interaction moments

**J1. Begin work.** Agent: "work on dotsync+harness for issue 46". CloneX
creates a context (composition V checkout) with a working-copy change. No
branch names needed.
**J2. See outstanding changes.** "What is unpublished?" → per event, per
history, per remote status table.
**J3. New event.** JJ-style: working copy is an event; `new` starts another.
**J4. Edit existing event.** `edit E` / auto-rebase descendants in all
histories.
**J5. Choose where a change belongs** (hunk → event). Standard.
**J6. Move hunk between events** across histories: standard.
**J7. Insert an event between X and Y.** `new --after X --before Y`.
**J8. Transaction referring to not-yet-existing commits.** The motivating
program:
```
let N = new(after: X, before: Y, msg: "extract config")
move(from: @, to: N, paths: "tools/*/src/config.rs")
squash(from: Z, into: N, hunks: matching("deconfuse"))
```
executed atomically; one op in the op log; undo reverts all. Q: the language
is *pure transformations over an immutable graph value* with symbolic
handles for new nodes; commit is replace-graph-atomically.
**J9. Commit several dirty concerns separately.** Program that partitions the
working copy by path/hunk predicate into N new events.
**J10. Multiple heads.** Normal.
**J11. Why did history change?** Op log entry: "sync from T GH: t41..t45
(outsider PR #12 merged) → rebased your 2 events".
**J12. Undo.** Op log covers local state. Published effects are not undone
automatically; undo of a publish = explicit "retract" (force-push PR branches
back; impossible for immutable). Q: undo scope must be honest about remotes.
**J13. Recover from bad transformation.** Op log restore.
**J14. "What changed because of E?"** Show E across all histories.
**J15. "Where does E appear?"** List reps: U@u10, T main@x, T rel@y, D PR#…
**J16. "What compositions contain this history?"** Only knowable for
compositions CloneX has seen (local knowledge). No global index without a
server.
**J17. "What will publication affect?"** Dry-run plan: per remote, per
branch, commits to push, PRs to open.
**J18. "What state will CI see?"** For U: U's tree. For V (ephemeral): the
compose result of named heads — show it.
**J19. Agent generating an operation.** Agent writes a transaction program;
CloneX type-checks it, dry-runs it, shows the resulting graph diff, commits
atomically. Must be deterministic and explainable.

## K. Intersections (the important section)

**K1. Repeated history + rewrite.** E touches L (t5) and F (t2) → T reps x,
y. Amend E's F part only. x unchanged (D4 property), y → y'. Publish: only
F's branch changes.
**K2. Repeated history + cross-component event + plain git.** Plain git
user in U edits `tools/trunc/a` and `fixtures/trunc-old/a` identically and
`tools/dotsync/b`. Derived identity from SHA; filter: T gets two commits
(different bases) or one (C2); D gets one. All carry the same derived id.
**K3. Nested composition + partial fetch.** M ⊃ U ⊃ T. Alice fetched M, not
T. She needs T's events: filter M at `at/tools/trunc` — available from M
alone. Partial knowledge is still consistent.
**K4. Cross-repo event + GitHub PR review.** E → T PR, D PR, U PR. Reviewer
requests change on T PR (comments on T's rep). Alice amends E in U (T part)
→ T PR force-updated, U PR updated, D PR untouched (D4). Review comments on
T PR stay attached (GitHub keeps PR, marks outdated). Good.
**K5. Multiple heads + composition.** Alice has two unpublished heads in U,
both touching T. Publishing only head 1: T gets head1's T reps; head 2's
reps stay local. Per-head publication.
**K6. Component permissions + composed CI.** U CI needs read access to all
tool repos? No — U contains content. CI needs no cross-repo tokens for
reading. For publishing component reps from U CI, it needs write tokens.
**K7. Event fusion + squash.** T had separate reps of E1, E2 (published,
immutable on T main). In U, Alice squashes E1 into E2 (both unpublished in
U? no—they're in T main already, immutable) → refused. If only published on
PR branches: T rep becomes single commit; fine.
**K8. Event split + conflict.** Splitting E in U where one half conflicts with
D main on rebase: first-class conflict stays local; publication of D blocked,
T proceeds.
**K9. Path move + identity.** Move `tools/trunc` → `tools/text/trunc`:
manifest move + tree move in one U event. Filter at T: no T change (T's tree
unchanged!). The move is a U-only structural event. Good: identity of T
history unaffected by paths.
**K10. Plain git rebase in T + later composition.** D13 + E6.
**K11. Partial publication + retry after remote rewrite.** F3 + F7: T accepted
E_T to main, then T main was force-rewritten (ledger repair) — E_T now
exists as E_T' with different SHA. Retry publish of D: unaffected. U sync
maps E_T' ~ E_T via trailer.
**K12. Agent transaction + undo.** Agent's program moved hunks across 3
tools and inserted 2 events; published T but not D; human says "undo". Undo
restores local graph; T PR branch now has commits that no longer exist
locally → CloneX must report "T PR #14 still contains the undone events;
retract?". Op log records publication as an *external effect* entry.
**K13. Ephemeral composition + release provenance.** V (ephemeral) CI
produced the evidence for release of D v0.9.2. Evidence must name V's state:
(D@x, H@y). If V's composed commit SHA is deterministic, cite it; else cite
tuple.
**K14. tdd-ratchet bot + cross-cutting change (real).** Adding a
cross-cutting feature with tests across T and D from U:
1. `R_E` (tests, red) in U touching T and D.
2. Publish → T PR, D PR get `r_T[R_E]`, `r_D[R_E]`.
3. T's CI bot pushes `bot_T` (ledger: pending) onto T PR branch; D's bot
   pushes `bot_D`.
4. Alice syncs: T PR branch = r_T─bot_T. U must absorb bot_T and bot_D as
   events (derived ids) on top of R_E: U = R_E─bot_T─bot_D (or merge).
5. `G_E` (impl) on top, publish: T PR = r_T─bot_T─g_T, D PR = r_D─bot_D─g_D.
6. Bots push again (`passing`). Sync. Merge PRs (merge commits on GH).
7. U absorbs merges.
Requirements: sync of *PR branches* (not just main) into the local stack;
remote-originated events inserted *under* local unpublished events (auto
rebase of G_E onto R_E+bots); U's own PR contains everything. Also: the bot
commits are per-tool events that U history must show. **If U were forced to
represent every tool PR bot commit, the U history gets 2×N bot commits per
cross-cutting change.** Fine, honest.
**K15. Ledger repair (real) + compositions.** tb and agent-harness mains were
rewritten; U pinned new SHAs. In CloneX world: U contains old reps. Sync
must see T main force-moved; with trailer identity, trees mostly equal
(ledger file differs); U applies a correction event for the diff and records
new correspondence; old U history keeps old reps (U main immutable). Filter
U at T then yields old reps then correction — ≠ T's rewritten history (E6
again). With provenance, the correction merge says "T := t40'", so filter
at T from that point on follows T's actual commits.
**K16. Repeated history + GitHub merge commits.** L and F both follow
branches in T (main and release). T's PR merges on each branch → merge
commits → U absorbs both.
**K17. Occurrence at subpath + write-back.** Sec contains R at
`src/ledger` only. Sec event touches it. R rep = R parent tree with
`src/ledger` replaced. R parent = R's main tip *as known*. Publication to R:
the rep's parent is R main, fine. But Sec doesn't have R's other files — to
build R's full tree it needs R's parent tree → Sec must have R's full commit
objects available (not checked out). **Sub-path compositions require the full
component objects locally.** Or: rep construction deferred to publication
time when R is fetched.
**K18. Two compositions both publishing the same component concurrently.**
Alice from U, Bob from V, both touch D. Standard race (F6).
**K19. An event that touches only composition-owned files and changes the
manifest** (adds occurrence) — structural event, not published to components.
**K20. Composition includes itself / cycle.** U contains M which contains
U? Content-containing compositions make cycles *finite at each snapshot*
(you contain an older state of yourself) — actually meaningful ("fixtures
contain a snapshot of this repo"). Filter at the nested path is a history of
U itself (older states). Weird but well-defined. Probably disallow in v1.
**K21. Agent-generated transaction touching repeated histories.** "Apply this
rename to every occurrence of T" — selector `occurrences(T)` must exist in the
query language.
**K22. Plain git user in U removes the manifest entry but not the dir.**
Structure and content disagree → cx reports "fixtures/trunc-old is no longer
declared; treating as plain U-owned files from commit u_k". Degradation
understandable.
**K23. Deterministic compose + committer identity.** Two people compose V
from same heads: same SHAs? Requires committer/date of synthetic commits to be
derived (from component commit metadata), never "now/me".
**K24. Deterministic filter + existing component reps.** When filtering U at
T produces a commit that should equal the existing T commit t41 (imported
from T), the filtered object must be *t41 itself*. Without provenance, the
filter would have to reproduce t41 byte-for-byte (author, committer, dates,
message, gpgsig, encoding, extra headers, parent order). gpgsig on GitHub
merge commits: GitHub signs its merge commits → U rep can't reproduce the
signature unless stored → **filter-by-regeneration cannot round-trip signed
commits; filter-by-provenance can.**

---

## Questions the reservoir forced (to be answered in model.md)

- Q-A: Is `filter(composition) == component` a law, or is the relationship
  *correspondence*? (E6, E7, F7, K15, K24 all say: regeneration alone fails;
  provenance makes it a law.)
- Q-B: What records that an occurrence's subtree *is* a component commit?
  (C9, C12, C14, E6, K24 → per-occurrence provenance is essential.)
- Q-C: Where does occurrence *intent* live (follow main / frozen / forked)?
  (C4, C10, C11 → in the composition's tree, versioned.)
- Q-D: Event identity carrier? (D5, D13, D16, G7 → message trailer, as a
  *set*; derived identity for commits without one.)
- Q-E: Does identity change force re-publication? (D5, D6 → keep
  representation content-stable; identity must not churn on split.)
- Q-F: How does rewrite propagate across repos? (D12, F7, K11 → supersession
  by same-identity + branch replacement; obsmarkers?)
- Q-G: Is composed history needed or only composed state? (H4, I3 →
  both; state is trivial, history is the product.)
- Q-H: Who creates composition sync commits? Can they vanish? (F1, G9.)
- Q-I: Must CloneX own the client? (J8, K12, K14 → local graph + op log +
  auto-rebase across histories is needed for the core workflows.)
- Q-J: Publication model? (F3, F5, J17 → idempotent desired-state
  convergence, components and composition independently.)
- Q-K: Immutability is per (history, remote, branch). (D3, K7.)
