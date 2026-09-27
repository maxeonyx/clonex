# CloneX — agent instructions

## Why this exists (external truth — from the owner, not up for redesign)

Agent Tools is many ordinary, independent Git repositories (tools with their
own CI, releases, Pages, outside contributors) plus larger compositions of
them (the umbrella; frontend/backend/security views; ephemeral CI and agent
contexts). The owner wants:

- a change across many tools, made from a composition, to be one logical
  change: no repo switching, patch copying, subtree rituals or pointer
  bumps;
- work done in a component to become usable in compositions without a
  canonical monorepo;
- tools that stay completely ordinary Git repos, plain-Git contributors,
  and GitHub hosting that keeps working;
- repository boundaries that stop constraining logical history and
  development context.

If CloneX has to own the client, it must be JJ-grade or better; it should not
replace Git/jj needlessly. Agents are first-class users.

## The model (current; confidence moderate-high for the core)

Read `design/model.md`, especially **§8, which overrides earlier sections**.
In one paragraph:

A composition is an ordinary Git repo that *contains* component histories
verbatim. An adoption is a merge whose extra parent **is** the component
commit, marked `Clonex-Adopt: <path> <sha>`. `.clonex.toml` declares intent
only (path → remote, followed branch); it never records revisions.
`get_P` (derive) is a total, pure function from composition commits to
component commits. An adoption with no remaining changes maps to the adopted
commit itself, so SHAs, signatures and statuses round-trip.

`put_P` (adopt) reapplies only the composition's own work. Publication is
convergent: push `get_P(HEAD)`; replace a branch only when its tip is what we
last published. Change identity (jj change-id → `Clonex-Change`) is an
overlay for rewrite-aware merging and queries. Content correctness does not
depend on it. jj (colocated) or plain Git is the everyday client.

### Invariants (tested; keep them tested)

| | Invariant |
|---|---|
| Tree law | `tree(get_P(c)) == tree(c)[P]` |
| PutGet | Adopting `t` with nothing of our own gives `get_P == t`, the same SHA |
| Determinism | `get_P` depends only on objects. Committer := author, no clock, no user; independent clones derive identical SHAs |
| Locality | Reordering or rebasing composition commits that don't touch P leaves `get_P` unchanged |
| Nesting | `get_{A/Q}(M) == get_Q(get_A(M))` |
| Content fusion | Equal bases plus equal edits derive one commit |
| Append-only | Followed branches only move forward; published PR branches are replaced only under a lease |
| No lost upstream work | Every commit that reached a followed branch is contained after sync |

### Counterexamples that were expensive to find (don't rediscover them)

- **Interleaving events by identity cannot be the storage model.**
  `compose∘filter ≠ F` in 83% of random histories. Opposite-order rebases in
  two remotes create cycles, and half-arrived events destabilize every
  descendant (`design/research/algebra-attack.md`).
- **Every metadata carrier dies somewhere.** Headers die on any rewrite,
  including jj's. Trailers get mangled by squash and copied by cherry-pick.
  Notes and refs don't transfer. See `research/git-interop-matrix.md`.
- **Gating the lens on the manifest is wrong**; it loses pre-declaration
  history.
- **A change id derived from a SHA breaks locality** under plain rebase.
- **A 3-way merge from `mergebase(get_P, t)` is wrong** after an upstream
  rewrite, after a squash-then-revert, and after an amend over a reviewer's
  commit. Hence the own-work reapplication.
- **`git rebase --rebase-merges` is not safe on compositions.** It replays
  component commits at the root with exit code 0. `jj rebase -b` was unsafe
  until component tips were placed under `refs/remotes/` (immutable in jj).

### Deliberate compromises (not accidents)

- `git log` in a composition shows component commits; use
  `--first-parent`. `git blame` attributes adopted lines to the adoption
  merge.
- Sub-path occurrences are unsupported: an occurrence holds a whole
  component tree.
- Stale adoptions, sync placement under unpublished work, `clonex mv` and
  publish guards are designed but not built (§8 lists the minimal cases).
- The programmable transaction language (insert a not-yet-existing commit,
  move hunks, one undoable operation) is a separate, deferred track. It is
  orthogonal to composition, because every cross-history edit is an edit of
  one repository. It belongs on jj-lib, not in this engine.
- Git plumbing via the `git` CLI (one persistent `cat-file --batch`),
  chosen for fidelity over speed. Real umbrella `status`: about 0.13s.

## How to work on CloneX (the discipline is part of the product)

This project is unusually vulnerable to elegant-but-wrong models. The first
design (the event-identity algebra) was elegant and wrong, and so were three
rules of its replacement. The way they were caught is the method:

1. **Expand episodes before changing architecture.** Before any change to
   the model's rules, add concrete episodes to `design/episodes.md`: real
   actors, small symbolic DAGs, before and after state in *every* affected
   repo, and what can fail. Include intersections: repeated occurrence +
   rewrite, partial publish + retry, bot commits + cross-repo change,
   plain-Git + composition. Most defects lived at intersections. Stop when
   new cases are duplicates, not when the first few pass.
2. **Generate competing models for consequential choices**, and replay the
   same hard episodes against each. Judge by terminal behaviour: what users
   see, what Git holds, what can fail. Do not judge by elegance.
3. **Existing code is evidence, not structure.** No module has seniority
   over the model. If a rule needs exceptions, suspect the ontology first.
4. **Implementation friction reopens design.** A failing test, a special
   mode, a message that reads wrong in one history, a hidden reconciliation
   step: each is evidence about the model. Nesting (L5) was found this way.
5. **Challenge independently.** For consequential changes, use independent
   agents that do not see your model (blind episode enumeration), attack
   it with executable scenarios, and try to delete concepts. Incorporate,
   test, or reject each material objection for a concrete reason. **Give
   your own sub-agents this same discipline**; don't hand them a
   conclusion to confirm.
6. **Every law claim is a test**, and every new test must be shown to fail
   without the code it covers (mutation or red-first), matching the
   ecosystem's TDD ratchet.
7. **Ask the owner only for external-world value judgments** (priorities,
   workflow facts, purity-versus-traceability tradeoffs). Bring the
   alternatives and the episodes that make the choice matter. Mechanics are
   yours to discover; do not transfer design work to the human. Open owner
   questions: `design/questions.md`.

## Layout and commands

- `src/git.rs`: the only code that talks to Git.
- `src/lens.rs`: `get`/`put` and adoption; the semantic core.
- `src/manifest.rs`: `.clonex.toml` and the trailers.
- `src/ops.rs`: declare/sync/publish/status/where.
- `tests/episodes.rs`: named episodes on real repos.
- `tests/laws.rs`: property tests.
- `design/`: episodes, model, questions, and independent research. Research
  files are dated evidence, not specs.

```bash
cargo test                       # episodes + generated-history laws (~30s)
cargo run -- status              # in any composition
cargo run -- --json status       # agents: complete structured state
```

Standalone crate (its own `[workspace]`). It is not yet a published tool
repo or umbrella submodule, and not yet under the ecosystem's tdd-ratchet
ledger: see `design/questions.md`.
