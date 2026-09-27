# Questions for the owner

Only value judgments and external facts that engineering can't settle. Each
question gives the alternatives, the episodes that make it matter, and a
recommendation. Mechanics have been decided in `model.md`.

## Q1. May tool commits carry a `Clonex-Change: <id>` trailer?

It is written only when the composition commit was made with jj (its
change-id). It is visible in every tool's `git log`, like Gerrit's
`Change-Id`.

- **Keep it.** An amend after a reviewer's commit on the PR branch merges
  cleanly (D12). `clonex where <id>` finds sibling commits across repos
  (J5/J15). Stale bot commits can be detected (planned, K14).
- **Drop it.** Tool histories stay free of product metadata (blind req. 1).
  D12-style amends conflict, as in plain Git, and sibling discovery needs
  the composition.

Recommendation: **keep**, jj-only as now.

## Q2. Should `publish` push straight to a followed branch (e.g. `trunc` `main`)?

Today publishing from a composition branch named `main` pushes to each
tool's `main`.

- **Allow** (current). Fast for personal repos.
- **Refuse by default.** Tool `main` moves only through the tool's own PR,
  integration workflow and ledger bot (your AGENTS.md process); a
  composition's `main` pushes nothing unless asked with `--branch main`.
  This also blocks the attack's worst plain-Git outcomes: pinned-back
  reverts and "ours" merges published to `main`.

Recommendation: **refuse by default** for tools, since the tools' CI and
ledger rules require PRs.

## Q3. Convert the real umbrella from submodules to contained histories?

The trial on a copy worked (7 children; about 1,700 tool commits enter the
umbrella's object graph).

What changes:
- A plain `git clone` gets everything, and pointer bumps become `clonex
  sync` merges (≈30% of umbrella commits today).
- The "tool PR lands before umbrella pointer" ordering rule is no longer
  needed for availability.
- `pinned_main_parity`, `standalone_publishability`, the ledger's submodule
  checkout and `generate-version-json.py` change meaning.
- `git log` without `--first-parent` becomes noisy.

It is outward-facing and touches every concern, so it's your call, and when.
Recommendation: yes, on a long-lived trial branch first, after Q2 and the
stale-adoption work (K14 correctness for ratchet repos) land.

## Q4. Publish CloneX as `maxeonyx/clonex`, a tool under the umbrella?

That means GitHub repo creation, the ecosystem concerns and the ratchet
ledger. Also: issue #4's `cx` (clone/workspace lifecycle) shares the name.
The design found they converge. An agent task context (issue #4's
"workspace of multiple repos") *is* a composition checkout (episodes I4,
J1).

- **One product.** CloneX compositions are the workspace model; issue #4's
  lifecycle (tmux/session/PR/cleanup) layers on top.
- **Two products.** Separate names, and `cx` stays a clone manager.

Recommendation: **one product**, with lifecycle as a later layer.

## Q5. Priority of the transaction-language track

This is the "insert a not-yet-existing commit, move hunks into it, one
undoable operation" idea (J8). The design found the composition model doesn't
need it, since jj already provides local rewriting and undo. It is a general
VCS improvement best built on jj-lib.

- Build next.
- Defer until compositions are in daily use.

Recommendation: **defer**; stale adoptions and sync placement matter more
for real work.
