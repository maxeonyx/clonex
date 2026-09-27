# Adversarial review of C-ANCHOR (model.md §3)

Target: `design/model.md` as selected (anchored occurrences, `Clonex-Base`
trailers, base derivation through parents, delta, `get_P`/`put_P` with
keep-merges, change-id overlay).

Method: I implemented §3 **literally** over real Git objects
(`lens.py`: rules 1–3 of `get_P`, `put_P` via `git merge-tree --write-tree`,
reachability-checked trailers, maxima in component ancestry), then built
minimal histories with plain git 2.55 and jj 0.44. Scripts:
`/tmp/claude-1000/-home-maxeonyx-agent-tools-workspace/d1bb7304-0863-4655-bda0-4bf326c5a042/scratchpad/anchor-attack/`
(`run_all.sh` reproduces everything; output is in `results.txt`). Scenario ids
`S01`–`S19` below refer to those scripts. Each result was observed in a run;
nothing here is predicted without a run unless it says **(reasoned)**.

Notation: `T` = trunc (root layout), `U` = composition with `T` at
`tools/trunc`. `a(x,y)` = merge with parents x,y. `[B: P t]` = trailer
`Clonex-Base: P t`. `d(c)` = derived commit `get_P(c)`.

**Verdict.** The core idea survives: state anchoring, the round-trip law
(rule 1), content fusion (S16) and "U is never rewritten because a component
was" (X28) all hold. But §3 as written has **four model-level defects**. Each
one has a working repair, and each repair changes the rules (they are not just
implementation work):

1. `get_P`/`base` are **not scoped to the composition's own history**. Manifests
   and trailers on *component* parents leak into the composition (S01, S15,
   and a trust hole). Nested adoption breaks L5 (S14).
2. Occurrence identity is **keyed by path**, and undeclared commits are
   **opaque** ("maps to none"). A move, or a deleted-then-restored manifest,
   produces a new root history (S02, S03). Extraction loses pre-declaration
   history (X14).
3. `put_P`'s merge base is wrong whenever the component history no longer
   contains `get_P(c)`: a rewrite, squash/rebase-merge, host-recreated commits,
   or a local amend of published work. The results are ghost un-reverts (S08),
   conflicts with an empty delta (S09) and spurious conflicts (S07).
4. Rewriting beneath an adoption has **no invalidation rule**. The tdd-ratchet
   flow then produces a history the ratchet must reject, with the bot's ledger
   laundered onto content it never judged (S17).

Also: the claim that "jj and `--rebase-merges` are safe" is **false** for both
(S05). The claim that change-id is non-load-bearing is **false** for merge
quality and rewrite propagation (S06, S07, S17). Several plain-Git outcomes turn
a *composition-local* intent into a *publishable component commit*: a pin-back
becomes a revert (S04), picking a side becomes an "ours" merge (S13), a squash
becomes a jump copy (S11, S12). An automatic publisher (G2) would push all of
them.

---

## 1. Findings (severity-ranked)

### A1. Base/get traversal crosses into component parents: other occurrences' bases leak — **needs model change**

§3 computes `base(c,P)` and `get_P(c)` from *all* parents. It evaluates a
parent's own manifest to decide whether P is declared there. An adoption's
component parent can itself be a composition: a nested U, or a fixture of this
very repo. When it is, *its* occurrences with the same path join *our*
candidate set.

S01 (nested, colliding path). M declares `tools/trunc` (frozen at t0) and `at`
(U). U declares `tools/trunc` at t2.
```
T:  t0 ─ t1 ─ t2
U:  u1 = a(u0, t2) [B: tools/trunc t2]
M:  m1 = a(m0, t0) [B: tools/trunc t0]   m2 = a(m1, u1) [B: at u1]
get_tools/trunc(m2) = new commit, parent t2, tree = t0     ← observed
```
That is a revert of t1..t2 that nobody authored, and it is publishable to T.

S15 (self-containment, the K20 case). U adds `fixtures/self` = a U commit `f`
from an unmerged feature branch that had upgraded trunc to t2. `get_tools/trunc`
of main then derives on t2 with t0's tree. The same revert follows.

Trust variant **(reasoned)**. `base()` is defined by trailers on *any*
ancestor. Suppose a T commit from an outsider's PR carries
`Clonex-Base: tools/dotsync <x>` and reaches `x` through a `-s ours` merge,
which leaves T's tree unchanged. That T commit then feeds U's *dotsync* base
set. §7 says trailers are "verifiable", but only reachability and tree are
checked. *Who* may make the claim is not checked.

Repair (the "scoped lens"). For occurrence P of composition C at commit c:
- a parent p of c that is named by c's `Clonex-Base: O p` is a **component
  parent**. It contributes `get_{Q}(p)` only when P = O/Q (nesting), evaluated
  in p's own composition space. It never contributes for unrelated paths.
- every other parent is a composition-side parent and contributes
  `get_P(p)`.
- trailers and manifests are read only from commits reached along
  composition-side edges.

This also defines L5, which §3 cannot currently express: rule 3 maps an
undeclared `at/tools/trunc` to none.

### A2. Nested adoption breaks L5 and reintroduces jump commits — **needs model change**

S14. M contains U at `at`. Alice, working in M, syncs the nested occurrence:
`m2 = a(m1, t2) [B: at/tools/trunc t2]`. Then:
```
get_at(m2)            = new U commit, parent get_at(m1) only   (t2 is not a U commit → dropped)
                        message carries "Clonex-Base: at/tools/trunc t2" (meaningless in U)
get_tools/trunc(that) = new T commit, parent t0, tree t2        ← jump copy, not t2
```
Repair: when `get_O` derives a commit, it must **translate** the trailers
`Clonex-Base: O/Q t` into `Clonex-Base: Q t`, keep t as a parent of the
derived commit (so the derived U commit is itself a U adoption merge), and
strip every trailer for paths outside O. Rule 1 in U then gives `t2` exactly.
Without this, K3 ("Alice fetched M, not T") and blind C7 cannot work.

### A3. Occurrence identity is keyed by path; undeclared parents are opaque — **needs model change**

S02 (move, K9/X15). `git mv tools/trunc tools/text/trunc` plus a manifest
update, with no trailer. `get_{tools/text/trunc}(mv)` = a **new root commit**.
Publishing then pushes an unrelated history to T.

S03. A plain-git user deletes `.clonex.toml`, edits trunc, and later restores
the manifest. `base()` still says t2, because it propagates through
undeclared commits, but `get_P` gives a **root commit**, because rule 3 maps
undeclared parents to none. The two definitions disagree.

X14/A5 **(reasoned from the same rule)**. Declaring an occurrence for an
existing directory (the help-test extraction) gives one root commit holding the
whole directory. The 200 prior commits are lost. In-flight branches forked
before the declaration are folded into the merge's derived commit, under the
merger's authorship.

Repair:
- (a) Key occurrences by a stable `name` (already in the manifest), with the
  path resolved per commit from that commit's manifest. The trailer becomes
  `Clonex-Base: <name> <sha>`.
- (b) Replace "maps to none" with **transparent reconnection**, as
  git-filter-repo does. An undeclared commit that has the path is filtered as
  a plain path filter with no base. An undeclared commit without the path
  passes through `cand` unchanged.
- (c) Publication, not derivation, is gated by declaration.

These rules remain local to each commit, so L3 still holds.

### A4. `put_P`'s merge base is wrong when the component no longer contains `get_P(c)` — **needs model change**

§3 uses `mergebase(get_P(c), t)` computed in component space. That is correct
only when `get_P(c)` is an ancestor of the landed history.

S09 (F7/K15/X6, the real tb repair). T is rewritten `t1',t2'`: same code,
repaired `.test-status.json`. U follows T with an **empty delta**. Adopting
`t2'` uses base t0, so ours=`v2` against theirs=`v2-repaired`: **conflict**,
although U holds nothing of its own.

S08 (host recreated our commit). Our derived `e_T` lands on T main as a
squash or rebase-merge copy `s` (new SHA, same tree), and then `y` reverts it.
Adopting `y` uses base t2, so ours keeps FEATURE and theirs equals the base:
**the upstream revert is silently undone**. `get_P` becomes a merge that
re-applies the feature. The next publish pushes the un-revert to T main.

S07 (X17). The reviewer adds a suggestion commit `w` on top of the published
`g_T`, and Alice amends `g` locally. `mergebase(g*_T, w)` = t0, which gives a
**conflict**. With base = the published predecessor `g_T`, the merge is clean
(observed).

Repair:
- **Empty delta**: adoption is a *replace*, `tree := tree(t)`, whatever the
  ancestry. This is the only sane meaning of "follow".
- **Non-empty delta**: use the *anchor*, not `get_P(c)`. When `base(c)` is an
  ancestor of t, use today's rule. Otherwise **rebase the delta**, as
  `merge3(base = tree(base(c)), ours = tree(c)[P], theirs = tree(t))`.
- **Base hint**: when a derived commit in `get_P(c)`'s ancestry was published
  and has since been replaced or recreated (found by change-id, or by jj
  predecessors), use the published predecessor as the base.

This last point makes change-id *load-bearing for merge quality*. §3's claim
must be weakened to "correctness of *content* does not depend on it; freedom
from spurious conflicts and ghosts does".

### A5. Rewriting beneath an adoption: no invalidation, so the ratchet history is broken — **needs model change**

S17: K14 end to end with real jj.
```
U: u1 ─ r(red) ─ a1 = a(r, bot_T) [B: tools/trunc bot_T]
T PR:  t0 ─ r_T ─ bot_T(.test-status: pending)
reviewer: "change the red test"  →  jj edit r; jj auto-rebases a1 keeping bot_T as parent
get_T(a1*) = a(r*_T, bot_T), message "Adopt tools/trunc at …"
   r*_T parent t0 (never recorded by the bot)
   merge tree: new test + bot's "pending" record for the OLD test
```
The push is a fast-forward, so no force-push is needed. The resulting T PR
history contains the old red, the bot's record, an unrecorded new red, and a
merge that grafts the old ledger onto new test content. The ratchet judges
every snapshot. The new red was never recorded, and the ledger in the merge
attests to a test it did not run. The ratchet must reject this, or worse, may
accept a laundered ledger. §7's default ("append if foreign commits present")
is exactly what happens here, and it is the wrong default for attesting bot
commits.

Correct flow: drop `bot_T` (abandon a1 in U), force-update the T PR to
`t0 ─ r*_T`, re-dispatch the bot, and re-adopt. Force-updating a PR branch is
normal GitHub PR workflow.

Repair:
- An adoption is **stale** when the component commit it adopted descends from a
  derived commit that the rewrite superseded.
- Staleness is detected by change-id (`Clonex-Change` on `r_T` equals `r*`'s
  id) or by jj's predecessor graph. Change-id is load-bearing again.
- Publish refuses a stale adoption and offers two choices: *drop foreign
  commits above the rewrite* (force-update the PR branch, re-run the bots) or
  *keep them* (an explicit merge).
- Component policy (`.clonex.toml`: `foreign_commits = "attesting"` for
  ratchet repos) selects the default.

### A6. "jj and `--rebase-merges` are safe" is false — **needs model change** (immutability) plus a documented degradation

S05. The branch is `r ─ a1 = a(r, bot_T) ─ g`. U main moves, and the branch is
rebased onto it.

| Operation | Observed |
|---|---|
| `git rebase --rebase-merges main` | rc=0, **silent**. Git re-creates the adoption with a fresh ort merge. The merge base is t0, which is T-shaped and at the root, so T's changes since t0 (`test.txt`, `.test-status.json`) land **at U's root**. `tools/trunc/.test-status.json` disappears. |
| `git rebase main` | Replays `r_T` and `bot_T` as U commits and puts T files at U's root (the known degradation). |
| `jj rebase -b feat -d main` (the default `-b @` form) | "Rebased 6 commits". jj **rewrites the foreign commits** `r_T` and `bot_T` into U-shaped commits (`bot'` has `.clonex.toml`, `README` and `.test-status.json` at the root). The original `bot_T` is no longer reachable, so the trailer is ignored and `get_T` gives a derived commit, not `bot_T`. The tree at the head is fine. The anchor and the SHA are lost. |
| `jj rebase -s r -d main` | Safe: the adoption keeps `bot_T` as its parent. |

Repair:
- `clonex init` must configure jj's `immutable_heads()` to include every
  commit named in any `Clonex-Base` trailer, and every commit with no manifest.
  That makes `-b` stop at the foreign commits.
- Document that plain Git has no equivalent: `-r` is *not* safe. A `pre-push`
  hook or `clonex check` should detect component-root files that appear at the
  composition root.
- Fix the text in §6 "Known costs".

### A7. Composition-local intents become publishable component commits — **needs documented degradation plus publish guards**

- S04 (pin-back, B13/C4). Branch A pins trunc back to t0 with a valid set-base
  commit (rule 1 gives t0). After an ordinary `git merge` into a branch still
  on t2: base = max{t0,t2} = t2, and tree = t0. `get_P` is then a **revert of
  t1..t2** on T.
- S13 (the real "never pick a side" rule). Two U branches adopt the divergent
  t45 and t44'. A plain-git user resolves `tools/trunc` by taking ours.
  `get_P(m)` = `a(t45, t44')` with tree == t45, an **"ours" merge in T**. When
  T later merges them properly and U adopts, the result is clean but `b.txt`
  loses t44's change. The deletion persists and publishes.
- S11 (cherry-pick `-m1` of an adoption). The trailer names an unreachable SHA
  and is ignored. The result is a **jump commit** copying t1..t2.
- S12 (GitHub squash-merge of a U PR). The adoption's second parent is lost.
  The derived commit is a single squash copy of `r_T+bot_T` on t0. Recovery
  works if T's merge is adopted first (observed: clean, `get == tm`). If U CI
  publishes first, the copy goes to T main, **bypassing T's PR, bot and
  ratchet**.
- S10 (`git subtree pull`-style merge without a trailer). T's commit is a
  parent with an equal tree, but `get_P` still makes a jump commit.

Repairs:
- (a) **Implicit adoption**: a parent that is a component commit whose root
  tree equals `tree(c)[P]` counts as `[B: P parent]`. This is local and
  deterministic, and it fixes S10.
- (b) **Publish guards** refuse by default to push a derived commit that:
  - has a tree equal to an *ancestor's* tree (backwards move: S04);
  - is a merge whose tree equals one parent's tree while the other side has
    changes (ours-merge: S13);
  - is non-merge with a tree equal to a known component commit outside its
    ancestry (jump or copy: S11, S12).
- (c) Recommend that composition repos disable squash and rebase merges, as
  the tool repos already do.
- (d) G2 (automatic publication from U main) is unsafe without (b).

### A8. The change-id derivation breaks L4, L3 and D6 for plain Git — **needs model change**

S06. The same U commit is rebased by plain Git. The author, date, message,
tree and derived parent are all identical, yet the derived SHA changes
(`691a7ef2` becomes `6aa2548c`), because `Clonex-Change` = the composition
commit's SHA. Consequences:
- every plain rebase or cherry-pick force-updates every PR (D2 and D6 fail);
- a jj commit rebased by plain Git loses its `change-id` header, so the same
  thing happens to it;
- with `git.write-change-id-header=false`, the jj id lives only in jj's store,
  so a plain clone derives a different id (L3 fails across machines).

Conversely, if the owner rejects product trailers in component commits (blind
T1/I4), derived commits lose the only field that separates two
same-parent/same-tree/same-author/same-second commits, for example
`jj duplicate`, which preserves the author timestamp. Distinctness then also
depends on the overlay.

Repair:
- the id must be a function only of rebase-invariant fields;
- use the jj `change-id` header when present;
- otherwise use `H(author ident + author date + message + patch-id)`;
- never derive the id from the composition SHA;
- specify that an existing `Clonex-Change` is **kept, never replaced**
  (L5 needs this);
- never copy jj's `change-id` *header* onto derived commits. With C3, two
  derived T commits would carry the same header, and jj in T would report
  them as divergent.

### A9. Sync placement is unspecified, and it decides K14 and X11 — **needs model change** (spec)

S19. Suppose green `g` was made on `r` before syncing, and sync applies
`put_P(head, bot_T)` literally. Then `get_T` = `a(g_T, bot_T)`. The green
commit **does not descend from the bot commit**, so the ratchet rejects it.
X11 has the same problem: adopting a squash at the head leaves the duplicate
L1 commits in L2's PR.

Repair: sync adopts at the **deepest composition commit c whose `get_P(c)` is
an ancestor of t** (the published prefix), then rebases the descendants with
`-s` semantics, as A6 requires. §3 must state this. "may be rebased" is not
optional.

### A10. Multi-base occurrences: defined but not specified — **needs documented degradation**

S18. U main adopts t45 and a U branch adopts the divergent t44'. A U merge
resolves them.
- `get_P(m)` = `a(t45, t44')` with U's resolution and **U's message**
  ("Merge branch 'b'"). The message is published into T. A U-side
  `Merge pull request #N` autolinks to *T's* PR #N.
- If T later merges them identically, adoption is clean and `get == tm`
  (observed).
- If T's resolution differs, adoption goes through a criss-cross (two merge
  bases, an ort virtual base) and conflicts exactly on the resolved region
  (observed). This is a genuine conflict, and acceptable.
- A plain user who picks a side gives S13.

Also underspecified:
- the **parent order** of derived merges. AGENTS.md requires "merge main
  *into* the feature branch", so the first parent becomes the feature side,
  and a fast-forward publish breaks T's `--first-parent` history;
- what the "delta" of a multi-base occurrence is.

Repair:
- order the parents of a derived merge by the component's followed branch
  first;
- publish derived merges only as PR heads (let T make its own merge), never as
  fast-forwards onto a protected branch;
- define delta against the ort virtual base.

### A11. Publishing to the followed branch bypasses component gates — **needs model change** (publication target)

"Publish = push `get_P(head)` to the occurrence's remote/branch". For a
`follow = branch` occurrence, that is T main. **(reasoned from GitHub
behaviour)** A derived U merge (A10) or a jump (A7) pushed to T main
fast-forwards past T's CI and ratchet. If it contains a T PR head, GitHub marks
that PR "merged", so `integrated-ci` never gets recorded and `latest_ci_green`
goes red.

Repair: the publication target of derived commits is always a per-change PR
branch. The followed branch only ever moves by *adoption*. Direct pushes need
explicit per-occurrence permission.

### A12. Smaller items

**Needs documented degradation**
- Pointer bumps are not eliminated, they are renamed. Every sync is one
  adoption merge per occurrence. K14 adds 2 per tool per cross-cutting change.
  X20 still needs a commit on every open U branch. §2's "B3 ✓ one U commit"
  should say so.
- Derived messages leak composition metadata into components: "Adopt
  tools/trunc at …" (S17), U merge subjects, and `Clonex-Base` trailers copied
  verbatim. Strip composition trailers on derivation.
- I4 (private description in a public component): derived message = `c`'s full
  message, with no per-occurrence override. Add an optional per-occurrence
  message section, which stays deterministic because it is in `c`.
- X31 (composition-local override that must never publish): the model's delta
  cannot tell "unpublished work" from "local patch", so publish would push the
  path-dependency override. Add a publish-excluded patch marker. Until then,
  document it.
- `git revert` of an adoption produces a component revert commit, not an
  un-adopt. This is correct Git meaning, but surprising. Pair it with the
  S04 guard.
- F8 (vendoring into tb) makes tb a composition whose repo then contains the
  third-party history. Allowed, but heavy.

**Cosmetic**
- L2 (`put_P(c, get_P(c))`) adds a redundant parent when the base is already an
  ancestor, and writes virtual commits into U's graph.
- jj sets the author timestamp when a change is *created*. Checked: 0.44 keeps
  it stable across snapshots and rebases. Derived commit dates therefore show
  when work started, not when it finished.
- "committer := author" makes derived commits claim that the U author
  committed them to T.

### What committer := author does and does not break (question 6)

- In jj 0.44 the author timestamp stays stable across snapshots, describe and
  `rebase -s` (checked). Git amend keeps the author date. The rule is
  therefore stable.
- I found no collision of two genuinely different derived commits while
  `Clonex-Change` is present.
- Two cases remain:
  - non-determinism enters through the *change-id*, not through dates (A8);
  - without the trailer, `jj duplicate` onto the same parent with an
    unchanged tree fuses two changes that were meant to be distinct (D7).
- Signing required by branch protection conflicts with L3. §7 already
  acknowledges this.

---

## 2. The requested probes, answered directly

**Q2: divergent adoptions merged in U.**
- `base = {t45, t44'}`. `get_P(m)` is a derived component merge
  `a(t45, t44')` carrying U's resolution and U's message.
- Publish pushes that merge, fast-forwarding T main (A11).
- If U's resolution differs from T's later merge, the next adoption conflicts
  on exactly that region (S18). This is genuine and acceptable.
- If a plain user picks a side, T gets an "ours" merge, and t44's work is lost
  permanently, even after T merges properly (S13).

**Q3: when the component-space merge base is wrong.**
- rewrite (S09);
- squash or rebase-merge followed by a revert (S08);
- local amend plus a remote commit on top (S07);
- editing beneath a bot adoption, where the merge base is correct but the
  intent is wrong (S17).

Criss-cross itself is handled by ort's virtual base (S18).

**Q4: repeated occurrences, nesting, self-containment.**
- Equal base with edits that differ in one file gives two sibling T commits
  with the same change-id. Equal edits fuse (S16, holds).
- Nesting where M also declares T directly leaks through the colliding path
  (S01).
- Nested adoption breaks L5 (S14).
- A self-fixture of a non-ancestor corrupts the other occurrences (S15).

**Q5: plain Git operations.**

| Operation | Outcome |
|---|---|
| rebase | S05, S05b |
| rebase `-r` | S05 |
| squash-merge of a U PR | S12 |
| cherry-pick of an adoption | S11 |
| revert | A12 |
| subtree-style merge | S10 |
| manifest deletion | S03 |

A move is covered by S02.

**Q6: committer := author.** See above. The rule itself is sound. The change-id
derivation is the defect.

**Q7: K14 end to end.** Red publish, bot, adopt and green all work *only* with:
- sync placement at the published prefix (A9);
- jj `-s`-style rebases, or immutable foreign commits (A6).

Editing red after the bot breaks the ratchet (S17, A5).

---

## 3. Replay: every [OWNER-DECISION] episode

✓ = handles · ~ = handles with a known degradation · ✗ = fails as specified.

| Episode | Result | Note |
|---|---|---|
| lead B10 (where review happens) | ✓ | The U PR shows the full diff. Tool PRs are still needed for tool CI and the ratchet. Unsafe if U CI auto-publishes before the tool PRs merge (A11). |
| lead D10 (backport identity) | ~ | Two derived commits carry the same `Clonex-Change`. Lost by squash (overlay). |
| lead F15 (URLs in tree) | ✓ | In the manifest, with local overrides. |
| lead G2 (auto-publish from U main) | ✗ without A7(b) guards | It would push the S04 reverts, S13 ours-merges and S12 jump copies. |
| blind A10 (two views, one component) | ✓ (option b) | Separate repos, visible after publish and sync. |
| blind C3 (same edit in both occurrences) | ✓ (option a) | S16. |
| blind C7 (repeated component in a nested composition) | ✗ | S01 and S14. Fixed by A1 and A2. The answer then becomes (b), independent occurrences, and the product should add a divergence warning. |
| blind F8 (vendor third-party crate) | ~ | Option (a) works. tb's repo then contains the upstream history. Nested L5 needs A2. |
| blind G8 (composition state vs component state) | ✓ = option (b) | The U remote carries unpublished T content. This leaks, so the owner must accept it. |
| blind H6 (plain clone of a composition) | ✓ | Content is in the tree. |
| blind I4 (description leak) | ~ | The full message is always copied. A12. |
| blind X5 (atomic) | ✓ = option (c) | Only U main is atomic. |
| blind X18 (ephemeral death) | ~ | Unpublished work is only as durable as a push of the composition branch somewhere. |
| blind T1 (trailer vs purity) | ~ | Dropping the trailer worsens A4 and A5 and opens the D7 fusion. |
| blind T13 (workspace routing) | n/a | Not a model question. |

## 4. Replay: blind section X

| # | Result | Why |
|---|---|---|
| X1 | ~ | Works only with A9 placement and A6 immutability. Editing a published red is ✗ (A5). |
| X2 | ~ | The model has no component history constraints (no squash refusal for ratchet repos). |
| X3 | ~ | Replace or append policy works. Remote commits on top conflict spuriously (S07). |
| X4 | ✓ | The unlanded part stays as U delta, visible as delta. |
| X5 | ✓ | Option (c). |
| X6 | ~ | S09: conflicts with an empty delta. A4 fixes it. The re-pointed tag is just a new adoption. |
| X7 | ✓ | Parts are per occurrence (S16). |
| X8 | ✓ | U contains the content. No cross-repo read is needed. |
| X9 | ~ | U CI judges L with L's own `ledger.yml`. This is a policy question, not the lens. |
| X10 | ✓ | Adopt h2, then adopt the merge. |
| X11 | ~ | Works with A9 placement. Without it, duplicate L1 commits appear. |
| X12 | ~ | No rewrite map. The attestation mapping is heuristic. |
| X13 | ~ | S09 and S08 make recovery conflict-prone. A4 fixes it. |
| X14 | ✗ | A3: non-retroactive declaration and the in-flight squash. |
| X15 | ✗ | S02. A3 fixes it. |
| X16 | ✓ | Only tb's derived commit changes. |
| X17 | ~ | S07. |
| X18 | ~ | Owner decision. |
| X19 | ✓ | A query over derived and adopted commits. |
| X20 | ~ | Follow plus auto-adopt removes the red state, but every open branch needs an adoption commit. |
| X21 | ~ | Proposing edits against main needs delta-rebase (A4 repair). |
| X22 | ~ | Designed (§7 partial paths), not built. |
| X23 | ✓ | Local undo. Remote effects are reported. |
| X24 | ~ | U's 3-way merge of ledger files. T's rules re-judge only on publish. |
| X25 | ✓ | Adopts only what exists. |
| X26 | ✓ | A connected history. Shared change-id. |
| X27 | ~ | A rerun creates new change-ids, so derived SHAs differ from the published ones unless the script is idempotent at the U level. |
| X28 | ✓ | Strong. Old SHAs remain as adoption parents. |
| X29 | ✓ | Bot commits are distinct adopted commits. |
| X30 | ✓ | Like branches in one repo. |
| X31 | ✗ | No publish-excluded local patch (A12). |

---

## 5. Consolidated rule changes proposed for §3

1. **Scoped traversal** (A1, A2): component parents are identified by the
   parent commit's own `Clonex-Base` trailers. They contribute only to
   occurrences nested under the named occurrence. Deriving translates trailers
   to be relative to the occurrence and keeps the named parents.
2. **Occurrence identity by name** (A3). Undeclared commits are transparent,
   not none. Declaration gates publication, not derivation.
3. **Adoption semantics** (A4): an empty delta replaces. A non-empty delta
   merges from the anchor when the anchor is an ancestor of t, and otherwise
   rebases from the anchor. The published-predecessor hint uses change-id.
4. **Stale adoptions** (A5), with a per-component `foreign_commits` policy.
5. **Sync placement** at the published prefix, with `-s`-style rebases (A9).
   `clonex init` marks foreign commits immutable in jj (A6).
6. **Rebase-invariant change-id**, kept when already present, never copied as
   a header (A8).
7. **Implicit adoption** for a component parent whose tree is equal (A7a), and
   **publish guards** for backwards, ours-merge and jump commits (A7b).
8. **Publication targets are PR branches**. Followed branches move only by
   adoption (A11).
9. Restate §3's overlay claim: "change identity is not needed for *content*
   correctness. It is needed for merge-base selection after rewrites and for
   invalidating stale foreign commits."
