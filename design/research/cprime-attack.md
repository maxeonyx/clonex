# Adversarial review of C′ and C″ ("replay onto the adoption point" / "build on the parents' representatives")

Dated 2026-09-28. Targets:
- **C′**, the candidate in `constraints.md` §4;
- **C″**, the coordinator's refinement of it, made while this review ran;
- the owner's model in `stories.md` ("every tool commit has its own umbrella commit"; W8, W13), plus the hard requirement **W14**: a tool commit published from umbrella commit `u` maps back to `u` itself, so `ρ(get_P(u)) = u`.

Experiments used real git 2.55.0 and jj 0.44.0, in
`$SCRATCH/cprime-attack/`. `$SCRATCH` is
`/tmp/claude-1000/-home-maxeonyx-agent-tools-workspace/d1bb7304-0863-4655-bda0-4bf326c5a042/scratchpad`.
- `cxp.py` is a reference implementation. It reuses the byte-exact encoding in `recon/rho.py` and takes a switchable placement `POLICY`:
  - `L` is the literal one-liner;
  - `N` is "newest state in the adoption head", the adoption-point reading;
  - `M` is "minimal / canonical";
  - `C2` is C″ with W14.
- `CTX` chooses the tree rule outside `P` for merges.
- `fuse.py` implements fusion.
- Scenario scripts are `c*.sh` and `w*.sh`, with their outputs in `*.out`.

Shared setup (`common.sh`), which is the prior attacks' DAG rebuilt under C′:

```
T: t0 ─ o (outsider);  m = "Merge pull request #5" = (o, x)
U: h0 (README=u, tools/dotsync/d=D0);  h1 = ρ(t0) = h0 + tools/trunc:=t0   (root rep, sits on the declaring commit h0)
   u on h1: tools/trunc/a.txt=A2, tools/dotsync/d=D1, README=u2;   x = get_T(u) (parent t0)
```

---

## 0. Verdict in brief

1. **C′ as written ("replay onto the adoption point") is refuted.** Read literally, it silently deletes unpublished work (c5-L). Read as "newest state of the head" (N), it is not shared across branches. The same outsider commit adopted on two branches becomes two umbrella commits, and an upstream revert of it is then silently undone by an ordinary merge (c3-N, rc=0). Publication markers cannot fix this, because the marker's representative differs per branch (c2-N). jj immutability also freezes the user's own work (F7 returns, c6-N).
2. **C″ ("build on the parents' representatives", with W14 `ρ(x)=u`) is the right repair.** It fixes F1, F4-as-nondeterminism, F5, F8/G2 (no markers needed), UC29, the adoption-side G1 in most cases, UC9 extraction and nesting (composed). Its representatives are real, full umbrella states, and identical on every branch and clone.
3. **Four things remain, each reproduced:**
   - **UC23 × W14 × plain merge is jointly unsatisfiable** (w1, a three-line proof in §3.1). The owner must choose between blocking trunc progress, landing the whole of `u`, or giving up W14 on heads that lack `u`.
   - **G1 survives in a narrowed form.** Canonical (shared) contexts are stale by construction. A file added outside `P` after the lineage's context can still capture a component's edit under Git's rename detection (c4-C2-late, rc=0). This is the price of "same umbrella commit on every branch". Freshness and sharing cannot both hold, and c3-N shows that giving up sharing is worse.
   - **Amend × reviewer (F3) is back with plain `git merge`** (w3). CloneX's P-scoped adoption fixes it.
   - **Fusion is timing-dependent** (w9). A fused and an unfused umbrella commit for the same tool commit undo a revert when merged.

   With these repairs and named residuals, the stories' requirements hold jointly, except for the UC23 choice, which is an owner decision (§5).

---

## 1. Making C′ precise (the definition attacked)

Every choice the one-liner leaves open is resolved below. The choice made is marked, and the scenario that forces it is cited.

- **Adoption head `H`.** The branch head into which tool commits are brought. It fixes only *which* commits are already present, and whether the final `git merge` is a fast-forward.
- **Occurrence.** A pair `(T, P)`. Each occurrence has a **declaring commit** `D_P`: the umbrella commit on which `P` first appears (for UC10, the conversion commit). Representatives of different occurrences are always different commits, by requirement.
- **Anchor of a tool commit `p` in `H`.** An umbrella commit whose derivation is `p`, used as the parent of `ρ` of `p`'s children:
  - `L`: `H` itself, whenever `p ≤ get_P(H)`. **Refuted, c5-L.**
  - `N`: the newest `c ∈ ancestors(H)` (first-parent walk, then topological order) with `get_P(c) = p`. This is "the adoption point". **Refuted, c3-N and c2-N.**
  - `C″`: if `p` was made here (`p = get_P(u)` for an umbrella `u` that introduces it), the anchor is `u` (**W14**), searched head-relatively. Otherwise the anchor is the canonical `ρ(p)`, built recursively, whatever `H` is.
- **Representative `ρ(t, P)` of a foreign `t`:**
  - **parents:** `[anchor(p) for p in parents(t)]`, with parents new in this lineage built first. A root `t` gets parent `D_P`.
  - **tree outside `P`:**
    - one parent: that parent's tree;
    - several parents, one of which descends from all the others: that parent's tree;
    - otherwise Git's ort merge of the parents' trees, with `P` removed on every side and base = their merge base (`CTX=merge`). **Forced by c1(c) and w4**: `CTX=first` silently drops `u`'s other parts.
    - If that merge conflicts, construction **refuses** (w4).
  - **`P`:** `tree(t)`.
  - **metadata:** exact, plus the last-line trailer `Clonex-Source: P t`; extra headers are renamed to `clonex-*` (the recon encoding).
- **Own-made commits.** A composition change `u` is never re-represented. `x = get_P(u)` follows the lens rule (committer := author), and `ρ(x) = u`. No shadow representative is built and no marker is added. Inspecting `u` lists `{get_P(u)}` for every occurrence it touches.
- **Verification `is_rep(c)`.** It is independent of the placement policy:
  - decode the claim `(P, t)`;
  - the reconstruction parents are `get_P` of *c's own* parents (a root has one parent with no `P`);
  - the reconstruction must hash to `t`;
  - `tree(c)` must equal `ctx(parents(c))` with `P := tree(t)`;
  - re-encoding must be byte-identical to `c`.

  A representative whose parent is a composition commit `u` is therefore verified by *deriving* `u` (the lens over U's history). That makes shallow clones fail (§3.8).
- **`get_P`.** A verified representative gives `t`. Otherwise:
  - the lens rule: keep merges, take the maximal candidates, committer := author;
  - rep-dominance (r2pp G3);
  - decoding of failed claims (r2pp G8).
- **Adoption.** Build the lineage, then `git merge ρ(tip)`. **Exception (forced by w3):** when an anchor is not in `ancestors(H)`, CloneX writes a *P-scoped* merge instead:
  - tree = `H`'s tree with `P := merge3(base[P], H[P], t[P])`;
  - parents `[H, ρ(tip)]`.
- **Nesting.** Only composed shifts are used, `ρ_A(ρ_Q(t))`, with one trailer per level and the last trailer read per level. `get_{A/Q} := get_Q ∘ get_A` (w5).
- **Fusion.** One commit covering several linked parts `(Pᵢ, yᵢ)` that have identical metadata:
  - parents are the anchors of all the parts' parents;
  - the tree is the context with every `Pᵢ := tree(yᵢ)`;
  - one trailer per part, sorted by path;
  - per part, reconstruction strips the trailer block and uses the maximal `get_{Pᵢ}` of the parents (w9).

**What the adoption point is, under C″:** it no longer shapes `ρ` at all. Adopting into a branch and adopting into main give the *same* objects whenever the anchors agree, and they agree for all foreign lineages. The head matters only for `ρ(x) = u` lookups (F4) and for fast-forward versus merge.

---

## 2. Findings against C′ as written

### H1. The literal rule silently deletes unpublished own work — **violates K6/W4 (UC19)** (c5-L)
U has unpublished `w` (it adds `tools/trunc/c.txt`). Outsider `o` is based on `t0`. The literal rule makes `ρ(o)` equal to `H` with `P := tree(o)`:
```
rho(o) parent: 0911829 own trunc+dotsync work (unpublished)   isrep=None
diff rho(o)^..rho(o): A tools/trunc/b.txt  D tools/trunc/c.txt
git: Fast-forward             main: trunc files a.txt b.txt   (want a b c)
get_T(main) = ed89a88 "out: outsider" [9c898a8=w]  files: a.txt b.txt   ← an outsider-authored commit deleting c.txt, pushed on publish
```
Verification catches the false claim, because the reconstruction parent is `get_T(w) ≠ t0`. But the result is then *own work* that deletes `w`'s file under the outsider's name.

**Repair:** anchor at a state where `get_P = parent(t)` (N, M or C″ all do this: rc=0, a b c kept).

### H2. Head-relative placement (N) gives duplicate umbrella commits, and an upstream revert is undone by a plain merge — **violates K6, W13, owner's "same umbrella commit on every branch" (UC29)** (c3-N)
- Alice's branch A (`h1 ← a1`) and Bob's branch B (`h1 ← b1`) each adopt `o`.
- A lands on main. T reverts `o` (`r2`), and main adopts it.
- B runs "merge main into the branch":
```
rho_A(o)=02efe75 parent 2c2d89b   rho_B(o)=df82fbb parent 3e7c7b8   identical? NO
B merges main: rc=0 b.txt exists? yes (want no)
get_T(B) = b35b96b "Merge branch 'main' into B" [99feea1 = r2]    ← publishes a re-add of the reverted commit
```
Merging the duplicates themselves is clean:
- default `git log -- tools/trunc` shows `o` once;
- `--full-history` shows it twice;
- blame names `out`, and `get_T` = `o` is proven;
- `git revert` of one copy removes both copies' effect.

Only the *revert across branches* is harmful. Under M or C″ both adopters build the identical object `eceaa00` and the revert sticks (c3-M, c3-C2). This refutes the "Correction" paragraph's claim that UC29 needs only "no harm", not identical objects: without identical objects there *is* harm.

### H3. Under N the publication marker cannot be shared, so F8 comes back unrepairable — **violates K6 (UC23)** (c2-N)
Main moved on with dotsync-only `n` before adopting `m`. `ρ(x)` as computed on F at publish time, and `ρ(x)` as built by main's adoption, differ:
```
rho(x) computed on F at publish: c585d34 (parent a03b247);  built by main's adoption: 96ee18e (parent 9eb8331) same? NO
(ii) F with marker merges main: rc=0 a=A2 (want A)         ← T's revert undone
```
Under M both are `c585d34`, and the marker works (`a=A`). The G2 residual stays for stacked branches that lack the marker.

### H4. Under N, representatives are children of own work, so jj immutability freezes the user's commit — **F7 returns (UC28/UC20)** (c6-N)
```
jj squash into u:  Error: Commit d11f9b597a3e is immutable    rc=1
```
The same happens under C″, because `ρ(b)` is a child of `u` (c6-C2). §3.6 argues that this is acceptable under W14.

### H5. Mixed-context component merges need an outside-P merge — **F2 face; violates K6 with `CTX=first`** (c1(c), w4)
Branch F (which holds `u`) adopts `m = (o, x)`. Its parents are `[ρ(o) on h1, u]`, whose outside-P trees differ. With "tree from the first parent":
```
CTX=first: git: Fast-forward   F: README=u d=D0      ← u's umbrella and dotsync parts silently gone
CTX=merge:                     F: README=u2 d=D1     ✓
```
In w4 (x4 adapted, where U main has already merged `u1` and `u2`, and `u2` changed dotsync), `CTX=first` silently reverts `d` from D2 to D0 (rc=0). `CTX=merge` is correct.

---

## 3. C″ (with W14) under attack

### 3.1 W14 × UC23: jointly unsatisfiable with plain merges — **structural** (w1)
T merged `m = (o, x)`, dotsync has not merged `y`, and the U PR is open. W14 forces `ρ(m)`'s parents to be `[ρ(o), u]`. That makes `u` an ancestor of U main. Only the tree is left to choose:

| tree of the adoption | main after | F (`u ← v`) runs "merge main into the branch" |
|---|---|---|
| outside-P merge (C″ rule) | `README=u2 d=D1`: **trunc's merge button lands dotsync's unmerged part and the umbrella part** (F2 face 1) | fine |
| first parent / P-scoped CloneX merge | `README=u d=D0` ✓ | **rc=0, `README=u d=D0`**: F loses `u`'s parts, and publishing F's dotsync part pushes `"Merge branch 'main' into F", d=D0` onto dotsync's PR (at `y`, `d=D1`) (F2 face 2) |

**Proof that no tree works:**
1. If `u ∈ ancestors(main)` and `main[D] = D0` (UC23), then for any F that holds `u` and has not changed `D` since, `merge-base(F, main) ⊒ u`.
2. That merge has base `D1`, ours `D1` and theirs `D0`.
3. Any 3-way merge of those gives `D0`.

So W14 on a head that lacks `u`, UC23 as worded, and plain `git merge` cannot all hold. No marker helps: the defect is that ancestry already *claims* `u`.

**Repairs that keep W14 (owner choice):**
- **(a) Block.** A head that lacks `u` does not adopt any tool commit that descends from `x`. Trunc's main is then "ahead, waiting for U PR #12", and `status` says so. This conflicts with X20 ("upstream progress never blocks unrelated work"). Every later trunc commit descends from `m`, so the wait lasts until the U PR lands.
- **(b) Land all of `u`.** The umbrella PR merges when its first tool part lands. U main then holds dotsync's part before dotsync accepts it, and `get_D(main) = y` (a real dotsync PR commit, so W5 holds). A later dotsync rejection is an explicit U revert. This is face 1, *chosen and flagged* rather than silent.
- **(c) Keep W14 only on heads that contain `u`.** Main gets a shadow `ρ(x)` and the R2″ marker. G2 and the "two umbrella commits for `x`" of W13 return.

This review recommends (a) with a `status` line, or (b) as policy. (c) is the R2″ trade-off in a new place.

### 3.2 F1, F2, F4, F5 — mostly fixed
- **F1 fixed.** No representative is P-only.
- **F2** is fixed inside branches that hold `u` (c1(c) `CTX=merge`, c2b). It is structural only in UC23 (§3.1).
- **F4** (`u` and a rebased `u′` both derive `x`) is now *deterministic given `H`*, because the lookup is head-relative. But after a plain rebase copy, `ρ(b)` on `u` and `ρ(b)` on `u′` differ. Merging them and then reverting `b` upstream is undone (w11, rc=0, ledger resurrected). This is Git's own rebase/cherry-pick duplicate hazard, which K6 already names.
- **F5 fixed** (w7). A representative that smuggles `.github/ci.yml` fails the context check (`isrep=None`). It becomes own work, although plain blame still names the outsider (Git's author field).
- A new K12 point specific to W14: own-made carriers are identified by *derivation equality*. They must be looked up **only in `ancestors(H)`**. A global lookup (needed only in UC23 or squash) would let an attacker's copy of `u` with extra outside-P content serve as the carrier and be merged in.

### 3.3 G2 — gone
The carrier of `x` is always `u`, so there is no second carrier and no marker. c2b:
- T merges `x` and then reverts it;
- F adopts both;
- a stacked `G = u ← v` without any marker merges F: rc=0, `a=A` (the revert is kept);
- exactly one umbrella commit carries `x`.

### 3.4 G1 — narrowed, still structural for shared representatives (c4)
| variant | N (fresh, not shared) | C″ / M (shared) | plain monorepo from the same base |
|---|---|---|---|
| dotsync existed before trunc's declaration | conflict rc=1 ✓ | conflict rc=1 ✓ | conflict |
| dotsync added *after* trunc's declaration | conflict rc=1 ✓ | **rc=0, trunc's bump lands in `tools/dotsync/release.yml`**; `git revert ρ(o2)` rc=0 reverts dotsync's own bump | same misdirection |
| outsider based on an old trunc commit, dotsync added later | **rc=0 misdirected** | **rc=0 misdirected** | same misdirection |

C′'s full-tree bases confine G1 to *files added outside `P` since the base*:
- Under N, the base is the newest state where the tool sat at the parent. That is exactly a monorepo branch point, so the risk equals a monorepo's.
- Under C″, the base is the representative's context. That context is frozen at the declaring commit, or at the last own-made merge that refreshed it (`ρ(m)`'s outside-P merge with `u`). So C″ behaves like "every upstream contributor branched from that old state".

Freshness requires head-dependence, and head-dependence loses sharing (H2). So the old trilemma K1 × K6 × context-freedom becomes **K1 × K6 × sharing**. Unlike context-freedom, sharing *is* demanded, by the owner and by c3-N.

**Repairs:**
- CloneX's own adoption uses the P-scoped merge (r2pp G1 repair (a)).
- Plain `git revert` or `cherry-pick` of a representative remains a named residual, narrower than R2″'s: only files added outside `P` after the context count, not every file.

### 3.5 F3 (amend × reviewer, UC20/UC21, X17) — back with plain merge; CloneX's merge fixes it (w3)
- `u` accidentally adds `tools/dotsync/secret.env` and is published as `x`.
- U privately amends to `u*`: the secret is removed and the trunc part changed (`x* ≠ x`).
- Reviewer `r` lands on `x`. W14 forces `ρ(r)`'s parent to be `u`.
```
plain git merge: rc=0 … tools/dotsync/secret.env …   secret resurrected? YES
get_D(F*) pushes: "Merge commit '7c2012f…' into F", files d secret.env           ← publishes the secret to dotsync
P-scoped CloneX merge: secret? no;  get_T = merge [x*, r]  (the honest "reviewer built on the old version")
```
**Repair:** whenever an anchor is outside `ancestors(H)`, CloneX refuses a plain merge and writes the P-scoped merge. This is correct here because `u*` supersedes `u`. It is *not* correct in UC23 (§3.1), where the dropped parts are merely unlanded. Plain `git merge ρ(r)` is a residual (flagged: `status` sees an anchor outside the head).

### 3.6 UC14 ratchet bot, and jj/Git rewrites (c6-C2, w8)
- Red → bot → green is reproduced exactly: `get_T(F)` = `green ← [bot] ledger ← alias ← t0`, and publishing is a fast-forward. `ρ(b)` is a child of `u` and is a buildable state.
- **Immutability versus mutability.**
  - If representatives are immutable, `u` becomes immutable as soon as a bot commit sits on `x` (`Error: Commit d11f9b5 is immutable`). That is jj's own rule for shared history, and it matches the owner's append-only preference (K9). A deliberate amend then needs `--ignore-immutable` and §3.5's merge.
  - If representatives are left mutable, `git rebase` and `jj rebase -b` rewrite them. `b` is then no longer in the derived chain (`4e7d42d bot:[bot] ledger`, `b kept? NO`), and publication is non-fast-forward (flagged, and refused at level 2).

  Choose one. This review recommends immutable.
- **GitHub squash of the U PR (UC26, X11).**
  - Head-relative lookup finds no `u` in main after the squash, so C″ builds representatives for the own-made `x` and `g_T` (W14 lost on main, W13 doubled).
  - A global lookup to the PR head `g` re-imports the unsquashed history, which restores W14 and K1.
  - In both cases content is correct (`README=u2 d=D1 z=Z a=A3 b=B`), and `get_T` is `m`, proven by rep-dominance.

### 3.7 Nesting (UC7), force-push (UC15), repeated occurrences
- **Nesting (w5).** `ρ_at(ρ_T(b))` is a child of `uM` and verifies at both levels; `get_T(get_at(·)) = b`, proven.
  - A "deep" `ρ_{at/tools/trunc}(b)` differs in both tree and commit. Its lineage duplicates `x` and `t0`, and `get_T∘get_at` of it is garbage (`4697a46`, derived).
  - **Spec:** composed shifts only, one trailer per level (`Clonex-Source: tools/trunc b`, then `Clonex-Source: at ρ_T(b)`).
- **Force-push (w6).** The rewritten lineage builds on `ρ(t0)` (context `README=u d=D0`). Merged into a main with newer `README=later d=D5 c=C`, all newer umbrella changes are kept. The only conflict is the genuinely changed ledger (`add/add`, rc=1). **Old context never reverts newer umbrella changes**, because theirs has no outside-P change against the base, except through G1 (§3.4).
- **Repeated occurrences.** Each path gets its own lineage. `u` carries every occurrence it touches (UC24). The overlap rule (r2pp G7) is unchanged and still needed.

### 3.8 Determinism, verification cost, shallow clones (w7)
- **Determinism.** A second clone with no T remote reconstructs `o` from `ρ(o)`, recomputes `ρ(o)`, and gets `eceaa00`, identical. Given the declaring commit per occurrence and the head (for own-made lookups), C″ is deterministic (reworded K8).
- **Shallow clones.**
  - In a depth-1 clone: `fatal: git cat-file eceaa00…: bad file`. Verification recurses through representatives *and* through derivations of own-made anchors, which is longer than R2″'s chain.
  - A blobless clone works.
  - The third result, *unknown*, is required (G9).

### 3.9 Outside-P merge conflict (w4)
`u1` and `u2` both edit README. U main resolved it as `R12`. `ρ(m2)` needs the outside-P merge of `u1` and `u2`, which conflicts, so **construction refuses** (loud).

`CTX=first` would instead present a spurious README conflict, `R12` against `u1`'s old `R1` (rc≠0, loud but misleading).

**Repair:** if `ancestors(H)` holds a commit whose parents are exactly those anchors (U's own resolution), use its outside-P tree. This is deterministic given `H`; otherwise refuse.

### 3.10 Extraction (UC8/UC9) — fixed without markers (w10)
The first outside commit on the extracted repo gets a representative that is a child of `u2` (W14). `git merge` gives rc=0, `a=3`, keeps `README=u3`, and `get_P` = `o` is proven. In R2″ this needed a marker and a duplicated born-inside lineage (s15).

---

## 4. The owner's model (W8, W13, W14): explicit answers

**(1) One trunc commit adopted onto two umbrella branches.**
- Under N (C′ as written) it gets two umbrella commits. After the branches merge:
  - `git merge`: clean;
  - default `git log -- P`: shows it once;
  - `--full-history`: twice;
  - blame: the real author;
  - `get_P`: the real SHA.

  That breaks "its own umbrella commit" at object level, and it is *harmful*: a revert is undone (H2).
- Under C″ it is **one** object on every branch and clone (c3-C2, w7). Each branch shows a merge "Merge commit 'eceaa00…' into A/B", and `--full-history` lists those merges.
- W13's "one umbrella commit per tool commit" then holds for foreign commits. For own-made commits it holds by W14, except in UC23 (§3.1), after a GitHub squash (§3.6), and after rebase copies (w11: `u` and `u′`, which is Git's own duplicate).

**(2) The convention "tool commits enter once, on one shared line; branches get them by merging that line"** (c7).
- **Strict version (only main adopts):** it breaks UC14 and W7. The bot commit `b` sits on trunc's *PR branch*, so main cannot take it, and the U PR branch cannot get it:
  ```
  get_T(F) chain: max:green max:alias everywhere tom:t0
  publishing g_T onto T PR#5 (at b) is a fast-forward? NO (ratchet order red->bot->green broken, W7)
  ```
- **Generalized version: the "shared line" is the representative lineage itself**, hung off shared anchors. That is exactly C″, where it holds by construction without any line ref.
- Counts after the whole UC14 flow, then T's merge, then U's PR merge (c7):
  - under N every own-made commit and the bot commit get **2** umbrella commits;
  - under M (with R2″ markers) the bot commit gets 1, but `alias` and `green` get 2 (`u` plus `ρ(x)`);
  - under C″ with W14 the target is 1 each, and it is achieved whenever the U PR lands before or with trunc's merge (§3.1 (a)/(b)).
- A shared-line *ref* (level 2) is still useful as a **policy for fresh contexts**: adopting foreign trunk commits onto main with N-style freshness is safe when *only* main does it, because every copy then reaches every branch through main. It does not cover PR-branch commits, so C″ placement is still needed for those.

**(3) Fusion of linked tool commits made elsewhere** (w9).
- A fused commit `f` has parents `[ρ(t0)=h1, ρ(d0)=h2]`, tree = context with both parts, and two trailers. Both parts verify byte-exactly (`5c85533… True`, `7676f79… True`), and `git log -- tools/trunc` shows V's one commit. The redundant parent (`h1 ≤ h2`) may be dropped when every part still reconstructs.
- **Inconsistent order** (T: `A_T` then `B_T`; D: `B_D` then `A_D`). `f_A` must descend from the carrier of `B_D`, and `f_B` from the carrier of `A_T`, which is a cycle. A deterministic rule is needed: fuse greedily in topological order of one fixed component, and leave the rest unfused.
- **Timing is the real defect.**
  - Ada adopts `yT` alone (`yD` has not landed yet), and Max later adopts the fused `f`.
  - That gives two umbrella commits for `yT`. T reverts `yT`, Ada adopts the revert, and Max merges Ada: rc=0, `a=A2` (want `A`).
  - **Repair:** decide fusion from the tool commits only, never from arrival:
    - fuse only if all linked parts are present at the *first* adoption of any of them;
    - later adopters must reuse the existing representative (found by lookup), otherwise stay unfused.
  - Residual: concurrent adopters who cannot see each other.

---

## 5. Replays

**r2prime x1–x9 and r2pp s1–s15, adapted to C″:**
- x1/s1 F1: fixed.
- x2 F2: fixed in branches that hold `u`; UC23 is structural (§3.1).
- x3 F3: returns with plain merge; fixed by the P-scoped merge (w3).
- x4: fixed with the outside-P merge (w4 clean); refused on conflict.
- x5/s10: fixed (w7).
- x6 extra headers: unchanged.
- x7/x8/x9/s2 F8, G2: gone; no marker needed (c2b).
- s3 G3: rep-dominance still needed (w8).
- s4/s4b G4 and s5 G5: marker issues vanish (no markers). Representative rewrites under rebase are the §3.6 choice.
- s7/s7b/s7d G1: narrowed (§3.4).
- s8 G6: composed shifts (w5).
- s9 G8: decoding still needed after rebases (w8).
- s11: expected unchanged (untested).
- s13 G9: unchanged (w7).
- s14 G7: unchanged; needs the overlap rule.
- s15: fixed (w10).

**Use cases:**

| UC | C″ result |
|---|---|
| 1, 4, 5, 11–13, 16–18, 22, 24, 30, 31, 33 | ✓ |
| 2 | ✓; merges add "Merge commit …" entries under `--full-history` |
| 3 | improved: every representative is a full state (old context plus the tool), usually buildable |
| 6 | ✓ (disjoint paths); G7 overlap rule still needed |
| 7 | ✓ with composed shifts |
| 8, 9 | ✓, no marker (w10) |
| 10 | root representatives sit on the conversion commit; the gitlink is replaced by `P := tree` (expected, untested) |
| 14 | ✓ (c6); needs the immutability choice (§3.6) |
| 15 | ✓ (w6) |
| 19 | ✓ (c5-C2) |
| 20, 21 | ✓ without amend; amend gives F3, needing the P-scoped merge (w3) |
| **23** | **owner choice (§3.1)** |
| 25, 28 | flagged: representatives rewritten, SHAs lost (w8) |
| 26 | ✓ content; W14 needs `u`'s objects (w8) |
| 27 | cherry-pick of a representative: content right, claim fails (own work), G1-late exposure |
| 29 | ✓ identical objects (c3-C2) |
| 32 | *unknown* result needed (w7) |

**Episode reservoir (object-level):**
- ✓: episodes C1–C16 per occurrence (C12 is a lineage merge); D12 via the P-scoped merge; K1, K14, K15 (w6), K16, K23, K24; blind X1, X6, X7, X10, X12, X13 (pure, offline), X14 (w10), X16, X24 (true merge bases), X26, X28, X30.
- F3: episodes D2; blind X3, X17 (P-scoped merge).
- §3.1: blind X4, X5.
- §3.6: episodes D16; blind X11.
- Fusion timing: episodes K7, K18.
- New occurrence lineage (untested): X15 (mount move).

---

## 6. Verdict and comparison

| | R3 (squashed adoptions + side refs) | R2″ (context-free reps + markers) | C′ as written (adoption point) | **C″ + W14 + repairs** |
|---|---|---|---|---|
| Every tool commit an umbrella commit (W8) | ✗ | ✓ | ✓ | ✓ |
| One umbrella commit per tool commit (W13) | n/a | ✗ own work doubled (`u` + `ρ(x)`) | ✗ per branch (H2) | ✓, except UC23, squash, rebase copies |
| `ρ(x)=u` (W14) | ✗ | ✗ | ✓ | ✓ (heads that contain `u`) |
| Buildable representative states | n/a | ✗ P-only | ✓ | ✓ (possibly stale context) |
| G1 rename misdirection | none | every file outside P | monorepo-equal | files added outside P since the context |
| Revert undone (F8/G2/H2) | none | G2 residual | ✗ unrepairable | none, except Git's own rebase/cherry-pick duplicates |
| UC23 partial landing | ✓ | ✓ (marker) | ✗ | **owner choice: block / land all / shadow** |
| Plain transport (K7) | ✗ | ✓ | ✓ | ✓ |
| Shallow (K13) | ✓ | unknown needed | worse | unknown needed (longer chains) |

**Do K1–K13 (read now as W1–W14) hold jointly?**
- **Under C′ as written: no.** H1–H3 are silent failures that no repair fixes without changing the placement rule.
- **Under C″ + W14 with the repairs:** outside-P merge for representative trees, refusal on conflict (or reuse of U's resolution), P-scoped CloneX adoption when an anchor is outside the head, immutable representatives, rep-dominance, decoding of failed claims, composed nesting, the overlap rule, an *unknown* result, head-local own-carrier lookup, and arrival-independent fusion. With those, the stories' requirements hold jointly **except**:
  1. **UC23 × W14** is a proven impossibility under plain merges. The owner must pick (a) block, (b) land all of `u`, or (c) a shadow on heads without `u`.
  2. **Named residuals, all plain-Git operations without CloneX:**
     - G1 for files added outside `P` after a representative's context: plain `git merge`, `revert` or `cherry-pick` of a representative;
     - `git merge` of a representative whose anchor is outside the head (F3);
     - duplicates created by Git's own rebase or cherry-pick (w11).

     Each is detectable by `status`, and each is narrower than R2″'s corresponding residual.
  3. **Trade-offs:**
     - representatives are immutable, so `u` freezes once foreign work builds on it; alternatively they stay mutable and rebases are flagged;
     - contexts are stale by design (sharing versus freshness);
     - verification of own-made anchors walks composition history (shallow clones say *unknown*).

C″ strictly dominates R2″ on the stories (W13, W14, G2, buildable states, narrower G1) and dominates R3 on W8. The only regression against R2″ is UC23, which R2″ handled only by violating W14.
