# CloneX design overview

> **Superseded as the root by `constraints.md`** (2026-09-28): use cases →
> constraints → a level-1 (objects-only) design, with refs deferred to a
> separate level 2. Where this page disagrees, `constraints.md` wins. In
> particular it replaces "squashed adoptions" with self-verifying
> representatives, because the owner requires the composition to look like
> a monorepo to Git (K1).

The current design on one page, consolidated after three independent
reviews:
- `research/refs-workflows.md`: blind contributor and ref episodes;
- `research/refs-attack.md`: 68 episodes replayed against the ref design;
- `research/beat-git-jj-audit.md`: 192 aspects, 19 experiments on the real
  tools.

Details are in `model.md` and `refs.md`; each has an authoritative §8.

Status: **experimental; the design is still under search. It does not yet
clear the adoption bar** (strictly beat Git and jj everywhere; see the
audit). The code in `src/` implements an earlier version (verbatim
parents, R1). It is evidence, not the product.

## Purpose

Tools stay ordinary, independent Git repositories. Any larger context (the
umbrella, a frontend view, an ephemeral agent or CI context) is also an
ordinary repository that contains those tools' content and derives their
histories. Work can start anywhere. A change across many tools is one
change, with no pointer bumps, patch copying or hand-ordering of repos.

## Three layers

**1. History** (immutable, identical in every copy, never names a ref or a
URL):
- Ordinary commits and trees. The composition's *branch history* contains
  only composition commits (no component commits, so every Git/jj/GitHub
  operation on it is safe).
- `Clonex-Adopt: <path> <from>..<to>`: "the content at `<path>` moved from
  component commit `<from>` to `<to>`". An adoption is an ordinary
  one-parent commit. The claim is valid only where the parent's derived
  state really is `<from>`, so cherry-picked or copied adoptions degrade to
  plain content and can't lie.
- `Clonex-Change: <id>`: the logical change, from jj's change-id.

**2. Refs** (mutable, per repository). Ordinary refs, plus *bindings* in
the ref layer:
- Each repo's trunk is its remote `HEAD`.
- The composition trunk integrates component trunks by adoption, and never
  publishes to them by default.
- A topic `N` is bound, per `(occurrence, topic)`, to a component branch:
  same name by default, templates or suffixes allowed. Bindings live in
  `refs/clonex/topics/<N>`, which records the branch name, a shared lease,
  the stack base and the state.
- Sync follows bindings only; it never adopts by name coincidence.
- Stacks are composition ancestry, projected per component.

**3. Config and objects** (mutable, shared, outside history):
- `refs/meta/clonex`: component identity → URL. A URL change is accepted
  only if the new remote contains the already-adopted commits (a
  continuity check), so it can redirect to mirrors or forks, not to
  different history.
- Component objects: fetched by SHA from the component remote, with
  `refs/clonex/adopted/*` anchors in the composition remote as a
  self-contained cache. A missing object is a hard, explained error, never
  a silently different derivation.

## The lens

`get_P(c)`: composition commit and path → component commit. It is pure,
deterministic and total.
- An adoption with no own work on top gives the adopted commit itself, so
  SHAs, signatures and statuses survive.
- Own work gives derived commits with the same author and message.

Everything is built on it:
- **push**: converge each bound branch to `get_P`;
- **adopt**: reapply only own work onto the new component state;
- **extract**;
- **nesting**: `get_{A/Q} = get_Q ∘ get_A`;
- **"which trunc is in release v1"**: `get_P(v1)`;
- **component-space views**: the log, blame and bisect of `get_P(HEAD)` are
  the *native* component answers (the audit verified exact matches).

Laws: tree law, PutGet, GetPut, determinism, locality, nesting, content
fusion, no lost upstream work. Tested in the experiment; to be re-tested
under R3.

## Product boundary (revised by the audit)

- **jj/Git own**: commits, rewriting, conflicts, undo, single-repo work.
  Under R3 CloneX never has to own rewriting. That is the main reason R3
  won.
- **CloneX must own**:
  - **Cross-history read views**: log, blame, bisect, show and describe in
    component space, and a bisect that descends from composition steps into
    component commits.
  - **Adoption as a real merge**: any revision, including backwards, tags,
    `refs/pull/N/head` and forks. Conflicts are handed to jj/Git as ordinary
    conflicts, not aborted.
  - **Publication**: honours hooks, signing, tags and LFS; forks and several
    remotes.
  - **A persistent derivation cache**, safety in shallow CI checkouts, and
    truthful status in fresh clones.
  - **Each CloneX operation recorded as one jj operation**, so `jj undo`
    works.
- **Hosting layer** (PR creation, sibling links, stack retargeting before
  delete-on-merge): needs something running at merge time, i.e. a bot or
  a server.

## Where it can decisively beat both

- One logical change published to N independent repos: leased,
  retryable, honest partial state.
- Component updates reviewed as content diffs instead of pointer bumps.
- Merges that survive upstream rewrites.
- One status and change lookup across repos.
- Blame and bisect across repos.
- Deterministic, throwaway compositions for CI and agents.

## Rejected, and why (don't re-propose without new evidence)

- **Event-ID interleaving as storage**: loses order and state, and
  creates cycles.
- **Verbatim component commits in branch history (R1)**: every rewrite
  tool mangles them (blockers REG-1/2/3).
- **Path-shaped copies as storage (R2)**: signatures lost, fragile
  reconstruction. Kept as a *computed view*.
- **In-tree manifest with URLs and branch names**: mutable names in
  immutable history.
- **Adopting by name coincidence**.

## Open

1. The owner questions in `questions.md`.
2. Stale bot commits after rewriting under them, and sync placement
   relative to unpublished work.
3. Whether bindings, config and anchors in private refs are acceptable, or
   whether a server should hold them.
4. Re-running the laws and the audit's 19 experiments against an R3
   prototype, *after* the design settles.
