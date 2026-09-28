# Refs and contributor workflows across composed repositories

Independent requirements research. Written without reading `design/model.md`
or `design/refs*.md`. Scope: named pointers (branches, bookmarks, tags,
remote-tracking refs, HEAD, PR heads, stacks) in a system where a repository
can contain other repositories' real histories, and the contributor
workflows that create, move and destroy those pointers.

Confidence markers: **[fact]** observed directly in this workspace on
2026-09-28; **[sure]** state-of-the-art behaviour I am confident of;
**[likely]** believed correct, verify before relying on it.

---

## 0. Ground truth from this ecosystem

Observed read-only (`gh pr list`, `git ls-remote`, AGENTS.md files, `at-main`):

- **[fact]** Every task gets a full clone `agent-tools-workspace/at-<branch>`;
  branch names are slash-free slugs. 36 such clones exist concurrently now.
  One session owns one clone; other clones are read-only to it.
- **[fact]** The same slug is reused across repos for one logical change:
  `issue-9-one-run-integration` is a PR in trunc#3, tmux-bridge#4,
  tdd-ratchet-rs#7, agent-harness#3, help-test#2; `issue-14-standalone` and
  `issue-13-ratchet-guidance` likewise; `trusted-tdd-ledger` in agent-tools#35
  and trunc#10; `website-baseline` in three tools. The name is the only thing
  tying these PRs together.
- **[fact]** Naming conventions drift per repo: dotsync uses an `MC-` prefix
  (`MC-first-principles`, `MC-6-add-scope-workflow`), others use `issue-N-…`
  or bare slugs; umbrella pointer PRs are named `dotsync-0-8-0`,
  `trunc-trusted-ledger-pointer`.
- **[fact]** Tool PRs merge through a dispatched, serialized integration
  workflow that merges with `--delete-branch`. GitHub then **closes** PRs whose
  base was that branch, so agents must retarget stacked PRs to `main` first.
  A live stack exists: agent-harness#14 `handoffs -> forked-subagents`,
  #13 `forked-subagents -> main`.
- **[fact]** A trusted ledger bot commits `.test-status.json` onto the PR head
  branch (or onto `main`) *after* the author's push. If the branch is deleted
  mid-run the bot's write fails (`Reference does not exist`) and its result is
  lost; if integration is dispatched before the bot's commit lands, the
  required status sits on a non-head SHA and auto-merge waits forever.
- **[fact]** Umbrella `main` accepts direct pushes but refuses force-pushes
  and deletions. Tool `main`s are protected with a required check.
- **[fact]** Merged branches linger on remotes from before the
  delete-on-merge era: dotsync and tdd-ratchet-rs still have
  `issue-13-ratchet-guidance`, `issue-14-standalone`; the umbrella has
  `trunc-docs`, `agent-cli-contract`, `MC-dotsync-*`.
- **[fact]** A history repair left `refs/heads/backup/pre-ledger-repair-main-2026-09-09`
  on agent-harness, although AGENTS.md prescribes a *tag* for such backups.
  AGENTS.md also documents that a rewrite forces re-pointing every later tag,
  that GitHub releases refuse `target_commitish` updates (404) and keep naming
  pre-rewrite SHAs, that commit statuses stay on old SHAs, and that branch
  protection must be relaxed and restored byte-exactly.
- **[fact]** Submodule path names differ from repo names: `tools/tb` is
  `tmux-bridge`, `tools/tdd-ratchet` is `tdd-ratchet-rs`. `oc` is archived but
  still pinned. The umbrella pins tools by gitlink; `git submodule status`
  shows describe-names like `v0.3.20-9-ge3588d7`, i.e. the pin is often not a
  tag and not a branch tip either.
- **[fact]** `at-main` is stale: its `main` is at `33bcf98`, remote `main` is
  at `8ee890f`. Nothing warns an agent reading it.
- **[fact]** Umbrella PR bodies name child PRs and merge commits in prose
  (agent-tools#45 cites dotsync#44 and `2a2c1cc`); agent-tools#45 was closed
  unmerged, superseded by a later release. The umbrella pointer PR for a
  release is a ref that goes stale the moment the next release happens.
- **[fact]** Tags are `vX.Y.Z` per tool; lexical `git tag` order puts
  `v0.4.12` before `v0.4.4`.
- **[fact]** dotsync itself is built on jj and models "a scope head is jj's
  RefTarget, in all three of its states" (absent / normal / conflicted) — the
  owner already thinks of refs as possibly-conflicted values.

---

## 1. Episodes

Format: **Actors · Start · Intent · Refs needed (local / remotes) · Over time ·
What goes wrong.** "Composition" = a repo that contains component histories
(umbrella, views, ephemeral contexts). "Component" = a tool repo.

### A. Single-repo basics (the bar every composition must still clear)

**E1. Solo fix in one tool.** Max · trunc `main` · fix a bug · local work ref,
`origin/trunc:fix-x`, PR head · pushed, bot adds ledger commit, integration
merges and deletes · *Wrong:* local ref still says `fix-x` tracks a remote
that no longer exists; plain git shows `[gone]` only after `fetch --prune`.

**E2. Agent starts work without choosing a name.** Agent · umbrella `main` ·
just start editing · nothing named locally; a name is needed only at push ·
agent pushes, PR opens · *Wrong:* jj `push -c` invents `push-kxrzqmvl`, which
is unreadable in the GitHub PR list and doesn't match the clone directory
`at-<branch>`; git forces a name up front (or detached HEAD loses work to gc).

**E3. Bot pushes onto my branch.** Ledger bot · PR head at A · records ledger
· remote head moves A→B (B child of A) while agent keeps committing C on A ·
agent pushes · *Wrong:* git rejects non-fast-forward, agent "fixes" with
`--force` and deletes the bot's commit; jj marks bookmark conflicted only if
it fetched first. Needed: the tool fetches/merges B automatically or refuses
with a clear "someone else moved this" message, never silently discards B.

**E4. Delete-on-merge while a bot write is in flight.** Integration + ledger
bot · PR merged · branch deleted, then bot tries to push · *Wrong:* bot result
lost (observed). A ref model that retains "recently merged" identity would
let late writers be redirected or rejected meaningfully.

**E5. Stale local trunk.** Agent reads `at-main` · local `main` 20 commits
behind · reasons about "current state" · *Wrong:* silently wrong analysis
(observed). Requirement: staleness of a trunk ref relative to its remote is
visible without a network round trip being assumed fresh.

**E6. Force-push by a human after rebase.** Contributor on a fork · PR head
rewritten · reviewers' local copies diverge · *Wrong:* git users get
"diverged" with no hint which side is newer; jj shows conflicted bookmark
`fix??` and both sides — better. GitHub loses review-comment anchors.

**E7. Abandoned work.** Agent crashes mid-task · local commits, maybe a pushed
branch, maybe an open PR · a later session wants to resume or clean up ·
*Wrong:* nobody knows whether `rework-help-sync` (dotsync, no PR) is abandoned
or in progress (observed: ownerless branches linger). Need queryable
"who/what/when last touched this ref, is there a PR, is it merged".

**E8. Unpushed anonymous heads lost.** Agent in jj or detached git makes
commits, then checks out main · git: work reachable only via reflog, expires;
jj: stays visible as a head until abandoned. Bar: jj's behaviour.

**E9. Undo a ref mistake.** Max deletes the wrong branch / moves main to the
wrong commit locally · wants it back · git reflog per ref (deleted-branch
reflog is deleted with it); jj `op log`/`op restore` restores all refs
atomically. Bar: jj, plus coverage of remote-tracking and component refs.

**E10. Tag a release.** Integration workflow · merge commit M on trunc `main`
· tag `v0.4.13`, GitHub release · tags must reach remote; local clones get
them on fetch · *Wrong:* `git push` doesn't push tags by default; a tag later
moved (re-point after rewrite) is not updated in clones that have it (git
never overwrites existing tags on fetch without `--force`/`+refs/tags`).

**E11. Two agents choose the same name.** Two sessions both create
`fix-help-text` in trunc · second push either fast-forward-fails or, worse,
succeeds as a force · *Wrong:* collision discovered only at push, or clones
`at-fix-help-text` collide on disk before that. Need: early detection of name
collision against the remote *and* against sibling local clones.

**E12. Name reused after merge.** `website-baseline` merged in tmux-bridge,
months later an agent creates `website-baseline` again for new work · stale
remote-tracking ref or old PR confusingly matches · *Wrong:* tools that key
stacks/PRs by branch name attach the new work to the old merged PR.

### B. Cross-repo changes (the core intersection)

**E13. One logical change across three tools + umbrella.** Agent · all at
`main` · e.g. `trusted-tdd-ledger` · refs: same-named branch in trunc,
tdd-ratchet-rs, umbrella; one PR each; umbrella PR names child PRs · children
merge (each deleting its branch), umbrella pointer updated to merged child
commits, umbrella merges · *Wrong:* umbrella PR's gitlinks point to child
*branch* commits that later get squashed/rewritten → dangling pins; child
merge order violated; one child PR closed and forgotten while the umbrella
still references it.

**E14. Start in the component, discover you need the composition.** Agent in a
tmux-bridge clone with branch `exit-marker` · realises umbrella standards must
change too · wants the umbrella to "see" the same logical branch without
re-cloning and re-naming · *Wrong:* today requires a second clone, a second
branch and manual gitlink bumps; name drift (`tb-exit-marker` vs
`trustworthy-exit-marker`, both observed).

**E15. Start in the composition, publish to components.** Agent in umbrella
edits `tools/trunc` and `crates/standards` in one commit · wants: a trunc
branch+PR carrying only the trunc part, an umbrella branch+PR carrying the
rest plus the pointer · refs must be created on two remotes with a coherent
name · *Wrong:* component branch created with an umbrella-internal name, or
the component PR contains umbrella paths.

**E16. Component merged, composition PR must repoint.** Integration merges
trunc#10 as merge commit M and deletes `trusted-tdd-ledger` · umbrella branch
still pins the pre-merge commit P · needs to become M · *Wrong:* forgetting
leaves umbrella `main` pinned to a commit reachable only from a deleted
branch (reachable via `refs/pull/10/head` on GitHub, which is not fetched by
default).

**E17. Both sides moved the same component pointer.** Umbrella `main` bumped
dotsync to v0.8.0; my umbrella branch bumped dotsync to my feature commit ·
merge · rule: pin a commit containing both, merging child branches first if
none exists (AGENTS.md) · *Wrong:* gitlink conflict resolved by picking one
side, silently reverting a release.

**E18. Cross-repo topic identity.** Reviewer wants "show me every PR, in every
repo, of logical change X" · today only the shared slug and prose links ·
Gerrit topics do this and can submit atomically. Need a queryable grouping
that survives one member being renamed or closed.

**E19. Partial landing.** Change spans trunc and dotsync; trunc merges, dotsync
PR fails CI for days · umbrella must decide: pin trunc now, or wait ·
*Wrong:* umbrella branch that references both PRs cannot merge (rule: all
referenced child commits available on remote) and blocks unrelated pointer
bumps (observed urgency: "land the pointer bump before other umbrella work").

**E20. Stacked across repos.** Tool change B depends on library change A in
another repo, umbrella change C depends on both · stack spans repos · after A
merges and its branch is deleted, B's dependency must retarget from A's
branch to A's merge commit · *Wrong:* no tool today tracks inter-repo stacks;
git-town/Graphite track parents only within one repo.

**E21. Same-repo stack with delete-on-merge.** agent-harness#14 on #13 ·
integration merges #13 with `--delete-branch` · GitHub closes #14 (observed
rule) · required: #14 retargeted to `main` beforehand, and its branch updated
to include the merge · *Wrong:* closed PR can't be reopened once its base is
gone; review history orphaned. **[likely]** GitHub auto-retargets only when
deletion comes from the repo's "automatically delete head branches" setting,
not API/CLI `--delete-branch`.

**E22. Composition-wide rename of a logical branch.** Agent realises
`issue-9-one-run-integration` should be `serialized-integration` in all five
repos · must rename branch on five remotes, which in GitHub means new branch
+ PR head can't change → close/reopen PRs · *Wrong:* half-renamed set, lost
PR discussion. GitHub supports renaming a branch in-place via API
(**[sure]**, it retargets PRs whose *base* is that branch, and redirects);
PR *head* branch rename is also supported by GitHub's branch rename and PRs
follow (**[likely]**).

**E23. Concurrent agents on different tools, same composition.** Agent X on
`trunc` branch `a`, agent Y on `dotsync` branch `b`, both need umbrella
pointer bumps · both push to umbrella `main` directly (allowed) · *Wrong:*
second push non-fast-forward; retry must merge, not overwrite X's bump.

**E24. Concurrent agents, same tool, different compositions.** Frontend view
and security view both contain `tdd-ratchet-rs` · agents in each create
branches on tdd-ratchet-rs · names must not collide across views and both
must land on the single component remote.

**E25. A composition pins a component branch, not trunk.** Security view
wants dotsync at `MC-first-principles` (unmerged) to review it · the pin must
follow the branch as it moves, or be deliberately frozen · *Wrong:* git
submodules' `branch = …` in `.gitmodules` is only honoured by
`update --remote`; ordinary clones get the frozen gitlink.

### C. Compositions with odd shapes

**E26. Component trunks named differently.** One component uses `master`,
one `main`, one `trunk`, a vendored upstream uses `develop` · composition
operations "update all to trunk", "branch everything from trunk" must resolve
per component · jj's `trunk()` alias does this per repo (**[sure]**,
checks main/master/trunk on origin/upstream); git has only `origin/HEAD`.

**E27. Trunk rename master→main in a component.** GitHub rename redirects web
and API, retargets PR bases (**[sure]**) · local clones still have `master`
tracking `origin/master`; `origin/HEAD` is only set at clone time and is not
updated by fetch (**[sure]**; `git remote set-head -a` fixes) · every
composition that records "component trunk = master" silently tracks a dead
name or errors · *Wrong:* umbrella tooling pins "latest master", which stops
moving without error.

**E28. Ephemeral composition with no remote.** CI builds a throwaway
composition of trunc+dotsync+agent-harness to run integration tests of a
change set, or an agent assembles one locally · refs exist only locally ·
must still be able to publish *component* branches from it, then discard the
composition without losing anything · *Wrong:* composition-only refs treated
as pushable; or discarding the composition deletes unpublished component work.

**E29. Fork of a composition.** Outside contributor forks the umbrella on
GitHub · their fork's gitlinks point to maxeonyx component URLs they can't
push to · they must create component forks too, and the composition fork
must refer to *their* component branches · *Wrong:* PR to umbrella references
commits on forks that maintainers can't fetch by default (GitHub does make fork
PR commits available under `refs/pull/N/head` of the base repo **[sure]**, but
only for the repo the PR is against).

**E30. Component with release branches.** tdd-ratchet-rs grows `release-1.x`
for backports while `main` is 2.x · umbrella pins 1.x, frontend view pins
main · a fix must go to both, with per-branch tags `v1.1.6`, `v2.0.1` ·
*Wrong:* composition "update to trunk" crosses a major version; cherry-picked
backports lose the link to their original.

**E31. Component renamed / moved org / moved host.** `tmux-bridge` renamed to
`tb`, or trunc moved to another org, or mirrored to Codeberg · GitHub
redirects old URLs for renames/transfers (**[sure]**) until a repo is created
at the old name · compositions must keep resolving historical pins, and refs
(`origin/*`) must follow · *Wrong:* old commits in the composition reference
a URL that later points to an unrelated repo (redirect hijack).

**E32. Component path ≠ repo name.** `tools/tb` is `tmux-bridge` · branch
naming, PR titles and ref display must not depend on which name the user
uses · *Wrong:* agents naming the branch `tb-…` in one repo, `tmux-bridge-…`
in another (observed drift).

**E33. Component appears in a composition partway through history.**
agent-harness added at `7486961` · older umbrella commits don't contain it;
refs of agent-harness predate the umbrella's inclusion · log/blame across the
boundary must be coherent; refs in the component (its tags) must remain
meaningful inside the composition.

**E34. Component archived.** oc archived; its refs frozen read-only on GitHub
· composition still pins it · operations over "all components" must not try
to push branches to it.

**E35. Composition includes the same component twice.** A view pins trunc at
`v0.4.5` (compat test) and trunc at `main` · refs for "trunc" are ambiguous
per instance.

**E36. Nested compositions.** Security view contains umbrella which contains
tools · a branch in the outer view implies branches at two levels.

### D. Tags, releases and history-rewrite fallout

**E37. Umbrella release aggregates tool tags.** Umbrella wants a
`2026.09` tag meaning "this set of tool versions" · tool tags live in tool
repos · *Wrong:* umbrella tag doesn't let you check out tool tags; tool tag
names collide inside the composition (every tool has `v0.1.0`).

**E38. Tag namespace collision inside a composition.** Fetching all
components' tags into one ref namespace: `v0.3.0` exists in five repos ·
git would clobber or reject · need per-component namespacing while a plain
git user of the component still sees plain `v0.3.0`.

**E39. History rewrite with backup.** Ledger repair (AGENTS.md procedure) ·
backup ref pushed, main force-pushed, later tags re-pointed, releases stuck
on old SHAs · every clone and every composition containing agent-harness
must learn main was replaced deliberately, not attacked · *Wrong:* jj/git
clones see divergence; compositions pin pre-rewrite SHAs now reachable only
from the backup ref; backup made as branch not tag (observed drift).

**E40. Moved tag.** Tag re-pointed after rewrite · clones that already fetched
keep the old one (git) · jj **[likely]** shows the tag as conflicted or
updates it; either way a composition pinning "v0.1.5" must detect the change.

**E41. Release superseded before pointer PR merged.** dotsync-0-8-0 umbrella
PR (agent-tools#45) closed because a newer release came · the pointer branch
is dead; nothing else depends on it · cleanup should be automatic.

**E42. Protected refs.** Umbrella `main`: direct push ok, no force, no delete;
tool `main`: required check · the local tool should know these policies to
refuse early rather than after a round trip · jj `immutable_heads()` is the
local analogue (**[sure]**, defaults include `trunk()`, tags, untracked remote
bookmarks).

### E. Plain-Git and hosting interop

**E43. Plain-git contributor on a component.** Outsider clones trunc with git,
opens PR from `patch-1` on a fork · composition users must be able to fetch
that PR head, review it in composition context, and pin it · *Wrong:* the
product needs extra metadata on the contributor side.

**E44. Plain-git contributor on the composition.** Outsider clones umbrella
with git and `--recurse-submodules` · must get working tree and sensible
branches; must not see product-private refs as junk branches in GitHub UI.

**E45. Product metadata refs on GitHub.** If the product stores ref-layer data
in custom refs (`refs/x/…`) · GitHub accepts arbitrary refs via push
(**[sure]**) but shows only heads/tags; some hosts/mirrors drop unknown refs;
`git clone` doesn't fetch them by default · *Wrong:* metadata lost on mirror.

**E46. GitHub PR heads.** `refs/pull/N/head` and `refs/pull/N/merge` are
read-only, server-managed (**[sure]**) · a PR is effectively a ref pair
(head ref + base ref) plus server state · product must treat "PR" as a
first-class remote ref kind, including PRs from forks.

**E47. CI in a detached checkout.** GitHub Actions checks out a detached SHA
(`actions/checkout` default, shallow) · no local branch, no remote-tracking
refs · the product must still function (compute component membership, trunk)
with only HEAD and maybe one fetched ref.

**E48. Shallow and partial clones.** Agents clone quickly with `--depth` ·
refs point at commits whose ancestry is missing · merge-base for "is my
branch merged" fails.

### F. Agents at scale

**E49. 36 concurrent clones.** Each clone has its own view of refs · Max asks
"what's in flight across all my clones and all remotes?" · today needs
walking 36 directories + `gh pr list` on 8 repos · Sapling's commit cloud
(**[sure]**: syncs draft commits/bookmarks across a user's workspaces) is the
reference point.

**E50. Agent handoff.** Session A pushes branch, session B continues in a new
clone · B must discover the branch, its PR, its stack parent, its sibling
branches in other repos, and bot commits since A · *Wrong:* B recreates a
same-named branch from `main` and force-pushes.

**E51. Agent retries a push after a flake.** Push partially succeeded across
repos (trunc updated, dotsync rejected) · retry must be idempotent and report
per-remote state · atomicity: git has `push --atomic` per remote only
(**[sure]**); no cross-remote atomicity exists anywhere except Gerrit
submit-whole-topic on one server.

**E52. Agent name generation.** Agents pick descriptive slugs; conventions
differ (`MC-` prefix) · product might auto-suggest names from task/change
description and enforce per-repo conventions · *Wrong:* jj `push-<changeid>`
names meaningless to humans; Sapling `pr123` names likewise.

**E53. Cleanup sweep.** Max: remove clones whose work is done (workspace rule:
clean tree, pushed, PR state known, no unpinned child work) · needs per-clone
"every local ref is either on a remote or intentionally discarded, every
component commit is pinned somewhere reachable" · *Wrong:* deleting a clone
that held the only copy of an ephemeral composition's component commits.

**E54. Bot-created umbrella pointer branches.** A future bot opens
`dotsync-0-9-0` pointer PRs on each tool release · names deterministic, so a
re-run must update rather than duplicate; previous unmerged pointer PR
superseded (E41) automatically.

**E55. Reviewing a change in a different composition than it was authored.**
Change authored in umbrella; reviewer uses the security view (which lacks
dotsync) · the reviewer must see the same named change restricted to the
components they have, and know parts are missing.

**E56. Component-local refs that must never enter a composition.**
`backup/…`, `gh-pages`, dependabot branches, `refs/notes` · compositions
should not import every component branch as its own branch.

**E57. Squash-merge in a component.** A plain-git maintainer squash-merges a
component PR · the composition's branch that pinned the original commits
must recognise "merged" despite no ancestry · git can't; jj can't; GitHub
knows via PR state. (The ecosystem prefers merge commits, but outsiders and
future repos may not.)

**E58. Direct push to component `main` bypassing PR.** Max hot-fixes trunc
`main` from a phone · every composition's notion of "trunc trunk" moves;
branches in compositions based on old trunk need no action until they merge.

---

## 2. Requirements (externally observable properties)

Refs layer, deduplicated. "Ref" = any named pointer.

**Anonymity and separation**
- **R1** History is ref-free: no commit's content or message must change
  because of a branch name, PR, topic or stack relation. Renaming or deleting
  any ref never changes any commit ID.
- **R2** Work does not need a name until it needs to be shared; unnamed work is
  never lost or garbage-collected silently (bar: jj visible heads).
- **R3** Product-only ref data never appears as junk to plain-git users of
  either component or composition remotes (no stray branches in GitHub UI),
  and survives mirroring or is reconstructible.

**Per-component truth**
- **R4** For every component, its own remote's branches, tags and PR heads are
  visible from any composition containing it, namespaced so identically named
  refs in different components never collide.
- **R5** A component's refs as seen by a plain-git clone of that component are
  exactly its native refs; the composition adds nothing to the component
  remote except ordinary branches the user asked to push.
- **R6** Each component's trunk is resolved per component (any name, including
  after rename), and a trunk rename is detected and followed, not silently
  stranded.
- **R7** Component identity survives repo rename, org transfer and host move;
  historical pins remain resolvable and cannot be hijacked by a new repo at an
  old URL.

**Cross-repo logical changes**
- **R8** One logical change spanning N repos can be named once and shows up as
  consistently named branches/PRs in each touched repo, and only those.
- **R9** Given any member (a branch, a PR, a commit), you can list all members
  of its logical change across repos, with each one's state
  (local-only / pushed / PR open / merged / closed / abandoned).
- **R10** Work can start in a component or a composition and later be
  continued from the other without renaming or re-creating refs.
- **R11** Publishing from a composition produces per-component branches whose
  contents touch only that component, plus a composition branch; a plain-git
  reviewer of a component PR sees an ordinary PR.
- **R12** Dependency order is enforced or at least reported: a composition ref
  never gets published pointing at component commits not available on their
  remotes.
- **R13** After a component merge (any strategy: merge, squash, rebase, direct
  push), the composition's pins and derived refs can be moved to the merged
  result with one operation, and "is this merged?" is answered correctly even
  without ancestry.

**Movement, concurrency, bots**
- **R14** Remote moves by others (bots, other agents, force-pushes) are never
  silently overwritten; a push that would discard remote commits is refused
  unless explicitly intended, per ref, per remote.
- **R15** Divergent local vs remote ref states are represented as a visible,
  resolvable conflict, not as an error that blocks all work (bar: jj
  conflicted bookmarks).
- **R16** Bot commits on a shared branch are integrated into the local line of
  work without ceremony.
- **R17** Name collisions (with remote refs, with other local clones, with
  previously used-and-merged names) are detected at name-choice time.
- **R18** Multi-remote operations report exactly which remotes succeeded, are
  safely retryable (idempotent), and leave no ambiguous half-state.

**Stacks**
- **R19** Stacks (within and across repos) are recorded outside history; when
  a lower member merges or its branch is deleted, upper members retarget to the
  right base automatically, before any hosting side effect closes them.
- **R20** Rewriting a lower member updates every ref above it in one step
  (bar: jj automatic rebase + bookmark follow; git `--update-refs`).

**Lifecycle and cleanup**
- **R21** Every ref has queryable provenance: who created/moved it last, from
  which clone/session, and whether a PR exists; ownerless stale refs are
  listable.
- **R22** Merged, superseded or abandoned branches are cleaned up (or listed
  for cleanup) locally and remotely with their dependants handled; deleting a
  merged branch never loses a pending bot write silently.
- **R23** All ref changes, including component and remote-tracking refs, are
  undoable atomically across the whole composition (bar: jj op log).
- **R24** "Is it safe to delete this clone?" is answerable in one command,
  covering every component and ephemeral composition inside it.

**Tags and releases**
- **R25** Tags are per component, namespaced within compositions, pushed when
  intended, and a moved tag is detected everywhere, not silently kept stale.
- **R26** A composition can carry its own tags/releases that pin a set of
  component versions, and checking one out yields every component at its
  version.
- **R27** Release branches are ordinary refs: a composition can pin a
  component's non-trunk line, and backports keep a discoverable relation to
  their originals (outside history).

**Compositions**
- **R28** Ephemeral compositions with no remote work fully; their refs are
  local-only by nature, and discarding one never discards unpublished
  component work.
- **R29** A fork of a composition can point its components at forks, and PRs
  from forks (component or composition) are fetchable and pinnable.
- **R30** A composition can choose to follow a component branch (floating) or
  pin a commit (frozen), visibly, per component.
- **R31** Views that omit some components show a logical change restricted to
  what they contain and say what is missing.
- **R32** Composition-level protection (immutable trunk(s), no-force policy)
  is known locally per component and enforced before network round trips.

**Status and freshness**
- **R33** Staleness of any local view of a remote ref is visible (last fetched
  time / behind count), and a single command refreshes all components.
- **R34** A single status view shows every in-flight line of work across
  components (and ideally across the user's clones), with its PR state.
- **R35** Works in CI detached, shallow checkouts with only HEAD available,
  degrading explicitly rather than guessing.
- **R36** Hosting-side refs (`refs/pull/*`, PR head/base pairs) are first-class
  read-only refs.

---

## 3. The bar: git vs jj on refs today

### Git — strong
- Universality: every host, CI, IDE, reviewer understands `refs/heads`,
  `refs/tags`, `refs/remotes`. Refspecs are a general mapping language
  (`+refs/pull/*/head:refs/remotes/origin/pr/*`). **[sure]**
- `push --force-with-lease`, `push --atomic` (single remote), `--follow-tags`,
  `fetch --prune`, `branch -vv` `[gone]`. **[sure]**
- Arbitrary custom ref namespaces (`refs/notes`, `refs/meta/config` in Gerrit)
  for out-of-history metadata. **[sure]**
- Rebase `--update-refs` (2.38) moves stacked branch refs. **[sure]**
- Worktrees prevent the same branch being checked out twice. **[sure]**

### Git — weak
- A "current branch" that moves with every commit forces naming up front;
  detached HEAD work is lost to gc after reflog expiry. **[sure]**
- `origin/HEAD` stale after trunc rename; no trunk abstraction. **[sure]**
- Tags: single global namespace, not overwritten on fetch, not pushed by
  default. **[sure]**
- Submodules: gitlinks carry no branch; `.gitmodules branch=` only for
  `--remote`; submodule HEADs detached; no cross-repo branch, PR or stack
  notion; conflicts on gitlinks are opaque SHA pairs. **[sure]**
- Reflog per ref, local, deleted with the branch; no atomic multi-ref undo.
  **[sure]**
- Divergence is an error at push time, not a state. No merged-detection for
  squash merges.

### jj — strong
- Anonymous heads: work needs no name, never silently lost. **[sure]**
- Bookmarks don't advance on commit but do follow rewrites; descendants
  auto-rebase. **[sure]**
- `name@remote` tracked/untracked distinction; conflicted bookmarks as a
  first-class state (`name??`). **[sure]**
- `trunk()` revset alias and `immutable_heads()` protecting trunk, tags and
  untracked remote bookmarks locally. **[sure]**
- Operation log: atomic undo/restore of all refs and working copy. **[sure]**
- Push safety: refuses when remote moved from last-known. **[sure]**
- `jj git push -c` creates a bookmark from a change. **[sure]**

### jj — weak
- Auto names `push-<changeid>` are human-meaningless; prefix configurable
  (`git.push-bookmark-prefix` / template **[likely]**).
- No submodule support: submodules ignored/opaque. **[sure]**
- No stacks-as-PRs natively; external tools needed. **[sure]**
- Tag creation arrived late (`jj tag set` **[likely]** ~2025); tag handling
  thinner than bookmarks.
- Colocated mode drift with git tools touching refs (mostly handled on import
  **[sure]**).
- Must remember to `track` remote bookmarks; new remote bookmarks are
  untracked by default (except on clone for the default branch **[likely]**).

### Others worth stealing from
- **Sapling**: smartlog of *your* drafts; no local branches needed; commit
  cloud syncs drafts across workspaces; `sl pr submit` creates host branches
  (`pr<N>`-style names **[likely]**). Maps to R2, R34.
- **Gerrit**: refs/for/<branch> push-to-review; patch sets
  `refs/changes/NN/N/P`; topics across repos with submit-whole-topic
  (atomic on one server **[sure]**); refs/meta/config config-as-ref.
  Weakness vs owner's model: Change-Id lives in the commit message — a
  history-embedded identifier, i.e. exactly the model violation.
- **ghstack** (`gh/<user>/<n>/{base,head,orig}` branches **[sure]**), **spr**
  (commit-id trailer in message **[sure]** — again history-embedded),
  **Graphite** (parent metadata in git refs/config, restack **[likely]**),
  **git-town** (parent in git config, `sync` reparents children when parent
  is shipped **[sure]**), **git-branchless** (smartlog, `move`, `sync`,
  undo **[sure]**). All single-repo.
- **repo tool**: `repo start <topic>` makes the same-named branch in many
  projects; manifests name per-project `revision`/`upstream`/`dest-branch`;
  manifest branches per release; `repo manifest -r` snapshots SHAs. **[sure]**
  Closest prior art for R8/R26/R30.
- **Chromium DEPS + autoroller**: pinned SHAs in a file, bots open roll CLs
  continuously, superseding previous ones. **[sure]** Maps to E54/E41.
- **Josh**: filtered views of one repo presented as repos, deterministic
  rewritten SHAs, pushes mapped back; refs mapped per filter. **[sure]**
- **Copybara**: `GitOrigin-RevId:` labels in messages to map commits between
  repos **[sure]** — history-embedded again.
- **git subtree**: `git-subtree-dir:` trailers in messages **[sure]**.

Observation: nearly every cross-repo tool that works puts an identifier *into
history* (Change-Id, GitOrigin-RevId, subtree trailers, spr commit-id). The
owner's R1 rules that out, so identity across repos must be derived from
DAG structure or kept in a separate layer that must itself be shared.

---

## 4. Tensions and owner value judgments

- **T1 Anonymous history vs shared cross-repo identity.** If commit messages
  may not carry IDs, the "same logical change" relation (R8, R9, R19) must live
  in a ref layer that travels with pushes — custom refs on GitHub (invisible,
  may be dropped by mirrors, R3) or an external store. **[OWNER]** Is a
  product-private ref namespace on the GitHub remotes acceptable, or must all
  ref-layer data be reconstructible from branch names + PR metadata alone?
- **T2 Naming is required by GitHub vs work shouldn't need names.** Branch
  names are the join key humans and agents use (clone dirs `at-<branch>`,
  cross-repo slugs). Auto-naming (jj/Sapling) is unreadable.
  **[OWNER]** Should the product generate names (from what?), require them at
  first publish, or at task start as today?
- **T3 Same name across repos vs per-repo conventions.** Uniform slugs make
  R8 trivial but collide with dotsync's `MC-` prefix and with pre-existing
  merged names (E12). **[OWNER]** Is one global name per logical change a
  rule, or should per-repo names be allowed with an explicit link?
- **T4 Floating vs frozen pins.** R30 requires both; the ecosystem's
  release-freshness concerns want "latest released", reviewability wants
  frozen. **[OWNER]** Default for a composition component: pin, follow trunk,
  or follow latest tag?
- **T5 Composition-branch = set of component branches, or its own ref?** If
  the composition's branch *is* derived from component branches, deleting a
  component branch on merge (policy) mutates the composition branch; if it's
  independent, it can drift. **[OWNER]**
- **T6 Delete-on-merge vs retained identity.** The ecosystem deletes merged
  branches (and that breaks stacks and late bot writes); retaining them clutters
  (observed lingering refs). The product needs "merged" as a state distinct
  from "deleted". **[OWNER]** Keep a local/private record of merged names, or
  rely on GitHub PR state?
- **T7 Cross-remote atomicity is impossible on GitHub.** R18 can only be
  "idempotent and honest", not atomic. **[OWNER]** Accept eventual
  consistency with ordered landing (tools first, umbrella last, as today)?
- **T8 Strictly beating git on familiarity.** Any new ref kind (topic, stack
  relation, composition pin) is something plain-git contributors can't see.
  Beating jj on anonymity while beating git on "what you see on GitHub is what
  there is" pull in opposite directions.
- **T9 Rewrites.** Owner's process allows rare, approved history replacement.
  R14 (never silently discard remote moves) vs deliberate force-push swaps:
  **[OWNER]** Should a deliberate rewrite carry a product-visible "replaced
  by" record so every clone/composition follows it automatically (E39)?
- **T10 Tag namespacing.** Composition-internal namespacing (R4, R38) vs
  plain-git users of the composition who'd expect `git tag` to list something.
  **[OWNER]** What does `git tag` in a plain-git umbrella clone show?
- **T11 Scope of "status".** R34 across all clones implies a shared
  per-user state (commit-cloud-like) — new infrastructure beyond GitHub.
  **[OWNER]** Is a service/state outside GitHub acceptable at all?
- **T12 Ephemeral compositions and GC.** R28 wants discard-safety, which
  requires knowing whether component commits exist elsewhere — a network
  question for a thing defined as having no remote.
- **T13 Squash-merge detection** (R13) needs host API or patch-id heuristics;
  heuristics can be wrong. **[OWNER]** Is merge-commit-only acceptable as an
  ecosystem rule, making squash support best-effort?
- **T14 Bots as first-class ref writers.** Should the product know that the
  ledger bot writes to PR heads (and wait for it before merge/delete), or is
  that CI's problem? The observed failures (E3, E4, lost results, stuck
  auto-merge) are all ref-timing bugs.
