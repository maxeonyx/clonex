# Does CloneX strictly beat Git and jj? An aspect-by-aspect audit

Independent audit, 2026-09-28. Scope: every aspect of daily development that
developers and agents use Git or jj for, not only composition. Three
representations of "a composition contains component histories" are compared:

- **R1 VERBATIM** (current, `model.md` §8): component commits are second
  parents of adoption merges in the composition's branch history.
- **R2 SHIFTED REPS**: each adopted component commit `t` appears as a
  deterministic composition-shaped commit `rep(t,P)` (tree = parent rep's tree
  with P replaced; foreign roots contain only P), same author/committer/message
  plus `Clonex-Source: <P> <sha>`, extra headers renamed `clonex-*`; `t` must
  be reconstructible bit-exactly.
- **R3 SQUASHED + SIDE REFS**: branch history contains no component commits.
  An adoption is an ordinary single-parent commit carrying
  `Clonex-Adopt: <P> <sha>` (parsed anywhere in the message; valid when the
  object exists; exact when `tree(c)[P] == tree(sha)`). Component objects are
  kept reachable by `refs/clonex/adopted/<P>/<sha>` on the composition's remote.

Baselines: **(a)** polyrepo + git submodules (today's umbrella), **(b)** plain
monorepo, **(c)** jj on either.

Method: I read `model.md` (§8 as authoritative), `AGENTS.md`, `src/*.rs`,
`research/anchor-attack.md` and `research/refs-workflows.md`, and re-used
`research/git-interop-matrix.md` for carrier survival. Then I ran experiments
with git 2.55, jj 0.44 and the `clonex` debug build, on copies of the real
tool repos (trunc, dotsync, tdd-ratchet-rs, agent-harness, and a
seven-tool composition). I built R2 and R3 with a prototype builder over the
same four real repos, so all three shapes hold identical content. Scripts are
in the session scratchpad `audit/` (`build.py` builds R1/R2/R3 and adopts;
`recon.py` checks R2's reconstruction). Anything I did not run is marked
**(reasoned)**. Anything about GitHub that was not tested on GitHub is marked
**[GH-unverified]**.

---

## 0. Evidence (experiments run)

| Id | Experiment | Result |
|---|---|---|
| X1 | `clonex status` in a fresh plain `git clone` of a composition | agent-harness is reported with **"141 change(s) not on the followed branch"**. The cause: `refs/remotes/clonex/*` isn't cloned, so `remote_tip = None`, and all of history counts as "ahead". An agent reading this would conclude there are 141 unpublished changes. |
| X2 | Path-limited history in each shape vs the native component (trunc `src/lib.rs`, `README.md`, `Cargo.toml`; dotsync `src/main.rs`). Native trunc: log 13 / blame 10 / blame 7 commits | `git log -- tools/trunc/src/lib.rs`: **R1 1, R2 13, R3 1**. `--follow`: R1 **0**, R2 13, R3 1. dotsync `main.rs` log: R1 1, R2 91, R3 1. So R2 matches native `git log -- path` exactly; R1 and R3 show only the adoption commit. |
| X3 | `git blame` distinct commits | lib.rs: R1 12, **R2 12**, R3 1. README.md: **R1 1 (wrong)**, R2 10 (native 10), R3 1. Cargo.toml: **R1 1 (wrong)**, R2 7 (native 7), R3 1. R1's blame works only when the component path does not collide with a composition root file. Git follows the component side by rename detection, but `README.md`, `Cargo.toml`, `LICENSE`, `AGENTS.md` and `.github/…` exist at the composition root, so they count as modified rather than renamed and blame stops at the adoption merge. After a sync, the upstream line `upstream line` was blamed on the adoption merge `9dc97cc`, not on `a780e14`. |
| X4 | `git bisect run` across an R1 sync (regression introduced upstream) | Every component commit had to be skipped (root layout, no `tools/trunc`). Result: "only skipped commits left to test", with 4 possible first-bad commits. `--first-parent` lands on the adoption merge. |
| X5 | `git rebase main` on a feature branch that adopted two upstream trunc commits, after composition `main` moved | **R1**: conflict `UU README.md`, because trunc's README change was replayed onto the *composition root* README. `--rebase-merges` gave **the same conflict**. **R2**: exit 0; reps linearized onto main, adoption merge dropped, `tools/trunc` tree equals the upstream tip, `Clonex-Source` trailers kept; `--rebase-merges` kept the rep chain byte-identical. **R3**: exit 0, adoption commit rebased, trailer kept, tree exact. |
| X6 | GitHub-style squash merge of the composition PR (`merge --squash`, commit with all messages) | **R1**: `clonex get tools/trunc` returned a **jump commit** `75a14a5` whose parent is the old base `7d283c6`. Upstream commits `a780e14` and `ac27125` are lost from derived history, and `publish --branch main` then reports "diverged". **R3**: the trailer is in the squash body, so the claim is still valid and exact. **R2**: the rep chain is gone from the branch; `Clonex-Source` lines appear in the body. |
| X7 | jj, fresh clone of an R1 composition, feature branch with adopted upstream commits | The adopted component commits are **mutable** (`mut`): the A6 fix (`refs/remotes/clonex/*`) exists only in the clone that ran `sync`. `jj rebase -b feat -d main` **rewrote three component commits** into conflicted composition-parented commits (`×`). |
| X8 | `clonex sync` in a colocated jj repo with a dirty `@` | Content was fine. The sync became **three jj operations** ("import git head", "snapshot", "import git refs"). One `jj undo` left the adoption in place **and deleted `refs/remotes/clonex/tools/trunc/main`**, the ref that makes component commits immutable. The `feat` bookmark was not advanced, because jj keeps HEAD detached. |
| X9 | `clonex sync` when upstream and composition edited the same README line | `clonex: … conflicts with changes the composition holds: README.md`, exit 1, **with no merge state, no markers and no jj conflict**. The manual `git merge --no-commit -Xsubtree=tools/trunc <tip>` produced a normal resolvable conflict (`UU tools/trunc/README.md`). |
| X10 | `git clone --depth 1` (what `actions/checkout` does by default) | Every `clonex` command fails: `commit 60aa6e5… is not in this repository`. `--depth 50` fetched **550 commits, of which 10 are first-parent**: component commits eat the depth window. |
| X11 | Plain `git clone` of each shape | R1: 883 commits, 3.1 MB. R3: **9 commits, 668 KB**, with no `refs/clonex`, so the claimed objects are missing. An explicit `git fetch origin 'refs/clonex/*:refs/clonex/*'` restores all 883. |
| X12 | Making R3's side refs travel | `jj git fetch` **ignores** a `+refs/clonex/*` refspec with a warning ("only refs/heads/ is supported"). Refs under `refs/tags/clonex/*` pointing at unrelated history are **not** fetched by `git clone` (0 tags, also at depth 1). So only a branch, or a parent edge, makes objects arrive by plain clone. |
| X13 | R2 bit-exact reconstruction on 872 real commits with a naive encoding | 688 exact, **184 mismatched**: web-flow-signed GitHub merges and messages without a final newline (32–45 of them per repo). On the reps, `%G?` = N: signatures are gone. R1 keeps 67 signed commits intact. An R2 encoding must record the exact message tail. |
| X14 | Scale: 3,010 first-parent composition commits touching trunc | `clonex get` 5.1–5.5 s and `status` 5.4 s, **on every invocation** (no persistent memo). That is about 1.7 ms per commit, so 100k commits would take about 3 min per status. The run also created 3,028 loose objects. |
| X15 | `git blame $(clonex get tools/trunc) -- README.md` and `git log … -- src/lib.rs` in R1 | **Exact native results**, including the upstream line attributed to `a780e14`. Lens read views are cheap. |
| X16 | Component hooks | dotsync ships `.githooks/pre-push` (a version-bump check on `main`), and its AGENTS.md tells release clones to set `core.hooksPath`. `clonex publish` pushes with `--no-verify` from the composition, so the hook never runs. |
| X17 | Component messages in composition history (R1/R2) | 80 `#N` references across the tools (e.g. "Merge pull request #50 from …"), 0 closing keywords today. On the composition's GitHub page they link to the *composition's* #50 **[GH-unverified]**. |
| X18 | PR commit list for "one composition commit + adopt 2 upstream commits" | R1 4, R2 4, **R3 2**. Declaring dotsync alone adds 480 commits to an R1/R2 PR, beyond GitHub's 250-commit PR list **[GH-unverified]**. |
| X19 | R2 rep trees | A mid-history dotsync rep contains **only `tools/dotsync`**. Checking it out removes every other occurrence, so a composition-level bisect step cannot build there, although a component-level test can. |

Code facts (`src/`):
- C1: `plan_adoption` skips a commit that is an ancestor of the current state
  (`already contained → continue`). The only way to adopt a chosen commit is
  `declare --at`, and only for a new occurrence. So there is **no pin-back or
  downgrade, no adopting a tag, no adopting `refs/pull/N/head` or a fork
  branch**.
- C2: derived commits are written with `--no-gpg-sign`; publish uses
  `--no-verify`.
- C3: `fetch` uses `+refs/heads/{branch}` against the one manifest URL; the
  tracking ref is keyed by path, so there is one remote per occurrence.
- C4: fetch uses `--no-tags`; there is no tag or release operation.
- C5: there is no LFS handling on publish.
- C6: `Composition::adoptions` requires the claimed sha to be a parent (R1).
  This rule is what X6 breaks.

---

## 1. Aspect inventory

Legend per cell: **vs Git / vs jj**. `+` better, `=` equal, `−` worse
(`−−` = blocker-grade), `n/a` not applicable. "Git" means the better of
polyrepo+submodules (a) and monorepo (b) for that aspect, used with git; "jj"
is the same with jj (jj has no submodule support, so jj-on-(a) means jj inside
each tool repo). A column assumes the CloneX binary **as built today** plus
git/jj as the everyday client. Where a row says "with views", it rates
CloneX after the §4 client views exist.

### A. Transport, repository shape, offline

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| A1 | Plain clone gets all content | (a) needs `--recurse-submodules`; (b) yes; jj: no submodules | + / + | + / + | + / + | tree holds content |
| A2 | Plain clone gets component *history* | (a) yes, per submodule; (b) yes | = / = | = / = (reps, not originals) | − / − | X11 |
| A3 | Clone size / time | (a) small superproject; (b) whole | − (whole histories) | − (reps + trees) | + (small) | X11 |
| A4 | Fetch / pull the composition | ordinary | = / = | = / = | = / − (jj drops side refs) | X12 |
| A5 | Fetch component updates | (a) `submodule update --remote`; jj n/a | + / + (`sync`) | + / + | + / + | |
| A6 | Push the composition | ordinary | = / = | = / = | − / − (side refs need extra refspec) | X12 |
| A7 | Push component changes | (a) cd + push each + bump | + / + (`publish`) | + / + | + / + | |
| A8 | Shallow clone usability | fine | −− / −− (clonex dies) | −− | −− (claims unresolvable) | X10 |
| A9 | Depth window meaning (`--depth N`) | N first-parent-ish commits | − (550 for 50) | − | = | X10 |
| A10 | Partial clone `--filter=blob:none` | works | = (lens needs trees; merges fetch blobs lazily) (reasoned) | = | = | |
| A11 | Sparse checkout of one component | (a) init one submodule; (b) sparse | = / = | = / = | = / = | |
| A12 | Mirrors / `--mirror` / bundles | ordinary | = | = | − (bundle clone drops custom refs) | matrix |
| A13 | Offline work after clone | full | = | = | − (no component history offline) | X11 |
| A14 | Multiple remotes per component (upstream + fork) | (a) native | − / − (one URL per path) | − | − | C3 |
| A15 | Fork of the composition | (a) forks + `.gitmodules` URLs; clumsy | = (`insteadOf`) | = | − (forks don't copy custom refs [GH-unverified]) | |
| A16 | Contributing from a fork PR (adopt `refs/pull/N/head`) | (a) fetch + checkout in submodule | − / − | − | − | C1, C3 |
| A17 | Archives / tarballs / GitHub zip | (a) **no submodule content**; (b) yes | + / + | + / + | + / + | |
| A18 | Repo moved, renamed or rehosted | URL edit | = (manifest edit; old commits keep old URL as data only) | = | = | |

### B. Inspection

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| B1 | `status` of the working copy | native | = / = | = / = | = / = | |
| B2 | Status of all components (ahead/behind) | (a) `submodule status`/`foreach` | + / + but **misleading in fresh clones** | same | same | X1 |
| B3 | `diff` in the working tree or across commits | native | = | = | = | |
| B4 | Diff of a component bump | (a) SHA pair only; (b) content | + / + (content diff) | + | + | |
| B5 | Default `git log` | (b) clean; (a) superproject clean | − (interleaved, 877 vs 5) | − | = | X2 |
| B6 | `log --first-parent` | native | = | = | = | |
| B7 | `log -- <component path>` | (a) in submodule; (b) native | −− (adoption only) | = | −− (adoption only) | X2 |
| B8 | `log --follow` | native | −− (0 results) | = | − | X2 |
| B9 | `show <component commit>` from the composition | (a) in submodule | − (root-layout paths) | = (composition paths) | − (object may be missing) | X11 |
| B10 | `blame` of component files | (a) in submodule; (b) native | − (wrong on root-name collisions) | = | −− (all lines = adoption) | X3 |
| B11 | Blame "which composition change introduced this line" | (b) native | = | − (blames rep, not the composition commit) | − | |
| B12 | `bisect` a component regression | (a) in submodule; (b) native | −− (all skipped) | − (P-only reps) | −− (lands on adoption) | X4, X19 |
| B13 | Bisect a composition regression caused upstream | (a) manual two-level; (b) native | − | − | − | X4 |
| B14 | `grep` the working tree / a tree | native | = | = | = | |
| B15 | Pickaxe `-S`/`-G` across component history | (a) per submodule; (b) native | = without a path, − with one | = | − | run |
| B16 | `shortlog` / authorship stats | native | = (counts component authors) | = | − (adopter gets credit) | run |
| B17 | `describe` / "which version of tool X" | (a) `submodule status` describe | − (no component tags) | − | − | C4 |
| B18 | `range-diff` of a republished PR | native | = | = | = | |
| B19 | `log --grep` / message search | native | = | − (dup hits rep + original) | − (component messages absent) | |
| B20 | `whatchanged`, `log --stat` on component commits | (a) native | − (root paths) | = | − | |
| B21 | jj `log` default view | clean | − (fresh clone floods mutable component commits) | − | = | X7 |
| B22 | jj `file annotate` | native | − (as B10) | = | −− | X3 |
| B23 | Component-space views through the lens (`clonex log`) | n/a | + (exact, X15) | + | + | X15 |

### C. Working copy

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| C1 | Working-copy model (index vs working-copy commit) | git index; jj `@` | = / = | = | = | |
| C2 | Editing across components in one checkout | (a) N repos, detached HEADs; (b) yes | + / + | + | + | |
| C3 | Stash | git stash; jj n/a | = / = | = | = | |
| C4 | Multiple working copies (worktrees / workspaces / clones) | native | = | = | = | |
| C5 | `.gitignore` / `.gitattributes` inside a component | (a) per repo; (b) hierarchical | = | = | = | |
| C6 | Root-only config of components (`.github/`, `.mailmap`, CODEOWNERS, `.git-blame-ignore-revs`, `core.hooksPath`, pre-commit) | (a) active per repo | − (inert inside the composition) | − | − | X16 |
| C7 | Build tooling (nested cargo workspaces, rust-analyzer) | (a) separate; (b) one | = (monorepo-shaped) | = | = | (reasoned) |
| C8 | No detached submodule HEADs / "forgot to commit in submodule" | (a) chronic pain | + / + | + | + | |
| C9 | CloneX moving HEAD under a jj colocated `@` | n/a | = (works; bookmark not advanced) | = | = | X8 |
| C10 | Filesystem modes, symlinks, case | native | = (trees verbatim) | = | = | |

### D. Refs: branches, bookmarks, anonymous heads

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| D1 | Composition branches / bookmarks | native | = | = | = | |
| D2 | Anonymous heads (jj) | jj native | = | = | = | |
| D3 | Remote-tracking refs for components | (a) per submodule | = (`refs/remotes/clonex/<path>/<b>`) | = | = | |
| D4 | Immutability of foreign history (jj `immutable_heads`) | jj: remote bookmarks | −− (clone-local only) | − (reps mutable; rewrites keep P content) | = (nothing foreign in branch) | X7 |
| D5 | One logical branch name across repos | today: same slug by hand | + (publish uses the composition branch name) | + | + | |
| D6 | Stacks within the composition | git `--update-refs`, jj auto-rebase | = | = | = | |
| D7 | Stacks across repos | nobody | + (a composition stack publishes per repo) (reasoned) | + | + | |
| D8 | Delete-on-merge / `[gone]` | native | = | = | = | |
| D9 | Name collision detection | none | = | = | = | |
| D10 | `trunk()` per component | jj per repo | = (manifest `follow`) | = | = | |
| D11 | Protected refs known locally | jj immutable | = | = | = | |
| D12 | Bot commits on PR branches (ledger bot) | manual merge | + (`sync --branch`) but placement defect A9 | same | same | model §8 |

### E. Integration: merge, rebase, cherry-pick, revert, conflicts

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| E1 | `git merge` of composition branches | native | = | = | = | |
| E2 | Merging a component update | (a) gitlink conflict = opaque SHA pair | + (3-way content merge) | + | + | |
| E3 | Adoption with conflicts | (a) resolve in submodule; jj first-class | −− (refuses; no state) | −− | −− | X9 |
| E4 | Plain `git rebase` of a composition branch | native | −− (root replay conflict) | − (linearizes reps; merges dropped) | = | X5 |
| E5 | `git rebase --rebase-merges` | native | −− (same conflict) | = | = | X5 |
| E6 | `git rebase -i` (reorder, drop, edit) | native | −− (component commits are in the todo list) | − (reps are in the todo list) | = | (reasoned from X5) |
| E7 | `jj rebase -b/-s/-r` | native | −− (fresh clone) | − | = | X7 |
| E8 | Cherry-pick a composition commit | native | = (`-m1` for adoptions) | = | − (copies an adoption claim; see REG-17) | matrix |
| E9 | Revert a composition commit | native | = (`-m1`) | = | = | |
| E10 | Revert or roll back a component version | (a) checkout old SHA + bump | −− (no downgrade; a manual revert publishes a revert) | −− | −− (exact claim would fix it) | C1 |
| E11 | First-class conflicts after adoption | jj | = once the commit exists | = | = | |
| E12 | rerere / merge drivers | native | = (the adoption merge is `merge-tree`; no rerere) | = | = | (reasoned) |
| E13 | Upstream force-push or rewrite of a component | (a) dangling pin | + (§8 own-work reapply) | + | + | model §8 |
| E14 | Squash-landed-then-reverted upstream | (a) n/a | + (patch-id detection) | + | + | model §8 |

### F. Rewriting

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| F1 | Amend the last composition commit | native | = | = | = | |
| F2 | Squash / fixup composition commits | native | = if no adoption is inside; −− if one is | − | = (claims survive; parsed anywhere) | matrix |
| F3 | Split | native | = | = | − (`jj split` duplicates the claim; the exactness check disambiguates) | matrix |
| F4 | `jj absorb` | native | −− (can absorb into mutable component commits in a fresh clone) (reasoned from X7) | − | = | |
| F5 | Automatic descendant rebase (jj) | native | = | = | = | |
| F6 | Editing published cross-repo work | (a) edit + force-push N repos | + (republish under lease) | + | + | |
| F7 | Rewriting beneath an adoption (stale bot commits) | (a) manual | − (A5, not built) | − | − | model §8 |
| F8 | Filter-repo / history surgery on the composition | native | − (must preserve adoption parents) | − | = | |
| F9 | Messages of derived commits (single-repo readers) | n/a | − (a trunc commit says "tweak trunc and dotsync") | − | − | run |

### G. Undo, operation log, recovery

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| G1 | reflog for CloneX ref moves | native | = (`update-ref -m clonex`) | = | = | |
| G2 | jj `undo` of a CloneX operation | one op | − (3 ops; undo deletes the clonex tracking ref) | − | − | X8 |
| G3 | Undo a publish | git/jj: none (remote) | = | = | = | |
| G4 | Recover from a plain-git mistake | reflog | − (X5 damage) | = | = | X5 |
| G5 | Lost-work safety (unpublished component work) | (a) in submodule | + (it is composition content) | + | + | |
| G6 | Re-derivability after gc | n/a | + (deterministic) | + | + | |
| G7 | `fsck` / integrity | native | = | = | − (dangling claims if side refs are lost) | |

### H. Query languages, templates, scripting

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| H1 | Revsets over the composition | jj | = | = | = | |
| H2 | Revsets that span component histories | none | + with views | + | + with views | |
| H3 | Templates / format strings | native | = | = | = | |
| H4 | Machine-readable output | porcelain v2; jj templates/JSON | = (`--json` on most commands; `log --json` gives only a SHA) | = | = | main.rs |
| H5 | Stable exit codes / schema versioning | stable | − (no schema version) | − | − | |
| H6 | Aliases / extension points | native | = | = | = | |
| H7 | Determinism of derived results across machines | n/a | + (L3) | + | − unless missing objects fail closed | X11 |

### I. Hooks and policy

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| I1 | Component client hooks (pre-commit, pre-push) | (a) run per repo | − (bypassed) | − | − | X16 |
| I2 | Composition hooks | native (jj runs no hooks) | = | = | = | |
| I3 | Server-side protection on components | enforced on push | = (publish can be rejected; reported) | = | = | |
| I4 | Required signed commits on components | satisfiable | −− if enabled (derived commits unsigned) | −− | −− | C2 |
| I5 | CODEOWNERS for component paths | (a) per repo | = (composition can map paths) | = | = | |
| I6 | Guards against destructive plain-git ops | n/a | − (designed, not built) | = | = (nothing to guard) | model §8 |

### J. Signing, provenance, SHA-keyed data

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| J1 | Sign composition commits | native | = | = | = | |
| J2 | Component commit signatures kept | native | = (67 signed kept) | −− (reps unsigned) | = (objects kept) | X13 |
| J3 | Sign derived commits | native | − (breaks L3; §7 resolution unbuilt) | − | − | C2 |
| J4 | Commit statuses / ledger keyed by SHA visible from the composition | (a) in submodule | = | − (rep SHAs differ) | = via views | X13 |
| J5 | Round-trip exact SHAs (L1) | n/a | + | + if the encoding is exact (21% failed naively) | + | X13 |

### K. Tags and releases

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| K1 | Tag a component release from the composition | (a) cd + tag + push | − (no operation) | − | − | C4 |
| K2 | See component tags in the composition | (a) in submodule | − (`--no-tags`) | − | − | C4 |
| K3 | Composition release pinning component versions | (a) gitlinks; archives lack content | + | + | + | |
| K4 | Tag namespace collisions (`v0.1.0` × 7) | n/a in (a) | = (avoided by not fetching) | = | = | |
| K5 | Moved or re-pointed tags detected | poor | = | = | = | |
| K6 | GitHub releases per component | per repo | = (unchanged) | = | = | |

### L. Cross-repo work (the core)

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| L1 | One logical change across N repos | (a) N commits + bump; (b) one commit but no independent repos | + / + | + | + | |
| L2 | Publication to independent repos | (b) impossible without Josh/Copybara | + / + | + | + | |
| L3 | Pointer bumps | (a) required | + (none) | + | + | |
| L4 | Partial landing (one child fails CI) | (a) umbrella blocked | + (content-level; publish is convergent) | + | + | |
| L5 | Retry after a partial multi-remote failure | manual | + (idempotent) | + | + | |
| L6 | Adopt a PR branch with bot commits | (a) manual | + (`sync --branch`) | + | + | |
| L7 | Pin a component to a chosen older commit or tag | (a) trivial | −− | −− | −− | C1 |
| L8 | Frozen vs followed occurrences | (a) `.gitmodules branch` only with `--remote` | + | + | + | |
| L9 | Same component twice | (a) two submodules | = | = (distinct reps per path) | = | |
| L10 | Nested compositions | (a) nested submodules | + (L5 tested) | ? (untested) | ? (the claim must re-root) | |
| L11 | Extract a directory into its own history | filter-repo, one-shot | + (`declare --extract`) | + | + | |
| L12 | Subtree-style vendoring with a local patch | `git subtree` | + (delta) | + | + | |
| L13 | "Where does change X appear" across repos | none | + (jj change id) | + | + | |
| L14 | Occurrence moves (`git mv tools/x tools/y`) | (a) `git mv` of submodule works | − (A3: new root, `clonex mv` unbuilt) | − | − | model §8 |
| L15 | Sub-path occurrence (`R/src/ledger`) | sparse submodule impossible | − (unsupported) | − | − | model §8 |
| L16 | Ephemeral CI composition | (a) script | + (deterministic) but dies when shallow | same | same | X10 |

### M. Large files

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| M1 | LFS pointers in component subtrees | per repo | = (attributes are hierarchical) | = | = | |
| M2 | LFS objects reach the component's LFS store on publish | per repo | − (not pushed) | − | − | C5 |
| M3 | Large binary history cost in every clone | (a) per submodule, optional | − (always in clone) | − | + (only on fetch of side refs) | X11 |

### N. Performance and maintenance

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| N1 | Command latency at small scale | ms | = (0.13 s status) | = | = | X14 |
| N2 | Latency at 3k–100k commits | ms (commit-graph) | −− (5 s at 3k, linear, uncached) | −− | −− | X14 |
| N3 | Loose-object churn from derivation | n/a | − (3,028 loose objects) | − | − | X14 |
| N4 | GC safety of adopted objects | reachable | = | = | − (depends on side refs) | |
| N5 | Ref count growth | small | = | = | − (one ref per adoption unless consolidated) | |
| N6 | Object count growth | n/a | = | − (a rep per component commit per path) | = | X11 |

### O. GitHub UI and hosting

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| O1 | PR "Files changed" for a cross-cutting change | (a) N PRs + SHA bump; (b) one diff | + | + | + | |
| O2 | PR commit list | clean | −− (floods; 480 on declare; 250 cap) | −− | = | X18 |
| O3 | Branch commit list (`/commits/main`) | clean | − (interleaved) | − | = | X2 |
| O4 | File history view in the composition | (b) full; (a) in the tool repo | − (adoptions only) | = | − (adoptions only) | X2 |
| O5 | Blame view in the composition | (b) full; (a) in the tool repo | − (wrong on collisions) | = | − | X3 |
| O6 | Compare view | native | − (floods) | − | = | |
| O7 | "Create a merge commit" button | native | = | = | = | |
| O8 | "Squash and merge" button | native | −− (drops adoption parents) | − (objects only via `refs/pull`) | = | X6 |
| O9 | "Rebase and merge" button | native | −− (replays or recreates component commits [GH-unverified]) | − | = | matrix |
| O10 | "Update branch" (rebase variant) | native | −− | − | = | matrix |
| O11 | `#N` autolinks / closing keywords in commit messages | correct repo | − (80 references mislink) | − | = (adoption summaries still quote subjects) | X17 |
| O12 | Verified badges | native | = | −− on reps | = | X13 |
| O13 | Checks and statuses on commits | per repo | = | − | = | |
| O14 | Contributors graph / attribution | (b) all authors | = | = | − (fix: `Co-authored-by`) | |
| O15 | Code search, Dependabot, code scanning | (b) native; (a) per repo | = (tree content) | = | = | |
| O16 | "Commit does not belong to any branch" banner | n/a | = | = | − (side-ref objects) | |
| O17 | Archive / Pages / Codespaces of the composition | (a) no submodule content | + | + | + | |

### P. CI

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| P1 | `actions/checkout` default depth 1 | works | −− (clonex fails) | −− | −− | X10 |
| P2 | `fetch-depth: 0` cost | moderate | − (all component histories) | − | + | X11 |
| P3 | Path filters (`paths: tools/trunc/**`) | (b) native; (a) via gitlink change | = | = | = | |
| P4 | Component CI on a composition PR | (a) per tool PR | = (runs after publish, on the component PR) | = | = | |
| P5 | Component workflows active in the composition | (a) n/a | − (inert: `.github` must be at the root) | − | − | C6 row |
| P6 | Ledger bot / required-status choreography | fragile | = (A5/A9 open) | = | = | model §8 |
| P7 | Composition CI "is this pinned commit on the remote?" | (a) manual rule | + (publish status) | + | + | |
| P8 | Build-cache keys by commit SHA | per repo | = | = | = | |

### Q. Review

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| Q1 | Review a cross-repo change as one unit | (a) N tabs | + | + | + | |
| Q2 | Component maintainers review an ordinary PR | yes | = (derived commits are ordinary) | = | = | |
| Q3 | Review iterations (force-push of a republished PR) | native | = (lease) | = | = | |
| Q4 | Reviewer sees exactly what a component bump brings | (a) SHA only | + (content diff) | + | + (plus a subject list in the message) | |
| Q5 | Review of the adopted history itself | (a) in the tool repo | − (hundreds of commits in the PR) | − | = | X18 |

### R. Third-party tools

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| R1t | gitk / tig / `log --graph` | clean | − (huge graph with root-layout commits) | − (huge but composition-shaped) | = | X2 |
| R2t | lazygit / magit / GitHub Desktop rebase UIs | native | −− (they drive `git rebase`) | − | = | X5 |
| R3t | VS Code / GitLens / JetBrains file history and blame | native | − | = | − | X2, X3 |
| R4t | IDE multi-repo SCM (submodule detection) | (a) N repos | + (one repo) | + | + | |
| R5t | `gh` CLI (pr create, checks) | per repo | = | = | = | |
| R6t | git-branchless, spr, ghstack, Graphite | single-repo | − (merges with foreign commits confuse them) (reasoned) | − | = | |

### S. Agent-friendliness

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| S1 | Complete structured state in one call | none across repos | + (`--json status`) | + | + | |
| S2 | Truthful state in a fresh clone | yes | −− ("141 unpublished changes") | −− | −− | X1 |
| S3 | Non-interactive conflict path | git markers; jj conflicts | −− (no state) | −− | −− | X9 |
| S4 | Safe to run ordinary git or jj commands blindly | yes | −− (rebase, squash, absorb hazards) | − | = | X5–X7 |
| S5 | Idempotent retries | manual | + | + | + | |
| S6 | Deterministic IDs across concurrent clones | n/a | + | + | + if it fails closed | |

### T. Learning curve, recovery, security

| # | Aspect | Git / jj today | R1 | R2 | R3 | Evidence |
|---|---|---|---|---|---|---|
| T1 | New concepts to learn | (a) submodules are notoriously hard | + (declare/sync/publish) | + | + | |
| T2 | Footguns in familiar commands | (a) many | − (rebase, squash, jj) | − (rebase linearizes) | + | X5–X7 |
| T3 | Explaining the history shape to a newcomer | simple | − (why root-layout commits?) | − (why duplicate commits?) | + (plain monorepo shape) | |
| T4 | Purging a secret or licence-tainted history from a component | (a) purge the tool repo | −− (append-only composition main keeps it forever) | −− | − (content in adoption trees; side refs deletable) | |
| T5 | Trust: reviewing what an adoption imports | (a) opaque SHA | + | + | + | |
| T6 | Forged or misleading claims | n/a | + (claim must be a parent, tree checked) | = | − (object-exists only; inexact claims create merge edges) | reasoned |
| T7 | Supply chain: adopting history from untrusted forks | (a) fetch in submodule | = | = | = | |
| T8 | Recovering a clone after a CloneX bug | reflog | = | = | = | |

Count: 192 aspects.

---

## 2. Regression list

The severity scale is set by the owner's bar ("strictly beats on all
aspects"). **Blocker**: a common daily operation is broken, silently wrong,
or impossible. **Significant**: a real workflow degrades or needs a
workaround. **Minor**: cosmetic, rare, or easily worked around.
**Inherent** = follows from "compositions contain histories" in every
representation. **Accident** = representation or implementation choice.

### Blockers

| Id | Regression | R1 | R2 | R3 | Inherent? | Most promising fix |
|---|---|---|---|---|---|---|
| REG-1 | Plain `git rebase`, `--rebase-merges`, `rebase -i`, and rebase UIs (lazygit, magit) replay component commits onto the composition (X5) | ✗ | partial (linearizes; identity degrades) | ✓ | **Accident of R1**: foreign-shaped commits in branch history | R3. In R1 no client-side fix exists short of CloneX owning rebase. |
| REG-2 | GitHub squash, rebase-merge and "update branch (rebase)" drop adoption parents, so derived history jumps (X6) | ✗ | partial | ✓ | Accident (R1 rule C6: claim valid only as a parent) | R3's validity rule (object exists, retained by side refs). R1 could adopt the same rule as a fallback. |
| REG-3 | jj in a fresh clone treats adopted component commits as mutable; rebase, absorb and squash rewrite them (X7) | ✗ | partial (reps are mutable but composition-shaped) | ✓ | Accident of R1 plus the clone-local A6 fix | R3. Otherwise push a shared immutable ref and a jj `immutable_heads()` config. |
| REG-4 | An adoption conflict aborts with no resolvable state (X9) | ✗ | ✗ | ✗ | Accident (implementation) | Leave a normal in-progress merge: `git merge --no-commit -Xsubtree=P` (R1/R2), or `merge --squash -Xsubtree` (R3), with `MERGE_MSG` pre-filled with the claim. Better: write a jj conflicted commit. |
| REG-5 | Component file history, blame and bisect are absent or wrong from the composition with git/jj/IDEs (X2–X4) | ✗ (blame wrong on collisions) | ✓ for log/blame; bisect partial | ✗ | Inherent for R1/R3 when using *native* tools. X15 shows the lens gives exact answers. | CloneX read views (§4). GitHub UI stays worse than monorepo; see REG-20. |
| REG-6 | Shallow checkouts (Actions default): every `clonex` command fails (X10) | ✗ | ✗ | ✗ | Accident | Treat a shallow boundary as "unknown": deepen or fetch on demand (`fetch --deepen`, fetch the claimed sha from the component remote), otherwise answer partially and say so. Never derive across a graft. |
| REG-7 | Cannot pin, roll back or adopt a chosen commit, tag, `refs/pull/N/head` or fork branch (C1, C3) | ✗ | ✗ | ✗ | Accident; the model supports it (an exact claim) | `clonex adopt P <rev-or-url#ref>` including ancestors. Rule: an exact claim sets the state even when backwards. R3 makes this natural. |
| REG-8 | `status` in a fresh clone reports the whole history as unpublished (X1) | ✗ | ✗ | ✗ | Accident | Fetch on first use, or report "remote tip unknown, never fetched". Record tracking state in the manifest-declared remote, not in local-only refs. |
| REG-9 | PR commit lists, compare views and `/commits` flood with component commits; declaring one tool adds 480 (X18, X2) | ✗ | ✗ | ✓ | Accident of R1/R2 | R3. |
| REG-10 | Every command walks all history uncached: 5 s at 3k commits, linear (X14) | ✗ | ✗ | ✗ | Accident | A persistent memo `composition commit → derived commit` (in `refs/clonex/cache` notes or a local index keyed by commit), plus `git commit-tree` batching (fast-import). Blocker-grade only at scale; the real umbrella today takes 0.13 s. |

### Significant

| Id | Regression | Reps | Inherent? | Fix |
|---|---|---|---|---|
| REG-11 | Component pre-push/pre-commit hooks bypassed (`--no-verify`; dotsync's real hook, X16) | all | Accident | Publish runs the component's declared hooks in a materialized worktree of the derived commit (`clonex exec P`). Drop `--no-verify`. |
| REG-12 | Derived commits are unsigned; "require signed commits" protection would reject publish (C2) | all | Partly inherent (signing vs L3) | Sign at publish time, then adopt the signed object back (L1 makes it the base). Needs building. |
| REG-13 | `jj undo` after a CloneX op undoes a fragment (3 ops) and deletes the tracking ref (X8) | all | Accident | Run CloneX's ref updates as one jj transaction (jj-lib, or `jj` CLI with `--ignore-working-copy` plus `op` grouping). Keep tracking data in refs jj does not export-delete. |
| REG-14 | Secrets or tainted history in a component can't be purged from an append-only composition (T4) | R1/R2 worst; R3 better | Inherent for content in trees; accident for *history* retention | R3 (side refs deletable). A documented purge procedure. |
| REG-15 | One remote per occurrence; no fork or `refs/pull` adoption (C3) | all | Accident | Occurrence remotes become named git remotes (`clonex/<path>`, `clonex/<path>-fork`), with a refspec-accepting `sync --ref`. |
| REG-16 | R3: plain clone and jj lack the component objects; `jj git fetch` ignores side refs (X11, X12) | R3 | Accident of R3 | Lazy fetch-by-sha from the *component* remote (adopted foreign commits came from there), with side refs as the durability backup. Optionally one `clonex/history` branch (a keeper chain) so plain clones carry everything. |
| REG-17 | R3: cherry-picking an adoption copies its claim; an inexact claim then turns into a false merge edge in derived history | R3 | Accident of R3's rule | Accept inexact claims only when the commit's parent P tree equals a recorded `ours` tree (`Clonex-Adopt: P sha ours=<tree>`). Otherwise treat the commit as a plain content change. Exact claims are always safe. |
| REG-18 | R3: derivation depends on which objects are present, so two clones could publish different histories | R3 | Accident | Fail closed: a missing claimed object means fetch, else refuse to derive. |
| REG-19 | R2: rep SHAs ≠ component SHAs; signatures lost; exact reconstruction fails on 21% of real commits with a naive encoding (X13) | R2 | Accident of R2 | An exact encoding (record the message tail and header order) plus side refs for originals. That becomes R2 ∪ R3 in cost. |
| REG-20 | GitHub file history and blame for component paths *in the composition repo* show only adoptions (O4, O5) | R1 (partial), R3 | Inherent to R3/R1 on a server CloneX doesn't control | Equal to baseline (a): the component repo on GitHub has full history. To match (b), publish a derived, regenerable `clonex/history` branch of R2-style reps that GitHub renders (**[GH-unverified]**). |
| REG-21 | Component tags invisible; no tag or release publish; no `describe` (C4) | all | Accident | `clonex tag P v1.2.3` (tags the derived commit on the component remote); `status` shows `git describe` of the derived commit; tags namespaced under `refs/remotes/clonex/<path>/tags/`. |
| REG-22 | Root-only files of components inert in the composition: `.github/workflows`, `.mailmap`, `.git-blame-ignore-revs`, `core.hooksPath`, pre-commit config (C6, P5) | all | Inherent to one repository (monorepo has the same) | CloneX views honour component `.mailmap` and ignore-revs through the lens; hooks via REG-11. Workflows: accept it (component CI runs on the component PR). |
| REG-23 | Stale adoptions (A5), sync placement (A9), occurrence moves (A3), publish guards (A7) unbuilt | all | Accident | As designed in §8. R3 removes A7's rebase and squash hazards entirely. |
| REG-24 | LFS objects not pushed to component LFS stores (C5) | all | Accident | `git lfs push` of objects referenced by derived commits. |
| REG-25 | `#N` autolinks (80 today) and potential closing keywords in component messages act on the composition's issues (X17) | R1/R2 | Accident | R3. For R1/R2 there is no fix, because messages are verbatim by design. |
| REG-26 | Sub-path occurrences unsupported (K17) | all | Accident (designed-for) | Base = full component commit with a subpath replaced (§7). |

### Minor

| Id | Regression | Reps | Fix |
|---|---|---|---|
| REG-27 | Derived component commits carry composition-wide messages ("tweak trunc and dotsync" in trunc) | all | Per-occurrence message sections, or a `Clonex-Scope:` trailer the composition author writes. |
| REG-28 | `committer := author` misattributes the committer of derived commits | all | Accept (needed for L4), or use a fixed "CloneX" committer. |
| REG-29 | jj bookmark not advanced by `sync` (HEAD detached in colocated jj) (X8) | all | Move the bookmark that points at `@-`. |
| REG-30 | Loose-object churn (X14) | all | fast-import batching; REG-10 cache. |
| REG-31 | R2 reps of foreign roots contain only P, so a composition-level bisect can't build (X19) | R2 | `clonex bisect` synthesizes the full composition tree. |
| REG-32 | R3: "commit does not belong to any branch" banner; side-ref growth | R3 | Keeper commit per P (one ref per occurrence). |
| REG-33 | R3: GitHub contributor attribution goes to the adopter | R3 | Add `Co-authored-by:` for component authors in adoption messages. |
| REG-34 | No `--json` schema version; `log --json` returns only a sha | all | Versioned JSON; `log --json` returns commits. |

---

## 3. Which representation wins

**R3 wins, with CloneX read views, and only if REG-16/17/18 are closed.**

- R1's blockers (REG-1, 2, 3, 9, and the R1 half of REG-5) all come from one
  root cause: *foreign-shaped commits inside a mutable branch history*. Every
  familiar rewriting tool (git rebase in every mode, rebase UIs, jj
  rebase/absorb, GitHub's squash and rebase buttons) can damage them. R1 can
  only clear the bar if CloneX owns rewriting, and then CloneX becomes the
  client, which is what AGENTS.md says to avoid.
- R2 fixes the *read* side natively: X2/X3 matched the component's own log
  and blame exactly with plain git, which also covers GitHub and IDEs. It
  also makes rewrites mostly content-safe (X5). But it keeps the flood
  (REG-9), loses signatures and SHAs (REG-19), needs a fragile exact encoding
  (184 of 872 real commits failed naively), and still breaks on squash
  buttons without side refs. It is the best *view*, not the best storage.
- R3 leaves all git/jj write operations safe (X5, X6) and removes the flood,
  the mislinking, the depth-window cost and most purge problems. Its costs
  are read-side (REG-5, REG-20) and object availability (REG-16). CloneX can
  fix the read side cheaply (X15 shows `git blame $(clonex get P)` is exact),
  and fix object availability with lazy fetch-by-sha plus side refs.

Recommended hybrid: **R3 storage, R2 as a computed view.**
- `clonex bisect` and `clonex blame --composition` synthesize rep-like
  composition trees on the fly.
- Optionally, publish a regenerable `clonex/history` branch of reps. It lets
  GitHub render full blame and history, and makes plain clones carry
  component objects **[GH-unverified]**.

---

## 4. Where the product boundary must sit

The boundary claimed in `model.md` §6 ("no client needed") **does not hold**
for any representation. The bar requires CloneX to provide, client-side:

**Read views (required under R3; required under R1 too)**, each a thin lens
over ordinary git commands, with component `.mailmap` and ignore-revs honoured:
- `clonex log P [-- path] [--follow]`, `show`, `blame`/`annotate` with both
  attributions (component commit and composition commit), `grep`/pickaxe over
  history, `diff` in component space, and `describe`.
- `clonex bisect` over the interleaved logical log. Each step is a full
  composition tree with P at a component commit.
- A generic pass-through (`clonex git P -- <read-only git command>`), plus jj
  revset functions if jj-lib integration happens.

**Write operations CloneX must own** (none of them is rewriting):
- adopt any revision, including backwards (REG-7);
- conflict hand-off into git merge state or a jj conflict (REG-4);
- publish with hooks, signing and LFS (REG-11, 12, 24);
- component tags and releases (REG-21);
- remotes and forks (REG-15);
- `clonex mv`; stale-adoption detection and sync placement (REG-23).

**Infrastructure:**
- side-ref push and fetch, and lazy object resolution (REG-16, 18);
- a persistent derivation cache (REG-10);
- shallow-safe behaviour (REG-6);
- truthful fresh-clone status (REG-8);
- CloneX operations recorded as one jj operation (REG-13).

**Not CloneX's job under R3:** working copy, staging, commit, amend, rebase,
squash, split, absorb, undo of local edits, and review hosting. Under R1 all
of these would have to be owned too (REG-1/3).

---

## 5. Where CloneX can decisively beat both Git and jj

Neither git nor jj can do these, in any configuration:
1. One logical change across N independent repositories, published as
   ordinary per-repo commits (L1, L2, L3), with convergent, idempotent,
   leased multi-remote publication (L5).
2. A component bump reviewed as a content diff, instead of the SHA pair of a
   submodule bump (B4, Q4, E2). Plain clones, archives and Pages contain
   everything (A1, A17).
3. A 3-way *content* merge of component updates that survives upstream
   rewrites and squash-then-revert (E13, E14). Submodules can only offer an
   opaque gitlink conflict.
4. Truthful one-command cross-repo status and "where does change X appear"
   (B2, L13). This holds once REG-8 is fixed.
5. **Cross-repo bisect and blame**: find which upstream commit, among 50
   adopted, broke the composition's tests. Polyrepo users do this by hand at
   two levels; jj can't see submodules at all. This is only possible with
   the §4 views.
6. Deterministic ephemeral compositions for CI and agents (L16). This holds
   once REG-6 is fixed.
7. Continuous directory extraction (`declare --extract`, L11), where git has
   only one-shot filter-repo.

Until REG-1 to REG-10 are fixed, CloneX does **not** clear the owner's bar.
R3 clears the write-path blockers by construction. The remaining blockers
(REG-4, 5, 6, 7, 8, 10) are client-side engineering, not model changes.
