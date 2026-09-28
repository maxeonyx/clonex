# CloneX design overview

A one-page map of the current design. Details and evidence:
- `model.md` covers history and the lens;
- `refs.md` covers refs, transport and workflows;
- `episodes.md` and `research/` hold the search record.

Status: **experimental; the design is still under search.** The code in
`src/` is an experiment against an earlier version of this design, not a
product.

## Purpose

Tools stay ordinary, independent Git repositories. Any larger context (the
umbrella, a frontend view, an ephemeral agent or CI context) is also an
ordinary repository that *contains* those tools' histories. Work can start in
either. A change across many tools, made in a composition, is one change.
Nobody hand-writes pointer bumps, copies patches, or orders repositories. The
bar for adoption is strictly beating Git and jj everywhere, not only on
composition.

## Three layers

**1. History** (immutable, identical in every copy):
- trees and commits, exactly as Git has them;
- `Clonex-Adopt: <path> <sha>`: "the content at `<path>` incorporates
  component commit `<sha>`";
- `Clonex-Change: <id>`: "this commit belongs to logical change `<id>`"
  (jj's change-id).

Nothing in history names a branch, a remote or a URL.

**2. Refs** (mutable, per repository): ordinary Git refs. Relations between
repositories' refs are *computed*, never stored:
- A repo's trunk is its remote's `HEAD`.
- A composition's trunk integrates each component's trunk.
- A composition topic `N` induces a branch `N` in exactly the components it
  changes.
- Syncing topic `N` adopts only same-named component branches (bot commits,
  review commits). Component trunk work reaches topics by rebasing onto the
  composition trunk.
- Stacks are composition ancestry, projected per component.

**3. Configuration and anchors** (mutable, shared, outside history). Kept in
the composition's own remote, in ref namespaces plain Git never shows:
- `refs/meta/clonex`: occurrence → remote URL;
- `refs/clonex/adopted/...`: keeps adopted component commits reachable.

## One function does the work: the lens

`get_P(c)` maps any composition commit `c` and path `P` to the component
commit it corresponds to. It is pure, deterministic and total.
- Where `c` adopted component commit `t` with nothing of its own on top,
  `get_P(c) = t`, the same SHA, so signatures, statuses and tags survive.
- Where the composition has its own work, `get_P` derives ordinary component
  commits: one per composition commit touching `P`, with the same author and
  message.

Everything else is built from it:

| Operation | In terms of the lens |
|---|---|
| **Publish** | Make each component's induced branch equal `get_P(N)` |
| **Adopt** | Bring component commit `t` into `P`, reapplying only the composition's own work |
| **Extract** | `get_P` over a directory's whole history |
| **Nesting** | Composes: `get_{A/Q} = get_Q ∘ get_A` |
| **"Where does change X appear?"** | Follow `Clonex-Change` through the lens |
| **"Which trunc is in U's release v1?"** | `get_P(v1)` |

Laws (property-tested in the current experiment):
- **Tree law**: the derived commit's tree is exactly the composition's
  content at `P`.
- **PutGet**: adopt, then derive, gives the same SHA.
- **GetPut**: adopting the derived commit changes nothing.
- **Determinism** across clones.
- **Locality**: unrelated rewrites don't change derived commits.
- **Nesting**, as in the table.
- **Content fusion**: equal edits to equal copies derive one commit.
- **No lost upstream work.**

## Product boundary

- jj or Git owns the *write side* and single-repository work: commits,
  rewriting, conflicts, undo.
- CloneX owns *transport between repositories* (sync, push) and the
  *cross-history read views*: log, annotate, bisect and diff that descend
  from a composition into component history.

Every cross-repository edit is an ordinary edit of one repository, so jj's
working copy, auto-rebase and op log apply unchanged.

## Why this and not the original event-ID design

- Rebuilding a composition from components by event identity loses order and
  state (83% of generated histories).
- Opposite-order rebases in two remotes give cycles.
- Every place to store an ID is destroyed by some ordinary Git operation.

Content plus recorded adoption survives all of that. Event identity is kept
as the `Clonex-Change` overlay, where it earns its keep: clean merges after
rewrites, and finding sibling commits.

## Open, in order of consequence

1. **How adopted component history is held** in the composition: as merge
   parents (current code), as composition-shaped copies, or out of branch
   history and reachable through anchor refs (current lean).
   `research/beat-git-jj-audit.md` compares them.
2. **Ref-layer attack results** (`research/refs-attack.md`), including
   repeated occurrences inducing the same branch name.
3. Stale adoptions after rewriting under bot commits, and where sync places
   adoptions relative to unpublished work.
4. The hosting layer: PR creation, cross-repo linking, retargeting stacks.
5. The cross-history read views needed to clear the Git/jj bar.
