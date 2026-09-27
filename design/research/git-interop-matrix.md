# Git / JJ interop matrix for commit-metadata carriers

Status: empirical results from local experiments, run 2026-09-28 with git 2.55.0 and
jj 0.44.0 (`git.write-change-id-header = true`, the default). Everything in the
GitHub section is **UNVERIFIED**: it comes from prior knowledge, not a test.

Tools not installed, so not tested: `git-filter-repo` and `josh`/`josh-proxy`.
To stand in for filter-repo I ran the `git fast-export | git fast-import` pipeline
it is built on. `git subtree` (contrib) was available.

The reproduction script is `run.sh` (Appendix A). The full raw log was 900+ lines
and lived in `results.txt` in the session scratchpad. Each run starts from a fresh
copy of one base repo. That repo has `base` (tag `b0`), then three commits `E1`, `E2`
and `E3` (tag `orig`). E1 and E2 touch `sub/x.txt`, E3 touches `top.txt`. Each event
commit carries all five carriers at the same time:

| Key | Carrier | How it was made |
|---|---|---|
| **H** | Extra commit header `cx-event <id>` placed after the `committer` line (the same place jj puts `change-id`) | `git cat-file commit` → awk inserts the line → `git hash-object -t commit -w --stdin` (no `--literally` needed) |
| **T** | Message trailer `CX-Event: <id>` in the last paragraph | `git commit -m … -m "CX-Event: <id>"` |
| **N** | git note on `refs/notes/cx` containing `cx-event: <id>` | `git notes --ref=cx add` |
| **R** | Out-of-band ref `refs/cx/events/<id>` → commit | `git update-ref` |
| **F** | Tree file `.cx/events/<id>` | committed file |

The script's probe prints one line per commit: `sha subject H=[…] T=[…] N=[…] R=[…] F=[…]`.
`T` is read with `%(trailers:key=CX-Event,valueonly)`, so it reports what git
actually parses as a trailer. `R` lists the `refs/cx` refs that point at *this*
commit. `F` lists the `.cx/events/*` entries in the tree, and the list is cumulative.

Legend: **kept** = preserved on the new commit · **DROP** = gone from the new commit ·
**orphan** = the data still exists but is attached to the old SHA, so from the new
commit's point of view it is gone · **mangled** = present but no longer parses the
same way · **dup** = the same value ends up on two or more commits · **merged** =
values from several commits combined.

## 1. Results matrix

| Operation | H header | T trailer | N note (refs/notes/cx) | R refs/cx/* | F tree file |
|---|---|---|---|---|---|
| `git commit --amend` (msg or content change) | **kept** (copied by amend) | kept if the new message keeps it | orphan by default; **kept** with `notes.rewriteRef=refs/notes/cx` | orphan (ref still points at the old commit) | kept |
| `git rebase` onto a new base (merge backend) | **DROP** | kept | orphan by default; kept with `notes.rewriteRef` | orphan | kept |
| `git rebase --apply` | DROP | kept | kept with rewriteRef | orphan | kept |
| `git rebase --force-rebase` onto the same base | DROP | kept | kept with rewriteRef | orphan | kept |
| `git rebase` with nothing to do (fast-forward picks) | kept (same SHA) | kept | kept | kept | kept |
| `rebase -i` squash+fixup, first pick **not** rewritten (same base) | **kept** for the first commit only (squash is an amend) | **mangled**: E1's trailer is now mid-body, only `E2` parses; fixup's `E3` text is discarded | **merged** (concatenated E1+E2+E3) with rewriteRef, else orphan | orphan | kept (all) |
| `rebase -i` squash+fixup onto a new base | DROP | mangled (same as above) | merged with rewriteRef | orphan | kept |
| `rebase -i` with `exec git commit --amend` after each pick | kept only on the first commit (fast-forwarded, then amended); DROP on the rest | kept | **orphan even with rewriteRef** on every rewritten pick (see surprise 4) | orphan | kept |
| `git cherry-pick` | DROP | kept | orphan (cherry-pick does not copy notes, even with rewriteRef) | orphan | kept |
| `git cherry-pick -x` | DROP | kept. `(cherry picked from commit …)` is appended **inside** the trailer block and still parses | orphan | orphan | kept |
| `git merge --squash` + commit (default msg) | DROP | **mangled**: SQUASH_MSG indents every message by 4 spaces, so zero trailers parse | DROP | DROP | kept (union) |
| `git merge --squash` + `commit -m` | DROP | DROP | DROP | DROP | kept (union) |
| `git merge --no-ff` (true merge) | kept (the originals are ancestors) | kept on the originals; the merge commit has none | kept | kept | kept |
| `git revert <E2>` | not on the revert commit | not copied (`This reverts commit <sha>.`) | not copied | — | **removed**: the revert deletes `.cx/events/E2` from the tree |
| `git format-patch --notes=cx` \| `git am` | DROP | kept | DROP (note text goes below `---` and am discards it) | DROP | kept (the file is in the diff) |
| `git filter-branch --msg-filter cat` (no-op filter) | **DROP** | kept | orphan (`refs/notes/cx` is never rewritten) | kept only if the ref is in the rewritten set (`-- --all`) | kept |
| `git filter-branch --subdirectory-filter sub -- --all` | DROP | kept | orphan (`WARNING: Ref 'refs/notes/cx' is unchanged`) | **remapped**, and E3's ref moved to the E2 commit because E3 was pruned (dup) | **DROP**: `.cx/` is outside `sub/` |
| `git fast-export --all \| git fast-import` (the filter-repo engine) | **DROP** (the stream format has no field for extra headers) | kept | notes commits re-imported byte-identical, but they are keyed by the **old** SHAs → orphan | kept (refs are exported) | kept |
| `git subtree split --prefix=sub` | DROP (the SHAs match the filter-branch output exactly) | kept | orphan | not moved | DROP (outside the prefix) |
| `git subtree add` (non-squash) | the split commits come in as-is (already stripped) | kept | — | — | under `vendor/` |
| `git subtree add --squash` | DROP | DROP; only `git-subtree-dir:` / `git-subtree-split:` trailers | — | — | under `vendor/` |
| `git push <bare> main` | **kept byte-identical** (same SHA) | kept | **not pushed** | **not pushed** | kept |
| `git push … 'refs/notes/*:refs/notes/*' 'refs/cx/*:refs/cx/*'` | — | — | pushed | pushed | — |
| `git push --all` / `--mirror` | — | — | `--all`: no; `--mirror`: yes | `--all`: no; `--mirror`: yes | — |
| `git clone` (plain / file:// / `--depth 1` / `--filter=blob:none`) | kept | kept | **not fetched** | **not fetched** | kept |
| `git fetch origin 'refs/notes/*:refs/notes/*' 'refs/cx/*:refs/cx/*'` | — | — | fetched | fetched | — |
| `git clone --mirror` | kept | kept | fetched | fetched | kept |
| `git bundle create --all` | kept | kept | in the bundle | in the bundle | kept |
| `git clone <bundle>` | kept | kept | **not** created locally (only heads and tags) | **not** created locally | kept |
| `git replace <orig> <plain>` | hidden (`log`/`cat-file` show the replacement); still there with `--no-replace-objects` | shown from the replacement | still attached to the original SHA, and **displayed** on the replacement | still points at the original SHA | from the replacement |
| jj import of git commits (`jj git init --colocate`, `jj git clone`) | kept: import does not rewrite anything (same SHA) | jj parses it (`trailers` template → `CX-Event=E3`) | untouched | untouched | kept |
| `jj describe` | **DROP**; jj writes `change-id` in its place | kept if in the new message | orphan | orphan | kept |
| `jj rebase -s` | DROP (+ `change-id`; on conflict also `jj:trees` and `jj:conflict-labels` headers) | kept | orphan | orphan (jj never moves non-bookmark refs) | kept |
| `jj squash --from X --into Y -u` | DROP | source trailer DROP (destination message kept) | orphan | orphan | kept (union) |
| `jj squash` (combined description) | DROP | **mangled**: the destination's trailer ends up mid-body, only the source's `E3` parses | orphan | orphan | kept |
| `jj split -m "part 1" <paths>` | DROP on both halves | part 1: DROP; part 2 (remaining): kept | orphan | orphan | the file goes to whichever half gets the path |
| `jj split` (editor keeps the description) | DROP on both | **dup**: both halves carry `CX-Event: E1` | orphan | orphan | as above |
| `jj abandon` (middle commit) | the descendants get rewritten → DROP | kept | orphan | orphan | the abandoned commit's file disappears from the descendants |
| `jj edit` + working-copy change (auto-rebase of descendants) | DROP on the edited commit **and all descendants** | kept | orphan | orphan | kept |
| jj commit that carries a foreign `cx-event` plus jj's `change-id`, then `jj describe` | **cx-event DROP**, change-id kept | — | — | — | — |
| `jj git push -b` | kept byte-identical (`cx-event` and `change-id`) | kept | not pushed | not pushed | kept |
| `jj git clone` / `jj git fetch` from a remote that has notes and `refs/cx/*` | kept | kept | **not fetched** | **not fetched** | kept |
| git ops on a jj-made commit: `git commit --amend` | `change-id` **kept** | | | | |
| git ops on a jj-made commit: `git rebase` | `change-id` **DROP** | | | | |

### Summary by carrier

- **H (extra header).** Survives only when the object is left alone: push, fetch,
  clone, bundle, true merges, and jj import. `git commit --amend` is the one rewrite
  that keeps it. Every other rewrite drops it: rebase, cherry-pick, filter-*,
  fast-export, am, and every jj rewrite. jj keeps only its *own* headers.
- **T (trailer).** The most durable carrier across rewrites (rebase, cherry-pick,
  am, filter, jj). It breaks on anything that *combines* messages: squash/fixup,
  `merge --squash`, and jj squash all leave earlier trailers mid-body, where they
  stop parsing. It is also lost when a message is replaced (`-m`, split `-m`).
  `jj split` duplicates it.
- **N (notes).** Keyed by SHA, so every rewrite orphans it unless
  `notes.rewriteRef` is set, and even then only amend and rebase copy it
  (cherry-pick, am, filter and jj do not). Never transferred by default push,
  fetch or clone.
- **R (custom refs).** Keyed by SHA and never moved by rewrites. filter-branch
  `-- --all` is the exception, and it can remap a ref onto an ancestor. Never
  transferred by default. They do keep old commits reachable and protected from gc.
- **F (tree file).** Survives every rewrite and every transport (it is content).
  It merges by union on squash. It is lost in two cases: path filtering to a
  subdirectory, and `git revert` (which deletes it, logically correct but a trap).
  It shows up as a file in diffs, patches and archives.

## 2. Observed snippets

Base object with a custom header. `git fsck --strict --full` printed **nothing**,
and plain `git hash-object` (without `--literally`) accepted the object:

```
tree 4649026d…
parent ad69b9c7…
author A <a@x> 1700000000 +0000
committer C <c@x> 1700000000 +0000
cx-event E3

change E3
…
CX-Event: E3
```

Display: `git log --format=raw` shows `cx-event E3`. `--pretty=fuller` and default
`git log` do **not**. There is no `%(…)` placeholder for arbitrary headers, so the
only way to read one is `git cat-file commit` or `--format=raw`. `git log --notes=cx`
shows a `Notes (cx):` section. `%(trailers)` shows `CX-Event: E3`.

Duplicate and odd headers: one commit with `cx-event E1` / `cx-event E9` / a folded
multi-line `cx-multi line1\n continuation` / `cx-event E1`, plus a *second* commit
that also carries `cx-event E1`. fsck --strict stayed clean (it reported only
dangling objects). Two commits sharing a header value is not an issue. A header
placed *before* `tree` is refused unless you pass `--literally`:
`error: object fails fsck: missingTree: invalid format - expected 'tree' line / fatal: refusing to create malformed object`.
With `--literally` the object is written, and fsck then reports
`error: bogus commit object …`.

Rebase (default config):
```
74039371 change E1   H=[] T=[E1] N=[] R=[] F=[E1]
```
Same with `notes.rewriteRef=refs/notes/cx`: `N=[cx-event: E1]`.

Amend keeps the header:
```
38d7bf39 change E3 amended   H=[E3] T=[E3] N=[] R=[] F=[E1,E2,E3]
121ff7ca E3 again            H=[E3] T=[] N=[cx-event: E3] R=[] F=[E1,E2,E3]   (rewriteRef set, message replaced)
```

`rebase -i` squash E2 into E1 plus fixup E3, onto a new base:
```
07847d08 change E1   H=[] T=[E2] N=[cx-event: E1,,cx-event: E2,,cx-event: E3] R=[] F=[E1,E2,E3]
```
Resulting message: `change E1 / Body… / CX-Event: E1 / (blank) / change E2 / Body… / CX-Event: E2`.
Only the last paragraph counts as trailers.

`cherry-pick -x` message tail. `interpret-trailers --parse` still returns `CX-Event: E3`:
```
CX-Event: E3
(cherry picked from commit d312eae043e3a49e10c960da15fd9d9f7c53d7d2)
```
`git interpret-trailers --trailer "CX-Event: E3"` on that message **adds a duplicate
line** (the default `if-exists=addIfDifferentNeighbor` sees the `(cherry picked…)`
line as the neighbour). `--if-exists replace --trailer "CX-Event: E4"` removes the
old line and appends at the end. `git commit -s` appends `Signed-off-by:` inside
the same trailer block.

`git merge --squash` SQUASH_MSG (the indentation kills trailer parsing):
```
Squashed commit of the following:

commit d312eae0…
Author: A <a@x>
Date:   …

    change E3
    …
    CX-Event: E3
```
→ `0edb4cad Squashed commit of the following: H=[] T=[] N=[] R=[] F=[E1,E2,E3]`

filter-branch subdirectory-filter:
```
Ref 'refs/cx/events/E3' was rewritten
WARNING: Ref 'refs/notes/cx' is unchanged
3053128b change E2   H=[] T=[E2] N=[] R=[cx/events/E2,cx/events/E3] F=[]
```

fast-export stream: it contains `commit refs/notes/cx` blocks, but no `cx-event`
header anywhere. After fast-import:
`eb994925 change E1 H=[] T=[E1] N=[] R=[cx/events/E1] F=[E1]`. That SHA is the same
one that am, cherry-pick and filter-branch produce, because they all produce the
same header-stripped object.

Transport: `git push ../bare.git main` → the bare repo has only `refs/heads/main`,
and `base main=d312eae0… bare main=d312eae0…` (identical). A plain clone has only
heads, remotes and HEAD. `git bundle list-heads` includes `refs/cx/events/*` and
`refs/notes/cx`, but `git clone all.bundle` creates only `refs/heads`, `refs/tags`
and `refs/remotes/origin/*`.

Signatures (SSH): an extra header placed *before* `gpgsig` and signed together
with the rest of the object → `Good "git" signature for sig@x with ED25519 key…`.
Injecting a header into an already-signed commit →
`Signature verification failed: incorrect signature`. The signature covers
every header except `gpgsig` itself.

jj describe on an imported commit that carries `cx-event`:
```
committer J <j@x> 1790551083 +1300
change-id volonpwlkqoqokpruoztqwzrsqxunsnx
```
(`cx-event` is gone.) The change-id of an imported header-less commit is derived
deterministically from its commit ID: `volonpwlkqoq` for `d312eae0` in two
independent repos. jj rebase with conflicts adds its own headers:
`jj:conflict-labels …` and `jj:trees <t1> <t2> <t3>`.

jj split with the description kept → `0564ee9 change E1 | E1` and `ef51734 change E1 | E1`.

jj commit carrying both headers, before and after `jj describe`:
```
cx-event BOTH1                              change-id wrvmtwrt…
change-id wrvmtwrtumynnpzzuqxsvkxsvsyovzpu  (cx-event dropped)
```

## 3. Surprises / design-relevant findings

1. **`git commit --amend` preserves unknown extra headers. Nothing else in git
   does.** amend copies every header except `gpgsig`/`mergetag`. That makes the
   header look durable in casual testing, but a single `git rebase` strips it.
   It explains why jj change-ids survive amend but not rebase in git.
2. **The first pick of an interactive squash keeps the header** when the base is
   unchanged, because git fast-forwards that pick and then amends it. The same
   todo list onto a new base drops it. Header survival therefore depends on
   whether the picks were fast-forwarded.
3. **The squash family turns trailers into non-trailers.** git `rebase -i`
   squash, `git merge --squash`, and `jj squash` all concatenate messages, so
   every trailer except the last paragraph's stops parsing. Fixup silently
   discards its commit's trailer.
4. **`notes.rewriteRef` + `exec git commit --amend` inside a rebase loses notes.**
   The amend runs before the rebase copies notes for the pick, so the chain
   orig→pick→amend breaks. Notes also never follow cherry-pick or am.
5. **filter-branch can remap `refs/cx/*` onto a different commit.** When the
   target commit is pruned, its ref moves to the nearest ancestor, which silently
   attributes E3 to E2's commit.
6. **`git revert` deletes the tree-file carrier.** A revert commit therefore
   "un-records" the event unless the tool handles it.
7. **Bundles contain notes and custom refs, but cloning a bundle drops them.**
8. **jj never preserves foreign headers.** It keeps only `change-id` (and its
   `jj:*` conflict headers) on rewrite. jj does keep trailers, and its template
   language parses them natively.
9. `git hash-object` (non-literal) runs fsck checks, and unknown headers pass
   them. Headers must go after `committer`. If signing, they must go before
   `gpgsig` and be written before the signature is created.

## 4. GitHub behaviour — UNVERIFIED (from knowledge, not tested)

| Aspect | Expected behaviour |
|---|---|
| Pushing commits with custom headers | Accepted and stored byte-identical, so the SHA is unchanged. GitHub runs fsck-like checks on receive (it rejects things like bad timezones and malformed idents), but unknown headers after `committer` pass. jj users push `change-id` headers to GitHub routinely. |
| "Create a merge commit" | The PR commits are kept as-is (all carriers intact on them). A new merge commit is authored by the merging user, committer `GitHub <noreply@github.com>`, signed by GitHub's web-flow key. Default message: `Merge pull request #N from owner/branch` + blank line + PR title. The repo setting can switch this to PR title or PR title+description. No custom headers on the merge commit. |
| "Squash and merge" | One new commit with committer `GitHub`, signed by web-flow. H is dropped. N and R are orphaned. F survives (union). Default message (the "default message" setting): title `<PR title> (#N)`. For a single-commit PR, the body is that commit's message. For multi-commit PRs, the body is a bulleted list `* <subject>\n\n<body>…` of every commit message, so earlier trailers end up inside the body and **do not parse as trailers**. GitHub gathers `Co-authored-by:` trailers from the squashed commits (and from commit authors who differ from the merger) and appends them, deduplicated, as a trailer block at the end. As far as I know it does not do this for other trailer keys. The repo setting can use "PR title and description" instead, which discards the commit messages entirely. The merger can edit the message in the UI. |
| "Rebase and merge" | GitHub docs say it **always** creates new SHAs and updates committer info, even when a fast-forward was possible. Committer becomes GitHub (web-flow signed). Custom headers are dropped, as reported by jj users losing change-id. Trailers are kept. Notes and refs are orphaned. The original gpg signatures are replaced. |
| "Update branch" button | The merge variant adds a merge commit (the branch commits keep their carriers). The rebase variant recreates the commits and drops H. |
| Web-UI edits / suggestions | New web-flow commits with no custom headers. You can type trailers into the commit message box. |
| `refs/notes/*` push | Accepted and stored. GitHub stopped displaying notes in the UI in 2014. |
| Arbitrary `refs/cx/*` push | Accepted (any namespace except the read-only `refs/pull/*`). Not shown in the UI and not reachable through the branch/tag APIs. Readable through `git ls-remote` and the Git refs API (`/git/matching-refs/cx/`). |
| Plain clone / fetch | Only `refs/heads/*` and tags, as with local git. Notes and `refs/cx/*` need explicit refspecs. `--mirror` gets them (plus `refs/pull/*`). |
| Forks | A fork shares the object network, but only branches (and tags) are exposed in the fork. Notes and custom refs are not copied into the fork's ref namespace. |
| Archive downloads (zip/tarball) | Tree only, so **F survives** (unless `.gitattributes export-ignore`). H, T, N and R are absent. |
| Signature verification | GitHub verifies over the whole object minus `gpgsig`, so an extra header written before signing verifies as "Verified". jj-signed commits carrying `change-id` show Verified. A header injected after signing → "Unverified". |
| UI display | H: invisible. T: shown as plain message text, except that `Co-authored-by:` renders co-author avatars (and `Signed-off-by` is plain text). N: not shown. R: not shown. F: visible as a file in the tree and in PR diffs, which is noisy in review unless marked `linguist-generated`. |
| Server-side rewrites | Squash and rebase merges are the main risk. Carriers that must survive the merge buttons: F always; T only for single-commit squash or rebase-merge; H only with a merge commit. |


## Appendix A — reproduction script (`run.sh`)

Run with `bash run.sh > results.txt 2>&1`. It creates `./work/`, sets a private `HOME`, `GIT_CONFIG_NOSYSTEM=1` and `JJ_CONFIG`, and touches nothing else.

```bash
#!/usr/bin/env bash
# Empirical survival test of 5 commit-metadata carriers across git/jj operations.
# Usage: bash run.sh > results.txt 2>&1
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)/work"
rm -rf "$ROOT"; mkdir -p "$ROOT"; cd "$ROOT"
export HOME="$ROOT/home"; mkdir -p "$HOME"
export GIT_CONFIG_NOSYSTEM=1 GIT_AUTHOR_NAME=A GIT_AUTHOR_EMAIL=a@x GIT_COMMITTER_NAME=C GIT_COMMITTER_EMAIL=c@x
export GIT_AUTHOR_DATE="1700000000 +0000" GIT_COMMITTER_DATE="1700000000 +0000"
git config --global init.defaultBranch main
git config --global advice.detachedHead false
cat > "$HOME/jjconfig.toml" <<'EOF'
user.name = "J"
user.email = "j@x"
ui.editor = "true"
ui.paginate = "never"
[revset-aliases]
"immutable_heads()" = "none()"
EOF
export JJ_CONFIG="$HOME/jjconfig.toml"

hr(){ echo; echo "=================== $* ==================="; }
run(){ echo "\$ $*"; "$@" 2>&1 | sed 's/^/    /'; }

# carriers(): one line per commit in <range>: sha subject | H(header) T(trailer) N(note) R(refs/cx) F(.cx file)
carriers(){
  local range="$1"
  for c in $(git rev-list --reverse "$range"); do
    local subj H T N R F
    subj=$(git log -1 --format=%s "$c")
    H=$(git cat-file commit "$c" | sed -n '/^$/q;s/^cx-event //p' | paste -sd, -)
    T=$(git log -1 --format='%(trailers:key=CX-Event,valueonly,separator=%x2C)' "$c")
    N=$(git notes --ref=cx show "$c" 2>/dev/null | paste -sd, -)
    R=$(git for-each-ref --points-at "$c" --format='%(refname:short)' refs/cx | paste -sd, -)
    F=$(git ls-tree -r --name-only "$c" -- .cx 2>/dev/null | sed 's#.cx/events/##' | paste -sd, -)
    printf '    %s %-28s H=[%s] T=[%s] N=[%s] R=[%s] F=[%s]\n' "${c:0:8}" "$subj" "$H" "$T" "$N" "$R" "$F"
  done
}

# add_header <commit> <value>: rewrite commit object inserting "cx-event <value>" after committer line
add_header(){
  local c="$1" v="$2"
  git cat-file commit "$c" | awk -v v="$v" '{print} /^committer / && !d {print "cx-event " v; d=1}' \
    | git hash-object -t commit -w --stdin
}

# mk_event <id> <file> <content>: create a commit on HEAD carrying all 5 carriers for event <id>
mk_event(){
  local id="$1" f="$2" content="$3"
  mkdir -p "$(dirname "$f")" .cx/events
  echo "$content" >> "$f"; echo "event $id" > ".cx/events/$id"
  git add -A
  git commit -q -m "change $id" -m "Body text for $id." -m "CX-Event: $id"
  local new; new=$(add_header HEAD "$id")
  git update-ref HEAD "$new"
  git notes --ref=cx add -m "cx-event: $id" HEAD
  git update-ref "refs/cx/events/$id" HEAD
}

hr "SETUP base repo"
git init -q base; cd base
mkdir sub; echo base > sub/x.txt; echo base > top.txt; git add -A; git commit -q -m "base"
git tag b0
mk_event E1 sub/x.txt one
mk_event E2 sub/x.txt two
mk_event E3 top.txt three
git tag orig
echo "\$ git cat-file commit HEAD"; git cat-file commit HEAD | sed 's/^/    /'
carriers b0..HEAD
run git fsck --strict --full
run git log -1 --format='%H%n(raw headers:)%n%B' HEAD
echo "\$ git log -1 --format=raw | head   (does git log show extra headers?)"
git log -1 --format=raw HEAD | sed 's/^/    /'
echo "\$ git log -1 --pretty=fuller (extra headers visible?)"; git log -1 --pretty=fuller HEAD | sed 's/^/    /'
echo "\$ git log -1 --format='%(trailers)'"; git log -1 --format='%(trailers)' | sed 's/^/    /'
echo "\$ git log --notes=cx -1"; git log -1 --notes=cx HEAD | sed 's/^/    /'
cd ..

fresh(){ rm -rf "$1"; cp -a base "$1"; cd "$1"; }

hr "fsck/--literally/duplicate values"
fresh t_fsck
# a second, unrelated commit with the SAME header value E1
dup=$(git commit-tree -p HEAD -m "dup E1" HEAD^{tree}); dup=$(add_header $dup E1); git update-ref refs/heads/dup $dup
# header with weird value / multiple same-name headers
w=$(git cat-file commit $dup | awk '{print} /^committer /{print "cx-event E1"; print "cx-event E9"; print "cx-multi line1"; print " continuation"}' | git hash-object -t commit -w --stdin)
git update-ref refs/heads/weird $w
run git fsck --strict --full
echo "\$ carriers for dup/weird"; carriers HEAD..weird
run git cat-file commit weird
# header placed BEFORE tree line (malformed ordering)
bad=$(git cat-file commit HEAD | awk 'NR==1{print "cx-event E1"} {print}' | git hash-object -t commit -w --stdin --literally)
run git fsck --strict --full
run git cat-file -t $bad
echo "(non-literal hash-object of same bad object:)"; git cat-file commit HEAD | awk 'NR==1{print "cx-event E1"} {print}' | git hash-object -t commit -w --stdin 2>&1 | sed 's/^/    /'
run git log -1 --format='%(trailers:key=CX-Event)' dup
cd ..

hr "git commit --amend (default, then notes.rewriteRef set)"
fresh t_amend
run git commit --amend -q -m "change E3 amended" -m "CX-Event: E3"
carriers b0..HEAD
git reset -q --hard orig
git config notes.rewriteRef refs/notes/cx
echo x >> top.txt; git add top.txt
run git commit --amend -q --no-edit
carriers b0..HEAD
echo "(amend preserves header with different committer date too:)"
GIT_COMMITTER_DATE="1800000000 +0000" run git commit --amend -q -m "E3 again"
carriers b0..HEAD
cd ..

hr "git rebase (plain, onto new base) default config"
fresh t_rebase
git checkout -q -b newbase b0; echo nb > nb.txt; git add nb.txt; git commit -q -m newbase
git checkout -q main
run git rebase newbase
carriers newbase..HEAD
echo "(rebase --force-rebase with notes.rewriteRef=refs/notes/cx)"
git reset -q --hard orig; git config notes.rewriteRef refs/notes/cx
run git rebase newbase
carriers newbase..HEAD
echo "(rebase with nothing to do / fast-forwardable: rebase b0 when already on b0 — no rewrite)"
git reset -q --hard orig
run git rebase b0
carriers b0..HEAD
echo "(rebase --force-rebase b0: rewrite in place)"
run git rebase --force-rebase b0
carriers b0..HEAD
echo "(rebase --merge -r vs --apply backend)"
git reset -q --hard orig
run git rebase --apply newbase
carriers newbase..HEAD
cd ..

hr "git rebase -i squash/fixup"
fresh t_rebase_i
git config notes.rewriteRef refs/notes/cx
# squash E2 into E1, fixup E3 into result
export GIT_SEQUENCE_EDITOR="sed -i -e '2s/^pick/squash/' -e '3s/^pick/fixup/'"
export GIT_EDITOR=true
run git rebase -i b0
carriers b0..HEAD
echo "\$ git log -1 --format=%B"; git log -1 --format=%B | sed 's/^/    /'
run git log -1 --format='%(trailers:key=CX-Event)'
echo "\$ notes on squashed commit:"; git notes --ref=cx show HEAD 2>&1 | sed 's/^/    /'
echo "(same squash/fixup but ONTO A NEW BASE so the first pick is rewritten too)"
git reset -q --hard orig
git branch nb b0; git checkout -q nb; echo nb > nb.txt; git add nb.txt; git commit -q -m newbase; git checkout -q main
run git rebase -i nb
carriers nb..HEAD
unset GIT_SEQUENCE_EDITOR
echo "(same squash w/ default notes config: notes.rewriteRef unset)"
git reset -q --hard orig; git config --unset notes.rewriteRef
GIT_SEQUENCE_EDITOR="sed -i -e '2s/^pick/squash/'" run git rebase -i b0
carriers b0..HEAD
echo "(notes.rewrite.mode concatenate is default; try ignore / overwrite for reference)"
echo "(reword via exec: rebase -i with 'exec git commit --amend' on each commit)"
git reset -q --hard orig; git config notes.rewriteRef refs/notes/cx
GIT_SEQUENCE_EDITOR="sed -i -e '/^pick/a exec GIT_COMMITTER_DATE=1800000000 git commit --amend -q --no-edit'" run git rebase -i b0
carriers b0..HEAD
unset GIT_EDITOR
cd ..

hr "git cherry-pick (plain, -x) and interpret-trailers"
fresh t_cp
git config notes.rewriteRef refs/notes/cx
git checkout -q -b other b0
run git cherry-pick orig~2
carriers b0..HEAD
echo "(cherry-pick with notes.rewriteRef set: are notes copied? cherry-pick is not a 'rewrite')"
run git cherry-pick -x orig
carriers b0..HEAD
echo "\$ git log -1 --format=%B"; git log -1 --format=%B | sed 's/^/    /'
echo "\$ git log -1 --format=%B | git interpret-trailers --parse"; git log -1 --format=%B | git interpret-trailers --parse | sed 's/^/    /'
echo "\$ git log -1 --format='%(trailers:only,unfold)'"; git log -1 --format='%(trailers:only,unfold)' | sed 's/^/    /'
echo "(add trailer to cherry-picked msg with interpret-trailers --trailer CX-Event: E3b, where=end)"
git log -1 --format=%B | git interpret-trailers --trailer "CX-Event: E3b" | sed 's/^/    /'
echo "(if-exists defaults: add trailer that already exists identically -> addIfDifferentNeighbor)"
git log -1 --format=%B | git interpret-trailers --trailer "CX-Event: E3" | sed 's/^/    /'
echo "(--if-exists replace)"; git log -1 --format=%B | git interpret-trailers --if-exists replace --trailer "CX-Event: E4" | sed 's/^/    /'
echo "(trailer block broken by a non-trailer line after it)"
printf 'subj\n\nbody\n\nCX-Event: E1\nnot a trailer line\n' | git interpret-trailers --parse | sed 's/^/    [parse] /'
printf 'subj\n\nbody\n\nCX-Event: E1\nSigned-off-by: X <x@x>\n' | git interpret-trailers --parse | sed 's/^/    [parse] /'
echo "(commit -s adds Signed-off-by into same trailer block?)"
git reset -q --hard orig; git commit -q --amend -s --no-edit; git log -1 --format=%B | sed 's/^/    /'
cd ..

hr "git merge --squash"
fresh t_squash
git checkout -q -b tgt b0
run git merge --squash orig
echo "\$ cat .git/SQUASH_MSG"; sed 's/^/    /' .git/SQUASH_MSG
GIT_EDITOR=true git commit -q
carriers b0..HEAD
run git log -1 --format='%(trailers:key=CX-Event)'
echo "(git merge --squash + commit -m 'x')"
git reset -q --hard b0; git merge -q --squash orig; git commit -q -m "squashed"; carriers b0..HEAD
cd ..

hr "git merge (true merge commit, --no-ff)"
fresh t_merge
git checkout -q -b tgt b0; echo m > m.txt; git add m.txt; git commit -q -m tgtwork
run git merge --no-ff --no-edit orig
carriers b0..HEAD
run git log -1 --format=%B
cd ..

hr "git revert"
fresh t_revert
run git revert --no-edit orig~1
carriers orig..HEAD
run git log -1 --format=%B
run git ls-tree -r --name-only HEAD .cx
cd ..

hr "git format-patch | git am"
fresh t_am
git checkout -q -b tgt b0
git format-patch -q --notes=cx -o ../patches b0..orig
echo "\$ head of patch 1"; sed -n '1,30p' ../patches/0001-*.patch | sed 's/^/    /'
run git am ../patches/*.patch
carriers b0..HEAD
cd ..

hr "git filter-branch --subdirectory-filter sub"
fresh t_fb
FILTER_BRANCH_SQUELCH_WARNING=1 run git filter-branch -f --subdirectory-filter sub -- --all
echo "(branch main after:)"; carriers main; echo "(refs/cx after filter-branch -- --all:)"; git for-each-ref refs/cx refs/notes | sed 's/^/    /'
run git cat-file commit main
echo "(filter-branch --msg-filter cat, no path filter, on main only)"
cd ..; fresh t_fb2
FILTER_BRANCH_SQUELCH_WARNING=1 run git filter-branch -f --msg-filter cat -- b0..main
carriers b0..main
cd ..

hr "git fast-export | fast-import (the engine git-filter-repo uses)"
fresh t_fe
git fast-export --all --signed-tags=strip > ../fe.stream
echo "\$ grep -n 'cx-event\\|^commit \\|^data' fe.stream | head"; grep -n 'cx-event\|^commit \|^data\|notes' ../fe.stream | head -20 | sed 's/^/    /'
cd ..; rm -rf t_fi; git init -q t_fi; cd t_fi
run git fast-import --quiet < ../fe.stream
carriers b0..main 2>/dev/null || carriers main
git for-each-ref | sed 's/^/    /'
cd ..

hr "git subtree split / add"
fresh t_subtree
run git subtree split --prefix=sub -b subonly orig
carriers subonly
echo "(git subtree add into another repo)"
cd ..; rm -rf t_subtree_host; git init -q t_subtree_host; cd t_subtree_host; echo h > h; git add h; git commit -q -m host
run git subtree add --prefix=vendor ../t_subtree subonly
git log --format='%h %s' | sed 's/^/    /'
carriers HEAD~1..HEAD
echo "(subtree add --squash)"
cd ..; rm -rf t_subtree_host2; git init -q t_subtree_host2; cd t_subtree_host2; echo h > h; git add h; git commit -q -m host
run git subtree add --squash --prefix=vendor ../t_subtree subonly
git log --format='%h %s' --all | sed 's/^/    /'; run git log -1 --format=%B HEAD^2
cd ..

hr "git replace"
fresh t_replace
plain=$(git cat-file commit orig | grep -v '^cx-event' | sed 's/^change E3$/change E3 (replaced)/' | git hash-object -t commit -w --stdin)
run git replace orig $plain
carriers orig~1..orig
run git log -1 --format='%H %s' orig
echo "\$ git --no-replace-objects cat-file commit orig | grep cx"; git --no-replace-objects cat-file commit orig | grep cx | sed 's/^/    /'
run git rev-parse orig
cd ..

hr "clone / fetch / push to local bare"
rm -rf bare.git; git init -q --bare bare.git
cd base
run git push ../bare.git main
echo "(refs in bare after plain 'git push <url> main')"; git -C ../bare.git for-each-ref | sed 's/^/    /'
echo "(header byte-identical in bare? sha equal)"; git -C ../bare.git cat-file commit main | grep cx-event | sed 's/^/    /'; echo "    base main=$(git rev-parse main) bare main=$(git -C ../bare.git rev-parse main)"
run git push ../bare.git 'refs/notes/*:refs/notes/*' 'refs/cx/*:refs/cx/*'
git -C ../bare.git for-each-ref | sed 's/^/    /'
run git push --dry-run --all ../bare.git
run git push --dry-run --mirror ../bare.git
cd ..
rm -rf clone1; run git clone -q bare.git clone1; cd clone1
echo "(refs after plain clone)"; git for-each-ref | sed 's/^/    /'
carriers b0..main 2>/dev/null || carriers main
run git fetch origin 'refs/notes/*:refs/notes/*' 'refs/cx/*:refs/cx/*'
carriers main
cd ..
rm -rf clone2; run git clone -q --mirror bare.git clone2.git; echo "(refs after --mirror clone)"; git -C clone2.git for-each-ref | sed 's/^/    /'
echo "(clone with --no-local / file:// transport)"; rm -rf clone3; git clone -q file://$ROOT/bare.git clone3; git -C clone3 cat-file commit main | grep cx-event | sed 's/^/    /'
echo "(shallow clone depth 1)"; rm -rf clone4; git clone -q --depth 1 file://$ROOT/bare.git clone4; git -C clone4 cat-file commit main | grep cx-event | sed 's/^/    /'
echo "(partial clone --filter=blob:none)"; rm -rf clone5; git clone -q --filter=blob:none file://$ROOT/bare.git clone5; git -C clone5 cat-file commit main | grep cx-event | sed 's/^/    /'

hr "git bundle"
cd base
run git bundle create ../all.bundle --all
run git bundle list-heads ../all.bundle
run git bundle create ../main.bundle main
cd ..; rm -rf fromb; run git clone -q all.bundle fromb; git -C fromb for-each-ref | sed 's/^/    /'
git -C fromb cat-file commit origin/main | grep cx-event | sed 's/^/    /'

hr "signatures: ssh-signed commit with extra header"
fresh t_sig
ssh-keygen -q -t ed25519 -N '' -f ../sigkey -C sig@x
echo "sig@x $(cat ../sigkey.pub)" > ../allowed
git config gpg.format ssh; git config user.signingkey ../sigkey.pub; git config gpg.ssh.allowedSignersFile ../allowed
# 1: header added, then signed manually (signature covers header)
body=$(git cat-file commit orig | grep -v '^cx-event' | awk '{print} /^committer /{print "cx-event SIG1"}')
printf '%s\n' "$body" > ../unsigned.txt
ssh-keygen -q -Y sign -n git -f ../sigkey ../unsigned.txt
signed=$(awk -v sigf=../unsigned.txt.sig 'BEGIN{while((getline l<sigf)>0){s=s (n++? "\n ":"") l}} {print} /^cx-event /{print "gpgsig " s}' ../unsigned.txt | git hash-object -t commit -w --stdin)
run git cat-file commit $signed
run git verify-commit $signed
# 2: header injected into an already signed commit (post-hoc) -> signature should fail
git commit -q --allow-empty -S -m "signed then header"
tampered=$(add_header HEAD SIG2)
run git verify-commit HEAD
run git verify-commit $tampered
echo "(git rebase of a signed commit w/o -S drops gpgsig; with header too)"
cd ..

hr "JJ: colocated repo importing git-made commits with custom header"
fresh t_jj
run jj git init --colocate
run jj log -r 'b0::' --no-graph -T 'commit_id.short() ++ " " ++ change_id.short() ++ " " ++ description.first_line() ++ "\n"'
echo "(did jj import rewrite any git commit? compare sha)"; echo "    orig=$(git rev-parse orig) main=$(git rev-parse main)"
echo "(does jj see the trailer?)"; run jj log -r main --no-graph -T 'trailers.map(|t| t.key() ++ "=" ++ t.value()).join(",") ++ "\n"'
echo "--- jj describe on E3 (tip)"
run jj describe -r main -m "change E3 described

CX-Event: E3"
run jj log -r main --no-graph -T 'commit_id ++ "\n"'
git -c core.bare=false log -1 --format=%H main | sed 's/^/    main=/'
echo "\$ git cat-file commit main"; git cat-file commit main | sed 's/^/    /'
carriers b0..main
cd ..

jjfresh(){ fresh "$1"; jj git init --colocate >/dev/null 2>&1; }
jjc(){ echo "    git: $(git log --format='%h %s' "$1" | head -5 | paste -sd'|' -)"; carriers "b0..$1"; }

hr "JJ rebase (move E2..E3 onto new base)"
jjfresh t_jj_rebase
git checkout -q -b nb b0 2>/dev/null; echo nb > nb.txt; git add nb.txt; git commit -q -m newbase; git checkout -q main 2>/dev/null
jj git import >/dev/null 2>&1
run jj rebase -s 'orig-' -o nb
jjc main
echo "\$ git cat-file commit main"; git cat-file commit main | sed 's/^/    /'
echo "(does jj preserve refs/cx after its rewrite? jj doesn't move non-branch refs)"; git for-each-ref refs/cx | sed 's/^/    /'
cd ..

hr "JJ squash (E3 into E2)"
jjfresh t_jj_squash
run jj squash --from orig --into 'orig-' -u
jjc main
run git log -1 --format=%B main
cd ..; jjfresh t_jj_squash2
echo "(squash with combined description: editor=true keeps combined template)"
run jj squash --from orig --into 'orig-'
run git log -1 --format=%B main
jjc main
cd ..

hr "JJ split (E1 into two commits by path)"
jjfresh t_jj_split
run jj split -r 'orig--' -m "split part 1" .cx
jjc main
run git log --format='%h %s' -4 main
cd ..
jjfresh t_jj_split2
echo "(split with editor=true: both halves keep original description?)"
run jj split -r 'orig--' .cx
run git log --format='%h %s | %(trailers:key=CX-Event,valueonly)' -5 main
cd ..

hr "JJ abandon (E2)"
jjfresh t_jj_abandon
run jj abandon 'orig-'
jjc main
cd ..

hr "JJ new+edit on top, then modify E1 content (auto-rebase descendants)"
jjfresh t_jj_edit
run jj edit 'orig--'
echo more >> top.txt
run jj status
jjc main
cd ..

hr "JJ: commits made by jj itself — header visible to git, survives git ops?"
jjfresh t_jj_native
run jj new main -m "jj-native commit

CX-Event: J1"
echo j > j.txt; jj status >/dev/null
run jj bookmark create jjb -r @
jj git export >/dev/null 2>&1
echo "\$ git cat-file commit jjb"; git cat-file commit jjb | sed 's/^/    /'
run git fsck --strict
echo "(git ops on jj-made commit, in a plain git clone so the jj working copy is untouched)"
cd ..; rm -rf t_jj_native_g; git clone -q t_jj_native t_jj_native_g; cd t_jj_native_g
git checkout -q -b gtmp origin/jjb
GIT_COMMITTER_DATE="1800000000 +0000" run git commit --amend -q --no-edit
git cat-file commit HEAD | grep -E 'change-id' | sed 's/^/    after amend: /'
run git rebase -q --force-rebase HEAD~1
git cat-file commit HEAD | grep -E 'change-id' | sed 's/^/    after rebase: /'; echo "    (no 'after rebase' line = dropped)"
cd ..

hr "JJ rewrite of a commit carrying BOTH jj change-id and a foreign cx-event header"
jjfresh t_jj_both
git -c core.bare=false show -s >/dev/null
jj new main -m "both headers" >/dev/null 2>&1; jj bookmark create both -r @ >/dev/null 2>&1; jj git export >/dev/null 2>&1
nc=$(add_header both BOTH1); git update-ref refs/heads/both $nc
run jj git import
run git cat-file commit both
run jj describe -r both -m "both headers, redescribed"
jj git export >/dev/null 2>&1
run git cat-file commit both
cd ..

hr "JJ git push / fetch to bare"
rm -rf jjbare.git; git init -q --bare jjbare.git
cd t_jj_native
git checkout -q --detach 2>/dev/null
run jj git remote add origin ../jjbare.git
run jj git push -b jjb
run jj git push -b main
git -C ../jjbare.git for-each-ref | sed 's/^/    /'
git -C ../jjbare.git cat-file commit jjb | grep -E 'change-id|cx-event' | sed 's/^/    bare jjb: /'
git -C ../jjbare.git cat-file commit main | grep -E 'change-id|cx-event' | sed 's/^/    bare main: /'
cd ..
rm -rf jjclone; run jj git clone --colocate jjbare.git jjclone; cd jjclone
git for-each-ref | sed 's/^/    /'
run jj log -r 'jjb@origin' --no-graph -T 'change_id ++ " " ++ commit_id.short() ++ "\n"'
run jj git fetch
echo "(same change-id across clone? — yes if header transferred)"
cd ..

hr "JJ fetch of commits with custom header, then rebase them (header survival across jj rewrite)"
rm -rf jjclone2; jj git clone --colocate bare.git jjclone2 >/dev/null 2>&1; cd jjclone2
echo "(refs after jj git clone of a remote that has refs/notes/cx and refs/cx/*)"; git for-each-ref | grep -v jj/keep | sed 's/^/    /'
run jj git fetch
git for-each-ref refs/notes refs/cx | sed 's/^/    after jj git fetch: /'
run jj log -r 'main' --no-graph -T 'change_id.short() ++ " " ++ commit_id.short() ++ "\n"'
git cat-file commit main | grep -E 'change-id|cx-event' | sed 's/^/    main: /'
run jj describe -r 'main' -m "E3 redescribed after fetch" --ignore-immutable
git cat-file commit main | grep -E 'change-id|cx-event' | sed 's/^/    main after: /'
cd ..

hr "DONE"
```
