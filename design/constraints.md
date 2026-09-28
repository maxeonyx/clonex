# Use cases, constraints, and a design that satisfies them

This is the root of the design. **Use cases** are the external truth; each
one is concrete and drawn from the Agent Tools ecosystem or the owner's
stated goals. **Constraints** are derived from use cases, and each one cites
the use cases that justify it. **Derived rules** are consequences, and each
has its reason written down. A rule without a reason is not part of the
design.

Scope of this document: **level 1 only — objects.** That means commits,
trees, and change identity (opaque change ids plus commit hashes). Refs
(mutable labels, their sync and their naming) are **level 2**, designed
separately afterwards (§6). Nothing at level 1 may depend on level 2.

Notation: `U` is the umbrella composition, `T` trunc, `D` dotsync, `R`
tdd-ratchet. Lower-case letters are commits. `P` is a path in a
composition.

---

## 1. Use cases

### Working in a composition
- **UC1 Cross-cutting change.** Add a `CLAUDE.md` alias beside every
  `AGENTS.md` in all seven tools, plus the umbrella's checker, as one commit
  made in U. (This really happened: one umbrella commit, seven tool PRs,
  seven pointer bumps.)
- **UC2 Read the composition like a monorepo.** In U: `git log --
  tools/trunc/src/lib.rs`, `git blame tools/trunc/src/lib.rs`, GitHub's file
  history and blame views. They show trunc's real commits, authors, dates
  and messages.
- **UC3 Bisect.** U's CI goes red; bisect finds the trunc commit responsible.
- **UC4 Plain clone just works.** `git clone U`, then build and test every
  tool. No submodule init, no extra tool, works offline.
- **UC5 Throwaway context.** An agent task needs dotsync + agent-harness
  together for an afternoon, then discards the context.
- **UC6 Two revisions of one tool.** U holds tdd-ratchet current at
  `tools/tdd-ratchet` and a frozen copy at a fixture path for ledger
  compatibility tests.
- **UC7 Nesting.** A personal repo contains U, which contains T.
- **UC8 Extract.** Turn `crates/standards` into its own repo (`crosscut`),
  keeping its history.
- **UC9 Born inside.** Start a new tool inside U; give it its own repo
  later.
- **UC10 Convert.** Turn today's submodule-based U into a composition,
  keeping U's existing history.

### Components stay independent
- **UC11 Outsider PR.** Someone who only knows Git sends a PR to trunc;
  it's merged with a merge commit on GitHub.
- **UC12 Standalone work.** The owner develops dotsync directly with jj and
  pushes.
- **UC13 SHA-keyed world.** trunc's tags, releases, CI statuses
  (`latest-ci-green`), GitHub signatures and `state.json` attestations all
  name trunc commit SHAs.
- **UC14 Ratchet flow.** In each tool, a red (test) commit is pushed, a CI
  bot adds a ledger commit on top on the PR branch, then the green commit
  follows.
- **UC15 Tool history repair.** An approved force-push rewrote tb's and
  agent-harness's histories (this really happened).
- **UC16 Ignorance is fine.** Someone working only on dotsync never needs
  to know CloneX exists.

### Flow between them
- **UC17** New trunc commits (UC11, UC12) become part of U.
- **UC18** A change made in U becomes ordinary commits in each tool it
  touches, with the right author and message.
- **UC19** trunc moves on while U has unpublished work touching trunc.
- **UC20** A published-but-unmerged change made in U is amended after
  review.
- **UC21** A reviewer adds a commit on top of U's change in trunc's PR; work
  continues in U.
- **UC22** Two compositions (U, and a view V) both carry work on dotsync.
- **UC23** A cross-cutting change is half landed: trunc merged it, dotsync
  hasn't yet.
- **UC24** One CVE fix is applied to tdd-ratchet current and to the frozen
  fixture copy.

### Ordinary tools on the composition
- **UC25** A plain-Git user rebases a U branch before pushing.
- **UC26** A U PR is merged on GitHub (merge commit; squash if it ever
  happens).
- **UC27** A U commit is cherry-picked to another U branch.
- **UC28** A jj user squashes, splits and rebases U history.

### Everything else
- **UC29** Two people bring the same new trunc commits into U at the same
  time.
- **UC30** U with ~2,000 contained commits must feel instant.
- **UC31** A commit in U wrongly claims to correspond to a trunc commit
  (mistake or forgery).
- **UC32** U's CI runs in a depth-1 checkout.
- **UC33** Everything works through any Git host. No other service.

---

## 2. Constraints (each derived from use cases)

- **K1 The composition is a monorepo to Git, jj and GitHub.** Every
  component commit a composition incorporates appears in the composition's
  own commit history as a commit that changes the component's *path*. It
  keeps the original author, date and message. Content is plain files.
  *(UC2, UC3, UC4, UC25–28)*
- **K2 Components are ordinary repositories.** Nothing CloneX-specific is
  required in a component. Commits that originate in a composition may carry
  trailers (the owner accepts this). *(UC11–16)*
- **K3 Component commit identity survives.** A component commit that exists
  in the component repo is identified by its *real SHA* from the
  composition, and work leaving a composition builds on real component
  commits. *(UC13, UC14, UC15)*
- **K4 One change, one composition commit.** A change across N components
  is one composition commit, and it corresponds to one ordinary commit per
  touched *occurrence*. Two copies of one tool on different bases need two
  tool commits, so "per component" was impossible as first worded (found by
  the R2′ attack). *(UC1, UC18, UC24)*
- **K5 No canonical side.** Work originates in any repo and flows in both
  directions. *(UC11, UC12, UC17, UC18, UC22)*
- **K6 Ordinary operations never make results silently wrong.** Merge,
  rebase, cherry-pick, squash, GitHub's merge button, jj rewrites, and
  upstream force-pushes, on either side. The worst allowed outcome is
  "coarser" or "explicitly flagged". A named residual: Git's own
  cherry-pick/revert hazard. If a squash or a plain rebase drops CloneX's
  publication marker, a later upstream revert of that change can be undone
  by a merge, exactly as in plain Git. `status` detects it. *(UC15, UC25–28)*
- **K7 Everything travels by ordinary fetch/push through any Git host.**
  Extra state may only be extra refs, preferably only in composition repos.
  *(UC4, UC33, UC16)*
- **K8 Determinism.** The same inputs give the same commits on every
  machine. *(UC29, UC5 reproducibility)*
- **K9 Append-only by default.** Nothing ever *requires* rewriting
  published history on either side; private rewriting before publishing is
  fully supported. *(UC15, UC20; owner preference)*
- **K10 Repeated and nested occurrences.** *(UC6, UC7, UC24)*
- **K11 Extract, convert and create keep history.** *(UC8–10)*
- **K12 Claims are verifiable, never trusted.** Any statement that
  "composition commit c corresponds to component commit t" must be checkable
  from objects. *(UC31, K6)*
- **K13 Cheap and shallow-tolerant.** *(UC30, UC32)*

---

## 3. Derived rules and their reasons

**D1: history records only facts that stay true, and nothing semantic
depends on a mutable name.** This is not a principle; it is a consequence.

Why Git keeps refs out of commits:
- a commit's hash covers its content, so anything written into it can
  never change without making a new commit;
- names change after the fact (branches are renamed, repos move, trunks go
  `master` → `main`);
- the *same* commit lives under *different* names in different repos (a
  fork's `main` is upstream's `feature-x`).

A name inside a commit is therefore wrong either immediately (in another
repo) or eventually (after a rename). The only fix would be rewriting
history, which K9 forbids.

The CloneX cases that show this:
- `.gitmodules` URL rot: checking out an old umbrella commit points at URLs
  that have since moved;
- help-test's trunk rename;
- a fork of U that wants different component URLs.

Names may still appear as **descriptive text**. Git's own merge messages
do, and nothing breaks, because nothing *resolves* them. So the rule is:
nothing semantic reads a name out of history. Paths are not an exception:
a path in a composition's tree is part of that composition's own content, a
fact about that commit, and it stays true.

**D2: correspondence is proven by content, not asserted by metadata.** The
interop experiments showed every metadata carrier is dropped, copied or
mangled by some ordinary operation (K6). So a trailer may *propose* "this is
trunc commit t", but it only counts when the objects prove it (K12).

**D3: change identity is an overlay.** Change ids (jj's, carried as
`Clonex-Change`) help people, and help merges after rewrites. Since carriers
die (D2), no correctness property may depend on them.

---

## 4. The search: which representation satisfies K1–K13?

Candidates for "the composition contains the component's history":

| | K1 monorepo to Git | K3 real SHAs | K6 safe under ordinary ops | K7 travels with plain push |
|---|---|---|---|---|
| **R1**: component commits verbatim as merge parents (current `src/`) | ✗ trees at repo root | ✓ | ✗ rebase dumps them at root; squash-merge drops them | ✓ |
| **R3**: squashed adoptions + side refs | ✗ component commits absent | ✓ | ✓ | ✗ side refs don't travel |
| **R2′**: self-verifying representatives (below) | ✓ | ✓ | ✓ or flagged (walkthroughs) | ✓ |

R1 and R3 each violate a constraint outright, not by degree. Only R2′
survives, so it is worked out in full.

### R2′ definition

**Representatives.** Every component commit `t` that a composition
incorporates at path `P` has exactly one *representative* `ρ(t, P)` in the
composition's history:
- If `t` came *from* this composition (`t = get_P(u)` for a composition
  commit `u`), then **`ρ = u` itself**. The composition commit is the
  representative, and nothing is duplicated.
- Otherwise `ρ` is a *shifted copy*:
  - tree: `ρ(first parent of t)`'s tree with `P` replaced by `t`'s tree (a
    component root commit gives a tree containing only `P`);
  - parents: the representatives of `t`'s parents;
  - author, committer and message identical to `t`'s, plus one trailer,
    `Clonex-Source: <P> <sha of t>`;
  - `t`'s extra headers (signature, jj change-id) preserved under
    `clonex-`-prefixed names, so the shifted copy isn't mistaken for a
    signed commit or a duplicate jj change.

**Derivation** `get_P(c)` gives the component commit for composition commit
`c`:
- If `c` proposes `Clonex-Source: P t`, reconstruct `t` from `c`: strip the
  trailer, restore the headers, take the tree at `P`, and take as parents
  the derivations of `c`'s parents. **If the reconstruction hashes to `t`,
  then `get_P(c) = t`. This is proven, not trusted (D2).** If it doesn't
  (for example `c` was rebased), fetch `t` by SHA from the component and
  check `tree(t) == tree(c)[P]`. Otherwise the claim fails and `c` is
  treated as the composition's own work.
- Otherwise use the lens rule from `model.md`:
  - no change at `P` → the parent's derivation;
  - the composition's own change → a derived commit with `c`'s author and
    message, committer := author, and parents = the parents' derivations
    (merges kept).

**Adoption is just a merge.** Bringing new trunc commits into U means:
1. fetch them;
2. create their representatives (a path-shifted lineage: "fetch through the
   lens");
3. **merge that lineage with ordinary `git merge` / `jj new`.**

Git's own merge does the 3-way merge in composition space, with Git's own
conflict handling. No adoption trailer, no special adoption commit, no
CloneX merge engine.

**Laws:**
- `get_P(ρ(t,P)) = t`, the exact SHA, for every incorporated `t`.
- `ρ(get_P(u),P) = u` for composition-made changes.
- `get_P` is pure. `ρ` is pure given the composition's history.

> **R2′ is refuted; see "R2″" below, which replaces it.** The walkthrough
> that follows is kept as the record of what was believed. The attack
> (`research/r2prime-attack.md`, reproduced with real Git) showed:
> - the rule `ρ(get_P(u)) = u` mixes full composition trees into
>   component-only lineages, so UC18 plus a plain `git merge` silently
>   deletes the rest of the umbrella (F1);
> - no alternative tree choice fixes it (F2);
> - it contradicts Locality, so representatives depend on context (F4,
>   breaking K8);
> - verification that compares only trees lets a "representative" smuggle
>   other changes (F5);
> - a network fallback makes `get_P` impure (F6).

### Walking the use cases through R2′ (refuted)

- **UC1/UC4/UC18 (K4).** The alias change is one composition commit `u`.
  `get_T(u)` and `get_D(u)` are ordinary trunc and dotsync commits with
  `u`'s author and message. **`u` is the representative of both.** This is
  the original design document's "one event, several representations",
  falling out of the construction rather than being asserted.
- **UC11 then UC17 (outsider PR, merged with merge commit `m`).** Fetch
  through the lens creates `ρ(o)` and `ρ(m)`. `ρ(m)` has parents `ρ(t_prev)`
  and `ρ(o)`, and its tree changes only `tools/trunc`. `git merge` brings
  them into U. Then:
  - `git log -- tools/trunc` shows the outsider's commit with their name
    (K1);
  - `get_T` at the merge gives `m` itself, the real SHA (K3).
- **UC18 then GitHub merges U's change into trunc.** trunc's merge commit
  `m` has parents `t_prev` and `x = get_T(u)`, so `ρ(m)` has parents
  `ρ(t_prev)` and **`u`**. U's history gets no duplicate of the change. The
  merge in U simply shows that trunc accepted it.
- **UC14 (ratchet bot).** The bot commit `b` sits on top of `x`, so `ρ(b)`
  has parent `u`, and its tree is `u`'s tree with the ledger file changed.
  Merge it; the green commit `g` is made on top; `get_T(g)` has parent `b`.
  In trunc: red → bot → green, as the ratchet requires.
- **UC19 (trunc moved during unpublished work).** Merge `ρ(new trunc)` into
  U. Git merges `u`'s change with the outsider's in composition space. The
  derived trunc history is a trunc merge of `x` and the new tip, exactly
  what AGENTS.md asks for ("merge current child main into the branch").
  Conflicts are ordinary Git/jj conflicts in U.
- **UC15 (trunc force-pushed).** The rewritten lineage gets new
  representatives and is merged into U. U is not rewritten (K9). U's
  history truthfully records that it held the old version and then the new
  one. If both touched the same lines (the ledger file), that is an ordinary
  conflict, and "take theirs" is the resolution when U has nothing of its
  own there. If U never published the old version, U may instead rewrite
  privately.

  **Answer to "if the child rewrites, must the parent rewrite?": no. It
  may, if it hasn't published.**
- **UC20 (amend after review).** A private amend before publishing is
  ordinary jj. After publishing, the owner's append-only preference means a
  follow-up commit, and that just works. An actual amend of a published `u`
  is Git's usual situation. It is no worse than in plain Git, and the
  follow-up case needs nothing special.
- **UC21 (reviewer commit `r` on trunc's PR).** `ρ(r)` has parent `u`;
  merge it and continue. If `u` was *also* amended locally, the reviewer
  built on the old version. In jj that shows as a divergent change, which is
  exactly true. It is resolved the jj way: rebase `ρ(r)` onto the new
  version. Honest, and the same as plain Git.
- **UC24 (CVE fix in both copies).** One commit `u` touches both paths.
  `get_{tools/tdd-ratchet}(u)` and `get_{fixture}(u)` are two tdd-ratchet
  commits on different bases, and `u` represents both.
- **UC6/UC10 (two copies, conversion).** Each copy has its own lineage of
  representatives, `ρ(t, tools/…)` and `ρ(t, fixture)`. Conversion merges
  each tool's shifted lineage once; old U commits keep their gitlinks
  untouched.
- **UC7 (nesting).** The personal repo holds `ρ(ρ(t, tools/trunc), at)`:
  trees shift twice and trailers accumulate. The derivations compose.
- **UC8/UC9 (extract, born inside).** `get_P` over U's own history derives
  the new repo's history. Later outside commits get representatives whose
  parent is the U commit they grew from (by the rule `ρ(get(u)) = u`).
- **UC25 (plain `git rebase` of a U branch that merged representatives).**
  Git linearizes and replays each representative as a patch at
  `tools/trunc/`. That is correct content: component paths, not the root.
  - Where the replayed tree still matches, reconstruction fails (new
    committer) but the fetched `t` matches, so the SHA is kept.
  - Where the base changed, the claim fails verification. The commit becomes
    visibly "a copy of t on a new base", which is what the rebase did.
    `status` flags it.
  - Git also skips representatives the new base already contains (patch-id).

  Coarser or flagged, never silently wrong (K6).
- **UC26 (squash merge of a U PR).** One commit carries several
  `Clonex-Source` proposals. The latest one whose tree matches is proven, so
  the correspondence survives. Merge commits need nothing.
- **UC27 (cherry-pick).** The claim is valid only if the content really
  equals `t` there. If so, the correspondence is simply true, and a
  non-fast-forward publication would be refused later, at level 2.
- **UC28 (jj rewrites).** Same as UC25. Before publishing, representatives
  can be marked immutable through jj's own `immutable_heads()` so they
  aren't rewritten by accident.
- **UC29 (concurrent adoption).** Representatives are deterministic, so
  both people create identical objects. Their merges differ, like any
  concurrent merges.
- **UC31.** A forged `Clonex-Source` fails reconstruction and content
  checks, so it is own work.
- **UC16/UC33.** Components see only ordinary commits. Representatives
  travel in the composition's ordinary history, so no side refs are needed
  for objects (K7).

**Correspondence is about state (from the reconstruction experiment).**
After an amend or rebase, a representative's own object changes, but its
tree at `P` usually doesn't. The fetched original then still matches, and
`get_P` keeps answering `t`. That is intended: the claim says "the content
here *is* trunc commit `t`", and that remains true. What the rewrite changed
is only the composition's own history around it.

If the rewrite moved the commit onto a base whose derivation already
*descends* from `t`, then `get_P` reporting `t` is a content-true backwards
step. Level 2 must refuse to publish it as a fast-forward. Feasibility is
settled: a byte-exact encoding rebuilt **all 1,715 real tool commits** with
0 failures (`research/reconstruction-experiment.md`). The trailer must be
located with CloneX's own last-line rule, not Git's trailer parser.

### What R2′ still costs (not hidden)

- **Unbuildable intermediate states.** Representatives of pure component
  lineages have trees containing only `P`. Plain `git bisect` in U can land
  on such a state (unbuildable, so skip it). `--first-parent` bisect plus a
  CloneX step that tests "U state + trunc commit k" gives exact results.
  That is the same as a monorepo formed by merging repos with history.
- **Signatures on representatives.** Representatives aren't validly signed
  in U's GitHub view. The originals, reconstructed or fetched, keep their
  signatures.
- **Noise in lists.** U's commit and PR lists include component commits:
  that is what "monorepo" means (K1).
- **Squash or rebase merges in a *component*.** U's change `x` lands as a
  new commit `s`, so U then holds both `u` and `ρ(s)`. The content merges
  cleanly. A later upstream *revert* of `s` would not undo U's copy, which
  is Git's usual squash hazard. Merge-commit-only (the owner's preference)
  removes it.
- **Reconstruction must be exact.** An earlier naive attempt failed on
  184/872 real commits (signatures, encodings). Failure is safe (fall back
  to fetch and check), but it costs a fetch, so the encoding must be made
  exact and tested on every real repo.

### R2″: representatives depend only on the component (current candidate)

The repair removes the rule that caused every failure above:
- **`ρ(t, P)` is a pure function of `t`'s history and `P` alone.** Its
  tree contains only `P`; its parents are the representatives of `t`'s
  parents; its metadata is exact, plus the trailer. It is the same object
  in every composition, on every machine (K8), including machines that
  have never seen the component remote.
- **A commit is a representative only if it is *byte-identical* to the
  canonical `ρ` recomputed from the commit it claims.** That check is local
  and needs no network (K12, and `get_P` stays pure). Anything else,
  including a rebased copy or a copy with extra changes, is the
  composition's own work: honest, and visible in `status`.
- **The composition's own changes are never representatives.** When a
  composition change `u` is published as tool commit `x = get_P(u)`, the
  composition records it with a **marker merge**: parents are the
  composition head and `ρ(x)`, and the tree is unchanged (an "ours" merge).
  It says "our content already includes trunc's `x`". Later:
  - GitHub's merge `m` of `x` has `ρ(m)` with parents `ρ(t_prev)` and
    `ρ(x)`: a trunc-only lineage;
  - `git merge ρ(m)` in U has a merge base containing `ρ(x)`, so it merges
    cleanly, and an upstream *revert* of `x` is applied correctly (F8).
- **Adoption is still just `git merge`** of a representative lineage, with
  Git's and jj's own conflicts.

Laws:
- `get_P(ρ(t,P)) = t`, exactly;
- `ρ` is context-free;
- `get_P` is pure and offline.

The costs, stated:
- **Each published change appears twice** in the composition's `git log --
  tools/trunc`: once as `u` (made here) and once as `ρ(x)` (trunc's copy),
  joined by the marker. That is what a monorepo that upstreams patches
  looks like. It is the price of K8, because a context-free copy can't *be*
  `u`.
- **Component-only representative states** stay unbuildable for plain
  bisect (the K1×K8 trade-off, unchanged).

Status: **R2″ has not yet had its own adversarial replay.** The attack's
experiments suggest it fixes F1–F4, F7 and F8. Until it has been replayed
against the whole reservoir, it is a candidate, not a result.

### Is anything *impossible*?

**Claim:** no constraint in K1–K13 is violated by R2′; the costs above are
degradations within the stated tolerance.

**The one near-conflict is K1 × K8.** Intermediate representatives can't be
both deterministic (K8) and full composition states (bisectable). Their
tree would have to come from some composition context, which differs
between compositions and adoption times. R2′ chooses determinism and pays
with unbuildable intermediate states, which only affects plain,
non-first-parent bisect. This is a real trade-off and is recorded as such.

### Bonus: the original design's "compose from components"

When a composition *never* made a change but its components share a
`Clonex-Change` id, for example a view V built later from trunc and dotsync,
V has two separate representatives of one logical change. They could
optionally be **fused**: one V commit carrying both parts, with parents the
representatives of both parents. This is exactly the original design's
composition. It is deterministic and never required for correctness (D3),
and it must be skipped where the two components' histories order changes
inconsistently (the cycle counterexample).

---

## 5. Validation still to do (design, not building)

- An adversarial replay of the whole episode reservoir (`episodes.md`,
  `research/*`) against R2′, especially intersections: repeated copies ×
  rewrites, nesting × squash, bot commits × amend.
- An exact-reconstruction encoding checked against all ~1,700 real tool
  commits (an experiment, not product code).

## 6. Level 2 (refs) — deliberately not designed yet

Refs are mutable labels synced between repos, with no content history of
their own (jj keeps an operation log of them; a second layer, not
history). Level 1 above never reads a ref.

The level-2 use cases to walk next:
- which labels a composition offers for its components' labels;
- how a U change gets a trunc label for a PR;
- bot commits arriving on such a label;
- abandoned and reused names;
- two copies of one tool;
- forks;
- label renames;
- config stored as commits in a composition-only ref.

The owner likes this last idea: jj stores operations as objects too.
