# Adversarial review of refs.md (refs, transport, R3 anchors)

Target: `design/refs.md` (candidate), read against `model.md` §8 (the lens as
implemented), `research/refs-workflows.md` (E1–E58, R1–R36, T1–T14) and
`research/anchor-attack.md`. Bar: CloneX must strictly beat plain Git **and**
jj on every aspect of daily use.

Method. Every episode was replayed against the rules as refs.md states them.
Where Git behaviour decides the outcome I ran it with git 2.55
(`/tmp/claude-1000/-home-maxeonyx-agent-tools-workspace/d1bb7304-0863-4655-bda0-4bf326c5a042/scratchpad/refs-attack/`:
`run.sh`, `s2_backport.sh`, output in `results.txt`; local bare repos only,
nothing pushed anywhere real). Checks `S1`–`S6` refer to those scripts.
**(observed)** = seen in a run; **(reasoned)** = derived from the stated rules
of refs.md/model.md §3 rule 2, not run (there is no R3 lens in `src/`: the
current code is R1, where a claim counts only if its sha is a parent);
**[likely]** = belief about GitHub that was not verified.

Notation: U = composition (umbrella), T = trunc, D = dotsync, V = another
composition. `U:N` = branch N on U's remote. `t0─t1` = component commits.
`[A: P x]` = `Clonex-Adopt: P x`. `get_P` as in model.md.

## Verdict

The three-layer split (history / refs / shared config) is right, and the
owner's diagnosis of `follow = "main"` is mostly right (§5 refines it). But
refs.md's central simplification, that **ref relations are computed from
names and never stored**, fails on real data from this ecosystem. Three
things in refs.md fail outright, with minimal cases:

1. **Identity sync adopts foreign branches that happen to share the name**
   (F1). These already exist on the remotes (lingering merged and
   abandoned branches, `dependabot/*`, `gh-pages`). Two agents in different
   compositions pick the same name. refs.md's own push-refusal message
   ("sync first") then tells the user to do the harmful thing.
2. **Repeated occurrences of one component collide** (F2). The desired state
   for `T:N` is two different commits.
3. **R3's "validated by object existence + tree equality" is weaker than R1's
   "is a parent"** (F4, F5). Plain transports drop the anchors (S4), and a
   copied claim is then accepted where it shouldn't be (S2). Both turn into
   *wrong* publishable component commits, not merely coarser history. So
   refs.md's case for R3 ("its failure modes under ordinary tools are all
   'coarser' rather than 'wrong'") is false as written. It becomes true only
   with two repairs (§4).

Serious but repairable: the mapping between the composition trunk and the
component trunks is undefined (F3). `refs/meta/clonex` is unreviewed,
unprotected and not scoped to a branch (F6). Stack retargeting "before
delete-on-merge" has no process that could run it (F7). Leases are local
(F8).

The best rival (§3, **M-A "topic bindings"**) is identity naming by
default, with each induced branch **recorded as a binding** in the
composition's ref layer. It beats refs.md on F1, F2, F8, E22, E25, E50
and dotsync's `MC-` prefix, and loses nothing refs.md has, except that it is
one more mutable store. It does not violate the owner's rule: nothing
enters history.

---

## 1. Findings (severity-ranked)

### F1. Same-name adoption imports someone else's work — **fails** (R17, R14, E11, E12, E24, E56)

Rule attacked: "Syncing composition branch N adopts only the same-named
component branch, if it exists."

Minimal state:
```
D remote:  main: d0─d1         rework-help-sync: d0─x1   (abandoned, other author; observed on dotsync today)
U local:   topic rework-help-sync (new), own work in tools/dotsync on d1
cx sync rework-help-sync  →  fetch D:rework-help-sync, adopt x1
                          →  tools/dotsync = merge3(d0, ours, x1): x1's abandoned work is now in my topic
cx push                   →  D:rework-help-sync fast-forwards from x1 to my commits, carrying x1 into my PR
```
No step warns. The same happens when:
- **Another composition got the name first (W5 in sync-first order).** Agent
  1 (in U) pushed `D:x`. Agent 2 (in V) runs the routine `cx sync x` before
  its first push and adopts agent 1's work. W5's "refuses before any damage"
  holds only if push comes first. Worse, when push comes first the refusal
  says "someone else's commits are on D x: sync first", and following it
  produces this contamination.
- **Host- or bot-generated names collide.** `dependabot/cargo/serde-…` exists
  in U and in every Rust tool. U's `gh-pages` (an orphan site branch) would
  adopt T's `gh-pages` into `tools/trunc` of the site branch. A U
  `release-1.x` (umbrella versioning) adopts T's `release-1.x` (trunc's
  versioning), which is a different line with the same name ("Long-lived
  composition lines adopt from same-named component lines").
- **A merged name is reused (E12).** This is harmless only when the old
  branch was actually merged: its tip is then an ancestor, and merge3 is a
  no-op. The workspace shows lingering *unmerged* branches (§0 of
  refs-workflows), and that case is F1.

Why it's structural: identity mapping cannot distinguish "the branch this
topic induced" from "a branch with this name". Only a stored fact can: a
lease or binding, or ancestry connecting the branch to commits this topic
published. Git and jj never adopt a branch you didn't name. **CloneX is
worse than both here.**

Repair: adopt `C:N` only if it is **bound** to the topic. It is bound
when this topic published into it (the lease, shared as in F8) or when the
user bound it explicitly (`cx topic bind`; this is also how E14/E25/R10
start). An unbound same-named branch is reported, never adopted. The refusal
message must say "bind or rename", not "sync first". Exclude host-owned
namespaces (`dependabot/`, `gh-pages`, `gh-readonly-queue/`) from
induction entirely.

### F2. Two occurrences of one component induce the same branch — **fails** (E35, C3/C12, E36)

```
U:  tools/trunc    trunk = T main          (at t9)
    compat/trunc   trunk override = T release-0.4   (at r3)   — both have transport
topic fix-x edits both (fix on main + backport in one change: natural in a composition)
desired state:  T:fix-x = get_tools/trunc(fix-x)   (parent t9)
                T:fix-x = get_compat/trunc(fix-x)  (parent r3)       ← contradictory
cx sync fix-x:  both occurrences adopt T:fix-x → compat/trunc receives main-line history (crosses a major version)
```
Content fusion (L8) saves only the case where both bases and both edits are
equal. Nesting makes it worse. If M contains U at `at` and also T at
`tools/trunc`, then M topic N induces `T:N` both directly and through
`U:N` (anchor-attack S01 at the ref layer).

Repair: the mapping key is `(occurrence, topic)` → branch name. The default
is `N`. When a second occurrence resolves to the same remote, it gets a
derived distinct name (`N--compat-trunc`), recorded as a binding (M-A).
Sync never adopts one occurrence's branch into another occurrence.

### F3. Trunk mapping is undefined; identity makes `cx push main` push to component trunks — **fails as specified** (R12, E19, E26, anchor-attack A11, questions Q2)

refs.md defines two relations, and they disagree for the trunk:
- "trunk integrates trunk" (sync of U `main` adopts `T:HEAD`);
- "composition branch N induces branch N wherever `get_P(N)` is not
  contained in the component trunk".

For N = U's trunk:
```
U main direct-push (allowed in this ecosystem) edits tools/dotsync   → get_D(U main) ⊄ D main
desired state: D:main = get_D(U main)    → cx push main fast-forwards D main past D's PR, CI and ledger bot
help-test trunk is `master`: U main induces a NEW branch help-test:main next to master
```
E19 produces the same state without anyone doing anything odd. The U topic
PR merges while the D PR is still open, and the D PR is later closed. U main
now carries unlanded D work, and the next `cx push main` publishes it
straight to D main. A U topic that happens to be named `master` targets
help-test's trunk directly.

Repair: the trunk maps to the trunk and induces **nothing**. U-trunk delta
in an occurrence is reported as "unlanded work on trunk" and published only
to a named topic. A composition branch whose name equals any component's
trunk name is refused. Adopt questions.md Q2's "refuse by default" into
refs.md.

### F4. R3 severs Git's connectivity guarantee — **serious** (R3, R12, E29, E44, E47)

Under R1, a commit's existence implies its adopted component commits exist:
that is Git's connectivity invariant, and every transport respects it. Under
R3 the adopted objects are reachable only through
`refs/clonex/adopted/<P>/<sha>`, which **no ordinary transport carries**:

| Transport | Carries anchors? |
|---|---|
| `git clone` / `git fetch` | no **(observed, S4)** |
| `git push <branch>`, `jj git push` (refs.md §4: "jj/git keep the whole write side") | no **(observed, S4: clone of a plainly-pushed U lacks the claimed T commit)** |
| GitHub fork → PR (`refs/pull/N/head` in U) | no: fork's custom refs aren't copied [likely] |
| `actions/checkout` (shallow, one ref) | no |
| `git clone --mirror` | yes **(observed)** |
| host mirroring (GitLab, Codeberg) | unknown per host [likely no for some] |

Once the objects are missing, "validated by object existence" fails
**silently**. The claim is ignored, and `get_P` derives a *jump commit*
holding every upstream change since the previous claim as "own work". A
clone in that state that publishes pushes a copy of upstream history as a
new T PR (anchor-attack S12 shape). Recovery from the component remote
works only under protocol v2 and before the server GCs an unreachable
object **(observed, S5: v0 fails; v2 serves it pre-gc; gone post-gc)**.
Whether GitHub serves unreachable objects by SHA is unverified. Two further
consequences:
- L3 (determinism) now depends on repository state, not just on commit
  objects. Two clones of the same branch derive different component commits.
- Fork PR review (W9/E29) is worse than R1, not better. Maintainers get
  U's commits but not the T commits they claim, unless they discover the
  contributor's T fork.

Repair (required if R3 is kept):
1. A claimed object that is missing is a **hard error** naming the sha and
   the remotes it was looked for on, never a silent downgrade.
2. `cx` installs a `pre-push` hook (Git) and documents that `jj git push`
   of a composition branch must be followed by `cx push --anchors`.
3. `cx push` always ensures anchors for every claim in the pushed range.

With these, the failures really are "incomplete, detected" rather than
"wrong".

### F5. R3 claims are copyable, so a cherry-picked adoption becomes a false merge — **serious** (E30, R27, anchor-attack S11)

R1 requires the claimed sha to be a parent. R3 accepts any existing object,
and `git cherry-pick`/`jj duplicate` copy the message.
```
T:  t0 ─ t2 (main: a=2)        t0 ─ t1 (release-1.x: b=R)
U rel line:  [A: tools/trunc t1]
cherry-pick U main's adoption "[A: tools/trunc t2]" onto rel     (a backport of "take main's feature")
result:  tools/trunc a=2 b=R, message still claims t2            (observed, S2)
lens:    maximal bases {t1, t2} (divergent), tree equals neither
         → derived T commit = MERGE a(t1, t2) with tree a=2,b=R   (reasoned, model §3 rule 2)
cx push rel → T release-1.x now "contains" all of main; future merges of main into it are no-ops
```
Squash-merge of a U PR keeps every intermediate claim, because CloneX parses
claims anywhere in the message (`src/manifest.rs`; local `merge --squash`
shows the claim mid-body, **observed S1**, while `git interpret-trailers`
sees none). Suppose the topic adopted bot commit `b1` and later, after a
red-test edit, adopted `b1'`. The squash claims both. If the anchor for `b1`
exists, the maxima are `{b1, b1'}`, and the derived merge **resurrects the
superseded bot commit**. That is anchor-attack A5's laundering, now produced
by a GitHub button **(reasoned)**.

Repair: make the claim a transition, `Clonex-Adopt: <P> <from>..<to>`. It
is valid only if the parent's base for P is `<from>`, or a chain of claims
in one message composes `from→…→to` from the parent's base. A
cherry-picked or rebased claim then fails validation and the commit is
treated as a plain patch. That yields the correct backport commit on t1. A
squash collapses into the chain's endpoints, which drops stale intermediate
claims. This keeps R3's rebase/squash advantage and restores R1's
precision.

### F6. `refs/meta/clonex` is unreviewed, unprotected and not branch-scoped — **serious** (R7, E31, W7, W9)

- **Trust.** GitHub branch protection and rulesets target branches and
  tags [likely], so `refs/meta/clonex` can be rewritten, force-pushed or
  deleted by any collaborator *and by any workflow token with
  `contents: write`*. That includes the ledger bot and the integration
  workflow. W7's "a new repo at the old URL can't hijack anything" protects
  only *past* claims. After a URL change:
  ```
  refs/meta/clonex: tools/trunc → git@github.com:evil/trunc.git   (one unreviewed commit)
  every clone, next cx sync main: adopts evil/trunc HEAD into U main   (a claim by sha: "valid")
  every clone, next cx push N:     pushes unpublished T work to evil/trunc
  ```
  Today a submodule URL change is a reviewed diff in a PR, and a git remote
  URL is local to the clone. refs.md makes URL changes **less** reviewable
  than either.
- **Scope in time.** Occurrences vary by branch, but the config is global:
  - A topic that *adds* a component needs its transport entry before it can
    publish. That entry takes effect for everyone immediately and can't be
    reviewed in the topic's PR.
  - A topic that *moves* an occurrence (A3) needs both paths mapped.
  - A path reused across time (`tools/x` was repo A on `release-1.x` and is
    repo B on main) maps `release-1.x`'s occurrence to B's URL. Publishing
    then **pushes repo A's derived commits into repo B**.
- **Concurrency and forks.** Two clones editing the config give a
  non-fast-forward; cx must fetch, merge and retry (Gerrit does the same:
  engineering). A fork doesn't copy `refs/meta/*`, and W9's "clones inherit
  from the parent" needs a host API to find the parent. When the fork pushes
  its own config, precedence is unspecified.

Repair:
1. Key the config by **component identity**, not path. Identity = the
   component's root commit(s) plus the first adopted sha. Occurrence →
   component is resolved *from history*: the component whose DAG contains
   P's claimed shas. That fixes path reuse and moves, and needs no names in
   history.
2. **Continuity check on sync.** A new trunk tip must descend from the last
   adopted sha, or the user acknowledges a rewrite explicitly (E39's ledger
   repair is the legitimate case). A hijacked or unrelated URL then fails
   loudly.
3. Owner question O1 (§5) on whether branch-scoped, reviewable transport
   hints belong in the tree.

### F7. "Stacks are derived" can't retarget before delete-on-merge — **degraded; W4 fails as claimed** (R19, E20, E21)

- **Timing.** W4 says B's base "is recomputed to T main *before* GitHub's
  delete-on-merge closes it". The deletion happens inside the integration
  workflow (`--delete-branch`), at merge time. No CloneX process runs then.
  The hosting layer can't be proactive without a server or bot hook (M-C)
  or an ecosystem change: use the repo's auto-delete setting, which GitHub
  retargets for [likely per E21], and drop `--delete-branch`.
- **Shape.** A pure function of ancestry picks the wrong base in two cases:
  - B forked from the *middle* of A, and A has moved on. A's tip is not an
    ancestor of B, so B's T PR targets main and shows A's early T commits.
  - B merges A and C. A PR has exactly one base.

  git-town and Graphite record the parent for these reasons. A recorded
  parent is a ref fact (allowed), with the derived value as the default.

### F8. Leases are per clone, so handoff, rename and cleanup degrade — **degraded** (E7, E22, E41, E50, R21, R22)

"Replace only under a lease that the remote tip is what *this repository*
last published" (model §8: `refs/clonex/published/…`, local).
- **E50 handoff.** Session B clones fresh and has no lease. It can't replace
  `T:N` after amending A's work, and can't tell A's branch from a foreign
  one (so F1 bites). Git needs `--force-with-lease=<sha>` by hand here. jj
  needs `jj git fetch` and then works. CloneX is worse than jj.
- **E22 rename.** U topic N→M: M's branches are created and N's should go.
  But "delete a branch only when its tip is ours and landed" keeps unlanded
  N branches and their PRs alive. GitHub's branch-rename API is the right
  operation (it keeps PRs), and it is a hosting-layer verb.
- **Abandon (E7/E41).** Deleting the topic empties the desired set, but
  unlanded induced branches are "not landed", so they are never deleted.
  Abandonment needs a verb, plus a record so another session can see the
  topic is abandoned rather than in progress.
- **Squash-landed topic.** The desired state tests "contained in trunk" (by
  ancestry), while deletion tests "landed" (patch-id or change-id). After
  a multi-commit squash, `get_T(N)` is not contained, so `cx push N` (say,
  to push a U-only fix) **recreates the deleted `T:N`**, and a PR layer
  would reopen a zombie PR **(reasoned)**. Use one predicate, "landed", in
  both places.
- **Bot writes after merge (E4).** "Tip ours and landed" fails when the bot
  commit arrived after the merge. The rule should be "everything above the
  landed point is landed", and anything else is reported as a lost write.

Repair: push leases into the composition remote
(`refs/clonex/published/<topic>/<occurrence>`), which makes them shared
facts. Better, fold them into M-A's binding record.

### F9. Trunk discovery via remote HEAD — **degraded** (R6, E26, E27)

- A repository created with `init --bare` (default `master`) that has only
  `main` pushed advertises **no HEAD at all** **(observed, S6)**. That is a
  misconfigured repo, not a "host without HEAD", and it needs a hard error,
  not the config fallback silently applied.
- A clone's `refs/remotes/origin/HEAD` stays at `origin/master` after the
  server renames to `main` and the clone fetches with `--prune`
  **(observed, S6)**. cx must use `ls-remote --symref` each time, never the
  cached symref.
- The GitHub default branch is a casual setting. People flip it to make a
  PR target the default, or to point it at a docs or `develop` branch. A
  flip silently retargets every composition's trunk integration: U main
  adopts an unmerged line **(reasoned)**. The F6 continuity check catches
  it (the new tip doesn't descend from the last adoption), so trunk changes
  should require acknowledgment.

### F10. Nested compositions: whose config, whose refs, who syncs — **unspecified / degraded** (E36, K3, anchor-attack A2)

M ⊃ U (at `at`) ⊃ T (at `at/tools/trunc`). Take M topic N editing
`at/tools/trunc`:
- **Transport for T.** It must come from U's `refs/meta/clonex`, fetched
  from U's remote: which version, and what do M's overrides do? refs.md
  says nothing.
- **Push.** It must produce `U:N` (a derived U commit with the translated
  claim `[A: tools/trunc t]`, anchor-attack A2) *and* `T:N`. The derived
  U commit's claim is in U history. That is allowed (a fact), but under R3
  M must also create anchors **in U's remote namespace**.
- **Bots.** A ledger commit on `T:N` reaches M only after someone syncs `U:N`
  and pushes it, or if M's sync recurses into nested occurrences. Without
  recursion, K14 in M waits forever.
- **Trunks.** Does M main integrate T main directly, or only via U main? If
  directly, M's `at` occurrence gets a delta (the adoption) that U main
  lacks. F3's repair then makes it "unlanded work on trunk", which is
  correct but must be specified.

### F11. Smaller items

- **Anchor names from paths.** Paths with space, `:`, `~`, `..`, `.lock` or
  `@{` are invalid refnames **(observed, S3)**, so anchors need an
  encoding.
- **Anchor growth and GC.** About 2 anchors per tool per K14 change, never
  pruned. Protocol v2 prefix filtering keeps fetches cheap. Anchors retain
  force-pushed-away component content (secrets) forever, as R1's parents
  would. A `cx gc` could keep only the maximal claimed sha per occurrence
  reachable from `refs/heads`+`refs/tags` (engineering). One ref holding an
  append-only "anchor chain" commit is an alternative: O(1) refs and one
  mirrorable refspec, at the cost of non-fast-forward races.
- **Every topic creates `U:N`**, even for a T-only fix (E1). That is a U
  branch and PR with no U content, which is churn git and jj users don't
  have. Publish `U:N` only when N has U-root changes, or on request.
- **Membership (R9)** "needs no store" only while the name is intact and the
  commits were made with jj. Plain-Git-authored commits have no
  `Clonex-Change`, so a renamed member drops out of the query.
- **Manual component update.** A plain-Git user copies a newer trunc
  checkout over `tools/trunc` and commits without a claim. The result is a
  jump copy as own work, and if published it is a T PR duplicating upstream
  history. Under R1 the implicit-adoption repair (A7a) needs a parent,
  which R3 has none of, and a tree→commit index makes `get_P` depend on
  which objects were fetched (L3 again). Better: a publish guard that
  refuses a derived commit whose tree equals a known component commit
  outside its ancestry (A7b).
- **Composition trunk requires direct push.** "The composition trunk
  integrates component trunks" assumes U main accepts direct pushes (true
  here). A composition with PR-protected main gets one sync PR per
  component release, so pointer-bump PRs return (E41/E54).
- **Ephemeral V (W8).** "Nothing left to converge" ignores V-root work (an
  integration script written in V), which has no remote to converge to.
  Discard safety must report it.

---

## 2. Replay: every episode

✓ handled · ~ handled with degradation · ✗ fails as specified. "(F*n*)" points
to §1.

### refs-workflows.md §1

| # | Result | Minimal state / why |
|---|---|---|
| E1 solo fix | ~ | Works. Also creates `U:fix-x` with no U content (F11). |
| E2 no name | ~ | jj anonymous heads. A name is required at `cx push`. No generator (= jj). |
| E3 bot on my branch | ✓ | Lease refuses, sync adopts. Needs A9 placement (not built). |
| E4 delete while bot writes | ✗ | Not addressed. Deletion rule is "tip ours ∧ landed", so a post-merge bot commit is neither deleted nor reported (F8). |
| E5 stale trunk | ~ | refs.md lists R33 as covered, but nothing specifies last-fetch age. |
| E6 force-push after rebase | ✓ | Own-work rule plus sync of the rewritten `T:N`. |
| E7 abandoned work | ✗ | No provenance or abandoned state (F8). |
| E8 anonymous heads | ✓ | jj. |
| E9 undo | ~ | jj op log for U refs. Component remote refs aren't undoable (= git). |
| E10 tags | ✓ | Per component, unchanged. |
| E11 same name, same tool | ✗ | Sync-first order: `cx sync x` adopts the other agent's `T:x` (F1). |
| E12 name reuse | ~/✗ | ✓ if the old branch was merged or deleted. ✗ if an unmerged lingering branch exists (F1). |
| E13 3 tools + umbrella | ~ | W1. Nothing stops the U PR merging first (F3/E19). |
| E14 start in component | ✓ | U topic `exit-marker` adopts `tb:exit-marker`. Needs the same name (F1 repair: bind). |
| E15 composition → components | ✓ | By construction. |
| E16 repoint after merge | ✓ | Sync U main, rebase topic. Or merge U PR with the pre-merge pin (contained). |
| E17 both moved pointer | ~ | File-level 3-way merge, better than gitlinks. Plain-Git "take ours" still gives an ours-merge in D (anchor-attack S13; guard not built). |
| E18 topic identity query | ~ | Name plus change-id. A rename, or plain-Git commits, drop members (F11). |
| E19 partial landing | ✗ | U merges with D unlanded, and `cx push main` pushes it to D main (F3). |
| E20 cross-repo stack | ~ | Derived in simple shapes. Wrong for a mid-A fork or two parents (F7). |
| E21 same-repo stack | ✗ | No process at merge time (F7). |
| E22 rename logical branch | ✗ | Old unlanded branches and PRs linger (F8). |
| E23 concurrent U main pushes | ✓ | R3 adoptions of the same sha rebase to empty and drop. |
| E24 same tool, two compositions | ~/✗ | Push-first ✓, sync-first ✗ (F1). |
| E25 pin a non-trunk branch | ~ | Only via a trunk override in V's config. Topic-level following of `MC-first-principles` is impossible unless the topic is named that. |
| E26 different trunk names | ~ | HEAD works. A dangling HEAD gives nothing (F9). |
| E27 master→main | ✓/~ | ✓ only if cx uses `ls-remote`, not cached `origin/HEAD` (F9, observed). |
| E28 ephemeral, no remote | ~ | W8. V-root work isn't counted (F11). |
| E29 fork of composition | ~/✗ | Config precedence unspecified. Anchors don't travel with fork PRs (F4, F6). |
| E30 release branches | ✗ | Identity maps unrelated `release-1.x` lines (F1). A cherry-picked adoption becomes a false merge (F5). |
| E31 moved or renamed component | ~ | History ✓. Future syncs are hijackable through unprotected config (F6). |
| E32 path ≠ repo name | ✓ | Names come from the topic, not the path. |
| E33 appears partway | ✓ | Total lens (§8). |
| E34 archived component | ~ | Push is rejected by the host and reported. "Remove transport" = fixture ✓. |
| E35 same component twice | ✗ | F2. |
| E36 nested | ~/✗ | F10. |
| E37 umbrella release | ✓ | `get_P(v1)`, if anchors are present (F4). |
| E38 tag namespace | ~ | Unspecified where component tags land in U (`refs/remotes/clonex/<P>/tags/*`?). |
| E39 rewrite with backup | ✓ | Empty delta gives a replace, and anchors keep the old SHAs. Conflicts with the F6 continuity repair unless acknowledged. |
| E40 moved tag | ~ | Not addressed. |
| E41 superseded pointer PR | ✓ | No pointer PRs, unless U main is PR-protected (F11). |
| E42 protected refs | ~ | Not addressed (jj `immutable_heads` only locally). |
| E43 plain-Git on component | ✓ | Nothing CloneX-specific. Pinning a fork PR needs the R36 hosting layer. |
| E44 plain-Git on composition | ~ | Content ✓. A manual copy over an occurrence gives a jump copy (F11). A plain `git push` after `cx sync` strands claims (F4). |
| E45 metadata refs on mirrors | ~ | `--mirror` copies (observed). Host mirrors are unknown. Reconstructing needs the component remotes (F4). |
| E46 PR heads | ~ | Hosting layer. |
| E47 CI detached | ~ | Content ✓. Lens and config ✗ without explicit fetches. Must error, not guess. |
| E48 shallow | ~ | Containment checks are wrong without ancestry, so `cx push` must refuse in shallow clones. |
| E49 36 clones | ✗ | Not addressed. |
| E50 handoff | ~/✗ | No shared lease (F8), and F1 can't be told apart. |
| E51 retry | ✓ | Convergence is idempotent. |
| E52 name generation | ✗ | Not addressed. Identity forbids per-repo conventions. |
| E53 cleanup sweep | ~ | "Nothing to converge" plus V-root work plus unanchored claims. |
| E54 bot pointer branches | ✓ | Gone (see F11 caveat). |
| E55 review in another composition | ~ | V topic N adopts `T:N`. "What's missing" needs a name query over D (no store). |
| E56 component-local refs | ✗ | `backup/*` isn't imported unless U has the same name, but `dependabot/*`/`gh-pages` collide (F1). |
| E57 squash in component | ~/✗ | Single squash ✓ (patch-id). Multi-commit squash gives a zombie `T:N` on the next push (F8). |
| E58 direct push to T main | ✓ | Next trunk sync. |

### refs.md walkthroughs

| # | Result | Why |
|---|---|---|
| W1 | ~ | Step 4 needs A9 placement and A5 staleness (neither built). Step 5 inherits E4. Step 6 ordering isn't enforced (F3). |
| W2 | ✓ | |
| W3 | ✓ | |
| W4 | ✗ as claimed | Retarget before deletion needs a merge-time process (F7). |
| W5 | ~ | Holds push-first only (F1). |
| W6 | ✓/~ | Only with `ls-remote`. A dangling HEAD fails (F9). |
| W7 | ~ | Past claims ✓. Future syncs and pushes follow an unreviewed URL (F6). |
| W8 | ~ | V-root work, and V's trunk is undefined without a remote. |
| W9 | ~ | Parent discovery is host-specific, precedence unspecified, anchors missing in fork PRs (F4, F6). |
| W10 | ✓ | Needs anchors for `get_P(v1)`. Plain checkout gives content anyway. |

Tally over 68 rows: ✓ 20, ~ 30 (two of them ✓/~), ✗ or partly ✗ 18. None of the ✗ rows is ✗
for plain Git and jj used today with same-named branches *by hand*, except
E4, E21, E49 and E52, which fail everywhere. **The rows where CloneX is
worse than today's manual practice are F1 (E11, E12, E24, E56), F2 (E35),
F3 (E19) and F5 (E30).**

---

## 3. Rival ref models

All four keep refs.md's history layer: trailers stating facts, and no names.
They differ in where the relation "this component branch belongs to this
topic" lives.

### M-A. Topic bindings in the composition's ref layer (explicit, recorded)

`refs/clonex/topics/<N>` on U's remote is a small commit history of
`topic.toml`:
```toml
[member."tools/trunc"]   component = "<root-sha>"  branch = "config-concern"   published = "<sha>"
[member."tools/dotsync"] component = "<root-sha>"  branch = "MC-config-concern" published = "<sha>"
[member."compat/trunc"]  component = "<root-sha>"  branch = "config-concern--compat" published = "<sha>"
base = "trunk"            # or another topic; recorded only when derivation is ambiguous (F7)
state = "open"            # open | landed(<sha>) | abandoned
```
- **Default naming.** Identity, with per-component templates (`MC-{N}` for
  dotsync) and automatic disambiguation for repeated occurrences. Names are
  bound at first `cx push` (R17 at the moment a name first matters) or
  explicitly (`cx topic bind` for E14/E25/R10).
- **Sync adopts only bound branches.** `published` is the shared lease, so
  it covers E50.
- **Record history.** Each ledger update is a commit, so the ref's own
  history gives provenance (R21) and an abandoned/landed state (E7, E41,
  T6).
- **Loss.** A mirror that drops the ref degrades the topic to refs.md's
  identity rules *with a warning*, because bindings can be re-derived from
  published leases and names. The loss is visible.

Costs:
- A second mutable store that `jj bookmark rename` doesn't update. cx must
  reconcile: the U bookmark is truth for the U side, the binding for
  components.
- Concurrent writes need fetch-merge (members are independent keys, so
  merges are almost always clean).
- The same trust surface as `refs/meta/clonex`, but a forged binding can
  only redirect names within known components, not URLs.

### M-B. Virtual composition topics (derived from component refs)

The composition stores **no topic history**. The view of topic N is a
deterministic composition (like W8's `cx compose`): U-root from `U:N` if it
exists, and each occurrence from its bound component branch N, else from
the adopted trunk state. Local work is ordinary U commits on top of the
view. `cx push` splits it and publishes, then **recomposes**, so after a
push all work lives in component branches. U trunk alone accumulates real
adoptions, and a trunk is never rebased, so R1 verbatim parents are safe
there and the anchor problem (F4) disappears for topics.
- **Bots (K14).** No adoption commits in topics, so staleness (A5) and sync
  placement (A9) cannot occur. The view simply shows the bot commit under
  the draft, as `jj` does with a moved remote bookmark.
- **Weaknesses.**
  - A U PR needs a regenerated "view commit" branch, so review anchors are
    lost on every push (E6).
  - Unpublished multi-component drafts are only local (like jj without
    push).
  - Offline composition uses the last fetch.
  - F1 is *worse*: the view auto-includes any same-named branch, so it
    needs M-A's binding anyway.
  - F2 is the same as refs.md.

### M-C. Server-assisted (a GitHub App / CloneX service)

A hosted component holds the topic index, bindings, anchors and config with
ACLs. It reacts to merge webhooks: it retargets stacks *before* deletion
(W4/E21), holds or redirects late bot writes (E4), adopts into U main
automatically, and answers R9/R34/E49 across clones. It is the only model
that fixes F7-timing, E4 and E49. It needs infrastructure outside GitHub
(T11) and becomes a privileged place, which model.md rejected for the
object store (C-STORE). Here it would hold only ref-layer state, all of it
reconstructible.

### M-D. Change-id-derived component branch names (jj `push-<id>` style)

Component branch = `cx/<change-id-prefix>` (or `<N>-<id4>`), and U topic
names are human-only. Collisions (F1, F2) are impossible by construction,
and sync adopts `cx/<id>` only. It fails the readability bar (E2, E52: PR
lists become unreadable; T2) and breaks the ecosystem's same-slug
convention and `at-<branch>` join. It is included because it is the only
model that is correct with **no stored state and no names in history**,
which makes it the honest price of refs.md's "never stored" principle.

### Comparison on the hard episodes

| Episode | refs.md | M-A bindings | M-B virtual | M-C server | M-D id names |
|---|---|---|---|---|---|
| F1 lingering or foreign same-name branch | ✗ silent adopt | ✓ unbound → reported | ✗ worse (auto-view) | ✓ | ✓ |
| F2 two occurrences, one component | ✗ | ✓ distinct bound names | ✗ | ✓ | ✗ same change → same id |
| dotsync `MC-` convention | ✗ | ✓ template | ~ | ✓ | ✗ |
| E50 handoff | ~/✗ | ✓ shared lease | ✓ (state is on remotes) | ✓ | ~ |
| E22 rename topic | ✗ | ~ (rebinding; host rename API still needed) | ~ | ✓ | ✓ (rename is U-only) |
| E7/E41 abandon | ✗ | ✓ state | ~ | ✓ | ✗ |
| K14 bots, stale adoption (A5/A9) | ~ (unbuilt repairs) | ~ (same as refs.md) | ✓ no adoptions | ✓ | ~ |
| W4/E21 retarget before delete | ✗ | ~ recorded base, still no merge-time process | ~ | ✓ | ✗ |
| F4 anchors vs plain transport | ✗ unless repaired | same as refs.md | ✓ topics need none (R1 on trunk) | ✓ server holds | same as refs.md |
| F6 URL trust | ✗ | ✗ (orthogonal: needs the F6 repair) | ✗ | ✓ ACL | ✗ |
| E30 release lines | ✗ | ✓ bound, not name-matched | ~ | ✓ | ✓ |
| E19 partial landing | ✗ | needs F3 repair | needs F3 repair | ✓ policy at merge | needs F3 repair |
| Plain-Git/GitHub visibility (T8) | ✓ ordinary branches | ✓ | ✓ | ✓ | ✗ unreadable |
| New infrastructure | none | none | none | a service | none |

(M-D on F2: two occurrences of *one* change share a change id, so M-D
collides too unless the occurrence is folded into the name.)

**Recommendation.** refs.md + **M-A bindings** + the F3/F4/F5/F6 repairs.
M-A strictly dominates refs.md on the table and costs one store with a
graceful fallback. M-B's real contribution, no adoption commits in topics,
is worth prototyping later as a *view mode* over M-A, because it removes
the A5/A9 class of bugs entirely. M-C is the only route to W4/E4/E49 and is
an owner question (O5).

---

## 4. Repairs to fold into refs.md (engineering, no owner input needed)

1. Sync adopts only **bound** component branches. An unbound same-name
   branch is reported. Host-owned namespaces are never induced (F1).
2. Mapping key `(occurrence, topic)`. Repeated occurrences get distinct
   names (F2).
3. Trunk ↔ trunk only. The trunk induces nothing. A name equal to a
   component trunk is refused (F3).
4. R3, if kept: a missing claimed object is a hard error, `cx push`
   guarantees anchors, a pre-push hook, anchor ref names encoded (F4, F11).
5. Transition claims `P from..to`, validated against the parent's base.
   Squash chains collapse (F5).
6. Config keyed by component identity. Occurrence → component resolved
   from claims in history. A continuity check on every sync and on every
   trunk-target change (F6, F9).
7. `ls-remote --symref` each time. A missing HEAD is an error (F9).
8. One "landed" predicate for both desired state and deletion. "Everything
   above landed is landed" for deletion. Shared leases (F8).
9. Recorded stack base when derivation is ambiguous. Change the
   integration workflow away from `--delete-branch` or add a merge-time hook
   (F7).
10. Specify nested transport resolution and recursive sync (F10).

---

## 5. Owner value judgments

These are genuine preferences. Each lists the episodes that force it.

**O1. Is the rule "no names/URLs in history", or "history is never
*consulted* to resolve a mutable name"?** Git itself writes branch names
into every merge message ("Merge branch 'x' into main"), and nobody
considers those commits wrong, because they are *descriptive* (true at the
time) and nothing *acts* on them. `follow = "main"` was harmful because it
was *prescriptive*: read from history to drive today's sync. Evidence that
the stronger reading costs something:
- F6: a topic that adds a component can't carry its transport in its own
  reviewable PR, and a URL change becomes an unreviewed, unprotected ref
  write.
- A branch-scoped manifest is exactly what protects the path-reuse case.

A middle option keeps the owner's rule intact: history carries component
*identity* only (root sha, a fact), and URLs and names live in the ref
layer keyed by that identity (F6 repair). **Choose:** strict (refs.md) /
identity-in-history (recommended) / branch-scoped transport hints in-tree.

**O2. One name per logical change across repos: a rule, or a default?**
(T3) Identity breaks on dotsync's `MC-` convention, repeated occurrences
(F2), unrelated `release-*` lines (E30) and host-generated names (E56). If
it's a rule, the owner accepts that CloneX refuses those cases. If it's a
default, M-A bindings are required.

**O3. Is a product-private, shared, mutable ref store acceptable on the
GitHub remotes** (T1), given that GitHub doesn't protect it and any workflow
token can rewrite it? This covers `refs/meta/clonex`, anchors, and M-A
bindings alike. If not, the options are M-D (unreadable names, no store) or
M-C (a service).

**O4. Which failure class is preferable for adopted history: R1 ("wrong
under plain rewrites": rebase dumps component commits at the root, squash
drops parents) or R3 ("incomplete under plain transport": jj/git push, fork
PRs and CI lack the objects)?** With the F4/F5 repairs R3's failures are
detected errors. R1's rebase failures are silent (anchor-attack S05). This
decides whether `jj git push` of a composition is ever allowed without cx.
Also a candidate: M-B's split, with R1 on the never-rebased trunk and no
adoptions in topics.

**O5. Is anything outside GitHub acceptable** (a GitHub App, a webhook
service)? Without one, W4/E21 retargeting-before-delete and E4 late bot
writes can be fixed only by changing the integration workflow, and E49
(one view across 36 clones) stays unsolved (T11).

**O6. May a composition trunk ever publish into a component trunk?**
(questions.md Q2, restated by F3/E19.) Recommendation: never by default,
only by an explicit per-occurrence grant.

**O7. Must every topic have a composition branch and PR** even when it
touches one tool (E1, F11)? It is cheap but is churn neither git nor jj
users have.

**O8. Deliberate rewrites vs hijack defence** (T9, E39 vs F6). A continuity
check makes every legitimate force-push of a component trunk (the ledger
repair) need an acknowledgment in every composition. Is a signed or
recorded "replaced by" statement from the component owner wanted, so the
acknowledgment can be automatic?
