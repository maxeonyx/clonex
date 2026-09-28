# Adversarial review of R2″ (constraints.md §4, "representatives depend only on the component")

Dated 2026-09-28. Target: the candidate R2″ as it is written:
- `ρ(t,P)` is context-free: its tree contains only `P`, its parents are only representatives, and its metadata is exact plus the trailer;
- byte-identical recomputation is the only proof;
- composition changes are never representatives;
- publication adds an "ours" marker merge `[head, ρ(x)]`;
- adoption is a plain `git merge`.

**Verdict.** R2″ fixes everything the R2′ attack found (F1–F5, F7, F8; see §2), and its core laws hold in every experiment:
- `get_P(ρ(t,P)) = t`;
- `ρ` is context-free;
- `get_P` is pure and offline.

But R2″ is **not yet a result**. Seven findings below produce silently wrong content or history with exit code 0, using ordinary Git or jj operations. Five of them have a demonstrated, cheap repair. Two are **structural**, and a repair can only narrow them:
- **G1.** P-only representatives are bad *merge bases* for Git's rename detection. `git merge`, `git revert` or `git cherry-pick` of a representative can move a component's change into a *different* component. This is a second face of the K1×K8 trade-off, and plain Git hits it with default configuration.
- **G2.** The marker protects only the branches that contain it. Any branch that holds `u` but not its marker gets F8 back, with no lossy operation involved.

K1–K13 are satisfiable only if K6's named residual is widened to cover plain-Git and plain-jj operations that act *on representatives or on a branch without a marker* when CloneX is not running (§5).

Experiments were run with real git 2.55.0 and jj 0.44.0, in
`$SCRATCH/r2pp-attack/` (`$SCRATCH` = `/tmp/claude-1000/-home-maxeonyx-agent-tools-workspace/d1bb7304-0863-4655-bda0-4bf326c5a042/scratchpad`).
- `cx.py` is a reference implementation of R2″ semantics, reusing the byte-exact encoding from `recon/rho.py`:
  - `rho`: tree = only `P`, parents = `ρ(parents)`;
  - `is_rep`: decode, hash, recompute, and compare bytes;
  - `get`: a verified representative gives `t`; otherwise model.md lens rule 2 (keep-merges, maximal candidates, committer := author);
  - `marker`: the ours-merge.
- `REPAIR=1` enables the rep-dominance rule of G3.
- Scenario scripts are `s1.sh` … `s15.sh`, with their outputs in `s*.out`.
- Shared setup (`common.sh`) is the r2prime-attack DAG, rebuilt under R2″:

```
T: t0 ─ o (outsider);  m = "Merge pull request #5" = (o, x)
U: h0 (README=u, tools/dotsync/d=D0);  h1 = merge(h0, ρ(t0))
   u on h1: tools/trunc/a.txt=A2, tools/dotsync/d=D1, README=u2;   x = get_T(u) (parent t0)
   M1 = marker [u, ρ(x)], tree(u)
```

---

## 1. Findings (by severity)

### G1. A P-only merge base misdirects Git's rename detection: one component's change lands in another — **violates K6 and K1** (s7, s7b, s7d)

Every representative's tree contains only `P`. So whenever a representative is a merge base, every file *outside* `P` looks "added by ours". If U's own work deleted a file inside `P` (a common cross-cutting cleanup) and a similar file exists in a sibling tool, ort pairs the deletion with that sibling file as a *rename*. The component's edit is then applied to the sibling. The seven tools share near-identical boilerplate (`release.yml`, `LICENSE`, `AGENTS.md`), and ort's basename heuristic prefers exactly those pairs.

`s7.sh`:
- T has `release.yml`, and U's dotsync has the same file.
- U's own work `w` deletes `tools/trunc/release.yml` ("use the shared workflow").
- An outsider on T bumps `checkout@v4 → v5`, and U runs `git merge ρ(o2)`:

```
git: Merge made by the 'ort' strategy.
git:  tools/dotsync/release.yml | 2 +-
rc=0
tools/dotsync/release.yml now uses:       - uses: actions/checkout@v5
== control: same histories as a plain monorepo (base = full tree)
git: CONFLICT (modify/delete): tools/trunc/release.yml deleted in HEAD and modified in …   rc=1
== with -X no-renames
git: CONFLICT (modify/delete) …   rc=1
== jj new w r_o2
jj: tools/trunc/release.yml    2-sided conflict including 1 deletion
```

- `s7b.sh` gives the same result when dotsync's file is only *similar* (two lines differ).
- `get_D` of the merge is then a dotsync commit that nobody made, published as own work.

The hazard is not limited to adoption. `s7d.sh`: dotsync made its own `v5` bump, trunc's `release.yml` was later dropped, and a plain-Git user runs `git revert ρ(o2)` on the trunc commit shown in U's log:

```
dotsync before revert:       - uses: actions/checkout@v5
git: [main f1ccbd8] Revert "bump checkout to v5"   rc=0
dotsync after revert:        - uses: actions/checkout@v4
```

**Repair (partial).** CloneX adoption must not delegate the tree to `git merge`. It should merge *only P*:
- base `B*` = ours with `P := base-rep[P]`;
- theirs* = ours with `P := rep[P]`;
- run `git merge-tree --write-tree --merge-base=B* ours theirs*`;
- commit the result with parents `[head, ρ(t)]`.

`s7c.sh` confirms that this gives the monorepo's modify/delete conflict (rc=1), and that renames *inside* P are still followed: T moves `release.yml → ci/release.yml`, and U's edit follows it.

The result is still an ordinary merge commit, so K1 holds. jj's `jj new head ρ(t)` is safe as it is, because jj does no rename detection.

**Residual (structural).**
- A plain `git revert`, `cherry-pick` or `merge` of a representative by a user without CloneX stays unsafe under Git's default `merge.renames=true`.
- That config is local, so it cannot travel (K7).
- The hazard exists under any context-free (K8) representative, so it is the K1×K8 trade-off again: P-only states are unbuildable, *and* they are poisonous merge bases.

### G2. The marker covers only branches that contain it; F8 returns with no lossy operation — **violates K6 beyond its named residual** (s2)

K6 names the residual "a squash or plain rebase drops the marker". But the marker is a *descendant* of `u`. Any branch that holds `u` without it is unprotected. Examples:
- a stacked branch forked from `u` before publication;
- a colleague's clone of the U PR;
- a jj sibling child of `u`.

`s2.sh`:
1. T merges `x` and then reverts it (`r`), and main adopts `ρ(r)`.
2. G = `u ← v` (a dotsync-only follow-up) runs AGENTS.md's "merge current main into the branch".
3. G's PR is merged.

```
main a=A (T reverted: want A)
(a) F+marker merges main: a=A (want A)
(b) G (has u, no marker) merges main: rc=0 a=A2 (want A)
    get_T(G)=4f6bf88 … derived        4f6bf88 max: Merge branch 'main' into G [9c0de44 = r]
(c) main after merging G's PR: a=A2  d=D1
```

The next trunc publication pushes `4f6bf88`, a single-parent commit on `r` with a merge message, which silently undoes T's revert. In a plain monorepo, and in trunc itself, the merge base would be `u` or `x`, and the revert would stick.

**Repair (partial).**
- Insert the marker *below* `u`'s unpublished descendants. jj does this with `jj new u ρ(x) --insert-before <children>`, which is private rewriting and allowed by K9.
- Make the marker deterministic (author, date and message derived from `u` and `x`, committer := author) so that every clone that publishes creates the *same* marker (K8).
- Have `status` and a CloneX pre-merge check flag any head that holds `u` but not `ρ(x)`.

**Residual:** published branches of other people that forked from `u` before the marker, merged by plain Git.

### G3. A squash of a U PR contaminates every later publication forever — **violates K6 and K3 (and trunc's ratchet)** (s3)

The U PR contains `u` (red), the marker, the adoption of bot commit `b`, `g` (green), and a second marker. GitHub squashes it into main as `s`. T merges its PR (`m2`), main adopts `ρ(m2)`, and `tree(main)[P] == tree(m2)`. Even so:

```
get_T(main) = da06808… derived      T main = m2 = c8c64ab…
*    eff4ebd max: later work [da06808]
*      da06808 max: Merge commit '0b45e4f…' [6826ad8 c8c64ab]
| *      c8c64ab gh: Merge pull request #5 …
* /    6826ad8 max: alias everywhere (#12) [4c44507]      ← jump commit: red+ledger+green in one
```

Lens rule 2 keeps the merge because the jump commit `s′` and `m2` are unrelated candidates. U main cannot be rewritten (K9), so **every** future publication from U main drags `s′`, a ratchet-violating duplicate, into T.

r2prime-attack F9 called this "coarser". It is in fact permanent and silent, because pushing it is a fast-forward of the PR branch.

**Repair (demonstrated).** Add a *rep-dominance* rule: if a merge's `tree[P]` equals the tree of a candidate that is proven through a verified representative, `get_P` is that candidate. This is the old round-trip law, with the graph edge as the base. With `REPAIR=1`:

```
*    aa0ff31 max: later work [c8c64ab]
*      c8c64ab gh: Merge pull request #5 …
```

The rule is pure and offline, and it does not fire for amended markers, where the tree differs. It also makes every adoption whose content has fully landed a fast-forward in component space, which was L7 in model.md.

### G4. `jj rebase -b` rebases the representative behind a marker, and publication pushes a lying duplicate — **violates K6** (s4, s4b)

`ρ(x)` is reachable only through `M1` and is mutable in jj. `jj rebase -b F -d main` treats it as a root of `main..F`:

```
jj: Rebased 4 commits to destination.
marker 2nd parent = b861d6b… was r_x=b2b85d0… ; isrep=None
2nd parent tree: README tools/dotsync/d tools/dotsync/z tools/trunc/a.txt
get_T(F) = 976a063 "Published component part" [8ded881(=x) 50605dd]
   50605dd message: "alias everywhere\n\nClonex-Source: tools/trunc 8ded881…"
```

`get_T(F)` is a merge of `x` with a duplicate whose trailer claims to *be* `x`. It fast-forwards T's PR branch (which is at `x`), so level 2 would not refuse it.

**Repair (demonstrated in s4b).** Add `revset-aliases."immutable_heads()" = 'builtin_immutable_heads() | description(regex:"(?m)^Clonex-Source: ")'`.
- `jj rebase -b` then stops with `Error: Commit b2b85d0ee380 is immutable` (rc=1).
- `jj rebase -s u -d main` works, and `get_T` stays `x`.
- Because lineages are closed, immutability does not spread to own work (F7 stays fixed).
- `git rebase --rebase-merges` keeps `ρ(x)` untouched (it is a cousin), and `get_T = x`.

**Residual:** the jj config is local. A fresh jj user of U without CloneX's config is exposed (K7). Holding a remote-bookmark ref for each unlanded marker representative would make jj's *builtin* immutability cover it, but that is level 2.

### G5. jj rewrites of `u` interact badly with a marker made by CloneX (s5)

- **Abandon: beyond the stated tolerance.** `jj abandon u` rebases `M1` onto `h1`, and jj keeps `M1`'s (empty) diff relative to its auto-merge, so `M1` now *carries* `x`:

  ```
  mode abandon:  a.txt: A2   README: u  d: D0     get_T(F) = x proven
  ```

  The abandoned change's trunc part survives silently in a commit the user never made.
- **Amend of the trunc part: flagged, within tolerance, but with two defects.**
  - The marker becomes a jj conflict (`CONFLICT Published component part`, A3 vs A2).
  - Resolving it as "ours" makes `get_T` a merge `[x*, x]` whose message is the *marker's* message, and that message goes into trunc.
- **Amend of the dotsync part only: fine.** `get_T = x`, and F3 is fixed.

**Repair.**
- A marker is valid only while `tree(M) == tree(first parent)`. `status` flags a broken marker, and CloneX re-creates it after rewrites.
- A marker that projects into a component must carry a component-worthy message, or `get_P` must collapse it.

Plain jj users still see the abandon anomaly (residual).

### G6. Nesting: a "deep" representative breaks F8 and projects garbage; only composed shifts work — **violates K6/K10 unless specified** (s8)

M contains U at `at`, and U contains T. M edits `at/tools/trunc` directly and publishes `x`. The same `x` has two candidate representatives:

```
ρ_{at/tools/trunc}(x) = 1e855de…     ρ_at(ρ_T(x)) = e3a85d0…
(b) F with deep marker merges M main: a=A2 (want A)      ← T's revert undone
(b) F with comp marker merges M main: a=A  (want A)
(c) deep: get_at = merge[y, 5befded (P-only own-work commit, not a rep)] → get_T garbage (ffd66dc)
    comp: get_at = merge[y, b2b85d0 = ρ_T(x) (verified)]  → get_T = x
```

M receives T only through U, so the deep representative is a second carrier (F8).

The composed one works, and it also **projects into U as a proper U-level marker**. Publishing M's head to U therefore carries the marker down a level for free.

**Repair (spec):** a nested occurrence's representative is the composition of shifts `ρ_A∘ρ_Q`, and there is no deep representative. `get_{A/Q} := get_Q∘get_A` (last-trailer rule per level). `s8` shows that both give `x`.

### G7. Overlapping occurrences (fixture inside the tool) publish garbage lineages — **violates K10 unless forbidden or specified** (s14)

UC6 with the frozen copy under the current occurrence: `P = tools/tdd-ratchet` and `Q = P/tests/fixtures/v1`. After the CVE fix, `get_P(marker for Q)` pushes into tdd-ratchet:

```
406bf17 rat: v1          files: tests/fixtures/v1/ledger.rs tests/fixtures/v1/lib.rs   ← root commit, all of tdd-ratchet gone
4bb2a10 max: Adopt frozen copy (merge)
```

The fixture files also leak into tdd-ratchet's own published tree (`get_P(u)` includes `tests/fixtures/v1/*`).

**Repair:**
- a verified representative of `(t, Q)` maps to *none* for every `P ≠ Q`;
- and either occurrences may not overlap, or `P`'s projection excludes nested occurrence subtrees. A fixture that the tool's own CI needs makes the tool itself a composition (nesting, G6).

### G8. Plain `git rebase` (UC25) now loses component SHAs and leaks false trailers — **within tolerance (flagged), but the doc overclaims** (s9)

R2′ kept SHAs after a rebase through the network fallback. R2″ has none. After `git rebase main` on a branch that merged `ρ(b)`:

```
get_T(F) chain: 7416cc2 green / 4e7d42d bot: [bot] ledger / 8ded881 alias / t0
bot commit in derived chain == b? NO: message ends "Clonex-Source: tools/trunc 88e1330…"
```

The PR branch is at `b`'s child, so this is not a fast-forward and level 2 refuses it. The trailer, however, would publish a commit that falsely claims to be `b`.

**Repair (demonstrated):** when a claim fails, the lens decodes it (strips the trailer and restores the `clonex-` headers) before deriving. Then `derived == b: True` for every commit whose committer equals its author, which covers bot commits and all CloneX-derived commits. GitHub merges and signed commits stay coarsened.

### G9. Depth-1 checkouts cannot verify anything — **degrades K13; spec needed** (s13 inline)

Verification recurses to the root of each lineage, because a representative is valid only if its parents are. In `git clone --depth 1`, `get` fails (`fatal: git cat-file 0f9881f…: bad file`). A blobless clone works.

`get_P` needs a third result, *unknown*. Treating "unverifiable" as own work would change derived SHAs between shallow and full clones (K8).

### G10. Smaller items

**Within tolerance or spec gaps:**
- **Extraction and born-inside (UC8/UC9) need a marker** (s15). Without it, the first outside commit is `fatal: refusing to merge unrelated histories` (rc=128), or add/add conflicts with `--allow-unrelated-histories`. With the marker `[u2, ρ(n2)]` it merges cleanly (rc=0), and `get_P = o` proven. The cost is a full representative copy of the tool's born-inside history in U.
- **Empty-tree component roots** (s11). `ρ` holds an empty subtree, which `fsck --strict` and `receive.fsckObjects` accept. After a merge Git drops it, so `get_T = none` instead of `t0`. This is harmless but should be stated.
- **Forgery (UC31, x5 adapted: s10).** An extra file outside P gives `None`, and an extra parent gives `None`. K12 holds. The forged commit is still own work authored by the outsider, so `git blame` in U shows their name. That is Git's own author field and cannot be fixed at level 1.
- **Marker determinism.** A marker made with the publisher's identity and clock differs per machine, so two agents that publish the same `u` race in U (UC29). Derive the marker's metadata from `(u, x)`.

**Doc corrections:**
- "Each published change appears twice in `git log -- tools/trunc`" is true only with `--full-history`. The default simplification shows `ρ(x)` once, and blame attributes to `u` (s1).
- **T squash- or rebase-merges `x`.** The marker then points at an `x` that never lands, and a later revert of `s` can be undone by merging a branch that holds `u`. This is exactly the stated squash hazard, and it is unchanged.
- **Plain `git rebase` drops the marker and `ρ(x)`** ("patch contents already upstream"), as stated.

---

## 2. r2prime-attack x1–x9, adapted to R2″

| | R2′ | R2″ result |
|---|---|---|
| x1 UC18 deletes the umbrella | F1 | **fixed** (s1: `README=u d=D0`, `get_T(main)=m` proven) |
| x2 UC23 lands other parts | F2 | **fixed** (s1: main gets only trunc) |
| x3 bot × amend resurrection | F3 | **fixed** (s5 amendD; x9) |
| x4 two composition parents | F2 | fixed by construction (representatives have no composition parents) |
| x5 smuggling | F5 | **fixed** (s10) |
| x6 extra headers | F10 | unchanged |
| x7/x8 revert undone without marker | F8 | fixed with the marker (s2a), but **returns through G2** |
| x9 marker | repair | holds (s2a); UC25 drops it, as stated |

## 3. Use-case replay against R2″ as written (repairs from §1 in brackets)

| UC | Result |
|---|---|
| 1, 4, 5, 11, 12, 13, 16, 17, 18, 22, 23, 24, 30, 31, 33 | ✓ (s1, s10). 17 and 19 are exposed to **G1** with plain `git merge` [P-scoped merge] |
| 2 | ✓; the doc overstates the duplication (G10) |
| 3 | degraded as stated (P-only states) |
| 6 | ✓ when paths are disjoint; **✗ G7** when the fixture is inside the tool |
| 7 | ✓ with composed shifts; **✗ G6** with deep representatives |
| 8, 9 | ✓ with an extraction marker (s15) |
| 10 | first adoption needs a special step (unchanged) |
| 14 | ✓ (s3 up to the squash) |
| 15 | ✓, one new lineage per occurrence per rewrite (cost) |
| 19 | ✓ (x's stale parent is fine) |
| 20 | follow-up ✓; jj amend gives a marker conflict and a marker message in T (**G5**); abandon is **✗ G5** |
| 21 | ✓ (the base includes `ρ(x)`) |
| 25 | flagged; SHAs lost; trailer leak (**G8**) |
| 26 | merge ✓; squash **✗ G3** [rep-dominance] |
| 27 | a cherry-pick or revert of a representative is **✗ G1** (s7d); claims become own work (G8) |
| 28 | **✗ G4** [immutability], G5 |
| 29 | representatives identical ✓; markers nondeterministic (G10); others' branches **G2** |
| 32 | **G9** |

Episode reservoir, object-level items:
- ✓: episodes K1, K14, K24, D2, D12/X17; blind X1, X3 (with G5), X4, X10, X12, X26, X30; anchor A5.
- G3: episodes D16; blind X11 (component squash stays the stated hazard); anchor A7 S11/S12.
- G4: anchor A6.
- G1: blind X16.
- G9: blind X13 (offline is now fine, because `get_P` is pure).
- Cost, as stated: blind X6/K15/X28.

## 4. Level-2 implications

1. A ref that keeps each unlanded marker representative immutable in jj (G4) until trunk contains it.
2. Publication must refuse derived commits that carry a failed `Clonex-Source` claim (G8) and markers whose tree differs from their first parent (G5).
3. "Is `t` on trunc main?" remains a ref question (membership, F5).

## 5. Are K1–K13 jointly satisfiable?

**With repairs, yes, except for a residual that is inherent and must be named.** The repairs:
- (a) P-scoped adoption merges;
- (b) deterministic markers inserted below unpublished descendants, plus a pre-merge check;
- (c) rep-dominance;
- (d) representatives immutable in jj;
- (e) marker validity and re-creation;
- (f) composed nested shifts;
- (g) non-overlapping occurrences, or exclusion of nested ones;
- (h) decoding failed claims;
- (i) a third, *unknown* result in shallow clones.

These close every silent failure found here *whenever CloneX runs*. What remains is one trade-off with three symptoms. Each symptom is a plain-Git or plain-jj operation acting on the composition without CloneX:
1. **G1:** `git merge`, `revert` or `cherry-pick` whose base is a P-only representative, under default rename detection. This is unavoidable under K8: any context-free representative has an empty non-P tree, so non-P files always look added. K1 puts those representatives in history where ordinary tools can reach them. The only exits are:
   - config (local, so it breaks K7);
   - composition-dependent trees (breaks K8);
   - keeping representatives out of history (breaks K1).
2. **G2:** a branch that holds `u` but not its marker, merged by plain Git. Git's merge base is ancestry-only, and markers can only be appended. K9 forbids putting the marker under published history.
3. **G4:** jj without CloneX's immutability config. Config doesn't travel (K7).

So K6's residual ("squash or plain rebase drops the marker") must be widened to: *plain-Git or plain-jj operations whose merge base is a representative, or that act on a branch holding `u` without its marker, can be silently wrong unless CloneX's `status`, config or merge step runs.* With that wording, K1–K13 are satisfiable. Under the strict reading of K6 ("ordinary operations *never* make results silently wrong"), K6, K8 and K1 are jointly unsatisfiable, and G1 is the witness (s7d: `git revert` of a visible representative, rc=0, dotsync changed).

Recommendation: record G1 as the second face of K1×K8 next to "unbuildable intermediate states", adopt repairs (a)–(i), and re-run this replay once the lens rules are written down.
