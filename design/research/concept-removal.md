# Concept-removal review of the selected model (C-ANCHOR)

Status: adversarial review. Scope: `../model.md` §3, checked against
`../episodes.md` and `blind-episodes.md` (Summary 1 requirements R1–R40). The
reviewer's only goal is to delete or merge concepts. The reviewer did not edit
either source.

Verdict in one line: the model has about 14 named mechanisms, and **5 of them
carry weight**. The rest are derived queries, parts of `get_P`'s definition,
or fields that exist only to repair other fields. Three of the deletions also
fix real defects (§3).

---

## 1. Concept-by-concept

For each concept: what breaks if it is deleted, and the verdict.

### 1.1 Manifest `.clonex.toml` as the *occurrence declaration*

The model uses the manifest for two separate jobs:
(a) to declare that path P is an occurrence, which gates `get_P` (rule 3:
"P isn't declared at c → none"), and
(b) to hold transport and intent (remote, branch, follow).

**Job (a) can be deleted.** `get_P` is already defined for any path. Rule 3's
first half ("c has no P → none") is enough.
- A1/A5/F1 (new repo, extracting `crates/help-test`): these *need* `get_P`
  over commits made **before** any declaration, because F1 asks that "help-test
  repo contains only its own history". So declaration-gating is wrong for
  extraction, not just redundant.
- C9/C14 (add an occurrence): the base edge (§1.3) marks the start. No
  declaration is needed.
- C10/C5 (remove an occurrence): if the directory is deleted, rule 3 maps
  the commit to none, so nothing is published as "delete all files". Old
  commits stay reachable because they are graph edges.
- K22 (a plain-Git user removes the manifest entry but keeps the directory):
  the ambiguity **disappears**. The occurrence still exists (it has content
  and a base). Only its publish/follow config is gone.
- C16 (`cp -r` makes a copy): unchanged. `cx` can offer to add a base edge
  by tree equality.

So the occurrence set becomes derived: the paths P at `c` where `c` has a
base edge for P somewhere in the ancestry of P's current lineage.

**Job (b) stays, and shrinks** (§1.2). Current intent must be readable from
the current tree and must merge textually, so a tree file is the right
carrier. Trailers are wrong for it: they are history, not state, and they
don't merge. C4 (Alice and Bob must sync F the same way) and X20 (a bot needs
to know what to adopt) break if intent lives only on the command line.

### 1.2 Occurrence fields

| Field | Delete? | Reason |
|---|---|---|
| `path` | Keep; it is the key | Nothing else addresses a subtree. |
| `name` | **Delete** | It has three users, and each has a replacement. CLI addressing can use the path or its basename (tb/tmux-bridge shows name and repo already differ, and nothing breaks). Local overrides use git's own `url.<x>.insteadOf` (H8 mirrors, F7 forks, F5 renames). Move identity is handled by tree-equality move detection (§1.3), with one trailer form for move-with-edit. No episode needs a second key besides path. C6 is about commit identity, which is SHA and not name. |
| `remote` | Keep (rename `url`) | G6/F4 (per-component targets), F14/F15 (a fresh clone must know where to sync from). Local overrides use `insteadOf`, so the model's Q3 "local override mechanism" is deleted too. |
| `branch` ("for publication") | **Merge into follow** | The model conflates two branches: the branch to *follow* (main) and the branch to *publish a change to* (a PR branch). The publish branch is per change, not per occurrence (B3, G1, E4 stacked PRs, K14 PR branches). One manifest field cannot hold it. Derive it instead (§1.9). |
| `follow` (`branch`/`none`) | **Merge with branch into `follow = <ref>`** | A ref that moves (main, release-0.4) means "follow". A tag (v0.4.5), or no field at all, means "pinned". This gives C4, A11 (frozen fixture), C2-blind (backport occurrence follows release-0.4), and X20 (auto-adopt). |

**Could "follow" be implied by "has no delta"?** No. It breaks in both
directions.
- K14/X1/E6: an occurrence *with* unpublished delta must keep following
  (bots and outsiders land under local work). That is the core workflow.
- C4/A11: a frozen fixture *without* delta must not follow.

Following and having a delta are independent facts.

The resulting manifest is `path → {url, follow?}`. That is `.gitmodules`
without gitlinks. Whether the real `.gitmodules` file could carry it without
confusing `git submodule` is UNVERIFIED. Don't do it without an experiment.

### 1.3 `Clonex-Base` trailer

The base is really a **parent edge** (adoption makes `t` a parent). The trailer
only labels *which path* that edge belongs to. Tree equality supplies that
label in the common case:

> Parent `p` of `c` is P's base edge if `tree(p) == tree(c)[P]`.

This holds for every adoption with an empty residual delta. That covers every
fast-forward in T-space: GitHub merge commits (G9), bot commits (K14, X1, X10),
outsider PRs (B1, H1, H4), force-push adoption with tree-identical rewrites
(F7, X6, G5), and upgrading an occurrence (C12). In K14, the adoption of
`bot_T` over a published `r_T` has `tree(c)[T] == tree(bot_T)`, so no trailer
is needed.

A bonus: a plain-Git `git subtree pull --prefix=P` merge would satisfy this
test. If so, it would be recognised as an adoption with no CloneX involved
(R2, R9). This follows from the rule but has not been tested.

The trailer is still needed only where the parent edge can't be labelled by
tree equality:
1. **Adoption with residual delta** (X17 suggested-change `r` plus a local
   amend, E6 stale base): the merged tree is not equal to `tree(t)`. With a
   single occurrence of that component, lineage (`t` shares ancestry with
   exactly one occurrence's `get_P`) would still identify P. The trailer is
   *required* only for **repeated occurrences of one component** (C3, X7,
   X26).
2. **Adding an occurrence from a commit already reachable** (C14: `t4` is
   already in L's past). There is no new parent edge, so the trailer names it.
3. **Move with edits in one commit** (F4, X15, K9 done by plain Git). A pure
   move is detected by tree equality (the old subtree disappears and an
   identical one appears). `cx mv` always writes a pure move. The only
   ambiguous case is a plain-Git move with edits, and it can be repaired with
   the path form `Clonex-Base: <new> path:<old>`.
4. **Subpath occurrences** (K17, A12): tree equality with a root tree can't
   hold, so the trailer must carry `sha:subpath`.

Verdict: keep one trailer, **written only when tree equality or lineage
doesn't determine the base**. This takes metadata out of nearly every sync
commit. It also shrinks fact 2's exposure ("every carrier dies somewhere") to
the rare cases, and those are verifiable (the sha must be reachable and the
tree must match).

### 1.4 Base derivation rule ("otherwise base = maximal elements of the union of parents' bases")

**Delete.** It is a second recursion that runs parallel to `get_P` rule 2,
which already computes `cand` from the parents' `get_P`. The model uses
"base" for two things only:
- to define delta (§1.5);
- as input to `put_P`. But `put_P` actually uses `mergebase(get_P(c), t)`,
  not `base`.

"Base" is recoverable as the frontier of *non-virtual* commits in `get_P(c)`'s
ancestry. No episode reads base without also reading `get_P`.

### 1.5 Delta

**Delete as a concept. It is a query.**

> delta(c, P) = the commits in `get_P(c)`'s ancestry that are not reachable
> from any fetched ref of P's url.

Every use of "delta" is a status query: "unpublished" (J2, R23), "unlanded"
(X4, R24), and "deliberate local patch" (a vendored fixture with no url,
whose delta never goes away). It doesn't need to exist as its own concept.

One consequence the model misses: L3 determinism means **publishing alone
empties the delta**, because the pushed SHAs are the virtual SHAs. L7's
"publish, then adopt" needs no adoption step until the remote adds something.

### 1.6 Lens `get_P` / `put_P`

- `get_P`: **essential.** Every publish, status, and nesting law depends on it.
- `put_P`: keep the operation, but it is not a separate concept. It is
  "subtree-merge at P, using the component-space merge base
  `mergebase(get_P(c), t)`, then label the edge (§1.3)". `put_P` is the only
  thing `sync` does, so count them as one (§1.9).

### 1.7 Keep-merges rule

**Fold it into `get_P`'s definition. It can't be deleted.** Without it:
- E2/E3: GitHub merge structure isn't reproduced, so round-trip L1 fails on
  merge commits.
- L5 nesting fails (algebra F2).

Removing it would only be possible if nesting were forbidden (blind C7 lists
that as an option). Even then, E3 still needs merges kept. It is part of the
definition, not a separate concept.

### 1.8 `committer := author`

**Fold it into `get_P`, as "derived metadata is exactly c's author and message".**

It can't be deleted:
- D2/K4: every unrelated jj or Git rebase of U rewrites committer dates. With
  real committer data, each rebase would change every derived SHA and
  force-push every PR.
- K23: two machines must produce the same SHAs.

### 1.9 `publish`, `sync`, and publication state

Both are **stateless functions of (commit graph, remote refs)**:

- `publish P` = push `get_P(head)` to `url`. The branch is **the composition
  branch's own name** (or the `follow` ref for direct-to-main pushes).
  Deriving the branch name removes per-change naming state and gives
  collaborators the same answer (G2 "visible to any collaborator who fetches
  it"). Stacked PRs (E4) come from separate composition bookmarks.
- The status views all come from comparing refs, not from stored state:
  - partial publication (G2, X3, X27, F5 retries): compare each
    `get_P(head)` with the remote ref;
  - "which version is on which remote" (X3): the same comparison;
  - immutability (D3, K7): whether a commit is reachable from the remote's
    follow ref.
- `sync P` = fetch the `follow` ref and the published branch (K14 bots write
  to PR branches), then `put_P`.

The model already had no publish state. This review confirms that nothing
needs to be added, and that the per-occurrence `branch` field was the only
thing pretending to be publish state.

Should `sync` and `publish` be merged into one "converge" verb? No. Publishing
has an external effect and needs a dry run and explicit consent (J17, K12,
X23). Keep two verbs.

### 1.10 `Clonex-Change` trailer

**Delete it from `get_P`.** At most, keep it as an opt-in display annotation
(the owner decision in T1). The model's own claim is "losing it degrades
only conveniences". Checking each convenience:

| Use | Replacement | Episode check |
|---|---|---|
| Replace a published PR branch when its source is rewritten | The branch name (§1.9) is the identity. The update is force-with-lease, or an append commit when foreign commits exist | D2, K4, X3, X17 still pass |
| Recognise a squash-merge | Content: adopting `s` runs merge3 with identical changes on both sides, so the result is `tree(s)`, the delta is empty, and the change counts as landed | H2, X11, D16 pass with no metadata |
| Recognise a rebase-merge | `git patch-id` / range-diff against the virtual commits | H3 passes |
| "Where does change X appear" | From a composition: `get_P(c)` for each P, then look it up in remote refs. Across forks: same SHA (L3) | C5, J15, X26 pass |
| Find sibling commits from a lone component clone | **Degrades.** Use the PR description or branch-name link (G10) instead | R35 (J5) partly. This is the one real loss. |

Keeping the trailer in the core has two costs:
- **R1 purity.** Every derived component commit would carry product
  metadata, which is the T1 tension.
- **A defect (§3).** "Derived from its SHA when made by plain Git" breaks
  L4. The trailer also makes `get_P` depend on jj's change-id. D5 already
  observed this: identity in the commit forces republication on split.

### 1.11 The "logical log" view

**Delete it as a model concept.** B5-blind and R13 are satisfied by the
composition's own `git log`, which already interleaves because U contains the
component commits, plus `--first-parent`. The view is a CLI formatting
choice, not a concept.

### 1.12 Product boundary: jj as the client

**Keep. This decision is itself a deletion** (no owned working copy, op log or
transactions).

Once `Clonex-Change` leaves the core, CloneX has **zero coupling to jj**. jj
remains the recommended client because it rebases safely and has an op log.
But no law depends on jj's change-id, so the "or derived from its SHA"
fallback and its failure mode (§3) disappear.

---

## 2. Things that looked removable but are not

- **The manifest as a whole.** C4 needs shared follow intent, and F14/F15
  need a shared url. Deriving occurrences from base edges removes its
  *declaration* role. It doesn't remove the file.
- **The `Clonex-Base` trailer as a whole.** Repeated occurrences (C3, X7,
  X26), adding from a reachable commit (C14), subpaths (K17), and
  plain-Git move-with-edit all lack a unique tree-equality witness.
- **`put_P`'s special merge base.** Git's own subtree merge uses
  `mergebase(c, t)` in U-space. That misses the fact that the virtual commits
  were published, so it produces spurious conflicts when the remote changed
  the same lines, for example a reviewer amendment (X17). Keep the
  component-space base.

## 3. Defects the deletions fix

1. **L4 breaks under plain-Git rebase.** `Clonex-Change` is "derived from its
   SHA when made by plain Git". Rebasing such a composition commit changes
   its SHA, so the trailer changes, and so does every derived component SHA,
   even for components the rebase didn't touch. D6 and K4 fail. Removing the
   trailer from `get_P` fixes it.
2. **Rule 3's declaration gate conflicts with extraction.** A5 and F1 want
   the path's history from before any declaration. Gating on the manifest
   drops it. Removing the gate fixes it.
3. **The `branch` field conflates follow and publish targets.** Per-change PR
   branches (B3, E4, K14) can't live in a per-occurrence field. Deriving the
   branch from the composition branch fixes it.
4. **Removing a manifest entry would silently cancel an occurrence** (K22).
   With derived occurrences, it only loses follow/publish config.

## 4. Smallest model that satisfies the episodes

1. **Git commits only.** Any repo is a composition.
2. **Base edge.** Parent `p` of `c` is the base of path P when
   `tree(p) == tree(c)[P]`. Otherwise it is named by
   `Clonex-Base: P <sha>[:subpath] | path:<old>`, and `cx` writes that
   trailer only when the tree-equality test fails. Occurrences are derived:
   a path is an occurrence while its lineage has a base edge. *(Open: whether
   lineage alone should also disambiguate residual-delta adoptions of
   single-occurrence components, which would make the trailer
   repeated-occurrence-only. That adds a second rule, so the recommendation
   is to write the trailer.)*
3. **`get_P`.** This is the model's §3 algorithm without rule 3's declaration
   gate and without the change trailer. The derived commit is fully
   determined: tree = `tree(c)[P]`, parents = `cand`, author = committer =
   c's author, message = c's message verbatim, and keep-merges applies.
4. **`.clonex.toml`**: `path → { url, follow = <ref>? }`. A moving ref means
   follow. A tag or an absent field means pinned. Local overrides use git's
   `insteadOf`.
5. **Two stateless verbs.**
   - `sync` = fetch the follow ref and the published branch, then
     subtree-merge with `mergebase(get_P(c), t)` as the base.
   - `publish` = push `get_P(head)` to `url`, onto the composition branch's
     name.

**Everything else is a query:** base, delta, unpublished, landed, partial
publication, immutability, change correspondence (content, patch-id, SHA),
and the logical log.

**What is lost:** discovering siblings from a lone component clone without a
composition (J5/R35, partly). The optional `Clonex-Change` trailer can restore
it if the owner accepts T1's metadata cost.
