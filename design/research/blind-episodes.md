# Blind workflow episodes for a composition-aware version-control product

Status: research input, written without sight of any proposed design. Nothing
here prescribes a mechanism. Each episode is a concrete ground-level story; the
"Requirement exposed" line is phrased as something an outside observer could
check without knowing how the product works inside.

## Ground facts these episodes are built from

These come from the real `maxeonyx/agent-tools` ecosystem as it stood on 2026-09-28
(read from `at-clonex`, `at-main` and siblings under `agent-tools-workspace`).

- **Components.** trunc (`maxeonyx/trunc`), tb (`maxeonyx/tmux-bridge`, mounted
  at `tools/tb`, so repo name differs from mount name), dotsync, tdd-ratchet
  (`maxeonyx/tdd-ratchet-rs`, mount name differs again), oc (archived once, then
  back, Linux-only), agent-harness, and the library help-test
  (`libraries/help-test`, which began as an umbrella path crate
  `crates/help-test` and now appears to tools as
  `help-test = { git = "https://github.com/maxeonyx/help-test", tag = "v0.1.0" }`).
- **Umbrella.** `maxeonyx/agent-tools`: 268 commits on the clonex branch, of
  which roughly 35-50 are "Point <tool> at vX" / "Update <tool> ... pointer" /
  "Pin dotsync v0.9.1" commits. It holds `crates/standards` (the concern suite),
  `state.json` (review attestations keyed by `reviewed_commit` SHA),
  `docs/version.json` (generated from each tool's `docs/version.json` by
  `scripts/generate-version-json.py`), its own `.test-status.json`, and
  workflows `ledger.yml` and `pages.yml`.
- **Concerns that read pins.** `pinned_main_parity` (pin must equal or descend
  from the tool's remote `main`), `latest_ci_green` (a successful `ci.yml` run,
  or the `integrated-ci` commit status, on the exact pinned SHA),
  `release_freshness` (a GitHub Release for the Cargo.toml version whose tag
  points at the pinned commit), `standalone_publishability` (the tool builds
  from its own checkout, no workspace-relative paths), `version_artifacts`
  (`tool --version --json` and `/version.json` on the tool's Pages site agree),
  `workspace_routing` (tool AGENTS.md tells agents to develop from the
  umbrella). Commit 4e0784a: "a tool release regresses them everywhere until
  the pointer lands".
- **tdd-ratchet.** A new test must first be committed red and recorded
  `pending`; only a trusted `ledger.yml` workflow, dispatched with
  `--ref main`, writes `.test-status.json` as a bot commit onto the validated
  branch (`chore: record tdd-ratchet status [skip ci]`). Runs cost ~20 minutes.
  A red commit and its green fix need two dispatches. The history check judges
  every snapshot. Removing a test needs a `removals` entry in `.tdd-ratchet.json`
  that must be deleted in the next commit. The bot commit moves the head SHA and
  voids a `Ready` status recorded on the previous head. Deleting the PR branch
  while the bot runs loses the bot's write.
- **Tool CI.** `gh workflow run ci.yml --ref <branch> -f pr_number=<n>`
  serializes integration, records `Ready`, auto-merges with `--delete-branch`
  (which closes PRs stacked on that branch), publishes release artifacts and
  Pages, and records `integrated-ci` on the exact merge commit.
- **History repair actually happened.** tb main 485a9f1 -> 5458ab8, old history
  at `backup/pre-ratchet-history-main-2026-09-09`; agent-harness likewise.
  Consequences recorded in AGENTS.md: tags re-pointed, GitHub's signatures on
  merge commits lost, older Releases refuse `target_commitish` updates and keep
  naming pre-rewrite commits, branch protection had to be relaxed and restored,
  commit statuses left behind on old SHAs so `latest_ci_green` went red.
- **Workspace rules.** One full clone per task (`at-<branch>`), never worktrees,
  never touch another session's clone, tool PR merges before umbrella pointer
  bump, merge commits preferred, no force-push by default, history replacement
  needs explicit approval, gitlink conflicts resolved by pinning a common
  descendant, never by picking a side.
- **Cross-cutting changes seen or planned.** CLAUDE.md alias beside every
  AGENTS.md (2ec7eff, bbcab58); help-test adoption across tools; auto-update
  integration; tuned CI triggers (562a1a8); `deconfuse-rs` config library all
  tools would adopt; moving `docs/reviews/*.json` out of tool repos into the
  umbrella (TODO.md; those files are currently published by Pages).

Notation: `a─b─c` is a linear history, `(x)` marks a commit that exists only
locally, `@` is the working copy, `U` is the umbrella, `[bot]` is a
tdd-ratchet ledger commit, `P#n` is a GitHub PR. "Composition" means any larger
view (whole project, frontend-only, security view, task view...). "Component"
means one of the independent repos.

---

## A. Creation and composition

### A1. Compose the whole project from nothing
- **Actors:** Max, on a fresh laptop.
- **Start:** Nothing local. GitHub has trunc, tmux-bridge, dotsync, tdd-ratchet-rs, oc, agent-harness, help-test, agent-tools. Each `main` has moved independently.
- **Intent:** "Give me the whole ecosystem so I can work across it."
- **Ideal action:** One command naming the composition (or the umbrella URL).
- **Ideal outcome:** A working tree shaped like today's `tools/<name>` layout, every component at a defined revision, a single status/log view, no half-initialized submodule state. Nothing is written to any remote.
- **Can go wrong:** Some components are private or unreachable; one fetch is slow; the composition names revisions that no longer exist (rewritten tb).
- **Requirement exposed:** Materializing a composition is one step, and its result is fully defined even if some component fetches fail (it reports which).

### A2. Compose only what the task needs
- **Actors:** An agent assigned "fix trunc's numeric flag".
- **Start:** Composition "whole" defined; agent has a 2 GB disk quota.
- **Intent:** "I only need trunc and help-test, plus the standards crate so I can run the concerns."
- **Ideal action:** Materialize a task view: trunc + help-test + standards.
- **Ideal outcome:** Only those three are fetched; commands that would need other components say so instead of silently passing (today: "Standards inspect only initialized tools locally, but `CI=true` requires the complete inventory").
- **Can go wrong:** Partial view silently reports green for concerns across all tools.
- **Requirement exposed:** A partial composition knows what it omits and never reports a result as if it covered the omitted parts.

### A3. Ad-hoc composition for a security audit
- **Actors:** External auditor (read-only GitHub access).
- **Start:** No composition named "security" exists.
- **Intent:** "Show me every `.github/workflows/*.yml`, every `ledger.yml`, every `Cargo.lock` across the suite, at what is currently released."
- **Ideal action:** Declare a view by path patterns across components and a revision rule ("latest release tag of each").
- **Ideal outcome:** A browsable, diffable tree containing only those files, with each file traceable to its component and commit; nothing needs to be pushed anywhere for it to exist.
- **Can go wrong:** "Latest release" is ambiguous for oc (archived) or for tb whose older Releases name pre-rewrite commits.
- **Requirement exposed:** A composition can be a projection of subsets of files, selected by a revision rule, created without write access to anything.

### A4. Ephemeral composition for one CI run
- **Actors:** GitHub Actions runner.
- **Start:** A PR in dotsync; CI wants to test dotsync against the current `main` of help-test and tdd-ratchet.
- **Intent:** "Build this PR head with its neighbours as they are right now."
- **Ideal action:** Construct the composition in the job, run, discard.
- **Ideal outcome:** Test results name the exact revisions of every component used; the composition leaves no ref, branch or commit on any remote.
- **Requirement exposed:** A composition can exist only for the duration of a process and still yield a reproducible identity for what it contained.

### A5. Save an ephemeral composition after the fact
- **Actors:** Agent that ran A4-style exploration and found a bug.
- **Start:** Ephemeral view of trunc@a3, tb@f1, help-test@v0.1.0 with local edits in two.
- **Intent:** "This was just scratch, but I want to keep it and share it."
- **Ideal action:** Name it and publish it.
- **Ideal outcome:** Others can reproduce exactly that state, including the edits, without the scratch view having been declared "permanent" upfront.
- **Requirement exposed:** Promoting a throwaway composition to a shared one does not require redoing any work.

### A6. Compose a view with a component at two revisions
- See section C for the details; creation-side: Max wants "trunc at main" and "trunc at v0.4.5" side by side to compare `--help` output.
- **Requirement exposed:** Declaring a composition allows the same source repo to be included more than once under different names.

### A7. Adopt an existing hand-built directory
- **Actors:** Max.
- **Start:** `~/src/` holds plain clones of trunc, tb and dotsync, each with local branches and uncommitted edits.
- **Intent:** "These are already here; make them a composition without recloning or losing anything."
- **Ideal action:** Point the tool at the directories.
- **Ideal outcome:** Composition exists; each clone still works with plain `git`; uncommitted edits are visible in the composition's status.
- **Can go wrong:** Clones are on detached HEADs, have different remote names (`origin` vs `upstream`).
- **Requirement exposed:** Existing plain Git clones can join a composition in place, with no loss and without ceasing to be ordinary clones.

### A8. Create a new component from inside a composition
- **Actors:** Max creating `deconfuse-rs`.
- **Start:** Whole composition; no deconfuse-rs repo anywhere.
- **Intent:** "Start a new library here, next to the tools that will use it."
- **Ideal action:** Create a directory, mark it as a new component, commit.
- **Ideal outcome:** Work proceeds and commits accrue before any GitHub repo exists; later, creating `maxeonyx/deconfuse-rs` and publishing gives it a history that contains only its own files.
- **Can go wrong:** The first commit also touched trunc; publishing deconfuse-rs must not drag trunc files along.
- **Requirement exposed:** A component can be born inside a composition and published later as a standalone repo whose history is clean of its neighbours.

### A9. Composition defined by a different party
- **Actors:** An outside user, "Kim", who uses trunc and dotsync with her own tool.
- **Start:** Kim has no access to write agent-tools.
- **Intent:** "My project = my repo + trunc + dotsync."
- **Ideal action:** Kim defines her own composition in her own repo.
- **Ideal outcome:** Kim's composition works exactly like Max's; Max's repos are unchanged and need not know Kim exists.
- **Requirement exposed:** Anyone can compose repos they can read; no component needs to opt in or be modified.

### A10. Two compositions sharing components on one machine
- **Actors:** Max.
- **Start:** "whole" and "security" views both include tdd-ratchet.
- **Intent:** "I edited ledger.yml in the security view; the whole view should see it."
- **Ideal outcome:** [OWNER-DECISION] Either (a) both views share one mutable working state for tdd-ratchet (edit visible everywhere immediately), or (b) each view is isolated and the edit becomes visible in the other after an explicit commit/sync, or (c) the second view refuses to open the component while it has uncommitted edits elsewhere. The workspace's current rule ("one session owns one mutable clone") leans to (b).
- **Requirement exposed:** The relationship between concurrent views of the same component on one machine is explicit and predictable.

---

## B. Ordinary development

### B1. Edit one tool, commit, push
- **Actors:** Max.
- **Start:** Whole composition. trunc `main: a─b`.
- **Intent:** "Fix a typo in trunc's help text."
- **Ideal action:** Edit file, describe change, push.
- **Ideal outcome:** trunc gets `a─b─c` on a branch/PR in `maxeonyx/trunc`; the commit looks like any hand-made Git commit (author Max Clarke, no foreign metadata in the tree); nothing else changes anywhere, the umbrella gets no commit unless asked.
- **Requirement exposed:** A single-component change from a composition produces exactly the commit a plain-Git user would have made in that repo.

### B2. Working copy is always a commit
- **Actors:** Agent.
- **Start:** Agent editing dotsync and help-test simultaneously; it is killed mid-task.
- **Intent (Max, afterwards):** "What had it done?"
- **Ideal outcome:** Every edit up to the kill is recoverable as a snapshot of the whole composition, across both components, with a timestamp.
- **Requirement exposed:** Uncommitted work across all components is continuously recoverable, not just per repo.

### B3. Status across the whole composition
- **Actors:** Max.
- **Start:** Edits in trunc and tb; tb's upstream `main` moved; dotsync has an unpushed commit.
- **Intent:** "What's the state of everything?"
- **Ideal outcome:** One screen: per component, local edits, unpushed commits, upstream divergence, and whether the composition's recorded revision of that component is stale.
- **Requirement exposed:** One command answers "what is unpublished, what is stale, what is dirty" for every component.

### B4. Diff across the composition
- **Actors:** Reviewer.
- **Intent:** "Show me everything that changed since last Tuesday, all tools."
- **Ideal outcome:** A single diff with paths prefixed by mount name (`tools/tb/...`), not a list of six separate diffs and six gitlink hash changes.
- **Requirement exposed:** Diffs of a composition show file content changes in components, never opaque revision-number changes.

### B5. Log across the composition
- **Intent:** "When did help-test adoption happen in each tool?"
- **Ideal outcome:** A merged, time-ordered log showing commits from every component, filterable by component and path, where a cross-cutting change appears as one event.
- **Requirement exposed:** History of a composition is browsable as one history, with component attribution on every entry.

### B6. Blame through a composition
- **Intent:** "Who wrote this line in tools/dotsync/src/sync.rs and why?"
- **Ideal outcome:** Blame reaches the dotsync commit, and if that commit was part of a cross-cutting change, shows the logical change and its description too.
- **Requirement exposed:** Line provenance within a component is as good as plain `git blame` in that component, plus the cross-cutting context.

### B7. Run tests of one component from the composition
- **Actors:** Agent.
- **Start:** `cargo ratchet` inside `tools/trunc` in a composition.
- **Intent:** "Run trunc's tests exactly as trunc's own CI would."
- **Ideal outcome:** Build sees only what trunc's standalone checkout would see (no accidental use of `../../crates/help-test` path deps), matching `standalone_publishability`.
- **Can go wrong:** Composition makes a sibling directory visible and a path dependency silently resolves locally but not in trunc CI.
- **Requirement exposed:** A component built inside a composition can be made to behave exactly like its standalone checkout, and deviations are detectable.

### B8. Run tests of the composition as a whole
- **Intent:** "Does trunc's unreleased change break dotsync, which uses help-test the same way?"
- **Ideal outcome:** Composition-level test run against in-progress revisions of several components.
- **Requirement exposed:** A composition can be built and tested at unpublished revisions of several components together.

### B9. Undo a mistaken operation
- **Actors:** Agent ran a bulk search-and-replace over every tool's AGENTS.md, then committed.
- **Intent (Max):** "Undo that, all of it."
- **Ideal outcome:** One undo reverts the operation in every component, including the working copy; pushes already performed are reported as not undone locally-only.
- **Requirement exposed:** Undo operates at the granularity of the user's operation, spanning components.

### B10. Resume after a week away
- **Intent:** "Pull everything and tell me what others did."
- **Ideal outcome:** Fetch all components; summary of new upstream commits per component, including bot ledger commits separated from human work; local work rebased or flagged.
- **Requirement exposed:** Catching up a composition is one step and distinguishes automated commits from human ones.

---

## C. Repeated occurrences of one component

### C1. Same tool at two revisions for comparison
- **Actors:** Max.
- **Start:** trunc `main: …─v0.4.5─x─y`.
- **Intent:** "Mount trunc at v0.4.5 as `trunc-old` and trunc main as `trunc`, to diff their help output."
- **Ideal outcome:** Both present; editing `trunc` never alters `trunc-old`; a diff between the two mounts is available as an ordinary diff.
- **Requirement exposed:** A composition may contain one repo more than once, each occurrence independently versioned.

### C2. Edit an old occurrence
- **Start:** As C1.
- **Intent:** "Backport this fix into `trunc-old` to cut v0.4.6."
- **Ideal outcome:** Commit on top of v0.4.5 in trunc (a release branch), not on main; the main occurrence untouched.
- **Can go wrong:** Commit id collision/confusion: the same change in two occurrences.
- **Requirement exposed:** Commits made in different occurrences of one repo land on independent lines of that repo's history.

### C3. Same edit applied to both occurrences
- **Intent:** "Fix the typo in both `trunc` and `trunc-old` as one logical change."
- **Ideal outcome:** [OWNER-DECISION] Either (a) one logical change that produces two trunc commits on two lines, tracked as related (like a cherry-pick pair), or (b) the product refuses and asks the user to apply then port. Also: does the logical change show once or twice in trunc's log?
- **Requirement exposed:** A logical change can touch several occurrences of one repo and the relationship between the resulting commits is recorded.

### C4. Vendored copy as a second occurrence
- **Start:** help-test used by trunc via git tag v0.1.0; also present at `libraries/help-test` at main.
- **Intent:** "Which help-test does trunc actually build against here?"
- **Ideal outcome:** Composition shows both occurrences and that trunc's Cargo.toml resolves to the tag, not the mount; mismatch between build-dependency revision and mounted revision is visible.
- **Requirement exposed:** When a component's own build metadata names a revision of another component, the composition can show whether that agrees with what it mounts.

### C5. Two occurrences converge
- **Intent:** "`trunc-old` is done; fold it away."
- **Ideal outcome:** Removing one occurrence leaves the other and history untouched; commits made in the removed occurrence remain reachable in trunc's repo (on their branch).
- **Requirement exposed:** Removing an occurrence never loses commits made through it.

### C6. Same repo under different mount names in different compositions
- **Start:** tmux-bridge is `tools/tb` in whole, `tb` at root in security view, `vendor/tmux-bridge` in Kim's project.
- **Intent:** "A commit made in any of these is the same tb commit."
- **Ideal outcome:** Moving a change between compositions never re-creates it; it has the same identity everywhere.
- **Requirement exposed:** A component commit's identity is independent of where any composition mounts it.

### C7. Repeated component inside a nested composition
- **Start:** Composition "whole" includes composition "backend" which includes tdd-ratchet; "whole" also includes tdd-ratchet directly (because the umbrella uses it as a CI tool).
- **Intent:** "Bump tdd-ratchet once."
- **Ideal outcome:** [OWNER-DECISION] (a) both occurrences follow one declared revision (deduplicated), or (b) they are independent and the user sees a warning when they diverge, or (c) nesting is forbidden.
- **Requirement exposed:** Behaviour when nested compositions reach the same component is defined and visible.

---

## D. History editing

### D1. Amend a cross-tool change
- **Actors:** Max.
- **Start:** Logical change L "Alias AGENTS.md as CLAUDE.md" touches all 7 repos, not yet pushed.
- **Intent:** "I forgot help-test; add the alias there and fold it into the same change."
- **Ideal outcome:** L now spans 8 repos; still one logical change with one description; no "fixup" commits in any repo.
- **Requirement exposed:** A logical change can be amended to add components as easily as editing a file.

### D2. Edit a change in the middle of a stack
- **Start:** Unpublished stack L1 (help-test API) ← L2 (trunc adopts it) ← L3 (dotsync adopts it).
- **Intent:** "Rename the API function in L1; L2 and L3 should follow."
- **Ideal action:** Check out L1, edit, move on; descendants rebase automatically.
- **Ideal outcome:** L2/L3 rebased in trunc and dotsync; where they used the old name, a recorded conflict or a clean build failure that points at the parts to fix. (The partially published variant is X3.)
- **Requirement exposed:** Editing an earlier change in a multi-component stack automatically carries every later change along, in every component.

### D3. Split a cross-tool change
- **Start:** L touches trunc (help text) and dotsync (config parsing), unpublished.
- **Intent:** "These are unrelated; make them two changes."
- **Ideal outcome:** Two logical changes, each with its own description; each repo's resulting commits correspond.
- **Requirement exposed:** A logical change can be split by component and by hunk within a component.

### D4. Squash three logical changes
- **Intent:** "Red, green, refactor of the auto-update adoption across four tools: combine the refactor into green, keep red separate (ratchet needs red)."
- **Ideal outcome:** Two logical changes remain, per-repo histories show red then green in each repo.
- **Requirement exposed:** Squashing respects that some components require particular commit sequences (see tdd episodes).

### D5. Reorder
- **Intent:** "Move the help-test change before the trunc change that depends on it."
- **Ideal outcome:** Reorder at logical level; each repo's history reordered consistently; conflicts reported in the component where they occur.
- **Requirement exposed:** Reordering logical changes produces a consistent order in every affected repo.

### D6. Abandon
- **Intent:** "Drop the deconfuse-rs adoption entirely."
- **Ideal outcome:** Removed from all components; recoverable via undo/op log; published parts flagged (cannot be unpublished silently).
- **Requirement exposed:** Abandoning a logical change removes it everywhere it is unpublished and reports where it is published.

### D7. Rebase a stack onto moved upstreams
- **Start:** Stack L1─L2─L3 across trunc, tb, dotsync; meanwhile trunc main and tb main moved (tb was even rewritten, E-series).
- **Intent:** "Rebase my stack onto everything's latest main."
- **Ideal outcome:** Each component rebased; conflicts stay open in the working copy per JJ-style (first-class conflicts) rather than blocking; stack stays one stack.
- **Requirement exposed:** A multi-component stack can be rebased onto new upstreams of each component in one operation, with conflicts recorded rather than aborting.

### D8. Cherry-pick from a component into another occurrence or composition
- **Start:** Kim's composition; Max made trunc fix `c` in his.
- **Intent (Kim):** "I want just Max's trunc fix."
- **Ideal outcome:** Kim picks trunc commit `c`; if `c` was part of a larger logical change L, she can take only the trunc part and the relationship is kept.
- **Requirement exposed:** Part of a cross-component change can be taken independently, and the product records that it is a partial copy.

### D9. Transplant a change between components
- **Intent:** "I wrote a config-parsing helper in dotsync; move it into deconfuse-rs, keeping its history."
- **Ideal outcome:** Change now lives in deconfuse-rs with its authorship; dotsync gets the corresponding deletion; one logical event.
- **Requirement exposed:** Code and its history can move between components as one change.

### D10. Rewrite a published tool's history (the real tb repair)
- **Actors:** Max with explicit approval.
- **Start:** tb `main: …─485a9f1`; ledger history invalid in some old snapshot; umbrella pins tb at 485a9f1; tags v0.1.x point into the old line; Releases name old commits; statuses (`integrated-ci`) on old SHAs.
- **Intent:** "Replace this history so the ratchet accepts it, keep every tree and message, move tags, keep backup."
- **Ideal action:** Edit the specific ledger blob in the old commit; product rewrites descendants.
- **Ideal outcome:** tb main 5458ab8 with identical trees after the edit point; backup ref pushed; tags moved; umbrella and every composition that pinned old SHAs are told which new commit corresponds; attestations in `state.json` keyed by old SHAs mapped or flagged.
- **Can go wrong:** GitHub Release targets refuse update (404); signatures lost; statuses lost; outside forks keep old history.
- **Requirement exposed:** After a component history rewrite, every composition, pin, tag and attestation referencing old commits can find the corresponding new commit, and anything that could not be carried over (signatures, statuses, Releases) is reported.

### D11. Edit a very old commit in one component in the middle of a composition's history
- **Intent:** "Fix a secret accidentally committed to dotsync 40 commits ago."
- **Ideal outcome:** dotsync rewritten; composition history that referenced affected dotsync commits either rewritten consistently or annotated; others warned.
- **Requirement exposed:** Rewriting a component's deep history has a defined, visible effect on every composition history that referenced it.

---

## E. Branches, concurrency and merges

### E1. Two agents, same component, different compositions
- **Actors:** Agent A in `at-trunc-numeric` view, agent B in `at-trunc-docs` view.
- **Start:** Both touch trunc from main `a`.
- **Intent:** Each: "Do my task without stepping on the other."
- **Ideal outcome:** Two branches in trunc; neither sees the other's uncommitted edits; merging later is an ordinary merge in trunc.
- **Requirement exposed:** Concurrent sessions on the same component never corrupt or observe each other's in-progress state unless they choose to.

### E2. Both branches bump the same pin (the real gitlink conflict)
- **Start:** umbrella feature branch pins dotsync at `d5`; umbrella main pinned dotsync `d7`; `d5` and `d7` diverged.
- **Intent:** "Merge main into my branch."
- **Ideal outcome:** Product never silently picks a side; it offers the component-level merge of `d5` and `d7` and the composition references the result. (Mirror of AGENTS.md rule.)
- **Requirement exposed:** When two lines of a composition disagree about a component, merging them merges the component's lines of work, never discards one.

### E3. Merge where only one side touched a component
- **Intent:** "Merge my branch; I only changed tb."
- **Ideal outcome:** Other components take main's revisions; no conflict.
- **Requirement exposed:** Merging compositions is per-component three-way.

### E4. Stacked PRs across components
- **Start:** PR stack in trunc: P#20 → P#21 based on P#20's branch. CI merges P#20 with `--delete-branch`; GitHub closes P#21.
- **Intent:** "Keep my stack alive when the bottom lands."
- **Ideal outcome:** Upper change retargets automatically; nothing is closed that shouldn't be.
- **Requirement exposed:** Landing the bottom of a stack never destroys the stack above it, including stacks whose layers span components.

### E5. Concurrent cross-cutting changes touching the same files
- **Start:** Change L1 "CLAUDE.md alias" and L2 "rewrite AGENTS.md workspace routing section" both touch every tool's AGENTS.md.
- **Intent:** "Land both."
- **Ideal outcome:** Conflicts appear per component, in the logical change that introduced them; resolving once for all repos is possible when the conflict is identical in each.
- **Requirement exposed:** Identical conflicts repeated across components can be resolved once.

### E6. Merge commit vs linear
- **Intent (Max policy):** "Prefer merge commits."
- **Ideal outcome:** Composition-level merges produce merge commits in each affected component where needed and history is readable in plain Git.
- **Requirement exposed:** The product can land work in components using merge commits and does not require linear history.

### E7. Long-lived branch across components
- **Intent:** "dotsync next-gen rewrite lives on a branch for months while tools keep releasing."
- **Ideal outcome:** Branch in dotsync plus maybe deconfuse-rs; composition "next" follows the branch; periodic merges of main in.
- **Requirement exposed:** A composition can track different branches in different components.

### E8. Lock-free concurrent umbrella updates
- **Start:** Three tool PRs merge within an hour; each wants a pin bump PR in umbrella; today three "Point X at vY" PRs each rebase/merge main.
- **Intent:** "I don't want a PR per bump."
- **Ideal outcome:** Composition reflects all three new versions with zero or one human action, and history shows which moment adopted which version.
- **Requirement exposed:** Adopting new component revisions into a composition does not need a separate human ritual per component.

### E9. Conflict inside one component during a multi-component rebase, deferred
- **Intent:** "Rebase everything; I'll fix the dotsync conflict tomorrow."
- **Ideal outcome:** Other components completed; dotsync holds a recorded conflict; composition still builds except where conflicted; tomorrow's resolution propagates to descendant changes.
- **Requirement exposed:** A conflict in one component does not block progress in others.

---

## F. Topology changes

### F1. Extract a library from the umbrella (the real help-test move)
- **Start:** umbrella `crates/help-test`; trunc Cargo.toml `help-test = { path = "../../crates/help-test" }` (fails `standalone_publishability`).
- **Intent:** "Make help-test its own repo, keep its history, point tools at it by tag."
- **Ideal action:** Declare `crates/help-test` a component; publish as `maxeonyx/help-test`.
- **Ideal outcome:** help-test repo contains only its own history (commits that touched it, rewritten to its root); umbrella no longer contains the files but still shows their past; trunc builds standalone.
- **Requirement exposed:** A directory's history can become a standalone repo's history, and the parent keeps a traceable link from old to new.

### F2. Inline a component back into another
- **Intent:** "help-test is too small; fold it into tdd-ratchet."
- **Ideal outcome:** tdd-ratchet gains the files with history reachable; help-test repo archived; compositions that referenced help-test are told.
- **Requirement exposed:** Merging one component into another preserves both histories and updates compositions that referenced the absorbed one.

### F3. Split one tool into two
- **Intent:** "agent-harness → agent-harness + agent-harness-runner."
- **Ideal outcome:** Two repos, each history containing only its files; releases/tags stay on the original; CI of both independent.
- **Requirement exposed:** Splitting a component yields two ordinary repos whose histories a plain-Git user would find sensible.

### F4. Move a component to a new mount path
- **Intent:** "Move tools/tb to tools/tmux-bridge to match the repo name."
- **Ideal outcome:** Composition history shows a move, not delete+add; component history unaffected; blame through the composition follows.
- **Requirement exposed:** Changing where a component is mounted is a history-preserving move and does not touch the component itself.

### F5. Rename a component repo on GitHub
- **Start:** `maxeonyx/tdd-ratchet-rs` renamed to `maxeonyx/tdd-ratchet`.
- **Intent:** "Everything keeps working."
- **Ideal outcome:** Compositions follow the redirect and record the new canonical URL; old compositions at old revisions still materialize.
- **Requirement exposed:** A component's identity survives renames and URL changes of its hosting.

### F6. Archive a tool (the real oc episode)
- **Start:** umbrella commit 238482f "Move oc to archived tool history"; later oc returns.
- **Intent:** "Stop maintaining oc, but keep it in history; later, revive it."
- **Ideal outcome:** Composition drops oc from current state; past states still materialize with oc; revival restores it with history continuous.
- **Requirement exposed:** Components can leave and re-enter a composition without losing the composition's historical reproducibility.

### F7. Fork a tool
- **Intent (Kim):** "I need my own trunc with a patch Max rejected."
- **Ideal outcome:** Kim's composition swaps `maxeonyx/trunc` for `kim/trunc`; her other changes still align; she can still pull Max's updates.
- **Requirement exposed:** Swapping a component for a fork is a one-line change to a composition and keeps shared history identity.

### F8. Vendor a third-party crate as a component
- **Intent:** "Vendor a patched `tmux-interface` crate into tb."
- **Ideal outcome:** [OWNER-DECISION] Vendored code is either (a) a component with its own upstream (so fixes can go upstream), or (b) plain files in tb (tb's standalone clone must contain it). tb's standalone build must still work, so (a) requires tb's own repo to contain the bytes.
- **Requirement exposed:** A component that must stay standalone can contain code that is also tracked as a separate component, and its standalone clone is complete.

### F9. Generated content
- **Start:** umbrella `docs/version.json` is generated from each tool's `docs/version.json`; `.test-status.json` is bot-generated; Cargo.lock is tool-generated.
- **Intent:** "When a tool version changes, the composed version.json should update."
- **Ideal outcome:** Generated files are regenerated deterministically when inputs change; conflicts in generated files are resolved by regeneration, not hand-merge.
- **Requirement exposed:** Files derived from components' content can be kept consistent with the composition automatically, and are distinguishable from hand-written ones.

### F10. Cross-component file move
- **Intent:** "Move `review_attest` records from `tools/*/docs/reviews/*.json` into umbrella `state.json`" (real TODO).
- **Ideal outcome:** One logical change: deletes in six tool repos (and their Pages sites stop publishing the JSON), addition in umbrella.
- **Requirement exposed:** Moving content between a component and a composition-owned area is one atomic change.

---

## G. Transport

### G1. Publish a cross-cutting change to seven remotes
- **Actors:** Max.
- **Start:** Logical change L touches trunc, tb, dotsync, tdd-ratchet, oc, agent-harness, help-test.
- **Intent:** "Push it."
- **Ideal outcome:** Seven branches (or PRs) created; a single status shows all seven; each repo's branch contains only that repo's commit.
- **Requirement exposed:** Publishing a multi-component change is one command with per-remote outcomes.

### G2. Partial push failure
- **Start:** As G1, but tmux-bridge push rejected (branch protection) and oc push times out.
- **Intent:** "Retry the failed ones."
- **Ideal outcome:** Five pushed; two recorded as pending; `retry` pushes exactly those; no duplicate commits on the five. The logical change's state is "partially published", visible to Max and to any collaborator who fetches it.
- **Requirement exposed:** Partial publication is a first-class, visible, resumable state; retrying is idempotent.

### G3. Offline work
- **Intent:** "On a plane: work across tools, commit, stack changes."
- **Ideal outcome:** Everything local works; nothing requires network except publish/fetch.
- **Requirement exposed:** All authoring operations work offline for every component.

### G4. Plain clone then adopt
- **Start:** Contributor cloned `maxeonyx/dotsync` with plain git, made commits.
- **Intent:** "Now I want to see how my dotsync change affects the whole suite."
- **Ideal outcome:** They open the composition with their clone as the dotsync component; their commits visible; nothing rewritten.
- **Requirement exposed:** Work started in a plain clone of a component enters a composition without being rewritten.

### G5. Force-push by someone else
- **Start:** Max composed tb at `485a9f1`; tb main force-pushed to `5458ab8` (repair).
- **Intent:** "Fetch."
- **Ideal outcome:** Product notices the rewrite, shows correspondence (same trees), and offers to move local work onto the new line; local commits based on old line are never lost.
- **Requirement exposed:** An upstream force-push to a component is detected, explained, and never silently drops local work.

### G6. Different remotes per component
- **Start:** trunc pushes to GitHub; oc only to a private Gitea; help-test mirrored to Codeberg too.
- **Intent:** "Publish L to wherever each goes."
- **Ideal outcome:** Each component's remote(s) respected; multi-remote components published to all configured.
- **Requirement exposed:** Each component can have its own set of remotes and credentials.

### G7. Fetch only what's needed
- **Intent (agent on CI):** "Get the composition state at L without full history of every repo."
- **Ideal outcome:** Shallow/partial per component; later deepen on demand.
- **Requirement exposed:** A composition can be materialized with limited history per component and deepened later.

### G8. Publishing composition state vs component state
- **Intent:** "Push the composition so Kim can see how the pieces fit, but trunc changes aren't ready to push to trunc."
- **Ideal outcome:** [OWNER-DECISION] (a) composition can be published only when all component content is published to component remotes; (b) composition can carry unpublished component content somewhere (e.g. umbrella remote holds trunc objects); (c) composition publish implies component branch pushes. Tradeoff: no-privileged-repo vs. convenience vs. leaking unpublished trunc code via umbrella.
- **Requirement exposed:** The product defines where unpublished component work lives when a composition is shared.

### G9. Dangling reference after remote deletion
- **Start:** Composition refers to a trunc branch that got deleted after merge (`--delete-branch`), commit only reachable via squash result.
- **Intent:** "Materialize last month's composition."
- **Ideal outcome:** Works, or clearly reports which commit is gone and what replaced it.
- **Requirement exposed:** Historical compositions remain materializable after branches are deleted, or say exactly why not.

---

## H. Plain-Git users and the GitHub UI

### H1. Outside contributor PR to one tool
- **Actors:** Contributor who knows only Git.
- **Start:** Forks `maxeonyx/trunc`, opens P#31 via web.
- **Intent (Max):** "Review and merge it, and see it in my composition."
- **Ideal outcome:** Contributor needs nothing but Git; merged commit appears in compositions on next fetch; nothing in trunc's tree mentions the product.
- **Requirement exposed:** Contributors who never install the product can contribute to any component normally.

### H2. Squash-merge on GitHub
- **Start:** Max's logical change L produced trunc commits `c1─c2` on branch; merged via GitHub squash → `s` on main.
- **Intent:** "My composition should understand L landed in trunc."
- **Ideal outcome:** L is recognised as landed in trunc (by content/trailer/PR link), local `c1─c2` hidden, no duplicate change.
- **Requirement exposed:** A change landed via squash-merge is recognised as the same logical change.

### H3. Rebase-merge on GitHub
- **Start:** Same but "Rebase and merge": new SHAs, same content per commit.
- **Ideal outcome:** Same as H2; stable change identity survives the SHA change.
- **Requirement exposed:** Change identity survives server-side rebase.

### H4. Web edit on GitHub
- **Start:** Max edits trunc README on github.com; commit `w` on main.
- **Intent:** "Pull; my local stack should go on top."
- **Ideal outcome:** Ordinary; `w` shows as an upstream commit.
- **Requirement exposed:** Commits authored in the GitHub UI are indistinguishable in handling from any other upstream commit.

### H5. GitHub "Update branch" button
- **Start:** Product-published branch for L in dotsync; reviewer clicks "Update branch" (merge main into branch) on GitHub.
- **Intent (Max):** "Keep working on L locally."
- **Ideal outcome:** Local L picks up the server-side merge; no divergence war; no force-push needed.
- **Requirement exposed:** Server-side modifications to a published branch are absorbed, not overwritten.

### H6. Plain Git user checks out a composition
- **Actors:** A plain Git user given the umbrella URL.
- **Intent:** "`git clone --recursive agent-tools` and build."
- **Ideal outcome:** [OWNER-DECISION] Whether plain `git clone` of a composition must yield a working tree (keeps submodules or equivalent in the published form) or whether compositions require the product to materialize. Tradeoff: ordinary-Git accessibility vs. freedom from pointer mechanics.
- **Requirement exposed:** The product states what a plain-Git user sees when cloning a composition's published form.

### H7. GitHub source archive / Release tarball
- **Intent:** "Download the tarball of the umbrella at v1.2 and build."
- **Ideal outcome:** Today submodules are absent from GitHub archives. Product: either composition releases carry a complete archive, or the gap is documented.
- **Requirement exposed:** A released composition has a complete downloadable source form, or explicitly does not.

### H8. Mirrors
- **Intent:** "Mirror everything to Codeberg in case GitHub dies."
- **Ideal outcome:** Mirrors of components are plain Git mirrors; compositions can be switched to materialize from mirrors.
- **Requirement exposed:** Compositions don't hard-bind component identity to one host URL.

### H9. Fork of the umbrella by an outsider
- **Intent (outsider):** "Fork agent-tools to add my tool."
- **Ideal outcome:** Their fork composes Max's components plus theirs; they can PR the composition change back.
- **Requirement exposed:** A composition can be forked and proposed back like a repo.

### H10. Plain-Git user edits a file a composition-level change also edits
- **Start:** Max's L modifies tb/AGENTS.md; contributor's P#12 to tb also modifies it and merges first.
- **Ideal outcome:** L's tb part rebases/conflicts normally; other parts unaffected.
- **Requirement exposed:** Upstream changes in one component interact with a multi-component change only in that component.

---

## I. Hosting, collaboration, review, permissions

### I1. Review a cross-cutting change
- **Actors:** Max reviewing an agent's L across 6 tools.
- **Intent:** "One review page for the whole change."
- **Ideal outcome:** One diff/review view; per-repo PRs (needed for each repo's CI/merge) linked and kept in sync; approval once, or one per repo if repos require it.
- **Requirement exposed:** A multi-component change can be reviewed as a unit while each repo keeps its own required review.

### I2. Mixed permissions
- **Start:** Contributor has write on trunc, not on tb.
- **Intent:** "Make the cross-cutting change; I'll push what I can."
- **Ideal outcome:** trunc branch pushed; tb part published to contributor's fork and PR'd; composition shows both halves as parts of one change.
- **Requirement exposed:** A logical change can span repos where the author has different permissions, publishing each part through whatever route that repo allows.

### I3. Private component in a public composition
- **Start:** Suppose agent-harness is private; umbrella public.
- **Intent:** "Public viewers see the rest; I see all."
- **Ideal outcome:** Public materialization omits agent-harness gracefully (with a marker), never leaks its content, file names, or commit messages through composition history.
- **Requirement exposed:** A composition's published form never reveals content of components the viewer cannot read.

### I4. Logical change descriptions leak across visibility
- **Start:** L touches public trunc and private agent-harness; description mentions a private customer.
- **Ideal outcome:** [OWNER-DECISION] Does the trunc commit carry the full logical description, a per-repo description, or a link? Tradeoff: context for public readers vs. confidentiality.
- **Requirement exposed:** What part of a multi-component change's metadata is copied into each component is controllable per component.

### I5. Branch protection per component
- **Start:** tdd-ratchet main: no force-push, required `Ready` check; umbrella main: direct pushes allowed.
- **Intent:** "Land L everywhere."
- **Ideal outcome:** Product respects each repo's rules; where a direct push is forbidden it goes via PR.
- **Requirement exposed:** Each component's hosting rules are honoured individually by a single composed publish.

### I6. Code owners / reviewers
- **Intent:** "The CI workflow change in all tools needs security reviewer approval in each repo."
- **Ideal outcome:** Composition review surfaces each repo's required reviewers.
- **Requirement exposed:** Composed review shows every component's review obligations.

### I7. Collaborator works from a different composition
- **Start:** Max in "whole"; collaborator in "backend" view. Both work on L.
- **Intent:** "Collaborate on the same logical change from different views."
- **Ideal outcome:** Collaborator fetches L; the parts inside their view are editable, parts outside visible read-only or fetchable.
- **Requirement exposed:** A logical change is shareable between people using different compositions.

### I8. Agent with scoped credentials
- **Actors:** Agent with a token that can push only branches `agent/*` in 3 repos.
- **Ideal outcome:** Product publishes under allowed names; never needs broader scope for composition bookkeeping.
- **Requirement exposed:** Composition bookkeeping never requires more permissions than the component pushes themselves.

---

## J. CI, dev contexts, releases, versions, provenance

### J1. Tool CI stays tool-local
- **Start:** trunc `ci.yml` checks out only trunc.
- **Ideal outcome:** Nothing the product does requires tool CI to know about compositions.
- **Requirement exposed:** Component CI keeps working unchanged with no product installed.

### J2. Composed CI
- **Intent:** "Run the concern suite against the composition at L."
- **Ideal outcome:** CI job materializes the composition at L (unpublished parts included if authorised) and reports per-component results.
- **Requirement exposed:** CI can test a composition at a pending logical change before any part lands.

### J3. Release a tool as today
- **Start:** trunc Cargo.toml 0.4.13; CI releases v0.4.13 on merge, deploys Pages, posts `integrated-ci` on merge SHA.
- **Ideal outcome:** Unchanged, and the composition learns that trunc v0.4.13 exists and can adopt it automatically.
- **Requirement exposed:** Component releases happen entirely in the component, and compositions can follow them without hand edits.

### J4. Version a composition
- **Intent:** "What is agent-tools v1.2?"
- **Ideal outcome:** A composition version names exact component revisions; `docs/version.json` derivable from it.
- **Requirement exposed:** A composition revision has a stable identifier that determines every component revision.

### J5. Provenance of a binary
- **Intent (auditor):** "The dotsync binary on dotsync.maxeonyx.com — what source, which composition?"
- **Ideal outcome:** Binary traces to a dotsync commit; the logical change(s) it contains are discoverable, including cross-repo siblings.
- **Requirement exposed:** From any component commit, one can find the logical changes and sibling commits it belongs to.

### J6. Release freshness across a composition
- **Start:** `release_freshness` requires a Release tag pointing at the pinned commit.
- **Intent:** "Show me every component where the composition is ahead of the last release."
- **Requirement exposed:** For every component, "composed revision vs. latest release vs. main" is queryable.

### J7. Dev environment per component
- **Start:** each repo has devenv.nix; `cargo-ratchet` shim differs between umbrella and tools.
- **Intent:** "Run each component's tests inside its own devenv, from the composition."
- **Requirement exposed:** A composition can run a component's commands in the environment that component defines.

### J8. Pinned tool versions used by CI
- **Start:** umbrella "Pin ratchet 1.1.6 for trusted umbrella runs" — umbrella CI uses tdd-ratchet as a tool at a released version, independent of the tdd-ratchet mount.
- **Intent:** "Using tdd-ratchet as a dependency and developing it are different."
- **Requirement exposed:** A composition can distinguish a component it develops from a component it consumes as a pinned tool, even when they are the same repo.

### J9. Reproduce an old CI failure
- **Intent:** "Recreate exactly what CI saw in run #1234 for the umbrella."
- **Ideal outcome:** Composition state at that run materializes byte-identically.
- **Requirement exposed:** Any composition state CI ran on can be reproduced exactly later.

### J10. Pages sites
- **Start:** each tool's `docs/` deploys to `<tool>.maxeonyx.com`; umbrella site links to each.
- **Intent:** "Change the shared site theme across all Pages as one change and preview the whole family."
- **Requirement exposed:** A composed preview of per-component published artifacts can be built from an unlanded cross-cutting change.

---

## K. Client interaction moments (human and agent)

### K1. Agent learns state in one call
- **Actors:** LLM agent starting a task.
- **Ideal outcome:** One command emits machine-readable state: components, revisions, dirty files, current change, stack, unpublished parts, conflicts.
- **Requirement exposed:** Complete composition state is available as structured output in one call.

### K2. Agent writes a small program against the product
- **Intent:** "For each tool, if Cargo.toml lacks deconfuse, add it and commit as part of change L."
- **Ideal outcome:** Scriptable, idempotent operations; rerunning the script doesn't duplicate.
- **Requirement exposed:** Operations are scriptable and idempotent enough for generated programs to re-run safely.

### K3. Agent recovers from its own error
- **Intent:** "I ran the wrong rebase; restore to before."
- **Ideal outcome:** Op log shows the agent's operations with ids; `undo <op>` works across components.
- **Requirement exposed:** Every mutating operation is recorded and reversible by id.

### K4. Human interrupts an agent
- **Intent (Max):** "Stop; show me what you've changed since I last looked."
- **Requirement exposed:** A diff "since a named moment" is available across the composition.

### K5. Error messages for a partially-failed operation
- **Ideal outcome:** Per AGENTS.md error standards: context, facts, causal chain, next action, per component.
- **Requirement exposed:** Multi-component failures report per-component cause and a concrete next step.

### K6. Dry run
- **Intent (agent):** "What would publishing L do?"
- **Requirement exposed:** Every remote-affecting operation can be previewed exactly.

### K7. Stable ids for referencing
- **Intent:** Agent tells Max "see change `kqzvx`"; Max opens it a day later after rebases.
- **Requirement exposed:** Change ids stay valid across rewrites, rebases and composition changes.

### K8. Concise human UX for the 90% case
- **Intent:** "I just want to edit and push like before."
- **Requirement exposed:** A single-repo edit costs no more commands than plain Git or jj.

### K9. Discoverability of the composition layer
- **Intent (new contributor):** "Why does my trunc checkout contain nothing about agent-tools?"
- **Requirement exposed:** Components carry no mandatory product metadata; compositions are discoverable from the composition side.

---

## X. Combinations and intersections

These are the episodes most likely to break a model that handles each dimension
separately.

### X1. Cross-cutting change meets per-repo ratchet bot commits
- **Actors:** Agent, ledger bots in 5 repos.
- **Start:** L = "adopt auto-update integration" across trunc, tb, dotsync, tdd-ratchet, agent-harness. Each repo needs red commit (new test + deliberately broken impl) → ledger dispatch → `[bot] pending` → green commit → dispatch → `[bot] passing`. In each repo: `main: m─L.red─[bot]─L.green─[bot]`.
- **Intent:** "This is one logical change; I shouldn't have to babysit 10 bot runs by hand, and the bot commits shouldn't break my change's identity."
- **Ideal action:** Publish L.red everywhere, wait, publish L.green everywhere.
- **Ideal outcome:** Each repo history is exactly what the ratchet requires; bot commits interleave but are not "part of" L nor treated as foreign changes that force a rebase of L; local view of L continues to work after bots move each branch head.
- **Can go wrong:** Local L.green is based on pre-bot head, so push is non-fast-forward; product "helpfully" rebases L.green over the bot commit, changing SHAs; or it force-pushes and deletes the bot commit.
- **Requirement exposed:** Automated commits added by a component's CI onto a published part of a logical change are absorbed without the author re-doing or force-pushing anything.

### X2. Cross-cutting change requires a sequence of intermediate states in each repo
- **Start:** As X1. The red state must be observed by the bot in each repo before green is pushed.
- **Intent:** "Keep red and green as separate changes, stacked, everywhere."
- **Ideal outcome:** Stack L.red ← L.green spans five repos; squashing them (D4) is refused or warned for ratchet repos.
- **Requirement exposed:** A component can impose history-shape constraints (e.g. failing-first) that a cross-component operation respects and can check before publishing.

### X3. Amend a partially published cross-cutting change
- **Start:** L pushed to trunc, tb, dotsync; push to oc and agent-harness failed (G2). Now Max amends L's tb part.
- **Intent:** "Fix a bug in L, then finish publishing."
- **Ideal outcome:** trunc and dotsync unchanged remotely; tb branch updated (new commit on top if policy is no-force-push, or replaced if allowed); oc/agent-harness get the amended version on retry. L remains one change, with a record of which version is on which remote.
- **Requirement exposed:** A logical change can be amended while partially published, and each remote converges to the latest version by the least disruptive update that remote allows.

### X4. Partial landing: some repos merged, one rejected
- **Start:** L published as 4 PRs. trunc, tb, dotsync merged. tdd-ratchet maintainer (outsider) rejects.
- **Intent:** "What is the state of L? Can the composition use it?"
- **Ideal outcome:** L shown as 3/4 landed; composition at main has the 3 parts; the 4th remains a local/pending change; a composition configuration that needs all four is flagged as inconsistent.
- **Requirement exposed:** A logical change can be partially landed, and that state is visible and survivable indefinitely.

### X5. Atomic intent vs. non-atomic hosting [OWNER-DECISION]
- **Start:** L changes a help-test API and all callers; landing help-test first breaks trunc main until trunc's part lands.
- **Intent:** "Land atomically."
- **Ideal outcome:** Alternatives: (a) product orders landing (help-test, then consumers) and accepts a window of inconsistency in each repo's main; (b) product requires backwards-compatible steps (expand/contract) and refuses breaking cross-repo changes; (c) atomicity exists only in compositions (composition main advances once all parts landed), component mains are allowed to be temporarily inconsistent. GitHub cannot atomically merge across repos, so true atomicity is impossible on ordinary hosting.
- **Requirement exposed:** The product defines precisely what "atomic" means for a multi-repo change on hosting that cannot merge atomically.

### X6. Repeated component + history rewrite
- **Start:** Composition contains `tb` (main) and `tb-release` (v0.1.25). tb history is rewritten (D10): v0.1.25 tag re-pointed to new SHA with same tree.
- **Intent:** "Both occurrences should follow."
- **Ideal outcome:** `tb` follows new main; `tb-release` maps to the re-pointed tag, same tree, no diff shown in composition; if tag not re-pointed (like the older Releases that refused), the old SHA is still materializable from backup ref or the gap is reported.
- **Requirement exposed:** A rewrite is applied consistently to every occurrence of the component, and tree-identical rewrites show as no content change.

### X7. Repeated component + cross-cutting change
- **Start:** `trunc` and `trunc-old` occurrences; L "CLAUDE.md alias everywhere".
- **Intent:** "Everywhere means both."
- **Ideal outcome:** Two trunc commits on two lines; PRs: one to main, one to release branch; L shows 8 parts for 7 repos.
- **Requirement exposed:** A logical change's parts are per occurrence, not per repo.

### X8. Permissions + composed CI
- **Start:** Composed CI for L needs to materialize private agent-harness and unpublished parts on outsiders' forks; the umbrella's CI token cannot read Kim's fork.
- **Intent:** "CI for the whole change."
- **Ideal outcome:** CI either obtains exactly the parts it may read or reports "cannot evaluate L: tb part lives on kim/tb which this runner cannot read"; never tests a substituted revision silently.
- **Requirement exposed:** Composed CI fails loudly rather than silently substituting a revision it could read for one it could not.

### X9. Trusted-workflow boundary + cross-repo change
- **Start:** `ledger.yml` refuses dispatch from non-main refs so "a branch must not validate its own ledger with its own rules". L modifies `ledger.yml` itself in all tools and the umbrella.
- **Intent:** "Change the ledger workflow everywhere."
- **Ideal outcome:** Each repo's trusted workflow runs the *old* rules on the change; new rules take effect after merge; composed CI does not let L's version of the rules judge L.
- **Requirement exposed:** Composed CI preserves each component's rule that a change cannot be judged by the rules it introduces.

### X10. Bot commit + head-SHA-keyed status + composition pin
- **Start:** dotsync PR head `h1`; `Ready` recorded on `h1`; ledger bot pushes `h2`; composition pinned dotsync at `h1`.
- **Intent:** "Composition should adopt what actually merged."
- **Ideal outcome:** Composition learns `h2` (then merge commit `M`) is the landed version of its `h1`; statuses read against the correct SHA; no stuck auto-merge.
- **Requirement exposed:** A composition's reference to a component follows the component's automated successor commits to the landed commit.

### X11. Squash-merge + stacked multi-repo change
- **Start:** Stack L1 ← L2 across trunc and tb; trunc part of L1 squash-merged by GitHub.
- **Intent:** "Restack L2."
- **Ideal outcome:** L2's trunc part rebased onto squashed `s`, dropping L1's now-duplicate commits; L2's tb part untouched; stack intact.
- **Requirement exposed:** Restacking after a squash-merge in one component works without disturbing other components.

### X12. Rewrite + attestations keyed by SHA
- **Start:** `state.json` has `reviewed_commit: 06d8fc9...` for tdd-ratchet concerns. tdd-ratchet history rewritten; 06d8fc9 no longer on main.
- **Intent:** "Are those reviews still valid?"
- **Ideal outcome:** Product maps 06d8fc9 to its tree-identical successor and says the attestation applies to the same content, or flags it.
- **Requirement exposed:** Anything in a composition that refers to a component commit (attestation, pin, CI record) can be re-resolved after a rewrite.

### X13. Offline + partial publish + upstream force-push
- **Start:** Max offline edits L on stale tb (`485a9f1`). Back online: tb rewritten, L already half-published before the flight.
- **Intent:** "Get me back to a consistent state."
- **Ideal outcome:** Product shows three facts: tb rewritten, L's tb part based on old line, L's published parts in other repos. One command restacks L's tb part onto the rewritten line and continues publishing.
- **Requirement exposed:** Recovery from combined offline, partial-publish and upstream-rewrite states is guided and loses nothing.

### X14. Extract component + in-flight changes
- **Start:** Two agents have in-flight changes touching `crates/help-test` in umbrella while Max extracts it into `maxeonyx/help-test` (F1).
- **Intent (agents):** "My change should land in the new repo."
- **Ideal outcome:** In-flight changes to the extracted path are translated to the new component automatically; their ids stay the same.
- **Requirement exposed:** Topology changes carry in-flight changes along with them.

### X15. Move mount + repeated component + history
- **Start:** Composition history had tb at `tools/tb`; now also mounted as `vendor/tb` at v0.1.25.
- **Intent:** "Blame `tools/tb/src/main.rs` across the move to `tools/tmux-bridge`."
- **Requirement exposed:** Path history in a composition is tracked per occurrence, including through mount moves.

### X16. Plain-Git contributor + cross-repo change in flight
- **Start:** Max's L touches trunc and tb; outsider opens P#40 on tb that conflicts with L's tb part and merges first.
- **Intent:** "Resolve in tb only; trunc part stays as-is."
- **Ideal outcome:** Conflict isolated to L's tb part; resolution recorded; L re-published to tb only.
- **Requirement exposed:** An update to one part of a published logical change republishes only that part.

### X17. Web edit to a product-published branch + amend locally
- **Start:** L's dotsync branch published; reviewer uses GitHub "suggested change" commit `r`. Max meanwhile amends L's dotsync part locally.
- **Intent:** "Keep both."
- **Ideal outcome:** Product fetches `r`, merges it into L's dotsync part (or shows a conflict), publishes without force-push.
- **Requirement exposed:** Concurrent remote and local edits to the same published part are merged, never overwritten.

### X18. Ephemeral agent composition + unpublished component commits + agent death
- **Start:** Agent in ephemeral CI composition makes commits in 3 components, publishes 1, then the runner is killed.
- **Intent (Max):** "Recover its work."
- **Ideal outcome:** [OWNER-DECISION] Either (a) ephemeral contexts must checkpoint unpublished work somewhere durable (which remote? no privileged repo), or (b) loss of unpublished ephemeral work is acceptable and the published part is marked as orphaned partial change.
- **Requirement exposed:** The durability guarantee of unpublished work in ephemeral compositions is explicit.

### X19. Release tag + cross-cutting change + composition version
- **Start:** L lands in trunc and dotsync; trunc releases v0.4.14 from its CI, dotsync's release CI fails.
- **Intent:** "Is L released?"
- **Ideal outcome:** Composition shows L landed in both, released only in trunc; composition version cannot claim "L released" yet.
- **Requirement exposed:** Landed, released and adopted-into-composition are distinct, queryable states per component per change.

### X20. Tool release regresses umbrella concerns on every branch (real 4e0784a)
- **Start:** trunc releases v0.4.14; every open umbrella branch now fails `pinned_main_parity`/`release_freshness` until someone bumps the pin; ledger bot can't write for any umbrella branch meanwhile.
- **Intent:** "A release in a component shouldn't make every composition branch red."
- **Ideal outcome:** Composition branches that track "trunc main" adopt automatically; ones that pin are shown as stale with a one-step update rather than failing unrelated work.
- **Requirement exposed:** Upstream component progress never blocks unrelated work in a composition branch.

### X21. Security view + edit + standalone rules
- **Start:** Auditor's security view (workflows only) at latest releases. Auditor fixes a `permissions:` block in 6 workflows.
- **Intent:** "Propose this fix to each repo."
- **Ideal outcome:** Six PRs based on each repo's main (not on release tags the view was built from), or a clear choice; each PR a normal PR.
- **Requirement exposed:** Edits made in a view built at old revisions can be proposed against current heads, with the rebase made explicit.

### X22. Projection view + change touching hidden files
- **Start:** Frontend-only view (each `docs/` Pages dir). L made in "whole" touches `docs/` and `src/` of trunc.
- **Intent (frontend dev):** "Review L from my view."
- **Ideal outcome:** Frontend view shows L's docs part and indicates hidden parts exist; amending L from the frontend view does not drop the hidden parts.
- **Requirement exposed:** Editing a change from a view that hides some of its content preserves the hidden content.

### X23. Undo after publish across remotes
- **Start:** Agent published L to 4 repos then Max says undo.
- **Ideal outcome:** Local state undone; product offers explicit actions to close PRs/delete branches; never force-pushes main to undo.
- **Requirement exposed:** Undo never performs destructive remote operations implicitly, and says what remote effects remain.

### X24. Merge-base rule for ledgers + composition merge
- **Start:** Real TODO: a branch merging an older bot-written ledger can present a downgrade. Composition merge of two umbrella lines each carrying different tool ledgers.
- **Intent:** "Merge without laundering a downgrade."
- **Requirement exposed:** Composition merges of component-generated state files are auditable against each component's own merge-base, not just the two tips.

### X25. Branch deletion race + bot + composition
- **Start:** Integration merges tb PR with `--delete-branch` while tb ledger bot is still writing; bot write lost; composition referenced the bot's intended commit.
- **Ideal outcome:** Composition references the merged commit and warns ledger state is stale in tb.
- **Requirement exposed:** A composition never references a component commit that no remote holds without saying so.

### X26. Fork + repeated component + upstream merge
- **Start:** Kim's composition has `trunc` (Max's) and `trunc-kim` (her fork) side by side, to compare. She makes L across both plus dotsync.
- **Ideal outcome:** Parts go to maxeonyx/trunc PR, kim/trunc branch, dotsync PR; later, Max merges the upstream trunc part and Kim pulls it into trunc-kim with shared identity.
- **Requirement exposed:** Change identity is shared across a repo and its forks, so the same part is recognised as landed in both.

### X27. Agent-generated script + partial failure + idempotent retry
- **Start:** Agent script (K2) applies deconfuse-rs adoption across 6 tools; fails in tb (build error); script re-run after fix.
- **Ideal outcome:** Re-run leaves the 5 successful parts unchanged (same change ids, no duplicate commits), completes tb.
- **Requirement exposed:** Re-running a multi-component operation after partial failure converges instead of duplicating.

### X28. History rewrite of a component with bot commits + composition history
- **Start:** Umbrella history has 35-50 pointer-bump commits naming tb SHAs from before the repair.
- **Intent:** "Materialize umbrella at 3 months ago."
- **Ideal outcome:** Old tb SHAs resolvable (backup tag kept) or mapped to new; the product never requires rewriting the umbrella's history because a component rewrote its own.
- **Requirement exposed:** A component rewrite never forces a rewrite of compositions' histories, and old composition states remain materializable.

### X29. Review + bot commits + cross-repo diff
- **Start:** Reviewer looks at L's unified diff; five repos have `[bot]` ledger commits on L's branches.
- **Intent:** "Show me human changes, and ledger changes separately."
- **Requirement exposed:** A combined review can separate human-authored and automation-authored content within one logical change.

### X30. Two agents, one cross-cutting change each, overlapping repos, both partially published
- **Start:** Agent A's L1 (help-text) published to trunc,tb; Agent B's L2 (CI triggers) published to tb,dotsync; both touch tb's ci.yml.
- **Ideal outcome:** Each lands independently; the tb conflict is surfaced on whichever lands second, in that change only.
- **Requirement exposed:** Independently authored multi-component changes compose like independent branches do within one repo.

### X31. Standalone publishability + composition sibling visibility + extracted library
- **Start:** After F1, trunc in a composition sees `libraries/help-test` at main but Cargo.toml says tag v0.1.0. Agent edits help-test and trunc together (L), expecting trunc to use the edit.
- **Intent:** "Test my help-test change with trunc before releasing help-test."
- **Ideal outcome:** Composition can override trunc's dependency to the mounted help-test for local testing, and it is impossible to publish trunc with that override leaked into its Cargo.toml (standalone CI would break).
- **Requirement exposed:** Composition-local overrides of how components consume each other never leak into the components' published content.

---

## Summary 1. Deduplicated externally observable requirements

1. **Component purity.** A component's published history and tree contain nothing a plain-Git author would not have written; no product metadata is required in any component (B1, H1, K9, J1).
2. **Plain-Git contributors need nothing.** Contributions by plain-Git users, GitHub web edits, and GitHub's merge buttons are handled as ordinary upstream commits (H1, H4, H5).
3. **Change identity survives server-side rewriting.** A change is recognised after squash-merge, rebase-merge, and across forks (H2, H3, X11, X26).
4. **One command to materialize** a composition, full or partial, with explicit reporting of what is omitted or failed (A1, A2).
5. **Partial views are honest.** A view never reports a result as covering what it omitted (A2, X8, X22).
6. **Compositions need no write access and no opt-in** from components; anyone can compose what they can read (A3, A9).
7. **Ephemeral compositions** exist for a process lifetime yet have a reproducible identity (A4, J9).
8. **Promotion without redo:** scratch compositions/changes can be named and shared after the fact (A5).
9. **Adopt in place:** existing plain clones join a composition without being rewritten (A7, G4).
10. **Components can be born, split, merged, extracted, inlined, renamed, archived and revived** with histories a plain-Git user finds sensible and compositions kept reproducible (A8, F1-F6, F10).
11. **Multiple occurrences** of one repo in a composition, independently versioned, with parts of a change per occurrence (C1-C3, X7).
12. **Commit identity independent of mount path** and of which composition made it (C6).
13. **Whole-composition status, diff, log, blame** show content, never opaque pointer changes (B3-B6).
14. **Continuous recoverability** of uncommitted work across all components (B2).
15. **Operation-level undo and op log** spanning components, by id, for humans and agents (B9, K3).
16. **Undo never implies destructive remote actions** and reports remaining remote effects (X23, D6).
17. **Logical change editing:** amend, split, squash, reorder, abandon, transplant across components as easily as within one repo (D1-D6, D9).
18. **Multi-component rebase** with first-class, deferrable conflicts; a conflict in one component doesn't block others (D7, E9).
19. **Identical conflicts across components resolvable once** (E5).
20. **Merges are per-component and never pick a side** when both lines moved a component (E2, E3).
21. **Stacks survive** bottom-landing, squash-merge and `--delete-branch` (E4, X11).
22. **Publishing a multi-component change is one command** with per-remote outcomes (G1, I5).
23. **Partial publication is a first-class, visible, resumable, idempotent state** (G2, X3, X27).
24. **Partial landing is survivable indefinitely** and visible per component (X4, X19).
25. **Published parts are updated with the least disruptive update each remote permits**, never force-pushing where forbidden, never overwriting remote additions (X3, X17, H5, I5).
26. **Automated CI commits** (ledger bots) on published parts are absorbed without author rework (X1, X10).
27. **Components' history-shape rules** (failing-first, two-dispatch red/green) are respected and checkable pre-publish (X2, D4).
28. **Rewrite correspondence:** after a component rewrite, every pin, tag, occurrence, attestation and CI record referencing old commits can be re-resolved, and what could not be carried over is reported (D10, X6, X12, X28, G5).
29. **A component rewrite never forces composition history rewrite**; old composition states stay materializable (X28, G9, F6).
30. **Upstream component progress never blocks unrelated composition work** (X20, E8).
31. **No per-component ritual to adopt new component revisions** (E8, J3).
32. **Component CI works unchanged**; composed CI can test unlanded multi-component changes and fails loudly on unreadable parts (J1, J2, X8).
33. **Trust boundaries preserved:** a change is not judged by rules it introduces, in any component (X9).
34. **Standalone fidelity:** a component built inside a composition can be made to behave like its standalone checkout; composition-local overrides never leak into published content (B7, X31, C4).
35. **Provenance:** from any component commit or binary, find the logical change and sibling commits; composition versions determine every component revision (J4, J5).
36. **Landed vs released vs adopted** are distinct queryable states (X19, J6).
37. **Visibility-safe composition:** published compositions never leak unreadable components' content or metadata (I3, I4).
38. **Permissions:** a change spans repos with different permissions; bookkeeping needs no extra scope (I2, I8).
39. **Agent-grade interface:** complete structured state in one call, dry runs, idempotent scriptable ops, stable ids across rewrites, per-component actionable errors (K1, K2, K5-K7).
40. **The 90% case costs no more than plain Git/jj** (K8).

---

## Summary 2. Tensions

- **T1. Component purity vs. change identity across squash/rebase-merge.** Recognising a squashed change (H2, H3, X26) usually needs something in the commit (trailer, id), but Req 1 says nothing product-specific in components. Exposed by H2, H3, X11, X26, K9. [OWNER-DECISION] Accept a trailer in component commits, or accept heuristic (content/PR-link) recognition.
- **T2. Atomicity vs. ordinary hosting.** "One atomic logical event" (X5) cannot be delivered by GitHub, which merges repos independently; partial landing (X4) is inevitable. Exposed by X4, X5, X19, G2.
- **T3. No privileged repository vs. somewhere to keep unpublished/partial state.** Sharing a composition with unpublished component work (G8), durability of ephemeral agent work (X18), and composition versions (J4) all want a durable home; "no canonical monorepo" forbids a privileged one. Exposed by G8, X18, A5, J4.
- **T4. Stable change identity vs. per-repo history rules.** Amending a published part (X3) wants replacement; no-force-push and bot commits (X1, I5) want append-only; tdd-ratchet wants specific intermediate commits (X2). Exposed by X1, X2, X3, X17, D4.
- **T5. Easy history rewriting vs. SHA-keyed external state.** Statuses, Releases, signatures, attestations (`state.json`), and composition pins are keyed by SHA (D10, X12, X10). JJ-style fluid rewriting multiplies breakage once published. Exposed by D10, D11, X6, X12, X28.
- **T6. Plain `git clone` of a composition vs. freedom from pointer mechanics.** Plain-Git users (H6, H7) get a working tree only if the published form carries something like gitlinks, which is the pointer-bump ritual (E8) the owner wants gone. Exposed by H6, H7, E8, X20.
- **T7. Composition sibling visibility vs. standalone publishability.** Compositions are useful because siblings are visible (B8, X31), but that visibility is what lets path dependencies leak (help-test history, B7). Exposed by B7, B8, C4, X31.
- **T8. Tracking latest vs. reproducibility.** Auto-adopting new releases (X20, E8, J3) conflicts with exact reproducibility (J9) and with the umbrella's own concerns that deliberately compare pins to released state. Exposed by X20, J6, J9, E8.
- **T9. One review vs. per-repo governance.** Unified review (I1) vs. each repo's required reviewers, branch protection and trusted workflows (I5, I6, X9). Exposed by I1, I5, I6, X9, X29.
- **T10. Shared vs. isolated views on one machine.** Seeing an edit immediately in every view (A10, C7) vs. the workspace rule of one mutable clone per session (E1). Exposed by A10, C7, E1.
- **T11. Composed CI completeness vs. permissions and trust.** Composed CI must see all parts (J2) but must not substitute readable revisions (X8) nor let a change run its own new rules (X9). Exposed by J2, X8, X9.
- **T12. Visibility vs. context in change metadata.** One description for a logical change vs. confidentiality per component (I3, I4). Exposed by I3, I4, X22.
- **T13. Workspace routing vs. component independence.** The existing `workspace_routing` concern requires every tool's AGENTS.md to direct agents to the umbrella, which is a product/composition reference inside the component, and makes the umbrella privileged. Exposed by K9, A9, T3. [OWNER-DECISION] whether that concern survives.
- **T14. Repeated occurrence vs. single identity.** A change touching two occurrences (C3, X7) is one intent but lands as two commits on two lines; is it one change or two? Exposed by C3, X7, X26.
- **T15. Undo power vs. published effects.** Operation-level undo (B9, K3) is expected to be total, but remote effects cannot be undone silently (X23, D6). Exposed by B9, X23, D6.
