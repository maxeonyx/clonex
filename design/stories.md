# Stories — the root of the design

Everything else in `design/` is downstream of this file. A constraint that
can't point at a step in a story here is deleted. `constraints.md`'s K-list
is **suspended** until it has been re-derived from these stories (§9).

Each story is told twice:
- **Today:** what actually happens now, with the real submodule umbrella.
  This is reality, and it is the bar.
- **Wanted:** what each person would like to type and see. It is described
  only as *observable* things: commands, files on disk, what `git`, `jj`
  and GitHub show. It never mentions how CloneX works inside. Where a
  wanted step needs a new command, it is written `cx …`, with only its
  visible effect.

## Cast, machines, repositories

**People and machines**
- **Max**: the owner. Laptop `maxbook` (Linux, fish, git 2.55, jj 0.44).
  Works in `~/agent-tools-workspace/at-<slug>/`, one full clone per task.
  Has push rights everywhere.
- **Ada**: a coding agent (Claude) that Max starts on `maxbook`, in its own
  clone. It uses git, not jj. It follows `AGENTS.md` to the letter.
- **Jim**: an outsider. Desktop `jim-pc` (Windows, Git for Windows, VS
  Code). Has never heard of the umbrella. Finds a typo in trunc's help
  text.
- **Bob**: a collaborator with push rights on dotsync only. MacBook
  `bob-mac`, plain git. Develops dotsync standalone and never opens the
  umbrella.
- **CI**: GitHub Actions. **ratchet-bot**: the trusted ledger workflow,
  which commits `.test-status.json` to PR branches as
  `github-actions[bot]`.

**Repositories on GitHub** (merge-commit-only; squash and rebase are
disabled on the tool repos, as they really are):

`maxeonyx/trunc`, branch `main`:

```
t1  "Start trunc"                 src/lib.rs, src/help.txt, AGENTS.md
t2  "Add --max-lines"             src/lib.rs
t3  "Merge pull request #4 …"     (merge commit)
```

`src/help.txt` at t3:

```
Usage: trunc [--max-lines N] FILE
Truncates FILE to N lines. Defualt: 50.        ← the typo
```

`maxeonyx/dotsync`, branch `main`:

```
d1  "Start dotsync"               src/main.rs, AGENTS.md
d2  "Release 0.9.1"               Cargo.toml
```

`maxeonyx/agent-tools` (the umbrella U), branch `main`:

```
u1  "Start umbrella"              README.md, crates/standards/…, .gitmodules
u2  "Pin trunc at t3"             tools/trunc  → gitlink t3
u3  "Pin dotsync at d2"           tools/dotsync → gitlink d2
```

`.gitmodules` in U:

```
[submodule "tools/trunc"]   path = tools/trunc   url = git@github.com:maxeonyx/trunc.git
[submodule "tools/dotsync"] path = tools/dotsync url = git@github.com:maxeonyx/dotsync.git
```

---

## S1. Jim fixes a typo in trunc (the outsider story)

### Today

On `jim-pc`:

```
> gh repo fork maxeonyx/trunc --clone
> cd trunc
> git switch -c fix-typo
  (edits src/help.txt: "Defualt" → "Default")
> git commit -am "Fix typo in help text"          → commit j1, parent t3, author Jim
> git push -u origin fix-typo
> gh pr create --repo maxeonyx/trunc
```

State now:

| where | state |
|---|---|
| `jim-pc` `trunc/` | `main`=t3, `fix-typo`=j1 |
| `jimsfork/trunc` | `fix-typo`=j1 |
| `maxeonyx/trunc` | `main`=t3; PR #12 from Jim's `fix-typo`; `refs/pull/12/head`=j1 |
| `maxeonyx/agent-tools` | unchanged; `tools/trunc` → t3 |

Max reviews PR #12 on GitHub and clicks "Merge pull request". GitHub
creates `t4 = merge(t3, j1)`, committer GitHub, signed by GitHub. Now
`maxeonyx/trunc main` = t4.

The umbrella still points at t3. Nothing happens to it until Max does
this, in a clean clone on `maxbook`:

```
$ cd ~/agent-tools-workspace/at-bump-trunc
$ git submodule update --init tools/trunc
$ cd tools/trunc && git fetch && git switch --detach origin/main && cd ../..
$ git add tools/trunc
$ python3 scripts/generate-version-json.py
$ git commit -m "Point trunc at t4"             → u4
$ git push
```

What each person sees afterwards:
- **Jim:** his PR says "merged". He never learns the umbrella exists. ✓
- **Max, in the umbrella**, three weeks later, wondering who changed the
  help text:
  ```
  $ cd tools/trunc
  $ git log --oneline -- src/help.txt
  t4 Merge pull request #12 from jim/fix-typo
  j1 Fix typo in help text
  t1 Start trunc
  $ git blame src/help.txt        → line 2: j1 (Jim …)
  ```
  This works today because `tools/trunc` *is* a trunc repository.
- **Max, from the umbrella root:** `git log -- tools/trunc` shows u4
  "Point trunc at t4" and u2, the pointer bumps. The diff of u4 is one
  line: `-Subproject commit t3 / +Subproject commit t4`.
- **GitHub, umbrella view:** `tools/trunc` shows as "trunc @ t4", a link to
  the other repo. There is no file browsing, no blame and no search inside
  it.

### Wanted
- **Jim:** exactly the same as today. Nothing to install, nothing new to
  see. *(Q-S1a)*
- **Max:** Jim's fix reaches the umbrella without the bump dance. At most
  one command, and ideally something CI or anyone can run. *(Q-S1b)*
- **Max, three weeks later, from inside the umbrella:** a question like
  "who changed this help line?" is answered at least as easily as today's
  `cd tools/trunc && git blame src/help.txt`, which names **Jim** and
  **j1**, and names *t4 as the commit trunc knows it by*, the same SHA
  GitHub shows. *(Q-S1c)*
- **GitHub, umbrella view:** `tools/trunc/src/help.txt` is a real file
  there, browsable and searchable. *(Q-S1d)*

**Open, a question for the owner.** Should the umbrella's *plain* `git
blame tools/trunc/src/help.txt` (run from the umbrella root, not inside a
trunc repo) name Jim, or is it acceptable that plain Git names the
umbrella commit that brought the fix in, with `cx blame` naming Jim?
Today's baseline is only "`cd` into a trunc repo and blame there", so
either answer clears the bar as long as the Jim answer is one step away.

---

## S2. Max makes the CLAUDE.md alias change across two tools (the motivating story)

### Today (this really happened; reconstructed from the umbrella's log)

On `maxbook`:

```
$ cd ~/agent-tools-workspace
$ git clone git@github.com:maxeonyx/agent-tools.git at-claude-md-alias
$ cd at-claude-md-alias
$ git switch -c claude-md-alias
$ git submodule update --init tools/trunc tools/dotsync

# trunc
$ cd tools/trunc
$ git switch -c claude-md-alias origin/main
$ ln -s AGENTS.md CLAUDE.md
$ git add CLAUDE.md && git commit -m "Alias AGENTS.md as CLAUDE.md"      → t5 (parent t4)
$ git push -u origin claude-md-alias
$ gh pr create --fill                                                  → trunc PR #13
$ gh workflow run ci.yml --ref claude-md-alias -f pr_number=13
$ cd ../..

# dotsync — the same four steps                                        → d3, dotsync PR #51

# umbrella: the checker
  (writes crates/standards/src/concerns/claude_alias.rs)
$ git add crates && git commit -m "Require a CLAUDE.md alias beside every AGENTS.md"   → u5
  … waits for both tool PRs to merge (t6 = merge(t4,t5), d4 = merge(d2,d3)) …
$ cd tools/trunc && git fetch && git switch --detach origin/main && cd ../..
$ cd tools/dotsync && git fetch && git switch --detach origin/main && cd ../..
$ git add tools/trunc tools/dotsync
$ python3 scripts/generate-version-json.py
$ git commit -m "Point trunc and dotsync at the alias merges"                           → u6
$ git push -u origin claude-md-alias && gh pr create --fill                           → umbrella PR #44
```

The count: 3 PRs, 3 CI runs, 2 hand-written pointer bumps, and an ordering
rule the agent must remember ("tool PRs merge before the umbrella pointer").
When one tool PR needs a review change, the whole sequence repeats for that
tool.

### Wanted

```
$ cx clone maxeonyx/agent-tools at-claude-md-alias        (or git clone; see S4)
$ cd at-claude-md-alias
$ git switch -c claude-md-alias
$ ln -s AGENTS.md tools/trunc/CLAUDE.md
$ ln -s AGENTS.md tools/dotsync/CLAUDE.md
  (writes crates/standards/src/concerns/claude_alias.rs)
$ git add -A && git commit -m "Alias AGENTS.md as CLAUDE.md everywhere and check it"   → ONE commit
$ cx publish
```

After `cx publish`, Max sees:

| where | state |
|---|---|
| `maxeonyx/trunc` | branch `claude-md-alias` with **one** commit: "Alias AGENTS.md as CLAUDE.md everywhere and check it", author Max, containing only `CLAUDE.md`, parent t4 |
| `maxeonyx/dotsync` | same, containing only `CLAUDE.md`, parent d2 |
| `maxeonyx/agent-tools` | branch `claude-md-alias` with the one commit |
| PRs | one per repo, each linking the others (a hosting step, and may stay manual at first) |

- The tool PRs are ordinary PRs to anyone who looks at them: Bob, CI,
  reviewers. *(Q-S2a)*
- No pointer-bump commits exist anywhere, and nobody decides an order.
  *(Q-S2b)*
- In the umbrella, `git show HEAD` shows all three parts as one ordinary
  diff. *(Q-S2c)*
- If trunc's review asks for a change, Max edits `tools/trunc/…` in the
  umbrella, commits, and runs `cx publish` again. trunc's PR gets the fix;
  dotsync's PR is not touched. *(Q-S2d)*

When the tool PRs are merged by the integration workflow (t6, d4 on the
tools' `main`), the umbrella branch must be able to merge into umbrella
`main` without Max re-doing anything. Afterwards, from umbrella `main`,
"which trunc commit is `tools/trunc` now?" answers **t6**, the SHA trunc's
own `main` has, so `latest-ci-green` and tags keep working. *(Q-S2e)*

---

## S3. Bob works on dotsync while Ada changes dotsync from the umbrella (concurrency)

Starting state: dotsync `main` = d4 (after S2).

**Bob, on `bob-mac`, 10:00:**

```
% git clone git@github.com:maxeonyx/dotsync.git && cd dotsync
% (edits src/main.rs: renames fn load_scopes → read_scopes, updates all 3 call sites)
% git commit -am "Rename load_scopes"          → b1 (parent d4)
% git push origin main                          (Bob may push main directly)
```

`maxeonyx/dotsync main` = b1.

**Ada, on `maxbook`, 09:30–10:30,** in `at-config-concern`, cloned at 09:30
when dotsync was d4. Ada adds config loading to trunc and dotsync. In
dotsync that means a new call `load_scopes(&cfg)` in `src/config.rs`.
Umbrella commit `a1` touches `tools/trunc/src/config.rs`,
`tools/dotsync/src/config.rs` and `crates/standards/…`.

**10:30, Ada publishes.**
- **Today's equivalent:** Ada would be inside `tools/dotsync`; `git push`
  of her branch works (a new branch), and the PR shows a conflict-free
  merge. But her code calls `load_scopes`, which no longer exists on
  main, so CI fails **after** merge, or in the PR if CI tests the merge
  result.
- **Wanted:**
  - `cx publish` succeeds for trunc and for Ada's new dotsync branch
    (branches are new; nobody else's work is overwritten). *(Q-S3a)*
  - Ada can see, from the umbrella, that dotsync moved (b1) since her base
    (d4), with one command. *(Q-S3b)*
  - Ada can bring b1 into her umbrella branch with one command. It is an
    ordinary Git merge in the umbrella (`git merge` or equivalent). Her
    umbrella build then fails on `load_scopes` *in the umbrella*, where she
    can fix it. She commits `a2` (rename the call), publishes again, and the
    dotsync branch gets that fix. *(Q-S3c)*
  - In dotsync's history, Ada's branch is then an ordinary branch that
    merged `main` (b1) and fixed the call. That is exactly what AGENTS.md
    already asks ("merge current child main into the branch"). *(Q-S3d)*
  - **Bob never sees any of this except Ada's PR.** *(Q-S1a again)*

---

## S4. Carol clones the umbrella for the first time (plain Git)

Carol is new and has plain git.

**Today:**

```
$ git clone https://github.com/maxeonyx/agent-tools.git && cd agent-tools
$ cargo build -p standards
error: failed to read `tools/trunc/Cargo.toml` … (empty directory)
$ git submodule update --init --recursive       (7 more clones; slow; needs network)
```

Checking out an *old* umbrella commit (`git checkout u2`) also needs
`git submodule update`, and fails if a URL in the old `.gitmodules` has
moved. (That really happens with URL changes; help-test also renamed its
trunk.)

**Wanted:**
- `git clone` then `cargo build` just works; the tool files are real files.
  *(Q-S4a)*
- `git checkout <old umbrella commit>` gives that commit's tool content
  offline, forever, even if tool repos move or are deleted. *(Q-S4b)*
- Carol can do all of that without installing CloneX. *(Q-S4c)*

---

## S5. Ada's ratchet flow across two tools (bot commits land under her work)

Ada is adding a feature with a new test in trunc and dotsync. The ratchet
requires red before green *in each tool's history*, recorded by
ratchet-bot.

**Wanted sequence** (today it is the S2 dance, twice per tool):
1. Ada writes the failing tests in both tools, commits umbrella commit `r`
   ("Add failing tests for --config"), and runs `cx publish`. Now trunc
   branch `config` = `[r_T]` and dotsync `config` = `[r_D]`: each holds one
   commit with that message, containing only that tool's test.
2. Ada runs `gh workflow run ledger.yml --ref main -f target=13` for trunc
   PR #13, and the same for dotsync. ratchet-bot pushes `bot_T` on top of
   `r_T` ("chore: record tdd-ratchet status", `.test-status.json` →
   `pending`), and `bot_D` on dotsync.
3. **Ada must not overwrite the bot's commits.** Her next publish must
   either include them or refuse. *(Q-S5a)*
4. Ada brings the bot commits into her umbrella branch with one command.
   Afterwards `tools/trunc/.test-status.json` in the umbrella says
   `pending`. *(Q-S5b)*
5. Ada writes the implementation, commits `g`, and publishes. trunc
   `config` = `r_T → bot_T → g_T`: red, then ledger, then green, in that
   order in trunc's own history, as the ratchet requires. *(Q-S5c)*
6. ratchet-bot records `passing`, the integration workflow merges the tool
   PRs, and the umbrella takes the merges in as in S2. *(Q-S2e again)*

**The awkward variant:** after step 2, a reviewer asks for a change to the
*test*. Ada amends `r` locally. The bot's `bot_T` was computed for the old
`r_T`.
- Today with plain git: Ada force-pushes the PR branch (dropping `bot_T`)
  and redispatches the ledger.
- **Wanted:** at least as good as that. CloneX tells her the bot commit is
  now stale, rather than silently keeping it on top of the new test.
  *(Q-S5d)*

---

## S6. Max keeps an old tdd-ratchet for compatibility tests (two copies)

Max wants `fixtures/tdd-ratchet-1.1.6/` in the umbrella: tdd-ratchet as it
was at its v1.1.6 tag, frozen, so the umbrella's ledger tests can check old
ledgers. `tools/tdd-ratchet/` stays current.

**Today:** a second submodule entry pointing at the v1.1.6 commit, or a
copy of the files with no history.

**Wanted:**
- `cx add fixtures/tdd-ratchet-1.1.6 --from tdd-ratchet@v1.1.6`: real files
  appear, and the umbrella knows they are tdd-ratchet at v1.1.6 (its real
  SHA). *(Q-S6a)*
- Syncing never moves it. *(Q-S6b)*
- A CVE fix: Max edits both `tools/tdd-ratchet/src/ledger.rs` and
  `fixtures/tdd-ratchet-1.1.6/src/ledger.rs` in one umbrella commit.
  Publishing produces a normal PR on tdd-ratchet `main`, *and* makes a
  backport branch on top of v1.1.6 possible (e.g. for a 1.1.7 release),
  with the same commit message, recognisably the same fix. *(Q-S6c)*

---

## S7. The ledger repair: a tool's history is rewritten (really happened)

With Max's approval, tb's `main` was rebuilt: every commit after the
earliest bad ledger edit got a new SHA, with trees byte-identical except
the edited ledger blobs. The old tip was tagged
`backup/tb-main-2026-09-09`, and `main` was force-pushed.

**Today:** the umbrella's `tools/tb` gitlink pointed at an old SHA that is
no longer on `main`. Max bumped it: "Point tb at the repaired ledger
history". The concerns `latest-ci-green` and `release-freshness` went red
until an integration run landed on the new tip.

**Wanted:**
- The umbrella is **not** rewritten. Its old commits keep showing what
  they contained then, and `git checkout` of an old umbrella commit still
  works offline (Q-S4b). *(Q-S7a)*
- One command brings the new tb history in. It succeeds without Max
  resolving conflicts in files the umbrella never touched. *(Q-S7b)*
- Afterwards "which tb commit is `tools/tb`?" answers the new SHA.
  *(Q-S2e again)*

---

## S8. Umbrella CI

`.github/workflows/ci.yml` in U uses `actions/checkout@v4`, which defaults
to depth 1.

**Today:** it must also check out submodules, which needs tokens and
network per tool.

**Wanted:**
- `actions/checkout` with no extra options gives CI every tool's files.
  *(Q-S8a)*
- CloneX-specific CI steps, if any, say clearly when they need more
  history, and never silently compute something different in a shallow
  checkout. *(Q-S8b)*

---

## The model, as the owner states it (2026-09-28)

> The umbrella appears to be a monorepo. It is one Git repo, and CloneX
> knows how to treat it as multiple upstream repos. `cd tools/trunc && git
> blame` runs over the umbrella's own history, the same as from the root,
> and names Jim. **Every tool commit has its own umbrella commit. It's the
> whole model.** One umbrella commit across five tools maps to five tool
> commits, one per tool, and those five map *deterministically back to that
> same umbrella commit*. Checking out or inspecting the umbrella makes it
> clear that the umbrella commit subsumes those sub-commits. (There is no
> uncertainty about this; it is intentional.)

Grounding in the stories:
- S1: Jim's `j1` and GitHub's merge `t4` each get an umbrella commit, so
  plain blame in the umbrella names Jim. Their SHAs are the umbrella's;
  CloneX maps them to `j1`/`t4`.
- S2: Max's single alias commit *is* the umbrella commit for both trunc's
  commit and dotsync's commit.
- S6: the single CVE-fix commit is the umbrella commit for both
  tdd-ratchet commits (main and the v1.1.6 backport).
- The **same** umbrella commit should also cover tool commits that were
  made *elsewhere* but are one logical change. For example, a change made
  in a view V lands in trunc and dotsync, and the umbrella later brings in
  both.

**Tension to resolve (not yet decided). This is not about W14**, which is
settled: it concerns a *single foreign* tool commit arriving on two
branches independently. "Every tool commit has its own
umbrella commit" reads as *one* umbrella commit per tool commit. If Ada
and Max each bring the same new trunc commit into two different umbrella
branches, each branch gets an umbrella commit for it, and those two
necessarily have different trees (each branch's other content). Once the
branches merge, the umbrella contains two commits for one trunc commit.

The context-free alternative gives one representative everywhere, but it
breaks Git's merges (`research/r2pp-attack.md` G1). A candidate way out is a
convention: tool commits enter the umbrella once, on a shared line, and
branches get them by merging that line. That needs its own story (two
agents syncing at once).

**Owner's answer and a candidate (C″), now attacked
(`research/cprime-attack.md`):**
- The same outside commit *should* be the same umbrella commit on every
  branch. Two copies of one tool at two paths necessarily get different
  umbrella commits (different trees), and that is fine.
- Candidate: build the outside commit's umbrella commit on the umbrella
  commit that represents its tool parent, not on the branch it is pulled
  into. It is then branch-independent, and a real full umbrella state.
- For tool merge commits, the tree outside the tool path is Git's merge of
  the parents' umbrella trees.
- **Result:** C″ with W14 fixes F1, F5, F8 and G2 (no markers needed), UC29
  (one identical object on every clone), force-push, extraction and nesting.
  Plain C′ is refuted, and duplicate copies are *not* harmless (H2: an
  upstream revert is undone by an ordinary merge).
- **Residuals:**
  - G1 is narrowed but structural: a file added outside `P` after a shared
    representative's context can still capture a component edit under
    rename detection.
  - Plain `git merge` can resurrect content in the amend × reviewer case
    (w3); CloneX's P-scoped merge fixes that.
  - Fusion needs a deterministic tie-break and is sensitive to arrival
    timing (w9).
  - The S2b choice above.

## Current representation (candidate C″, endorsed by the owner as the direction)

Each tool commit gets **exactly one** umbrella commit per copy of that tool
in the umbrella, and it is identical in every clone:
- An outside commit's umbrella commit is built on the umbrella commit of
  its tool parent (for merges, a Git merge of the parents' umbrella trees
  outside the tool path).
- A tool commit published *from* an umbrella commit `u` maps back to `u`
  (W14).

Why this matters: Git's 3-way merge needs one shared ancestor for one
change. Two copies of the same change on two lines let an ordinary merge
silently undo a later revert (the cherry-pick/revert hazard; see H2 in
`research/cprime-attack.md`). Principle, from the owner: **minimise
duplication wherever it can be done deterministically.** A separate user
flow keeps history as minimal and pleasing as possible (cosmetic, not
correctness).

## S2b. Half-landed: trunc merged Max's change, dotsync hasn't yet

> **Reclassified after drawing the DAGs with the owner:** at level 1
> (commits only) there is no problem. `ρ(t6)` has parents `U3` and `u`, is
> "trunc = t6, dotsync = x_D", and maps down to real commits in both tools.
> Everything below is about which umbrella commit a *label* (`main`)
> should point at while dotsync's label hasn't moved. That is a
> **level-2 (refs) question**, deferred with the other ref stories. The
> timestep DAGs were drawn for the owner as an artifact page.

Starting point: S2's single umbrella commit `u` (alias in trunc and dotsync,
plus the checker) has been published. At 11:00 trunc's integration workflow
merges PR #13: `t6 = merge(t4, x_T)`, where `x_T` is `u`'s trunc part. At
11:00 dotsync PR #51 still has an open review comment; dotsync `main` = d2.

At 11:05 someone brings trunc's `main` (t6) into umbrella `main`. By W14,
t6's umbrella commit has `u` as a parent, because `x_T` *is* `u`. So
bringing in t6 brings in **all** of `u`, including dotsync's `CLAUDE.md`,
which dotsync hasn't accepted.

`research/cprime-attack.md` (w1) proves that with plain Git merges exactly
one of these must happen:
- **(a) Wait.** Umbrella `main` does not take trunc's `main` past `t6` until
  every part of `u` has landed. Jim's later typo fix on trunc waits too.
  This is today's rule ("tool PRs land before the umbrella"), made
  automatic.
- **(b) Atomic landing.** The first tool that accepts `u` lands *all* of `u`
  in umbrella `main`. Umbrella `main` is then ahead of dotsync `main` for
  `tools/dotsync`, and `cx status` shows "dotsync part of `u` not yet
  landed". If dotsync rejects it, the umbrella needs a follow-up commit.
- **(c) Shadow copy.** Umbrella `main` takes trunc's part through a separate
  copy of `x_T`, without `u`. This brings back duplicates, so an upstream
  revert can be undone by a later merge (the H2/G2 hazard).

**Level-2 question, deferred.**

## S9. What the stories actually demand

Every requirement below cites the steps that produce it. Anything from the
old K-list that doesn't appear here is dropped.

| # | Requirement (observable) | From |
|---|---|---|
| **W1** | Tool repos and their contributors need nothing new, see nothing new, and all their workflows (fork, PR, merge button, direct push, bots) are unchanged | S1 Jim, S3 Bob, S5 bot |
| **W2** | Tool content is real files in the umbrella: a plain clone builds; old umbrella commits check out offline forever; GitHub browses and searches them | S4a–c, S1d, S8a, S7a |
| **W3** | One umbrella commit can change several tools and the umbrella together, and publishing turns it into one ordinary commit per touched tool, on a branch in that tool's repo, each with only that tool's files | S2 |
| **W4** | Bringing tool commits into the umbrella (someone else's, a bot's, a rewrite's) is one command, and conflicts show up as ordinary Git conflicts *in the umbrella* | S1b, S3c, S5b, S7b |
| **W5** | From any umbrella commit, "which commit of tool X is this?" answers the tool's *real* SHA whenever the umbrella's content equals a real tool commit | S2e, S7, S6a |
| **W6** | Publishing never overwrites commits that others (bots, reviewers, Bob) added; re-publishing after an edit updates only the tools whose part changed | S2d, S3a, S5a |
| **W7** | The per-tool commit order the umbrella work implies (red → bot → green) is reproduced exactly in each tool | S5c |
| **W8** | Plain `git blame`/`git log` in the umbrella (from the root or from `tools/trunc`) show each tool commit as its own umbrella commit, with the real author, date and message; CloneX maps each to the real tool SHA | S1c, owner's model |
| **W13** | One logical change is one umbrella commit, whether it was made in this umbrella or elsewhere and brought in; every tool commit has an umbrella commit | S2, S6, owner's model |
| **W14** | Tool commits published from an umbrella commit `u`, and later brought back into the umbrella (after their PRs merge, or with bot or reviewer commits on top), map back to `u` itself, never to new copies. Inspecting `u` lists the tool commits it subsumes, with their real SHAs | S2e, S5, owner's model |
| **W9** | Nothing about a tool's history rewrite forces the umbrella to rewrite | S7a |
| **W10** | Two copies of one tool at different revisions can coexist, one frozen; one fix can go to both | S6 |
| **W11** | Staleness (a bot commit computed for an old version of Ada's commit) is reported, not silently kept | S5d |
| **W12** | Nothing silently computes a different answer with less history; it says what it needs | S8b |

**What happened to the old constraints:**
- **K1** is **reinstated from the stories and the owner's model** as W8 +
  W13: every tool commit has an umbrella commit, and one logical change is
  one umbrella commit. What K1 wrongly added was *context-freedom* (the
  empty umbrella trees). No story demands that.
- K3 → W5, K4 → W3, K5 → W3 + W4, K6 → W4 + W6 + W12, K9 → W9, K10 →
  W10.
- K7 → W2 + W1.
- K2 → W1.
- K8 (determinism): **no story demands it yet.** It stays out until one
  does. (Candidate stories: two agents syncing the same update at once;
  reproducible CI contexts.)
- K11 (extract/convert) and K12 (claims) need their own stories before
  they return.

**Stories still to write, splayed the same way, before any design
choice:**
- two agents syncing at once;
- a throwaway agent context of dotsync + agent-harness;
- extracting `crates/standards` into its own repo;
- converting the real umbrella;
- a plain-Git user rebasing an umbrella branch;
- a GitHub merge of an umbrella PR;
- a nested personal repo;
- Max using jj in the umbrella;
- a forged or mistaken claim.
