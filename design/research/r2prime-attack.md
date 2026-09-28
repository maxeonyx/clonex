# Adversarial review of R2′ (constraints.md §4)

Dated 2026-09-28. Target: the claim that R2′ ("self-verifying
representatives") satisfies K1–K13, with the listed costs as its only
degradations and K1×K8 as its only real trade-off.

**Verdict: the claim fails.** The weak point is the rule that makes the
composition commit the representative of its own derivation
(`ρ(get_P(u),P) = u`). Once that rule holds, some representatives have
*both* a P-only parent (a component lineage) and a full-tree parent (a
composition commit). No tree for such a commit is neutral outside `P`, so
an ordinary `git merge` of it into the composition deletes, lands, reverts
or resurrects content outside `P`. That is exactly R2′'s own headline flow
(UC18: GitHub merges U's change into trunc). Real git reproduces every
case below with exit code 0. The same rule also makes `ρ` multi-valued,
and so non-deterministic across heads, in the presence of the Locality law.
Separately, `get_P` is not pure: it falls back to the network. And
verification checks only `P`, so a forged representative can smuggle
content elsewhere in the tree.

All of these can be repaired. One repair sketch, *closed lineages plus
publication markers* (§4), fixes every silent-wrong case found here in the
experiments. It keeps the K1×K8 trade-off and adds one ordinary merge
commit to U per publication. K1–K13 are not jointly unsatisfiable
(§5), with one exception: K4, as worded, contradicts UC24, which K4 itself
cites.

Experiments: `$SCRATCH/r2attack/x1.sh … x9.sh`, run with real git 2.55.0
and jj 0.44.0. Representatives were built by hand with `git commit-tree`.
Shared setup (`common.sh`):

```
T:  t0 ─ o (outsider)          U: h0 (README, tools/dotsync/d=D0)
                               h1 = merge(h0, ρ(t0))          ρ(t0): tree = only tools/trunc
U change u on h1: tools/trunc/a=A2, tools/dotsync/d=D1, README=u2      x = get_T(u) (parent t0)
T main: m = Merge PR #5 = (o, x)                                        ρ(o): parent ρ(t0), P-only
```

---

## 1. Findings (by severity)

### F1. UC18 as written deletes the umbrella — **violates K6, K1** (x1)

In R2′, `ρ(m)` has parents `ρ(o)` and `u`. Its tree is "`ρ(first parent)`'s
tree with P replaced", and that is **P-only**, because `ρ(o)` descends from
T's root. GitHub always makes the target branch the first parent, so every
GitHub merge of a composition-made PR has this shape. `git merge ρ(m)` into
`u`:

```
merge base = u (full tree);  ours = u;  theirs = ρ(m) = only tools/trunc
→ rc=0, "delete mode README", "delete mode tools/dotsync/d"
U after adoption: tools/trunc/a.txt tools/trunc/b.txt       ← everything else gone
```

The deletion is silent. It happens whenever U has not touched the deleted
files since `u`; where U has touched them, Git reports modify/delete
conflicts instead.

### F2. Any other tree for `ρ(m)` fails too: UC23 lands the other components' parts — **violates K6** (x2, x4)

- **Tree from the composition-commit parent** (`tree(u)` with P := m, the
  obvious repair). U main does not contain `u`, because U's PR is still
  open and dotsync hasn't merged `y` (UC23). Adopting trunc's `m` into main
  makes `u` an ancestor of main. Main silently receives `README=u2` and
  `tools/dotsync/d=D1`: the unmerged dotsync part and the umbrella part of
  an unreviewed U PR, landed by trunc's merge button (x2, rc=0). The only
  alternative tree, P-only, gives F1. Git ancestry is all-or-nothing, so
  "main contains u's trunc part but not its dotsync part" cannot be
  expressed while `u` is the carrier.
- **Two composition parents.** T merges `x1` (from `u1`) then `x2` (from
  `u2`), where `u1` and `u2` are parallel U branches. `ρ(m2)` inherits
  `u1`'s non-P tree through `ρ(m1)`. U main, which already merged both,
  adopts `ρ(m2)`. The merge bases are {u1, u2}, ort builds a virtual base,
  and theirs lacks u2's README change: **u2's README change is reverted
  silently** (x4: before `README=u2`, after `README=u`).

The only representative that is neutral outside P under ordinary 3-way
merges is one whose parents all have the same non-P tree. P-only lineages
mixed with composition commits never do.

### F3. Bot commit × amend resurrects content across components — **violates K6** (x3)

`u` is published as `x`. U then amends `u` privately: it removes a stray
`tools/dotsync/secret.env` and leaves the trunc part unchanged. `u*` keeps
the author date and message, so `get_T(u*) = x` as well (checked). The
ratchet bot pushes `b` on `x`. With `ρ(x) = u` (the representative that
was published), `ρ(b)` has parent `u`. Then `git merge ρ(b)` into `u*`:

```
rc=0 → tools/dotsync/secret.env re-created, plus tools/trunc/.test-status.json
```

The doc says UC21 is "the same as plain Git". It is not: plain Git would
resurrect only trunc content in trunc's repo, but here a *trunc* bot commit
brings back *dotsync* content in U. jj shows a divergence only if `u` and
`u*` share a change-id header. A plain-Git user of U sees nothing.

### F4. `ρ` is multi-valued, so representatives are head-relative and K8/UC29 fail — **violates K8 as claimed, and "exactly one representative"**

The Locality invariant says that rebasing or cherry-picking `u` over
commits that don't touch P leaves `get_P` unchanged. So `u` and its rebase
`u′` both satisfy `get_T(·) = x`, and so do `u` and a cherry-picked copy
(UC27). The law `ρ(get_P(u)) = u` then asserts `u = u′`. **The two laws
are jointly inconsistent** whenever both commits exist.

Minimal case: Alice adopts `b` into a head containing `u`, while Bob adopts
it into a head containing `u′` (a rebased U PR). They produce different
`ρ(b)`, with different parents and different non-P trees. After their
branches merge, U holds two representatives of `b` for good (K9). UC29's
"both people create identical objects" holds only for lineages that never
touch a composition commit.

The lookup rule is unspecified: "the representative of `x`" could be looked
up globally or among `ancestors(head)`.
- Global lookup gives F3 and F2.
- Head-relative lookup makes `ρ` a function of the target head. That is
  fine at level 1, provided the doc stops saying "ρ is pure given the
  composition's history" and "UC29 identical".

Reconstruction cannot pick a single winner either: two different
representatives, whose non-P trees differ, can **both** reconstruct `t`
exactly, because reconstruction ignores everything outside P.

### F5. A valid-looking representative can smuggle content outside P — **violates K12 (UC31)** (x5)

Verification only looks at `P`: strip the trailer, take `tree[P]`, take
the parents' derivations. A commit that reconstructs `o` exactly can also
rewrite `.github/ci.yml`:

```
reconstructs to o? YES
git blame .github/ci.yml → author out, summary "outsider"
```

The outsider's name, date and message are attached to a CI change they
never made, and `clonex status` would label it "proven trunc commit o".
The same smuggling works through an extra parent with no P, if parents
whose derivation is "none" are dropped during reconstruction.

**Is a forgery that *does* reconstruct a forgery at all?** At P, no: its
content really is `t`'s, and a copied `clonex-gpgsig` even reconstructs a
validly signed `t`. Two things remain.
- It can smuggle content outside P (above).
- Reconstruction proves *correspondence*, not *membership*. An attacker can
  invent a commit `t` that trunc never had, and the claim is still
  "proven". Whether `t` is in trunc is a level-2 question (reachability
  from a component ref). The doc should word K12's guarantee that way.

**Repair:** verify the whole representative, not its projection.
Recompute `ρ(t,P)` from its definition and compare SHAs. Under closed
lineages (§4) this reduces to three checks: "non-P is empty, the parents
are representatives, and it reconstructs exactly".

### F6. `get_P` is not pure: the fetch fallback breaks K8 and the layering — **violates K8 and the stated law "get_P is pure"; level-1→level-2 dependency**

The fallback "fetch `t` by SHA from the component" needs the component's
URL. That URL is level-2 configuration, which the doc says level 1 never
reads (D1).

Minimal case (UC25 + UC15): a plain `git rebase` rewrites `ρ(t)`'s
committer, so reconstruction fails.
- Machine A is online, fetches `t` and proves it. Its `get_T` for the
  descendants has parent `t`.
- Machine B is offline, or asks after trunc's force-push let `t` be GC'd.
  It derives an own-work commit instead, and every descendant's derived
  SHA changes.

So the same objects give different commits (K8). A result can also flip
over time: `t` is GC'd, published derived commits no longer re-derive, and
publication then sees phantom changes.
- When `t` was committed by someone other than its author, or was signed
  (all GitHub merges, and the 184/872 case), the own-work copy is a
  **publishable duplicate of a commit trunc deliberately removed** (UC15).
- When committer = author and there are no extra headers, the lens
  reproduces `t` anyway, which hides the bug in simple tests.

**Repair:** make `get_P` a function of objects only. An unreconstructible
claim gets a deterministic derivation plus the state *unproven*. A fetch
can only upgrade that to "equivalent to t" as an annotation. Publication
refuses to push unproven claim-bearing commits. Any originals needed
offline go in extra refs (K7 allows this), or the loss is accepted as
coarsening.

### F7. jj `immutable_heads()` protection freezes the user's own work — **degradation beyond stated tolerance (UC28)**

The doc protects representatives with `immutable_heads()`. In jj, the
ancestors of immutable heads are immutable too. In R2′, `ρ(b)`'s ancestors
include `u` and all of U's history below it. So adopting one ratchet bot
commit makes the user's unpublished stack under `u` immutable. For example,
`w`, which touches only dotsync, can no longer be amended without
`--ignore-immutable`.

This is harmless under closed lineages: a representative's ancestors are
only representatives.

### F8. Duplicate carriers undo reverts (the "squash hazard" made general) — **violates strict K6; see §5** (x7, x8)

Any design in which U main receives trunc's merged `x` through an object
other than `u` hits this. UC23 forces such a design, because of F2.

In the minimal case (x8):
1. Main adopts `m` through a P-only copy `ρ(x)`.
2. Branch F holds `u` plus a plain-Git revert of its trunc part.
3. F runs "merge current main into the branch", as AGENTS.md requires.

The merge base contains neither carrier, so the revert is **silently
undone** (`a=A2`, rc=0). R2″ without markers (x7) has the same problem.

**Repair (x9):** when CloneX publishes `x`, it adds an *ours-merge*
`merge(head, ρ(x))` to the branch holding `u`. Its tree is unchanged and it
is append-only. `ρ(x)` then becomes a common ancestor. The revert is kept
(`a=A`), and main still has `d=D0`. Git's default history simplification
hides the shadow: `git log -- tools/trunc` shows the change once. The
marker is lost by a squash or plain rebase of F, and `status` can detect
that and re-add it.

### F9. GitHub squash of a U PR loses K1 and can produce a jump commit — **within K6's "coarser", but the walkthrough overclaims**

"The latest proposal whose tree matches" covers only PRs that end on a
representative. The UC14 PR (`u`, `ρ(b)`, then green `g`) ends on own
work, so `s[P]` matches no proposal. `get_T(s)` becomes a jump commit on
`t0` that contains red, ledger and green, and it can be published past
trunc's ratchet (as in anchor-attack S12).
- All proofs from a squash need the fetch (F6), because the parents
  differ.
- The adopted component commits vanish from U main: this is K1
  coarsening.
- Under closed lineages, K1 can be restored after the fact by re-merging
  the (pure, recomputable) lineage.

**Repair:** a publish guard (level 2), and state in the doc that squash
coarsens K1.

### F10. Smaller items

**Within tolerance, but should be stated in the doc:**
- **Nested trailers.**
  - `ρ(ρ(t,tools/trunc),at)` carries a U-relative trailer `Clonex-Source:
    tools/trunc t`. In an M that also mounts trunc at `tools/trunc`
    (anchor-attack S01), M reads that trailer as a proposal for its own
    path.
  - Only the *last* trailer should count at each level, with inner ones
    treated as message text.
  - Nested proofs by fetch need T's URL from inside M, which is U's level-2
    configuration.
- **Upstream rewrite × repeated occurrence (X6, UC15).** Each rewrite
  appends a full new representative lineage *per occurrence*, forever
  (K9). tb with two mounts after its repair means about 2×N extra commits.
  The content diff is correctly empty. This is a UC30 cost, not a
  correctness problem.
- **Cherry-pick (UC27).** The fetch-and-check path compares only
  `tree(t) == tree(c)[P]`, not ancestry. A cherry-picked representative
  "proves" `t` on a branch whose derived history lacks `t`'s ancestors.
  Content is safe; the later non-fast-forward is refused at level 2, as
  the doc says.
- **Extra headers (x6).**
  - `git fsck --strict` and `receive.fsckObjects` accept `clonex-gpgsig`
    and `clonex-change-id`.
  - `git cherry-pick` and jj rewrites drop them.
  - `git commit --amend` **keeps** them. An amended representative still
    advertises another commit's signature; it fails reconstruction
    harmlessly.
  - jj gives representatives hash-derived change ids, which are
    deterministic.
- **jj change identity.** Because the real `change-id` is moved to
  `clonex-change-id`, jj in U never shows component-side rewrites (UC12:
  owner amends in dotsync and pushes) as divergent. The doc's UC21 claim
  holds only for U-made commits.
- **Shallow checkouts (UC32/K13).** In depth 1, `get_P(HEAD)` needs history
  back to the nearest proven representative, with unbounded depth over
  own-work chains. Builds are fine. For identity questions, use a treeless
  or blobless partial clone rather than depth 1.
- **UC10 conversion.** The first adoption is an unrelated-histories merge
  whose root representative adds `tools/trunc` over an existing gitlink.
  That is not "just `git merge`"; it needs one special step.
- **Verification cost (UC30).** Recursive reconstruction is about 2,000
  hashes (milliseconds), memoized. After F6's repair it needs no network.

---

## 2. Use-case replay against R2′ as written

| UC | Result |
|---|---|
| 1, 4, 5, 8, 9, 11, 12, 16, 17, 22, 33 | ✓ (UC4 identity after a rebase needs network: F6) |
| 2 | ✓ at P; F1 destroys the tree it logs |
| 3 | degraded as stated |
| 6, 15 | ✓, with history duplicated per occurrence per rewrite (F10) |
| 7 | ✓ with trailer scoping (F10) |
| 10 | first adoption is special (F10) |
| 13 | ✓ when reconstructible; network-dependent otherwise (F6) |
| 14 | ✓ without amend; ✗ with amend (F3) |
| **18** | **✗ F1** (literal rule), **✗ F2** (repaired rule) |
| 19 | ✓ (the merge base includes `u`) |
| 20, 21 | private ✓; published amend + reviewer/bot **✗ F3** |
| **23** | **✗ F2 / F8** |
| 24 | ✓; K4's wording is wrong (§5) |
| 25 | degraded; network-dependent (F6) |
| 26 | K1 lost; jump commit possible (F9) |
| 27 | duplicate representatives (F4); tree-only check (F10) |
| 28 | F7 |
| **29** | **✗ as claimed** (F4) |
| 30 | ✓ with a cache |
| **31** | **✗ F5** |
| 32 | degraded (F10) |

Episode reservoir, object-level items only:

| Episode | Result |
|---|---|
| episodes C1–C16 | ✓ |
| episodes C12 | ✓ (occurrence upgrade = a lineage merge) |
| episodes D2, K1 | ✓ under closed lineages; R2′ gives F3 |
| episodes D12/X17 | F3 |
| episodes D16 | F9 |
| episodes K14 | F3/F7 |
| episodes K15/X28 | ✓ plus F10 cost |
| episodes K24 | ✓ (signatures reconstruct) |
| blind X1, X10 | F3 |
| blind X4, X16 | F2/F8 |
| blind X6 | F10 |
| blind X11 | F9 |
| blind X12 | ✓ (tree-identical successor) |
| blind X13 | F6 (offline) |
| blind X26 | ✓ |
| anchor-attack A5 (rewrite beneath a bot adoption) | F3 in new form |
| anchor-attack A6 | fixed for content (diffs now apply at P, not the root), but F7 |
| anchor-attack A7 S11/S12 | F9/F10 |
| anchor-attack A8 | unchanged |
| refs-attack F4/F5 | object-level versions are F6 and F5 |

## 3. Level-2 implications

None of the findings makes a good level 2 impossible, but three level-1
facts must be designed around:
1. **Representatives are head-relative** (F4). Level 2 must say which head
   an adoption targets.
2. **Membership is not proven by objects** (F5). "Is `t` on trunc main"
   is a ref question.
3. **The fetch fallback must move to level 2** as an annotation (F6).
   Otherwise level 1 depends on remote configuration.

## 4. Repair sketch: closed lineages plus publication markers ("R2″")

- A representative's parents are only representatives, so its tree is
  P-only everywhere. That makes `ρ(t,P)` a pure function of `(t, P)`: the
  same in every composition and every head. K8 and UC29 then hold as
  claimed.
- A composition-made `x = get_P(u)` gets a shadow `ρ(x)`. At publication,
  CloneX appends the ours-merge `merge(head, ρ(x))` (F8).
- Verification recomputes `ρ` in full (F5), and `get_P` stays pure (F6).

**Fixed in experiments:** F1, F2, F3 (x9: no resurrection), F4, F7, F8
(x9).

**Costs:**
- one marker merge per publication per component;
- the shadow shows in `--full-history` but is hidden by default
  simplification;
- lossy operations (squash, plain rebase) drop markers, which `status`
  detects;
- the K1×K8 trade-off (P-only intermediate states) remains.

**Not proven:** this sketch needs its own adversarial replay before
adoption.

## 5. Are K1–K13 jointly unsatisfiable?

**One literal contradiction.** K4 says a change across N components
"corresponds to one ordinary commit in each touched component". UC24,
which K4 cites, requires *two* tdd-ratchet commits on different bases:
one Git commit cannot have both parents. Reword K4 to "one per touched
occurrence, fused when bases and edits are equal".

**One near-impossibility, refuted.** Claim: with K1, K4, K9, a strict
reading of K6, ordinary merges and UC23, every representation undoes a
revert (F8). Proof attempt:
1. U main must take trunc's `x` without dotsync's `y` (UC17/X4). So main's
   carrier of `x` is some object `c ≠ u` (F2: ancestry is all-or-nothing).
2. Branch F holds `u` (K4) and reverts its trunc part.
3. `merge-base(main, F)` contains a carrier of `x` only if some ancestor of
   F other than `u` carries it.

The proof fails at step 3, because such an ancestor can be *appended*
after the fact (a marker ours-merge) without rewriting anything (K9).
So K1–K13 are **satisfiable by construction** (§4), except on paths that
pass through lossy operations (squash, plain rebase). K6 already admits
coarsening there, and those states must be *flagged*, which needs CloneX
`status` to run.

The residual is real but in tolerance. A plain-Git-only user who runs
`git rebase` on F, then later merges, can still get a revert undone
without being told unless something runs CloneX. That is Git's own
cherry-pick/revert hazard. The doc should name it in K6's tolerance
rather than claim "never silently wrong" unconditionally.

**K1×K8.** This remains the one trade-off, as the doc says. Closed
lineages make it cleaner: every intermediate representative is P-only,
none is composition-dependent.
