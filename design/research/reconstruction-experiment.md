# Reconstruction experiment: a byte-exact, reversible shifted-copy encoding

Status: empirical results. Run 2026-09-28 with git 2.55.0, jj 0.44.0 and
Python 3.14 on Linux. This is feasibility evidence for `constraints.md` §4,
"R2′ definition", not product code. All work was done on copies of the bare
mirrors, in the session scratchpad under `recon/`. Nothing was pushed.

**Result.** For every commit reachable from any ref in the 7 real repos
(1,715 commits, including 115 merges, 7 root commits and 127 GitHub-signed
commits), ρ(t, `tools/<repo>`) was built into a separate composition object
store. Then t was rebuilt from ρ alone, reading only the composition store.
The rebuilt raw object was **byte-identical to t in 1,715 of 1,715 cases**, and
its SHA matched the SHA named in the trailer. The synthetic edge-case corpus
(72 commits), the nested ρ(ρ) run (72) and a 200,000-case random fuzz of the
codec also had zero failures.

## 1. Encoding spec (byte level)

Notation: all values are raw bytes. `LF` = 0x0A. "Header section" means
everything before the first `LF LF` in the raw commit object, and "body" means
everything after it. If a raw commit has no `LF LF`, it has **no body**. That
is different from an empty body.

### 1.1 Parsing a commit (both directions)

- If the raw object contains `LF LF`, split at the first one: header section
  = `raw[:i+1]` (keeps its final LF), and body `M` = `raw[i+2:]`. Otherwise
  the whole object is the header section, which must end in LF, and the body
  is absent.
- Split the header section into lines on LF. The final empty element is
  dropped. A line that starts with SP (0x20) is a continuation of the
  preceding header, and nothing else counts as a continuation. Each
  *header group* is one non-continuation line plus its continuation lines,
  all kept verbatim.
- A group's *key* is the bytes of its first line up to the first SP, or the
  whole line if there is no SP.
- Serialising is the exact inverse: each line of each group is followed by
  LF, then `LF` + body if a body is present.

`LF LF` cannot appear inside the header section, because continuation lines
start with SP. So the split is unambiguous. GitHub's `gpgsig` ends in a
continuation line that is exactly `" "`, and it survives because lines are
never trimmed.

### 1.2 Constants

- `PREFIX = "clonex-"`
- `META = "clonex"`, a reserved header key. Renaming always produces
  `clonex-<non-empty>`, so it can never produce `META`.
- `PASS = {tree, parent, author, committer, encoding}`
- `L = "Clonex-Source: " P SP hex(t)`. There is no LF in `L`. `P` must not
  contain LF, and it may contain SP, because the decoder splits at the
  *last* SP.

### 1.3 Encode ρ(t, P), given ρ's tree `T′` and parents `p′₁…p′ₙ`

Headers are handled group by group, **in t's original order**:

| t's group key | ρ's group |
|---|---|
| `tree` (exactly one, single-line) | `tree T′` |
| k-th `parent` | `parent p′ₖ` (same count and positions) |
| `author`, `committer`, `encoding` | verbatim |
| anything else `K` (`gpgsig`, `gpgsig-sha256`, `mergetag`, `change-id`, `clonex-…`, unknown) | first line prefixed with `clonex-`; continuation lines verbatim |

If t has **no body**, append one extra group `clonex no-body`, and treat `M`
as empty below.

Message: split `M = C ‖ W`, where `W` is the longest suffix of `M` made only
of LF bytes, so `C` never ends in LF. Then:

```
sep = ""      if C == ""
      LF      if C ends in a trailer block (§1.5)   -- join it
      LF LF   otherwise                             -- new paragraph
R   = C ‖ sep ‖ L ‖ W
```

ρ's raw object = serialised header groups ‖ `LF` ‖ `R`. ρ always has a body.

This means ρ's message ends with **exactly the same run of LFs as t's**: none
for GitHub's web-UI merges and squashes, one normally, and two for the 109
real commits whose messages end in a blank line. The trailer sits just before
that run.

### 1.4 Decode, given ρ's raw object, `T = tree(ρ):P` and reconstructed parents `q₁…qₙ`

1. Parse ρ (§1.1). If there is no body, ρ is not a representative.
2. `W` = the trailing LF run of `R`, and `R₀ = R[:-|W|]`. `last` = the bytes
   after the last LF in `R₀`, or all of `R₀` if it has no LF. If `last` does
   not start with `Clonex-Source: `, ρ is not a representative.
3. `Rp = R₀[:-|last|]`. Then `C = ""` if `Rp == ""`, else `C = Rp[:-2]` if
   `Rp` ends in `LF LF`, else `C = Rp[:-1]`. `M = C ‖ W`.
4. `P, claimed = last[15:]` split at its last SP.
5. Headers, in ρ's order: `tree` → `tree T`, k-th `parent` → `parent qₖ`,
   `clonex` exactly → must be `clonex no-body` and `M` must be empty, which
   means "emit no body". Any key starting with `clonex-` gets that prefix
   removed. Anything else is verbatim.
6. Serialise, then check `sha1("commit " len NUL raw) == claimed`. **Only
   this check makes the result a proof.** Any mismatch means ρ is treated as
   the composition's own work, or the design's fetch-by-SHA fallback applies.

**Why decoding is unambiguous.** `C` never ends in LF. So `C ‖ LF` ends in
exactly one LF, `C ‖ LF LF` ends in exactly two, and `C = ""` leaves `Rp`
empty. The decoder recovers `sep` from the byte structure alone. It **never
consults the trailer heuristic**. So even if §1.5 disagrees with git about
what a trailer block is, reversibility still holds, and only git's display is
affected. `W` is copied around `L` unchanged. Header renaming is a bijection
on keys: add one prefix, remove one prefix. That also makes nesting work:
ρ(ρ(t)) turns `clonex-gpgsig` into `clonex-clonex-gpgsig`.

### 1.5 Trailer-block heuristic (encode side only; affects display, not correctness)

This mirrors git's `trailer.c`. The first paragraph (the title) can never be
trailers. Take the last paragraph of `C`, meaning the lines after the last
whitespace-only line. Lines that start with SP or TAB are continuations and
are not counted. A line matching `^[A-Za-z0-9-]+[ \t]*:` is a trailer line,
and anything else is a non-trailer line. The paragraph is a trailer block if
it has trailer lines and no non-trailer lines, **or** if it contains a
git-generated prefix (`Signed-off-by: `, `(cherry picked from commit `) and
`3·trailers ≥ non-trailers`.

### 1.6 Why `encoding` passes through (a deliberate deviation from the brief)

The brief listed `encoding` among the headers to rename. I kept it instead.
It carries no identity or signature meaning, and git needs it to display the
message. With it kept, `git log --format=%s` shows `café naïve` for both the
ISO-8859-1 t and its ρ. With it renamed, ρ would show mojibake. The ASCII
trailer is valid in every ASCII-compatible encoding, and git supports no
other kind. Renaming it instead would also round-trip correctly.

## 2. Results

### 2.1 Real history (all refs of each mirror, topo order)

| repo | commits | reconstructed byte-exact | fail | build | reconstruct |
|---|---:|---:|---:|---:|---:|
| agent-harness | 336 | 336 | 0 | 0.257 s | 0.142 s |
| dotsync | 480 | 480 | 0 | 0.384 s | 0.197 s |
| help-test | 14 | 14 | 0 | 0.011 s | 0.007 s |
| oc | 198 | 198 | 0 | 0.160 s | 0.078 s |
| tdd-ratchet-rs | 257 | 257 | 0 | 0.194 s | 0.103 s |
| tmux-bridge | 308 | 308 | 0 | 0.208 s | 0.127 s |
| trunc | 122 | 122 | 0 | 0.095 s | 0.052 s |
| **total** | **1,715** | **1,715** | **0** | | |

What the real corpus contains:

| Feature | Commits |
|---|---:|
| Merges | 115 |
| Root commits | 7 |
| `gpgsig` headers (GitHub web UI) | 127 |
| No final LF (GitHub merges and squashes) | 138 |
| One final LF | 1,468 |
| Two final LFs (message ends in a blank line) | 109 |
| Trailer block that ρ joins | 12 (7 with one final LF, 5 with two) |

None of the real commits had CR, non-UTF-8 bytes, an `encoding` header,
`mergetag`, `change-id`, an empty message or an existing `Clonex-Source`. The
synthetic corpus covers all of those.

Reconstruction reads **only the composition store**. It walks ρ in topo
order, reads each ρ, looks up `tree:P` by parsing trees, and maps ρ's parents
through the already-reconstructed ρ→t map. As an independent check, every
rebuilt raw object was then compared byte-for-byte with the object of the
same SHA in the component mirror.

**Timings** (Python prototype, one `git cat-file --batch` process per store,
loose-object writes, warm cache):
- **build: about 0.76 s per 1,000 commits**, including writing 1 commit and
  2 tree objects per ρ;
- **reconstruct and verify: about 0.41 s per 1,000**;
- the pure codec (encode plus decode, no I/O) takes about 11.6 µs per commit.

### 2.2 Synthetic edge-case corpus (`synth.py`)

There are 34 cases, each built with `hash-object --literally`, both as a root
commit and as a child. Four more topologies were added: a merge with
`mergetag` + `gpgsig`, an octopus with a no-body parent, a merge of two roots,
and a duplicate parent. That makes 72 commits, and **all 72 reconstructed
byte-exact**. Cases:

- plain;
- no final LF;
- body with no final LF;
- empty message;
- `"\n"`;
- `"\n\n\n"`;
- trailing blank lines;
- trailer block;
- trailer block with no final LF;
- trailer block followed by a blank line;
- a title that looks like a trailer;
- the 25 % rule, and a block under 25 %;
- trailer continuation lines;
- a `---` divider;
- CRLF, CRLF with no final LF, and a message ending in CR;
- ISO-8859-1 with `encoding`;
- invalid UTF-8;
- NUL, VT, FF, FS, GS, NEL and U+2028;
- a message that already ends in `Clonex-Source` (with and without LF), and
  a message that is only a `Clonex-Source` line;
- all extra headers in unusual order (`change-id`, `mergetag`,
  `gpgsig-sha256`, `encoding`, `gpgsig`);
- duplicate `gpgsig`;
- headers already named `clonex-gpgsig`, `clonex no-body` and `clonex`;
- valueless headers;
- no body, and no body but signed;
- `encoding` before `author`;
- no `author`;
- `gpgsig` before `committer`.

**Nested:** each of the 72 ρ was treated as a component commit of a second
composition at `vendor/c`. ρ(ρ(t)) decoded to ρ(t), 72 of 72.

**Fuzz:** 200,000 random commits were generated from atoms such as LF, CR,
SP, `:`, `Signed-off-by: X`, `Key: v`, `\xff`, `Clonex-Source: …`, `---` and
TAB. Each had 0–2 random extra headers (including `clonex-foo`, `clonex` and
valueless ones), 0–2 parents, and a 3 % chance of having no body. There were
**0 round-trip failures**.

### 2.3 `git fsck --strict`

- All 7 real composition stores give **rc 0**, with no warnings.
- Synthetic store: ρ is rejected **exactly when t is**. The 8 malformed t
  (`missingAuthor`, `missingCommitter`, `nulInCommit`) produce the same 8
  errors on their ρ, and there are no other errors.
- Renamed headers and `clonex no-body` never trigger an fsck message.
- `git hash-object -t commit --stdin` accepts ρ objects without `--literally`.

### 2.4 Is ρ shown as signed?

No. Tested on the real GitHub-signed trunc merge `7d283c6` and its ρ
`8f83ae3`, and on the synthetic merge carrying `mergetag` + `gpgsig`:

| | t | ρ |
|---|---|---|
| `git verify-commit` | runs gpg: "Can't check signature: No public key", rc 1 | no gpg invocation, no output, rc 1 |
| `git log --show-signature`, `%G?` | gpg output, `%G?` = `E` | no signature block, `%G?` = `N` |
| mergetag | git checks the embedded tag ("tag v1 names a non-parent…") | nothing shown |

### 2.5 Is `Clonex-Source` a real git trailer?

`git log --format='%(trailers:…)'` was run on every ρ.
- For **1,775 of 1,787**, git parses `Clonex-Source` as the last trailer.
  For the 28 t that already had trailers, git's parsed trailer list for ρ is
  exactly t's list plus `Clonex-Source` (`trailers_check.py`, 0 mismatches).
- The **12 exceptions** are all synthetic: empty message, `"\n"`, `"\n\n\n"`,
  no body, and NUL in body, each as root and as child. When `C` is empty, `L`
  becomes the title, and git never parses a title as trailers. With a NUL,
  git stops reading the message at the NUL.
- Reversibility is unaffected, because the decoder reads the last non-LF line
  itself. **Implication for the design:** clonex must find `Clonex-Source`
  with its own last-line rule, not with `%(trailers)`.

### 2.6 Do git and jj tools preserve the renamed headers?

Tested on real ρ from trunc, in a scratch clone.

| Operation | `clonex-*` headers | `Clonex-Source` trailer | Reconstruction |
|---|---|---|---|
| `git commit-tree` | dropped (no way to pass headers) | only if supplied in `-m` | fails (new committer and headers) |
| `git rebase` (merge backend) | **dropped** | kept | hash mismatch, so fallback |
| `git rebase --apply` | **dropped** | kept | hash mismatch |
| `git cherry-pick` | **dropped** | kept (with `-x`, `(cherry picked…)` follows it, so it is no longer the last line and decode rejects it) | hash mismatch |
| `git rebase --rebase-merges --force-rebase` on a merge ρ | dropped | kept | hash mismatch |
| `git commit --amend --no-edit` | **kept** (git copies extra headers except `gpgsig*` on amend) | kept | hash mismatch (new committer) |
| `git commit --amend -m …` | **kept** | removed | not a representative |
| `git fast-export \| fast-import` | **dropped** (10 of 10 lost) | kept | hash mismatch |
| `jj rebase` / `jj describe` | **dropped**; jj adds its own `change-id` | `rebase` keeps it, `describe -m` replaces it | hash mismatch |

As expected, rewriting tools do not carry renamed headers, with one surprise:
`git commit --amend` does. In every case the rewritten commit keeps its claim
(or loses it), fails the hash check, and **is not silently accepted**. This is
the D2 fallback path in `constraints.md`.

One design consequence: after `--amend --no-edit` or a rebase, the tree at P
is often unchanged. So the fallback "fetch t by SHA and check
`tree(t) == tree(c)[P]`" will usually *accept* the rewritten commit as
content-equal to t. The design should state whether that is intended.

## 3. Failure classes hit, and how the final encoding handles them

1. **Messages without a final LF** (138 real: every GitHub web-UI merge and
   squash). Encoders that always end ρ in LF cannot tell `"foo"` from
   `"foo\n"`. Handled because ρ copies t's trailing LF run `W` exactly.
   Ablation V1 below, "normalise to a final LF", fails on 274 of 1,787
   commits.
2. **Messages ending in blank lines** (109 real, mostly agent-written, ending
   `\n\n`). These break any "strip one trailing newline" rule, which is part
   of V1's failures. Handled by the same `W` rule.
3. **Trailer block followed by a blank line** (5 real agent-harness commits,
   `Co-Authored-By: …\n\n`). This one was **hit by my first version**. That
   version placed `L` after `W` in a new paragraph. It stayed reversible, but
   git's `%(trailers)` then saw only `Clonex-Source` and lost
   `Co-Authored-By`. Fixed by putting `L` *before* `W` and gluing it to `C`.
   The first version is kept as `rho_v1.py`.
4. **Text-mode handling** (Python `str`, `splitlines()`) breaks on CR, VT,
   FF, FS, GS, NEL, U+2028, invalid UTF-8 and trailing-newline structure.
   Ablation V2 fails on 167 commits. Handled because everything is bytes,
   split only on LF.
5. **Ambiguity from "smart" separators.** An earlier paper design chose
   `sep ∈ {"", LF, LF LF}` from the trailer heuristic and was ambiguous
   (`"foo"` and `"foo\n"` collided). Handled by construction: `C` never ends
   in LF, so the number of LFs before `L` identifies `sep`, and the decoder
   never uses the heuristic.
6. **No body vs empty body.** Both are legal objects with different SHAs.
   Handled with the reserved `clonex no-body` meta header.
7. **Headers already named `clonex-…` or `clonex`**, which happens with
   nesting. Handled because renaming is always exactly one prefix, and `META`
   is outside the image of renaming.
8. **An existing `Clonex-Source` in the message.** Handled because only the
   *last* line before `W` is the claim, and the rest is `C`.
9. **Header order, duplicates, valueless and multi-line headers.** Handled
   because groups are transformed in place, in order, with continuation lines
   verbatim.
10. **The trailer is not machine-visible to git** when `C` is empty or the
    message contains NUL. This is display only, and documented in §2.5.

Ablation (`ablation.py`, message-only round trip over real + synthetic,
1,787 commits):

| Variant | Round-trip failures |
|---|---:|
| final encoding | 0 |
| V1: normalise to final LF, strip trailer and blank line | 274 |
| V2: text mode (`utf-8`/`replace`, `splitlines`) | 167 |
| V3: fixed suffix `LF LF L LF`, strip exactly | 0 |

V3 is reversible but hides existing trailer blocks from git (class 3) and
changes the trailing-LF shape.

Out of scope, and failing verifiably rather than silently:
- a header section with no final LF;
- a multi-line or duplicate `tree` header;
- SHA-256 repositories (the codec is hash-agnostic, but only SHA-1 was
  exercised).

## 4. Files (session scratchpad `recon/`)

- `rho.py`: the codec and pipeline (Appendix A). Run it with
  `python3 rho.py real mirrors/*.git`.
- `synth.py`: the synthetic corpus, fsck, trailer visibility, nesting and
  fuzz (Appendix B).
- `ablation.py`, `trailers_check.py`: Appendix C.
- `rho_v1.py`: the first version, which has failure class 3.
- `comp/<repo>.git`: composition stores that use alternates to point at
  `mirrors/<repo>.git`.
- `work/`, `jjw/`: scratch clones for the tool-behaviour tests. §2.6 records
  the ad-hoc shell commands and their outcomes.

## Appendix A — `rho.py`

```python
#!/usr/bin/env python3
"""R2' shifted-copy encoding: build rho(t, P) and reconstruct t byte-exactly.

Feasibility experiment, not product code.

Usage:
  rho.py real   <mirror.git>... -> per-repo build + reconstruct + compare
  rho.py synth                  -> synthetic edge-case corpus
"""
import hashlib, os, re, subprocess, sys, time, zlib

PREFIX = b"clonex-"          # renamed-header prefix
META_KEY = b"clonex"         # reserved meta header (never produced by renaming)
PASS_THROUGH = {b"tree", b"parent", b"author", b"committer", b"encoding"}
TRAILER_KEY = b"Clonex-Source: "


class Unsupported(Exception):
    pass


# ---------------------------------------------------------------- raw parsing

def split_commit(raw):
    """-> (header_groups, body or None). Each group is a list of lines (no \\n)
    whose first line is 'key[ value]' and the rest are ' '-continuations."""
    i = raw.find(b"\n\n")
    if i < 0:
        if not raw.endswith(b"\n"):
            raise Unsupported("header section not newline-terminated")
        hdr, body = raw, None
    else:
        hdr, body = raw[:i + 1], raw[i + 2:]
    groups = []
    for line in hdr[:-1].split(b"\n"):
        if line.startswith(b" "):
            if not groups:
                raise Unsupported("continuation line before first header")
            groups[-1].append(line)
        else:
            groups.append([line])
    return groups, body


def key_of(group):
    return group[0].split(b" ", 1)[0]


def join_commit(groups, body):
    hdr = b"".join(b"\n".join(g) + b"\n" for g in groups)
    return hdr if body is None else hdr + b"\n" + body


def git_hash(kind, raw):
    return hashlib.sha1(b"%s %d\0" % (kind, len(raw)) + raw).hexdigest()


# ------------------------------------------------------------ trailer heuristic
# Mirrors git trailer.c closely enough to decide whether the message already
# ends in a trailer block (so the new trailer joins it rather than starting a
# new paragraph). Correctness of *reconstruction* does not depend on this.

_TOKEN = re.compile(rb"^[A-Za-z0-9-]+[ \t]*:")
_GIT_GENERATED = (b"Signed-off-by: ", b"(cherry picked from commit ")


def _blank(line):
    return line.strip(b" \t\r\f\v") == b""


def ends_in_trailer_block(msg):
    lines = msg.split(b"\n")
    if lines and lines[-1] == b"":
        lines.pop()
    # title paragraph cannot hold trailers
    t = 0
    while t < len(lines) and not _blank(lines[t]):
        t += 1
    body = lines[t:]
    while body and _blank(body[-1]):
        return False  # trailing blank lines: git would not glue onto them
    if not body:
        return False
    trailer = non = 0
    recognized = False
    for line in reversed(body):
        if _blank(line):
            break
        if line[:1] in (b" ", b"\t"):
            continue  # possible continuation
        if _TOKEN.match(line):
            trailer += 1
            if line.startswith(_GIT_GENERATED):
                recognized = True
        else:
            non += 1
    if trailer and not non:
        return True
    return recognized and trailer * 3 >= non


# ------------------------------------------------------------------ encoding

def encode(raw_t, t_sha, P, rho_tree, rho_parents):
    groups, body = split_commit(raw_t)
    out, pi, seen_tree = [], 0, False
    for g in groups:
        k = key_of(g)
        if k == b"tree":
            if seen_tree or len(g) != 1:
                raise Unsupported("multiple/continued tree header")
            seen_tree = True
            out.append([b"tree " + rho_tree])
        elif k == b"parent":
            if len(g) != 1:
                raise Unsupported("continued parent header")
            out.append([b"parent " + rho_parents[pi]])
            pi += 1
        elif k in PASS_THROUGH:
            out.append(list(g))
        else:
            out.append([PREFIX + g[0]] + g[1:])
    if pi != len(rho_parents):
        raise Unsupported("parent count mismatch")
    L = TRAILER_KEY + P + b" " + t_sha
    if body is None:
        out.append([META_KEY + b" no-body"])
        M = b""
    else:
        M = body
    # M = C + W, W = maximal run of trailing LF bytes; the trailer goes between.
    C = M.rstrip(b"\n")
    W = M[len(C):]
    if C == b"":
        sep = b""
    elif ends_in_trailer_block(C):
        sep = b"\n"          # join the existing trailer block
    else:
        sep = b"\n\n"        # start a new paragraph
    R = C + sep + L + W
    return join_commit(out, R)


def decode(raw_rho, tree_at_P, recon_parents):
    """-> (P, claimed_sha, raw_t). Raises ValueError if rho is not decodable."""
    groups, R = split_commit(raw_rho)
    if R is None:
        raise ValueError("rho has no body")
    # --- message: R = C + sep + L + W
    R0 = R.rstrip(b"\n")
    W = R[len(R0):]
    Rp, _, last = R0.rpartition(b"\n")
    Rp = R0[:len(R0) - len(last)]
    if not last.startswith(TRAILER_KEY):
        raise ValueError("no trailing Clonex-Source line")
    if Rp == b"":
        C = b""
    elif Rp.endswith(b"\n\n"):
        C = Rp[:-2]
    else:                     # Rp ends with exactly one LF (rpartition split there)
        C = Rp[:-1]
    M = C + W
    P, _, claimed = last[len(TRAILER_KEY):].rpartition(b" ")
    # --- headers
    out, pi, no_body = [], 0, False
    for g in groups:
        k = key_of(g)
        if k == b"tree":
            out.append([b"tree " + tree_at_P])
        elif k == b"parent":
            out.append([b"parent " + recon_parents[pi]])
            pi += 1
        elif k == META_KEY:
            if g != [META_KEY + b" no-body"] or M != b"":
                raise ValueError("bad meta header")
            no_body = True
        elif k.startswith(PREFIX):
            out.append([g[0][len(PREFIX):]] + g[1:])
        else:
            out.append(list(g))
    return P, claimed, join_commit(out, None if no_body else M)


# ------------------------------------------------------------ object stores

class Cat:
    def __init__(self, gitdir):
        self.p = subprocess.Popen(["git", "--git-dir", gitdir, "cat-file", "--batch"],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE)

    def get(self, sha):
        if isinstance(sha, bytes):
            sha = sha.decode()
        self.p.stdin.write(sha.encode() + b"\n")
        self.p.stdin.flush()
        hdr = self.p.stdout.readline().split()
        if hdr[1] == b"missing":
            raise KeyError(sha)
        n = int(hdr[2])
        data = self.p.stdout.read(n)
        self.p.stdout.read(1)
        return hdr[1], data

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def write_loose(gitdir, kind, raw):
    sha = git_hash(kind, raw)
    d = os.path.join(gitdir, "objects", sha[:2])
    f = os.path.join(d, sha[2:])
    if not os.path.exists(f):
        os.makedirs(d, exist_ok=True)
        with open(f + ".tmp", "wb") as fh:
            fh.write(zlib.compress(b"%s %d\0" % (kind, len(raw)) + raw))
        os.rename(f + ".tmp", f)
    return sha.encode()


def parse_tree(raw):
    ents, i = [], 0
    while i < len(raw):
        sp = raw.index(b" ", i)
        nul = raw.index(b"\0", sp)
        ents.append((raw[i:sp], raw[sp + 1:nul], raw[nul + 1:nul + 21].hex().encode()))
        i = nul + 21
    return ents


def build_tree(ents):
    ents = sorted(ents, key=lambda e: e[1] + (b"/" if e[0] == b"40000" else b""))
    return b"".join(m + b" " + n + b"\0" + bytes.fromhex(s.decode()) for m, n, s in ents)


def replace_path(gitdir, cat, base_tree, path, new_tree):
    """Return sha of base_tree (None = empty) with `path` set to new_tree."""
    first, _, rest = path.partition(b"/")
    ents = parse_tree(cat.get(base_tree)[1]) if base_tree else []
    old = next((s for m, n, s in ents if n == first and m == b"40000"), None)
    ents = [e for e in ents if e[1] != first]
    sub = replace_path(gitdir, cat, old, rest, new_tree) if rest else new_tree
    ents.append((b"40000", first, sub))
    return write_loose(gitdir, b"tree", build_tree(ents))


def tree_at(cat, tree, path):
    for part in path.split(b"/"):
        ents = parse_tree(cat.get(tree)[1])
        tree = next(s for m, n, s in ents if n == part and m == b"40000")
    return tree


def commit_tree_and_parents(raw):
    groups, _ = split_commit(raw)
    tree = next(g[0][5:] for g in groups if key_of(g) == b"tree")
    parents = [g[0][7:] for g in groups if key_of(g) == b"parent"]
    return tree, parents


def git(*a, gitdir=None, input=None):
    cmd = ["git"] + (["--git-dir", gitdir] if gitdir else []) + list(a)
    return subprocess.run(cmd, input=input, capture_output=True, check=True).stdout


# ------------------------------------------------------------------ pipeline

def build_composition(src, comp, P, commits):
    """commits: topo-ordered list of shas in src. -> {t: rho}, seconds."""
    cat_src, cat_comp = Cat(src), Cat(comp)
    rho, t0 = {}, time.perf_counter()
    for t in commits:
        raw = cat_src.get(t)[1]
        tt, tp = commit_tree_and_parents(raw)
        rp = [rho[p] for p in tp]
        base = None
        if rp:
            base = commit_tree_and_parents(cat_comp.get(rp[0])[1])[0]
        rtree = replace_path(comp, cat_comp, base, P, tt)
        rho[t] = write_loose(comp, b"commit", encode(raw, t, P, rtree, rp))
    dt = time.perf_counter() - t0
    cat_src.close(), cat_comp.close()
    return rho, dt


def reconstruct_all(comp, P, tips, src):
    """Walk the composition store only; rebuild every t; compare to src."""
    order = git("rev-list", "--topo-order", "--reverse", *[x.decode() for x in tips],
                gitdir=comp).split()
    cat_comp, cat_src = Cat(comp), Cat(src)
    recon, fails, t0 = {}, [], time.perf_counter()
    raws = {}
    for r in order:
        raw_rho = cat_comp.get(r)[1]
        rtree, rpar = commit_tree_and_parents(raw_rho)
        try:
            Pc, claimed, raw_t = decode(raw_rho, tree_at(cat_comp, rtree, P),
                                        [recon[p] for p in rpar])
        except (ValueError, KeyError, StopIteration) as e:
            fails.append((r, "decode: %s" % e))
            continue
        got = git_hash(b"commit", raw_t).encode()
        if Pc != P or got != claimed:
            fails.append((r, "hash mismatch"))
            continue
        recon[r] = got
        raws[got] = raw_t
    dt = time.perf_counter() - t0
    # independent byte comparison against the component (outside timing)
    for sha, raw_t in raws.items():
        if cat_src.get(sha)[1] != raw_t:
            fails.append((sha, "bytes differ"))
    cat_comp.close(), cat_src.close()
    return recon, fails, dt, order


def fresh_comp(path, alternates):
    subprocess.run(["rm", "-rf", path], check=True)
    git("init", "-q", "--bare", path)
    with open(os.path.join(path, "objects/info/alternates"), "w") as f:
        f.write("".join(os.path.abspath(a) + "/objects\n" for a in alternates))


def run_repo(src, comp, P, refs=None):
    fresh_comp(comp, [src])
    if refs is None:
        refs = [l.split() for l in git("for-each-ref", "--format=%(objectname) %(objecttype) %(refname)",
                                       gitdir=src).splitlines()]
        refs = [(s, n) for s, ty, n in refs if ty == b"commit"] + \
               [(git("rev-parse", n.decode() + "^{commit}", gitdir=src).strip(), n)
                for s, ty, n in refs if ty == b"tag"]
    commits = git("rev-list", "--all", "--topo-order", "--reverse", gitdir=src).split()
    rho, t_build = build_composition(src, comp, P, commits)
    for s, n in refs:
        git("update-ref", n.decode(), rho[s].decode(), gitdir=comp)
    tips = sorted({rho[s] for s, n in refs})
    recon, fails, t_rec, order = reconstruct_all(comp, P, tips, src)
    ok = sum(1 for t, r in rho.items() if recon.get(r) == t)
    return dict(n=len(commits), ok=ok, fails=fails, t_build=t_build, t_rec=t_rec, rho=rho)


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    if sys.argv[1] == "real":
        tot = [0, 0, 0.0, 0.0]
        for src in sys.argv[2:]:
            name = os.path.basename(src)[:-4]
            P = b"tools/" + name.encode()
            res = run_repo(src, os.path.join(here, "comp", name + ".git"), P)
            print("%-16s commits=%4d pass=%4d fail=%d build=%.3fs recon=%.3fs"
                  % (name, res["n"], res["ok"], len(res["fails"]), res["t_build"], res["t_rec"]))
            for f in res["fails"][:5]:
                print("   FAIL", f)
            tot[0] += res["n"]; tot[1] += res["ok"]; tot[2] += res["t_build"]; tot[3] += res["t_rec"]
        print("TOTAL commits=%d pass=%d  build=%.1f ms/1000  recon=%.1f ms/1000"
              % (tot[0], tot[1], 1e6 * tot[2] / tot[0], 1e6 * tot[3] / tot[0]))
```

## Appendix B — `synth.py`

```python
#!/usr/bin/env python3
"""Synthetic edge-case corpus + nested composition + message fuzz for rho.py."""
import os, random, subprocess, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rho import *

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "synth", "src.git")
A = b"author A U Thor <a@example.com> 1700000000 +1300"
C = b"committer C O Mitter <c@example.com> 1700000001 -0000"
SIG = [b"gpgsig -----BEGIN PGP SIGNATURE-----", b" ", b" iQEzBAABCAAdFiEE", b" =abcd",
       b" -----END PGP SIGNATURE-----", b" "]
SIG256 = [b"gpgsig-sha256 -----BEGIN SSH SIGNATURE-----", b" U1NIU0lH", b" -----END SSH SIGNATURE-----"]
MERGETAG = [b"mergetag object 4b825dc642cb6eb9a060e54bf8d69288fbee4904", b" type commit",
            b" tag v1", b" tagger T <t@x> 1700000000 +0000", b" ", b" tag message",
            b" -----BEGIN PGP SIGNATURE-----", b" xyz", b" -----END PGP SIGNATURE-----"]
CHANGE = [b"change-id kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk"]

# name -> (extra header groups placed after committer | full header override, body or None)
CASES = {
    "plain":               ([], b"subject\n"),
    "no-final-newline":    ([], b"subject"),
    "body-no-final-nl":    ([], b"subject\n\nbody line"),
    "empty-message":       ([], b""),
    "message-just-nl":     ([], b"\n"),
    "message-3-nl":        ([], b"\n\n\n"),
    "trailing-blank-lines": ([], b"s\n\nbody\n\n\n"),
    "trailer-block":       ([], b"s\n\nbody\n\nSigned-off-by: A <a@x>\nCo-authored-by: B <b@x>\n"),
    "trailer-no-final-nl": ([], b"s\n\nSigned-off-by: A <a@x>"),
    "trailer-then-blank":  ([], b"s\n\nSigned-off-by: A <a@x>\n\n"),
    "title-looks-trailer": ([], b"Fixes: the thing\n"),
    "mixed-25pct-block":   ([], b"s\n\nsome prose\nSigned-off-by: A <a@x>\n"),
    "mixed-under-25pct":   ([], b"s\n\nprose\nprose\nprose\nprose\nKey: v\n"),
    "trailer-continuation": ([], b"s\n\nKey: v\n  continued\nOther: w\n"),
    "divider":             ([], b"s\n\n---\nNot-A: trailer\n"),
    "crlf":                ([], b"s\r\n\r\nbody\r\n\r\nSigned-off-by: A <a@x>\r\n"),
    "crlf-no-final":       ([], b"s\r\n\r\nbody"),
    "ends-in-cr":          ([], b"s\r"),
    "latin1-with-encoding": ([[b"encoding ISO-8859-1"]], b"caf\xe9 na\xefve\n"),
    "invalid-utf8":        ([], b"bad \xff\xfe bytes\n\n\xc3\n"),
    "nul-and-controls":    ([], b"s\n\nx\x00y\x0bz\x0c\x1c\x1d\x85\xe2\x80\xa8\n"),
    "has-clonex-trailer":  ([], b"s\n\nClonex-Source: tools/other 0123456789abcdef0123456789abcdef01234567\n"),
    "has-clonex-no-nl":    ([], b"s\n\nClonex-Source: tools/other 0123456789abcdef0123456789abcdef01234567"),
    "only-clonex-line":    ([], b"Clonex-Source: tools/x 0123456789abcdef0123456789abcdef01234567"),
    "all-extra-headers":   ([CHANGE, MERGETAG, SIG256, [b"encoding UTF-8"], SIG], b"s\n"),
    "dup-gpgsig":          ([SIG, SIG], b"s\n"),
    "clonex-named-hdrs":   ([[b"clonex-gpgsig already-renamed"], [b"clonex no-body"], [b"clonex"]], b"s\n"),
    "valueless-header":    ([[b"x-flag"], [b"x-empty "]], b"s\n"),
    "no-body":             ([], None),
    "no-body-signed":      ([SIG], None),
}
# header-order cases override the entire header list ({T}/{P} placeholders)
ORDER_CASES = {
    "encoding-before-author": ([b"{T}", b"{P}", [b"encoding latin1"], A, C], b"s\n"),
    "no-author":             ([b"{T}", b"{P}", C], b"s\n"),
    "sig-before-committer":  ([b"{T}", b"{P}", A, SIG, C], b"s\n"),
}


def mk(tree, parents, extra, body, order=None):
    if order is None:
        groups = [[b"tree " + tree]] + [[b"parent " + p] for p in parents] + [[A], [C]] + extra
    else:
        groups = []
        for g in order:
            if g == b"{T}":
                groups.append([b"tree " + tree])
            elif g == b"{P}":
                groups += [[b"parent " + p] for p in parents]
            elif isinstance(g, bytes):
                groups.append([g])
            else:
                groups.append(g)
    return join_commit(groups, body)


def put(gitdir, raw):
    return git("hash-object", "--literally", "-w", "-t", "commit", "--stdin",
               gitdir=gitdir, input=raw).strip()


def main():
    subprocess.run(["rm", "-rf", os.path.join(HERE, "synth")], check=True)
    git("init", "-q", "--bare", SRC)
    blob = git("hash-object", "-w", "--stdin", gitdir=SRC, input=b"x\n").strip()
    tree = git("mktree", gitdir=SRC, input=b"100644 blob " + blob + b"\tf\n").strip()
    base = put(SRC, mk(tree, [], [], b"base\n"))
    names = {}
    allcases = {k: (v[0], v[1], None) for k, v in CASES.items()}
    allcases.update({k: ([], v[1], v[0]) for k, v in ORDER_CASES.items()})
    for name, (extra, body, order) in allcases.items():
        root = put(SRC, mk(tree, [], extra, body, order))          # as root commit
        child = put(SRC, mk(tree, [base], extra, body, order))     # as child
        names["root/" + name] = root
        names["child/" + name] = child
    # merges: 2-parent, octopus, merge of two roots, duplicate parent
    r2 = put(SRC, mk(tree, [], [], b"second root\n"))
    names["merge"] = put(SRC, mk(tree, [base, names["child/plain"]], [MERGETAG, SIG], b"Merge"))
    names["octopus"] = put(SRC, mk(tree, [base, names["child/crlf"], names["root/no-body"]], [], b"oct\n"))
    names["merge-of-roots"] = put(SRC, mk(tree, [base, r2], [], b"roots\n"))
    names["dup-parent"] = put(SRC, mk(tree, [base, base], [], b"dup\n"))
    for n, s in names.items():
        git("update-ref", "refs/heads/" + n, s.decode(), gitdir=SRC)
    fsck_t = subprocess.run(["git", "--git-dir", SRC, "fsck", "--strict", "--no-dangling"],
                            capture_output=True, text=True)

    comp = os.path.join(HERE, "synth", "comp.git")
    res = run_repo(SRC, comp, b"tools/synth")
    print("synthetic: commits=%d pass=%d fail=%d" % (res["n"], res["ok"], len(res["fails"])))
    for f in res["fails"]:
        print("  FAIL", f)
    fsck_r = subprocess.run(["git", "--git-dir", comp, "fsck", "--strict", "--no-dangling"],
                            capture_output=True, text=True)
    bad_t = {l for l in fsck_t.stdout.splitlines() + fsck_t.stderr.splitlines()}
    bad_r = {l for l in fsck_r.stdout.splitlines() + fsck_r.stderr.splitlines()}
    inv = {v: k for k, v in names.items()}
    rinv = {v: inv.get(k, k) for k, v in res["rho"].items()}
    print("fsck --strict on t store (rc=%d):" % fsck_t.returncode)
    for l in sorted(bad_t):
        sha = l.split()[3].rstrip(":").encode() if len(l.split()) > 3 else b""
        print("   ", l.split(": ", 1)[-1][:40], "<-", inv.get(sha, "?"))
    print("fsck --strict on rho store (rc=%d):" % fsck_r.returncode)
    for l in sorted(bad_r):
        sha = l.split()[3].rstrip(":").encode() if len(l.split()) > 3 else b""
        if sha in inv: continue  # component object seen through alternates
        print("   ", l.split(": ", 1)[-1][:40], "<- rho of", rinv.get(sha, "?"))
    # git trailer recognition on rho
    out = git("log", "--all", "--format=%H%x09%(trailers:key=Clonex-Source,valueonly,separator=|)",
              gitdir=comp).decode("latin1").splitlines()
    notrec = []
    for l in out:
        h, _, v = l.partition("\t")
        vals = v.split("|") if v else []
        if not vals or not vals[-1].startswith("tools/synth "):
            notrec.append(rinv.get(h.encode(), h))
    print("rho whose last Clonex-Source trailer git does NOT parse (%d):" % len(notrec), sorted(notrec))

    # nested: treat rho commits as components of a second composition at vendor/c
    comp2 = os.path.join(HERE, "synth", "comp2.git")
    refs = [(res["rho"][s], b"refs/heads/" + n.encode()) for n, s in names.items()]
    res2 = run_repo(comp, comp2, b"vendor/c", refs=refs)
    print("nested rho(rho): commits=%d pass=%d fail=%d" % (res2["n"], res2["ok"], len(res2["fails"])))

    # message/header fuzz: pure encode/decode round-trip
    rnd = random.Random(1)
    atoms = [b"a", b"\n", b"\n", b"\r", b" ", b":", b"Signed-off-by: X", b"Key: v", b"\xff",
             b"Clonex-Source: tools/z 0123456789abcdef0123456789abcdef01234567", b"---", b"\t"]
    hdrs = [SIG, SIG256, MERGETAG, CHANGE, [b"encoding x"], [b"clonex-foo bar"], [b"clonex"], [b"k"]]
    n, t0 = 200000, time.perf_counter()
    bad = 0
    for i in range(n):
        body = b"".join(rnd.choice(atoms) for _ in range(rnd.randrange(0, 12)))
        if rnd.random() < 0.03:
            body = None
        extra = [rnd.choice(hdrs) for _ in range(rnd.randrange(0, 3))]
        par = [b"%040x" % rnd.getrandbits(160) for _ in range(rnd.randrange(0, 3))]
        raw = mk(b"%040x" % 1, par, extra, body)
        sha = git_hash(b"commit", raw).encode()
        rp = [b"%040x" % rnd.getrandbits(160) for _ in par]
        rraw = encode(raw, sha, b"tools/f", b"%040x" % 2, rp)
        P, claimed, back = decode(rraw, b"%040x" % 1, par)
        if back != raw or claimed != sha or P != b"tools/f":
            bad += 1
            if bad < 5:
                print("  FUZZ FAIL", repr(raw))
    print("fuzz: %d random commits, %d round-trip failures, %.1f us each"
          % (n, bad, 1e6 * (time.perf_counter() - t0) / n))


if __name__ == "__main__":
    main()
```

## Appendix C — `ablation.py`, `trailers_check.py`

```python
#!/usr/bin/env python3
"""Show which naive encodings break, on the same corpus (real mirrors + synthetic)."""
import glob, os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from rho import *

HERE = os.path.dirname(os.path.abspath(__file__))
P = b"tools/x"
L = lambda sha: TRAILER_KEY + P + b" " + sha


def corpus():
    for src in sorted(glob.glob(os.path.join(HERE, "mirrors", "*.git"))) + [os.path.join(HERE, "synth", "src.git")]:
        cat = Cat(src)
        for s in git("rev-list", "--all", gitdir=src).split():
            yield os.path.basename(src), s, cat.get(s)[1]
        cat.close()


# -- V1: "normalise": rho message always ends in \n; decoder strips trailer + one blank line
def v1_enc(M, sha):
    core = M.rstrip(b"\n")
    return core + (b"\n" if ends_in_trailer_block(M) else b"\n\n") + L(sha) + b"\n"
def v1_dec(R):
    core = R[:-1].rpartition(b"\n")[0]          # drop trailer line
    core = core[:-1] if core.endswith(b"\n\n") else core
    return core if core.endswith(b"\n") else core + b"\n"   # assume git-style final \n


# -- V2: text mode: decode as UTF-8 (replace), splitlines(), rejoin with \n
def v2_enc(M, sha):
    t = M.decode("utf-8", "replace")
    return ("\n".join(t.splitlines()) + "\n\n" + L(sha).decode() + "\n").encode()
def v2_dec(R):
    t = R.decode("utf-8", "replace").splitlines()[:-2]
    return ("\n".join(t) + "\n").encode()


# -- V3: fixed suffix "\n\n" + L + "\n" (reversible, but separates trailer blocks)
def v3_enc(M, sha):
    return M + b"\n\n" + L(sha) + b"\n"
def v3_dec(R):
    return R[:R[:-1].rfind(b"\n") - 1]


def main():
    fails = {k: {} for k in ("final", "V1-normalise", "V2-text", "V3-fixed")}
    total, extra_hdr = 0, 0
    for repo, sha, raw in corpus():
        total += 1
        groups, body = split_commit(raw)
        if any(key_of(g) not in PASS_THROUGH for g in groups):
            extra_hdr += 1
        M = body if body is not None else b""
        hdr = raw[:len(raw) - len(M)] if body is not None else raw
        # final
        r = encode(raw, sha, P, b"0" * 40, [b"0" * 40] * sum(key_of(g) == b"parent" for g in groups))
        back = decode(r, commit_tree_and_parents(raw)[0], commit_tree_and_parents(raw)[1])[2]
        if back != raw:
            fails["final"][repo] = fails["final"].get(repo, 0) + 1
        for name, enc, dec in (("V1-normalise", v1_enc, v1_dec), ("V2-text", v2_enc, v2_dec),
                               ("V3-fixed", v3_enc, v3_dec)):
            try:
                ok = dec(enc(M, sha)) == M
            except Exception:
                ok = False
            if not ok:
                fails[name][repo] = fails[name].get(repo, 0) + 1
    print("corpus: %d commits (%d with non-core headers)" % (total, extra_hdr))
    for k, v in fails.items():
        print("%-13s round-trip failures: %d %s" % (k, sum(v.values()), dict(sorted(v.items()))))


if __name__ == "__main__":
    main()
```

```python
#!/usr/bin/env python3
"""Does git's own trailer parser see t's trailers + Clonex-Source on rho?"""
import glob, sys
sys.path.insert(0, '.')
from rho import git
RS = b'\x1e'
bad = tot = withtr = 0
for src in sorted(glob.glob('mirrors/*.git')) + ['synth/src.git']:
    name = src.split('/')[-1][:-4]
    comp = 'comp/%s.git' % name if name != 'src' else 'synth/comp.git'
    tr = '%(trailers:only,unfold,separator=%x01)'
    tt = dict(r.split(b'\0') for r in git('log', '--all', '--format=%H%x00' + tr + '%x1e', gitdir=src).split(RS + b'\n') if r.strip())
    for r in git('log', '--all', '--format=%H%x00%(trailers:key=Clonex-Source,valueonly,separator=%x01)%x00' + tr + '%x1e',
                 gitdir=comp).split(RS + b'\n'):
        if not r.strip():
            continue
        h, cs, got = r.split(b'\0')
        if not cs:
            continue  # git does not see the Clonex-Source trailer at all (reported by synth.py)
        tsha = cs.split(b'\x01')[-1].rsplit(b' ', 1)[-1]
        tot += 1
        withtr += bool(tt[tsha])
        exp = (tt[tsha] + b'\x01' if tt[tsha] else b'') + b'Clonex-Source: ' + cs.split(b'\x01')[-1]
        if got != exp:
            bad += 1
            print('  MISMATCH', name, tsha[:10].decode(), tt[tsha][:50], '->', got[:70])
print('rho checked:', tot, ' t having trailers:', withtr, ' trailer-set mismatches:', bad)
```
