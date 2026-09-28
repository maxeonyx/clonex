# Refs, transport and contributor workflows — design search

Status: **candidate design, not implemented, under attack.** Written after the
owner's observation that the manifest's `follow = "main"` commits a branch
name into history. Inputs: `research/refs-workflows.md` (a blind pass: 50+
episodes, R1–R36 requirements, tensions T1–T14), the GitHub custom-ref
experiment below, and the earlier model.

## 0. The error the owner spotted, stated precisely

The first model mixed three kinds of fact into the composition's tree:

| Fact | Example | Nature |
|---|---|---|
| what the content is and where it came from | `tools/trunc` equals trunc commit `7d283c6` | immutable history |
| where a history can be exchanged | `git@github.com:maxeonyx/trunc.git` | mutable transport |
| which named line follows which | U `main` integrates trunc `main` | mutable ref relation |

Only the first belongs in history. The other two are about *other
repositories' mutable namespaces*: trunks get renamed (master→main), repos
move between orgs and hosts, forks want different URLs. Committing them makes
past commits wrong the moment the outside world changes. That is the
`.gitmodules` URL-rot problem, and it would recur for branch names. Blind
requirement R1 puts it as an externally observable property: renaming or
deleting any ref, or moving any remote, never changes any commit ID or makes
any commit's content wrong.

## 1. Three layers

1. **History** (immutable, content-addressed, identical in every copy).
   Trees, commits, and two kinds of trailers that state facts about commits,
   never about names:
   - `Clonex-Adopt: <path> <sha>`: "this content at `<path>` incorporates
     component commit `<sha>`".
   - `Clonex-Change: <id>`: the logical change this commit belongs to.

   No manifest, no URLs, no branch names.
2. **Refs** (mutable, per repository). Ordinary Git refs, exactly as each
   repo already has them. CloneX adds nothing to a *component's* remote except
   branches the user publishes (R5).
3. **Configuration and anchors** (mutable, shared, outside history). This lives
   in a composition's own remote, in namespaces no plain-Git tool shows:
   - `refs/meta/clonex`: a small commit history of a config file
     (occurrence → remote URL, optional trunk-name override). It has the same
     shape as Gerrit's `refs/meta/config`.
   - `refs/clonex/adopted/<path>/<sha>`: reachability anchors that keep every
     adopted component commit in the composition's own repository (see §4).

   Verified on GitHub (2026-09-28, on maxeonyx/clonex): arbitrary `refs/meta/*`
   and `refs/clonex/*` refs are accepted and advertised. A plain clone doesn't
   fetch them; an explicit fetch does. Local per-clone overrides are ordinary
   `.git/config` (`url.<base>.insteadOf`).

What happened to the manifest's other jobs:
- *"This path is an occurrence"* is now history: a path is an occurrence of
  a history from its first adoption claim.
- A history *born* in the composition (not yet published anywhere) is just a
  directory until its config entry and first publication.
- *"Frozen"* stops being a flag. A fixture is an occurrence with no transport
  entry, so nothing ever syncs it, and its adoption history is still there.

## 2. How ref namespaces of related repos relate

Principle: **ref relations are computed from names plus the lens. They are
never stored.**

- **Trunk.** Each repo's trunk is its remote's `HEAD` symref: discovered, not
  declared, and followed across renames (R6). A config override exists for
  hosts without a HEAD. The composition's trunk *integrates* every
  transport-configured occurrence's trunk: syncing composition trunk adopts
  component trunks.
- **Topics.** A composition branch `N` *induces* a branch `N` in each
  component where `get_P(N)` is not already contained in that component's
  trunk, and only those (R8, R11). The mapping is identity, which matches
  the ecosystem's same-slug convention (`at-<slug>`, the same branch name in
  every repo).
- **Sync scope.** Syncing composition branch `N` adopts only the same-named
  component branch, if it exists. That is how CI bot commits, reviewer
  suggestions and rewrites of our own PR branch come in (R16). Newer
  component trunk work reaches a topic the ordinary way, by rebasing or
  merging the topic onto the composition trunk, which already adopted it.
  **Integration of component trunks happens on exactly one line (the
  composition trunk).** This single rule replaced the old `follow` field and
  the `--branch` flag, and it makes the K14 bot flow a plain `sync`.
- **Long-lived composition lines** (a release line `release-1.x`) adopt
  from same-named component lines when they exist, and otherwise stay where
  they are. "Pinned" falls out; nothing declares it.
- **Stacks are derived, not recorded** (R19). If topic `B` is based on topic
  `A` in the composition, then in each component where both have work, `B`'s
  base is `A`'s induced branch. Retargeting when `A` lands is a pure function
  of composition ancestry, so no stack metadata can go stale.
- **Logical-change membership** (R9) needs no store. Members share the topic
  name (a ref fact) and the `Clonex-Change` ids (history facts). Either finds
  the others.

## 3. Publication and sync as ref convergence

**Desired state** for a composition branch `N` that the user publishes:
- the composition remote has `N` at the composition commit;
- each component remote has `N` at `get_P(N)` wherever that isn't contained
  in the component trunk;
- no induced `N` exists in a component where it's now contained (landed) or
  where the topic no longer touches it.

`cx push N` compares desired with actual and acts, each remote
independently:
- create, or fast-forward;
- replace, only under a lease that the remote tip is what *this repository*
  last published;
- delete a branch only when its tip is ours and landed;
- report everything else (R14, R18). An example is "someone else's commits
  are on trunc `N`: sync first".

It is idempotent and retryable. Partial state is simply "some remotes
converged, some not", and it is visible (R18, T7). Deleting the composition
branch makes the desired set empty, which cleans up induced branches (R22),
including the "merged but still carrying bot writes" case: the landed check
comes before deletion.

**Sync of `N`**: for each occurrence with transport, fetch the component's
`N` (or its trunk when `N` is the composition trunk), then adopt. Adoption
keeps the own-work reapplication rule from `model.md` §8.

## 4. The representation question (reopened)

The ref-free principle, together with plain-tool behaviour, reopened how
adopted component history is held. There are three candidates:

- **R1 verbatim parents** (current code). Component commits are second
  parents of adoption merges. Simple, and the SHAs are literally present.
  The costs:
  - every tool sees component commits with root-level trees;
  - plain `git rebase` (even `--rebase-merges`) dumps them at the root;
  - a GitHub squash-merge of a composition PR drops the parents;
  - the composition log is noisy, and our own changes appear twice (`u` and
    its derived `x`).
- **R2 shifted representatives.** Composition-shaped copies of each
  component commit, which reconstruct the originals bit-exactly. Plain `git
  log -- tools/trunc/…` and blame work. The costs: a reconstruction layer,
  renamed signature headers, rep lineages whose trees change shape, and noise
  as in R1.
- **R3 squashed adoptions + anchor refs.** The composition's branch history
  holds no component commits. An adoption is an ordinary one-parent commit
  that changes `P` to the merged content and carries `Clonex-Adopt` lines.
  The component objects stay reachable through
  `refs/clonex/adopted/<P>/<sha>` in the composition's own remote.
  - Rebase and squash-merge are safe: the claim is a trailer, validated by
    object existence and tree equality.
  - Our own changes are never duplicated, and jj/git logs are the
    composition's own story.
  - Cost: *plain* Git in a composition sees adoption-granularity history
    (like a submodule bump, but with the content), and a plain clone lacks
    component commit objects.
  - So full-granularity log/annotate/bisect across component history become
    CloneX read views that descend through the lens.

Current lean: **R3**, because it is the only candidate whose failure modes
under ordinary tools are all "coarser" rather than "wrong". The audit
(`research/beat-git-jj-audit.md`, in progress) is comparing all three
against Git, jj, submodules and a monorepo. This choice is not made until
that audit and an attack have come back.

Consequence for the product boundary, if R3 holds: jj/git keep the whole
*write* side and single-repo read side. CloneX owns transport (sync, push)
and the *cross-history read views* (log, annotate, bisect and diff that
descend into component history). For the owner's "beat git and jj
everywhere" bar, those views must be at least as good as `jj log`/`git blame`
in a monorepo.

## 5. Contributor workflows under this design (walkthroughs)

U = umbrella (composition, has a remote); T = trunc; D = dotsync; each
component has a remote with trunk `main`.

**W1: agent cross-cutting task** (the motivating case).
1. `cx clone U` = `git clone` + fetch `refs/meta/clonex` + `refs/clonex/*`.
2. `jj new main`, edit `tools/trunc` and `tools/dotsync`, describe, then
   `jj bookmark create config-concern`.
3. `cx push config-concern` → U gets `config-concern`; T and D each get
   `config-concern` with exactly the derived commits. Siblings are linked by
   topic and change-id (PR creation: hosting layer, §7).
4. The ledger bots commit to T and D `config-concern`. `cx sync` on the
   topic adopts them (same-named branches); green work is added and pushed
   again.
5. Tool PRs merge; their branches are deleted by the integration workflow.
   `cx sync` on U `main` adopts T and D `main`. Rebasing the topic onto U
   `main` leaves no own work in T or D, so the induced branches' desired
   state is "absent"; they are already gone, and nothing needs doing.
6. U's PR merges. Nobody wrote a pointer bump or chose an order by hand.

**W2: outsider PR to trunc** (plain Git, fork). Nothing CloneX-specific
happens. U adopts trunc `main` on the next trunk sync (a person or a bot).

**W3: owner works in dotsync standalone with jj.** Nothing changes for
dotsync. U adopts on the next trunk sync.

**W4: stacked cross-repo topics.** `A` touches T and D, and `B` on `A` touches
T. T gets `A` and `B` (PR base `A`); D gets `A`. When T `A` lands, `B`'s base
is recomputed to T `main` *before* GitHub's delete-on-merge closes it (a real
failure today). This needs the hosting layer, but no stored stack data.

**W5: two agents, two compositions, one component.** Agent 1 in U works on
topic `x`; agent 2 in view V works on topic `y`; both touch D. They produce
distinct branches in D. If both pick `x`, agent 2's push finds a D `x` tip
that isn't its own and refuses (R17), before any damage.

**W6: component trunk renamed** (help-test `master`→`main`). The remote HEAD
changes, and the next trunk sync follows it. Nothing in history is wrong.

**W7: component moves org or host.** The `refs/meta/clonex` URL is updated
(one config commit). History is untouched, and adoption claims are SHAs, so
they are unaffected (R7). A new repo at the old URL can't hijack anything:
claims are validated by SHA and tree.

**W8: ephemeral view V** (dotsync + agent-harness for one agent task).
- `cx compose` gives a local repo whose first commit adopts D `main` and
  H `main` (deterministic, so the same inputs give the same SHA).
- Config lives in V's local `refs/meta/clonex`. There is no remote.
- Work under topic `t` pushes induced `t` branches to D and H only.
- Discarding V is safe exactly when `cx push` has nothing left to converge
  (R24/R28).

**W9: composition fork.** A fork of U on GitHub copies branches and tags,
not `refs/meta/*` or anchors. Its clones inherit config from the parent U
remote and use local overrides to point at component forks (R29).

**W10: release of U.** Tag `v1` on U. `get_P(v1)` *is* the version of every
component in that release. No `version.json` is needed to know it, though
one can be generated from it (R26). Component tags stay per component; `cx
status` describes each occurrence against its component's tags.

## 6. Requirement coverage (blind R-list)

Covered by construction:
- R1, R2 (jj anonymous heads + publication only on demand), R4, R5, R6,
  R7, R8, R9, R10, R11, R14, R16, R17, R18, R19 (derived), R20 (jj), R22,
  R28, R30 (by structure), R33.

Needs the hosting layer:
- R13 (merge detection across strategies: patch-id + change-id now; host
  API later), R19 retarget, R34 (PR state), R36 (`refs/pull/*` as read-only
  refs).

Open:
- R3: anchors are invisible on GitHub, but mirrors may drop them;
  reconstructible from component remotes.
- R12: a composition trunk carrying unlanded component work is policy.
- R15: divergent induced branches shown as jj-style conflicts?
- R21: provenance of ref moves.
- R23: an op log across remotes is impossible; local ref changes are jj's.
- R25–R27: tag namespacing and backport relations.
- R31: views over partial compositions.
- R35: CI detached/shallow.

## 7. Things this design deliberately does not decide yet

- **PR creation and retargeting** (hosting layer). Needed to clear the bar
  on W4, but orthogonal to the model.
- **Whether a CloneX-aware server** later replaces `refs/meta/clonex` and
  anchor refs with something first-class (event index, cross-repo topics).
  Nothing here requires one.
