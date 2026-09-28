"""Stories as data for storydags.py. Every step lists exactly what it adds.

Node ids are mermaid ids (ASCII); labels are what the reader sees.
Placement rule under test (C″): an outside tool commit's composition commit
is built on the composition commit that is the *rep* of its tool parent;
for tool merges, the tree outside the tool path is Git's merge of the
parents' reps' trees. A tool commit published from composition commit u
has u as its rep (W14). "Bringing in" is two explicit steps: create the
rep (deterministic, like a fetch), then an ordinary merge.
"""

TR = ("TR", "trunc repo")
DS = ("DS", "dotsync repo")
UM = ("UM", "umbrella repo")

STORIES = [
    {
        "id": "S2b",
        "title": "Half-landed cross-tool change",
        "repos": [TR, UM, DS],
        "compositions": ["UM"],
        "setup": "Max makes one umbrella commit <code>u</code> that changes trunc and dotsync. trunc merges its part first; dotsync later.",
        "steps": [
            {
                "title": "Start: the umbrella contains trunc at t4 and dotsync at d2",
                "commits": [("TR", "t4", "t4"), ("UM", "U3", "U3"), ("DS", "d2", "d2")],
                "is": [("U3", "t4"), ("U3", "d2")],
                "text": "The reps of t4 and d2 are earlier umbrella commits, not shown.",
            },
            {
                "title": "Max commits u in the umbrella; u is the rep of two tool commits",
                "cmds": ["max$ git commit -am 'Alias AGENTS.md as CLAUDE.md everywhere'"],
                "commits": [("UM", "u", "u"), ("TR", "xT", "x_T"), ("DS", "xD", "x_D")],
                "edges": [("U3", "u"), ("t4", "xT"), ("d2", "xD")],
                "made": [("u", "xT"), ("u", "xD")],
                "text": "Mapping <code>u</code> down gives <code>x_T</code> (parent t4) and <code>x_D</code> (parent d2). They were made from u, so u is their rep (W14).",
            },
            {
                "title": "trunc merges its part (t6); its rep ρ(t6) is created",
                "cmds": ["github$ merge trunc PR #13  → t6 = merge(t4, x_T)", "max$ cx fetch trunc          → creates ρ(t6)"],
                "commits": [("TR", "t6", "t6"), ("UM", "r_t6", "ρ(t6)")],
                "edges": [("t4", "t6"), ("xT", "t6"), ("U3", "r_t6"), ("u", "r_t6")],
                "rep": [("r_t6", "t6")],
                "is": [("r_t6", "xD")],
                "verdict": "ρ(t6)'s parents are the reps of t6's parents: U3 (for t4) and u (for x_T). Its tree outside trunc is Git's merge of U3 and u, which is u's. So it contains all of u. That's consistent: it <em>is</em> x_D in dotsync. Whether a label should point here is level 2.",
            },
            {
                "title": "Jim's j1 lands on trunc; its rep is built on t6's rep",
                "cmds": ["github$ merge trunc PR #14  → j1 (Jim), parent t6", "max$ cx fetch trunc          → creates ρ(j1) on ρ(t6)"],
                "commits": [("TR", "j1", "j1"), ("UM", "r_j1", "ρ(j1)")],
                "edges": [("t6", "j1"), ("r_t6", "r_j1")],
                "rep": [("r_j1", "j1")],
                "is": [("r_j1", "xD")],
            },
            {
                "title": "dotsync merges its part (d4); the umbrella now has two divergent lines",
                "cmds": ["github$ merge dotsync PR #51 → d4 = merge(d2, x_D)", "max$ cx fetch dotsync        → creates ρ(d4)"],
                "commits": [("DS", "d4", "d4"), ("UM", "r_d4", "ρ(d4)")],
                "edges": [("d2", "d4"), ("xD", "d4"), ("U3", "r_d4"), ("u", "r_d4")],
                "rep": [("r_d4", "d4")],
                "is": [("r_d4", "xT")],
                "verdict": "Nothing joins ρ(j1) and ρ(d4) yet. No umbrella commit contains both tools' latest. That's normal divergence, and every commit is still one real commit of each tool.",
            },
            {
                "title": "Someone explicitly merges the two lines in the umbrella",
                "cmds": ["max$ git merge ρ(d4)          (while on ρ(j1)'s line) → M"],
                "commits": [("UM", "M", "M")],
                "edges": [("r_j1", "M"), ("r_d4", "M")],
                "is": [("M", "j1"), ("M", "d4")],
                "verdict": "Clean: the shared ancestor is u, and each side changed a different tool since then.",
            },
        ],
        "conclusion": "every tool commit gets exactly one rep; a rep's parents are its tool parents' reps; the umbrella never follows a tool implicitly. Its commits join by explicit merges.",
    },
    {
        "id": "S10",
        "title": "Ada and Max bring in the same commit",
        "repos": [TR, UM],
        "compositions": ["UM"],
        "setup": "Ada and Max each have their own umbrella commit on U2 (Ada edits README, Max edits crates/). Jim's typo fix j1 lands on trunc. Each of them brings it in on their own machine.",
        "steps": [
            {
                "title": "Start",
                "commits": [("TR", "t4", "t4"), ("UM", "U2", "U2"), ("UM", "a", "a (Ada)"), ("UM", "m", "m (Max)")],
                "edges": [("U2", "a"), ("U2", "m")],
                "rep": [("U2", "t4")],
                "is": [("a", "t4"), ("m", "t4")],
                "text": "U2 is the rep of t4. a and m don't touch trunc, so both <em>are</em> t4 too, but neither is its rep.",
            },
            {
                "title": "Jim's fix j1 lands on trunc",
                "commits": [("TR", "j1", "j1 (Jim)")],
                "edges": [("t4", "j1")],
            },
            {
                "title": "Ada's machine creates j1's rep",
                "cmds": ["ada$ cx fetch trunc   → ρ(j1): parent U2 (rep of t4), tree = U2's tree with tools/trunc := j1"],
                "commits": [("UM", "r_j1", "ρ(j1)")],
                "edges": [("U2", "r_j1")],
                "rep": [("r_j1", "j1")],
            },
            {
                "title": "Ada merges it explicitly",
                "cmds": ["ada$ git merge ρ(j1)   → A1"],
                "commits": [("UM", "A1", "A1")],
                "edges": [("a", "A1"), ("r_j1", "A1")],
                "is": [("A1", "j1")],
            },
            {
                "title": "Max's machine creates j1's rep: the identical object, so no new commit",
                "cmds": ["max$ cx fetch trunc   → ρ(j1) again: same parent, tree, author, message → same SHA"],
                "text": "The DAG doesn't change. Max's copy of ρ(j1) is byte-for-byte Ada's, because every input to it (j1, U2) is shared.",
            },
            {
                "title": "Max merges it explicitly",
                "cmds": ["max$ git merge ρ(j1)   → M1"],
                "commits": [("UM", "M1", "M1")],
                "edges": [("m", "M1"), ("r_j1", "M1")],
                "is": [("M1", "j1")],
            },
            {
                "title": "Their lines are merged",
                "cmds": ["max$ git merge A1   → X"],
                "commits": [("UM", "X", "X")],
                "edges": [("M1", "X"), ("A1", "X")],
                "is": [("X", "j1")],
                "verdict": "The shared ancestor includes ρ(j1), so Jim's fix is one commit in the umbrella's history. A later trunc revert of j1 merges correctly (the duplicate hazard can't arise).",
            },
        ],
        "conclusion": "a rep is a function of the tool commit and its parent's rep only, so every machine creates the same one. Bringing in is create-rep, then explicit merge.",
    },
    {
        "id": "S11",
        "title": "Ada rebases with plain Git",
        "repos": [TR, UM],
        "compositions": ["UM"],
        "setup": "Continuing S10 before step 5. Ada wants her work on top of Max's m instead of U2, and runs plain <code>git rebase</code>.",
        "steps": [
            {
                "title": "Start",
                "commits": [("TR", "t4", "t4"), ("TR", "j1", "j1 (Jim)"), ("UM", "U2", "U2"), ("UM", "a", "a (Ada)"), ("UM", "m", "m (Max)"), ("UM", "r_j1", "ρ(j1)"), ("UM", "A1", "A1")],
                "edges": [("t4", "j1"), ("U2", "a"), ("U2", "m"), ("U2", "r_j1"), ("a", "A1"), ("r_j1", "A1")],
                "rep": [("U2", "t4"), ("r_j1", "j1")],
                "is": [("a", "t4"), ("m", "t4"), ("A1", "j1")],
            },
            {
                "title": "git rebase m replays a and ρ(j1) as new commits (the merge is dropped)",
                "cmds": ["ada$ git rebase m   (from A1)"],
                "commits": [("UM", "a2", "a′"), ("UM", "r_j1b", "ρ′(j1)")],
                "edges": [("m", "a2"), ("a2", "r_j1b")],
                "is": [("a2", "t4"), ("r_j1b", "j1")],
                "dim": ["a", "A1"],
                "hazard": "ρ′(j1) has the same trunc content, author and message as j1, so it still <em>is</em> j1, but it is a second umbrella commit for j1. If another line (Max's M1) has the real rep ρ(j1), merging the two brings back the duplicate hazard: a trunc revert of j1 could be undone.",
            },
            {
                "title": "What should have happened: move only Ada's own commit, keep the merge",
                "cmds": ["ada$ jj rebase -s a -d m   (or: git rebase of a alone, then git merge ρ(j1))"],
                "commits": [("UM", "a3", "a″"), ("UM", "A3", "A1″")],
                "edges": [("m", "a3"), ("a3", "A3"), ("r_j1", "A3")],
                "is": [("a3", "t4"), ("A3", "j1")],
                "dim": ["a2", "r_j1b"],
                "verdict": "ρ(j1) is untouched and still the only rep. jj's rebase keeps merges. Plain git rebase linearises them, so CloneX must detect ρ′(j1) (\"a second umbrella commit for trunc j1\") and offer this repair.",
            },
        ],
        "conclusion": "reps must never be rebased; a rebase that copies one creates a duplicate. Needs detection in <code>cx status</code>, and jj should treat reps as immutable. Plain <code>git rebase</code> is a hazard to document.",
    },
    {
        "id": "S12",
        "title": "GitHub squash-merges an umbrella PR",
        "repos": [TR, UM],
        "compositions": ["UM"],
        "setup": "Ada's PR head is A1 (her README change merged with ρ(j1)). Nothing else in the umbrella has j1 yet. GitHub's squash button makes one new commit S on top of m.",
        "steps": [
            {
                "title": "Start",
                "commits": [("TR", "t4", "t4"), ("TR", "j1", "j1 (Jim)"), ("UM", "U2", "U2"), ("UM", "a", "a (Ada)"), ("UM", "m", "m"), ("UM", "r_j1", "ρ(j1)"), ("UM", "A1", "A1")],
                "edges": [("t4", "j1"), ("U2", "a"), ("U2", "m"), ("U2", "r_j1"), ("a", "A1"), ("r_j1", "A1")],
                "rep": [("U2", "t4"), ("r_j1", "j1")],
                "is": [("a", "t4"), ("m", "t4"), ("A1", "j1")],
            },
            {
                "title": "Squash: S has one parent (m) and A1's combined changes",
                "cmds": ["github$ squash-merge Ada's PR   → S (author Ada), parent m"],
                "commits": [("UM", "S", "S")],
                "edges": [("m", "S")],
                "is": [("S", "j1")],
                "dim": ["a", "A1"],
                "hazard": "S changes tools/trunc from t4 to j1's content, but it isn't j1's rep. Its message carries ρ(j1)'s <code>Clonex-Source</code> line, and the content matches, so CloneX can prove S <em>is</em> j1: publishing won't push a copy of Jim's fix authored by Ada. But the umbrella's history now has Jim's change inside S, and merging S's line with a line that has the real ρ(j1) brings back the duplicate hazard. A merge commit instead of a squash avoids all of this.",
            },
        ],
        "conclusion": "squash merges of umbrella PRs hide reps inside a new commit. Merge-commit-only (the owner's preference) keeps every rep in the DAG. CloneX must still prove S is j1 by content and trailer, so it never publishes a false copy.",
    },
    {
        "id": "S13",
        "title": "Extract crates/standards into its own repo",
        "repos": [("CC", "crosscut repo (new)"), UM],
        "compositions": ["UM"],
        "setup": "The umbrella's own directory <code>crates/standards</code> becomes a new repo, crosscut, keeping its history. Later an outsider, Olga, contributes to crosscut directly.",
        "steps": [
            {
                "title": "Start: three umbrella commits, two of which touch crates/standards",
                "commits": [("UM", "U1", "U1"), ("UM", "U2", "U2"), ("UM", "U3", "U3")],
                "edges": [("U1", "U2"), ("U2", "U3")],
                "text": "U1 adds standards v1; U2 edits only the README; U3 edits standards and the README.",
            },
            {
                "title": "Derive crosscut's history: the umbrella commits are its reps",
                "cmds": ["max$ cx extract crates/standards   → k1 (from U1), k2 (from U3)"],
                "commits": [("CC", "k1", "k1"), ("CC", "k2", "k2")],
                "edges": [("k1", "k2")],
                "made": [("U1", "k1"), ("U3", "k2")],
                "is": [("U2", "k1")],
                "text": "k1 and k2 were made from U1 and U3, so those are their reps (W14). U2 changes nothing in standards, so it <em>is</em> k1 without being its rep. No new umbrella commits.",
            },
            {
                "title": "Meanwhile the umbrella moves on (U4), and Olga commits o1 to crosscut",
                "commits": [("UM", "U4", "U4"), ("CC", "o1", "o1 (Olga)")],
                "edges": [("U3", "U4"), ("k2", "o1")],
                "is": [("U4", "k2")],
            },
            {
                "title": "o1's rep is built on k2's rep (U3), a full umbrella state",
                "cmds": ["max$ cx fetch crosscut   → ρ(o1): parent U3, tree = U3's with crates/standards := o1"],
                "commits": [("UM", "r_o1", "ρ(o1)")],
                "edges": [("U3", "r_o1")],
                "rep": [("r_o1", "o1")],
            },
            {
                "title": "Explicit merge into the umbrella's current work",
                "cmds": ["max$ git merge ρ(o1)   (on U4) → U5"],
                "commits": [("UM", "U5", "U5")],
                "edges": [("U4", "U5"), ("r_o1", "U5")],
                "is": [("U5", "o1")],
                "verdict": "ρ(o1) looks like an ordinary side branch from U3 that changed only crates/standards; its merge base with U4 is U3, a real full state.",
            },
        ],
        "conclusion": "extraction needs no new umbrella commits: the existing ones become reps. The same placement rule covers later outside commits, and a composition-made rep has a full tree, so it's a good merge base.",
    },
    {
        "id": "S15",
        "title": "Convert a submodule umbrella",
        "repos": [TR, UM],
        "compositions": ["UM"],
        "setup": "Today's umbrella: u1 adds the submodule (gitlink → t2), u2 bumps it (gitlink → t4). Converting must keep u1 and u2 untouched and give the umbrella trunc's real history.",
        "steps": [
            {
                "title": "Start: gitlinks only",
                "commits": [("TR", "t1", "t1"), ("TR", "t2", "t2"), ("TR", "t3", "t3"), ("TR", "t4", "t4"), ("UM", "u1", "u1"), ("UM", "u2", "u2")],
                "edges": [("t1", "t2"), ("t2", "t3"), ("t3", "t4"), ("u1", "u2")],
                "text": "u1 and u2 contain a gitlink, not files. There's no \"is\" line: a gitlink is a pointer, not content.",
            },
            {
                "title": "Create reps for trunc's history, built on the commit being converted",
                "cmds": ["max$ cx adopt tools/trunc   (on u2) → ρ(t1)…ρ(t4)"],
                "commits": [("UM", "r_t1", "ρ(t1)"), ("UM", "r_t2", "ρ(t2)"), ("UM", "r_t3", "ρ(t3)"), ("UM", "r_t4", "ρ(t4)")],
                "edges": [("u2", "r_t1"), ("r_t1", "r_t2"), ("r_t2", "r_t3"), ("r_t3", "r_t4")],
                "rep": [("r_t1", "t1"), ("r_t2", "t2"), ("r_t3", "t3"), ("r_t4", "t4")],
                "text": "trunc's root t1 has no parent rep, so its rep is built on u2, the commit being converted: u2's tree with the gitlink replaced by t1's files. Every rep is a full umbrella state. <code>git log -- tools/trunc</code> now shows all of trunc's commits, with their authors.",
            },
            {
                "title": "The conversion commit",
                "cmds": ["max$ git merge ρ(t4) && git rm .gitmodules entry   → C"],
                "commits": [("UM", "C", "C")],
                "edges": [("r_t4", "C")],
                "is": [("C", "t4")],
                "verdict": "u1 and u2 are unchanged (checking them out still needs the submodule, as it always did). From C on, trunc's files are ordinary files. The same inputs (u2, trunc's history) give the same reps on every machine.",
            },
        ],
        "conclusion": "history before containment gets reps built on the first containing commit, so reps always have full trees. The old umbrella history is never rewritten.",
    },
    {
        "id": "S17",
        "title": "A throwaway composition for an agent",
        "repos": [DS, ("VV", "view repo V (local only)"), ("AH", "agent-harness repo")],
        "compositions": ["VV"],
        "setup": "Ada needs dotsync and agent-harness side by side for one task, in a new composition V that exists only on her machine.",
        "steps": [
            {
                "title": "Start: two tool repos, V empty",
                "commits": [("DS", "d1", "d1"), ("DS", "d2", "d2"), ("AH", "h1", "h1")],
                "edges": [("d1", "d2")],
            },
            {
                "title": "Create reps for both histories",
                "cmds": ["ada$ cx new V --with dotsync --with agent-harness"],
                "commits": [("VV", "v_d1", "ρ(d1)"), ("VV", "v_d2", "ρ(d2)"), ("VV", "v_h1", "ρ(h1)")],
                "edges": [("v_d1", "v_d2")],
                "rep": [("v_d1", "d1"), ("v_d2", "d2"), ("v_h1", "h1")],
                "hazard": "V has no earlier commit to build on, so these first reps contain only their own tool's directory. They are real V states, since V holds nothing else yet, but they are partial trees. A later merge whose base is ρ(d2) sees agent-harness's files as \"added\", the narrow rename-detection residual (G1). CloneX's own merges avoid it; plain <code>git revert</code> of ρ(d2) inside V would not.",
            },
            {
                "title": "V's first ordinary commit joins them",
                "cmds": ["ada$ git merge --allow-unrelated-histories   → V0"],
                "commits": [("VV", "V0", "V0")],
                "edges": [("v_d2", "V0"), ("v_h1", "V0")],
                "is": [("V0", "d2"), ("V0", "h1")],
            },
            {
                "title": "Ada's change across both tools is one V commit, the rep of two tool commits",
                "cmds": ["ada$ git commit -am 'Wire harness to dotsync config'   → w"],
                "commits": [("VV", "w", "w"), ("DS", "xD", "x_D"), ("AH", "xH", "x_H")],
                "edges": [("V0", "w"), ("d2", "xD"), ("h1", "xH")],
                "made": [("w", "xD"), ("w", "xH")],
                "verdict": "Publishing sends x_D and x_H to their repos. V can then be deleted. Nothing of Ada's work lives only in V once both are published.",
            },
        ],
        "conclusion": "throwaway compositions work unchanged, except that the very first reps have partial trees (a documented residual). Safe to discard V is a checkable property: no V commit is the rep of an unpublished tool commit.",
    },
]
