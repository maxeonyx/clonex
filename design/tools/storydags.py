#!/usr/bin/env python3
"""Render CloneX stories as per-timestep commit DAGs (no refs).

Each story is data (stories_spec.py): repositories, and steps that add
commits, parent edges, "is" lines and "rep" lines. Every step draws the
cumulative DAG, so nothing can be skipped by accident. Output: one HTML
page (mermaid).

    python3 design/tools/storydags.py > design/story-dags.html

Conventions (see AGENTS.md):
- circle = commit; dashed box = repository; arrow = parent -> child
- dotted "is": mapping this composition commit down (π) gives that tool
  commit
- thick "is · rep": this composition commit is also THE one that stands for
  that tool commit (ρ); every tool commit has exactly one. In the spec,
  "made" marks a rep the tool commit was derived from (W14), "rep" one
  brought in from outside (must be built on its tool parents' reps)
- faded = no longer part of any line of work
- a composition can itself be a component ("components" maps each
  composition to what it contains; by default, every non-composition repo)
- a commit is named by its label (U_3, π_T(U_3), ρ_U(T_5)); is/rep/made
  entries are (composition commit, tool commit) or, when one tool appears
  at two paths, (composition commit, tool commit, occurrence)
"""
import html
import re
import sys

from stories_spec import STORIES

# Per-step highlights: class -> (colour, legend text).
COLOURS = {
    "picked": ("#e8a317", "selected to be copied"),
    "copy": ("#e8a317", "a new copy (new SHA)"),
    "dropped": ("#d64545", "dropped"),
    "kept": ("#2e9d5b", "not touched"),
}

SUB = re.compile(r"(?<![A-Za-z])([A-Zρπ][0-9]?)_([0-9A-Za-z.]+)")


def fmt(s):
    """X_n → X<sub>n</sub>, for text that is already HTML."""
    return SUB.sub(r"\1<sub>\2</sub>", s)


def occ(line, repo_of):
    """The occurrence (tool copy) a line talks about: explicit, else the tool repo."""
    return line[2] if len(line) > 2 else repo_of[line[1]]


def state(story, upto):
    commits, edges, is_lines, rep_lines, dim = [], [], [], [], set()
    for step in story["steps"][: upto + 1]:
        commits += step.get("commits", [])
        edges += step.get("edges", [])
        is_lines += step.get("is", [])
        rep_lines += step.get("rep", []) + step.get("made", [])
        dim |= set(step.get("dim", []))
    return commits, edges, is_lines, rep_lines, dim


def legend(step):
    items = [
        f"<span><i style='border-color:{COLOURS[c][0]}'></i>{fmt(', '.join(html.escape(n) for n in names))}: {COLOURS[c][1]}</span>"
        for c, names in step.get("colours", {}).items()
    ]
    return f"<p class='legend'>{''.join(items)}</p>" if items else ""


def mermaid(story, upto):
    commits, edges, is_lines, rep_lines, dim = state(story, upto)
    ids = {c[1]: f"n{i}" for i, c in enumerate(commits)}
    out = ["flowchart LR"]
    for repo, title in story["repos"]:
        out.append(f'  subgraph {repo}["{repo} · {title}"]')
        for c in commits:
            if c[0] == repo:
                note = f"<br><small>{html.escape(c[2])}</small>" if len(c) > 2 else ""
                out.append(f'    {ids[c[1]]}(("{fmt(html.escape(c[1]))}{note}"))')
        out.append("  end")
    for a, b in edges:
        out.append(f"  {ids[a]} --> {ids[b]}")

    def link(kind, line):
        text = kind if len(line) < 3 else f"{kind} @{line[2]}"
        style = "-.-" if kind == "is" else "==="
        return f'  {ids[line[0]]} {style}|"{text}"| {ids[line[1]]}'

    out += [link("is · rep", line) for line in rep_lines]
    out += [link("is", line) for line in is_lines if line not in rep_lines]
    for repo, _ in story["repos"]:
        out.append(f"  style {repo} fill:none,stroke-dasharray: 6 4")
    for cls, names in story["steps"][upto].get("colours", {}).items():
        colour = COLOURS[cls][0]
        dash = ",stroke-dasharray: 5 3" if cls == "dropped" else ""
        fill = f",fill:{colour}44" if cls == "copy" else ""
        out.append(f"  classDef {cls} stroke:{colour},stroke-width:4px{dash}{fill}")
        out.append(f"  class {','.join(ids[n] for n in names)} {cls}")
    if dim:
        out.append("  classDef dim opacity:0.35")
        out.append("  class " + ",".join(sorted(ids[d] for d in dim)) + " dim")
    return "\n".join(out)


CSS = """
:root{--bg:#f6f7f9;--ink:#1d2330;--muted:#5b6475;--rule:#d9dde5;--card:#fff;--accent:#2f6fdb;--ok:#1f8a5b;--warn:#b5651d;
--mono:"JetBrains Mono",ui-monospace,SFMono-Regular,Menlo,monospace;--sans:"IBM Plex Sans",system-ui,-apple-system,"Segoe UI",sans-serif}
@media (prefers-color-scheme: dark){:root:not([data-theme="light"]){--bg:#12151b;--ink:#e6e9ef;--muted:#9aa3b5;--rule:#2a303b;--card:#191d25;--accent:#7aa7ff;--ok:#5fd39b;--warn:#f0a35e;color-scheme:dark}}
:root[data-theme="dark"]{--bg:#12151b;--ink:#e6e9ef;--muted:#9aa3b5;--rule:#2a303b;--card:#191d25;--accent:#7aa7ff;--ok:#5fd39b;--warn:#f0a35e;color-scheme:dark}
body{background:var(--bg);color:var(--ink);font-family:var(--sans);font-size:15px;line-height:1.55;padding-inline:16px;padding-block:24px 48px}
main{max-width:1000px;margin:0 auto;display:grid;gap:22px}
h1{font-size:1.7rem;margin:0;text-wrap:balance}h2{font-size:1.35rem;margin:18px 0 0;text-wrap:balance}h3{font-size:1.02rem;margin:0}
p{margin:0;max-width:70ch}code{font-family:var(--mono);font-size:.9em}
.lede,.key{color:var(--muted)}.key{display:grid;gap:3px;font-size:.92rem}
nav{display:flex;flex-wrap:wrap;gap:6px 14px;font-size:.92rem}nav a{color:var(--accent)}
section{background:var(--card);border:1px solid var(--rule);border-radius:8px;padding:16px;display:grid;gap:10px}
.step{font-family:var(--mono);font-size:.78rem;letter-spacing:.06em;text-transform:uppercase;color:var(--accent)}
pre.cmd{font-family:var(--mono);font-size:.85rem;background:var(--bg);border:1px solid var(--rule);border-radius:6px;padding:8px 10px;margin:0;overflow-x:auto;white-space:pre}
.diagram{overflow-x:auto}
.legend{display:flex;flex-wrap:wrap;gap:4px 16px;font-size:.88rem;color:var(--muted)}.legend i{display:inline-block;width:12px;height:12px;border:3px solid;border-radius:50%;margin-right:6px;vertical-align:-2px}
.verdict{border-left:3px solid var(--ok);padding-left:12px}.hazard{border-left:3px solid var(--warn);padding-left:12px}
"""

KEY = """<div class='key'>
<span>○ circle = a commit · dashed box = a repository with its own commit DAG · arrow = parent → child</span>
<span>each repository is a letter (U umbrella, T trunc, D dotsync, …); its commits are U_1, U_2, …, with U_2a, U_2b for two lines from one point, and U_3′ for a new version of U_3</span>
<span><b>π_T(U_3)</b> = the trunc commit that U_3 maps down to. Every umbrella commit that contains trunc maps down to exactly one trunc commit; it has this name when U_3 is where it first appears</span>
<span><b>ρ_U(T_5)</b> = the umbrella commit that stands for T_5 (its rep), when T_5 was brought in from outside. Every tool commit has exactly one rep, and π_T(ρ_U(T_5)) = T_5</span>
<span>dotted <b>is</b> = π: mapping this umbrella commit down gives that tool commit · thick <b>is · rep</b> = π, and ρ back: this is <em>the</em> umbrella commit for that tool commit</span>
<span>a tool in two places has occurrences R1, R2, written π_R2 and ρ_U.R2 · faded = no longer part of any line of work</span>
</div>"""


def check(story):
    """Refuse to draw a story that breaks the model's own bookkeeping."""
    repo_of, reps, is_of = {}, {}, {}
    comps = set(story["compositions"])
    tools = [r for r, _ in story["repos"] if r not in comps]
    parts = story.get("components", {c: tools for c in comps})
    for i, step in enumerate(story["steps"]):
        where = f"{story['id']} step {i}"
        for c in step.get("commits", []):
            assert c[1] not in repo_of, f"{where}: {c[1]} added twice"
            repo_of[c[1]] = c[0]
        for a, b in step.get("edges", []):
            assert a in repo_of and b in repo_of, f"{where}: edge {a}->{b} names a missing commit"
            assert repo_of[a] == repo_of[b], f"{where}: parent edge {a}->{b} crosses repositories"
        for d in step.get("dim", []) + [n for ns in step.get("colours", {}).values() for n in ns]:
            assert d in repo_of, f"{where}: dims or colours missing commit {d}"
        for kind in ("is", "rep", "made"):
            for line in step.get(kind, []):
                a, b = line[:2]
                assert a in repo_of and b in repo_of, f"{where}: {kind} {a}-{b} names a missing commit"
                assert repo_of[b] in parts.get(repo_of[a], []), f"{where}: {kind} must go from a composition to one of its components"
                o = occ(line, repo_of)
                prev = is_of.get((a, o))
                assert prev in (None, b), f"{where}: {a} is both {prev} and {b} at {o}"
                is_of[(a, o)] = b
        for kind in ("rep", "made"):
            for line in step.get(kind, []):
                key = (line[1], occ(line, repo_of))
                assert key not in reps, f"{where}: {key} has two reps ({reps[key][0]}, {line[0]})"
                reps[key] = (line[0], kind)
        edges_so_far = [e for st in story["steps"][: i + 1] for e in st.get("edges", [])]
        for (b, o), (a, kind) in reps.items():  # a is the rep of tool commit b at occurrence o
            tool_parents = [p for p, c in edges_so_far if c == b]
            comp_parents = [p for p, c in edges_so_far if c == a]
            for tp in tool_parents:
                if kind == "rep" and (tp, o) in reps:
                    # Brought in: built on the rep of each tool parent (C″).
                    assert reps[(tp, o)][0] in comp_parents, (
                        f"{where}: rep {a} of {b} is not built on {tp}'s rep {reps[(tp, o)][0]}"
                    )
                else:
                    # Made here (or parent's rep not drawn): some parent must BE the tool parent.
                    assert any(is_of.get((cp, o)) == tp for cp in comp_parents), (
                        f"{where}: {a} (rep of {b}) has no parent that is {b}'s parent {tp}"
                    )


def render():
    for s in STORIES:
        check(s)
    parts = [
        "<title>CloneX Story DAGs</title>",
        f"<style>{CSS}</style>",
        '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;600&family=JetBrains+Mono:wght@400;600&display=swap">',
        "<main>",
        "<header style='display:grid;gap:10px'><h1>CloneX Story DAGs</h1>",
        "<p class='lede'>Stories from <code>design/stories.md</code>, walked one event at a time. Commits only, no refs. Generated by <code>design/tools/storydags.py</code>.</p>",
        fmt(KEY),
        "<nav>" + "".join(f"<a href='#{s['id']}'>{s['id']} {fmt(html.escape(s['title']))}</a>" for s in STORIES) + "</nav></header>",
    ]
    for s in STORIES:
        parts.append(f"<h2 id='{s['id']}'>{s['id']} · {fmt(html.escape(s['title']))}</h2>")
        parts.append(f"<p>{fmt(s['setup'])}</p>")
        for i, step in enumerate(s["steps"]):
            parts.append("<section>")
            parts.append(f"<div class='step'>Step {i}</div><h3>{fmt(html.escape(step['title']))}</h3>")
            if step.get("cmds"):
                parts.append("<pre class='cmd'>" + fmt(html.escape("\n".join(step["cmds"]))) + "</pre>")
            if step.get("text"):
                parts.append(f"<p>{fmt(step['text'])}</p>")
            parts.append(f"<div class='diagram'><pre class='mermaid'>\n{mermaid(s, i)}\n</pre></div>")
            parts.append(legend(step))
            for key in ("verdict", "hazard"):
                if step.get(key):
                    parts.append(f"<p class='{key}'>{fmt(step[key])}</p>")
            parts.append("</section>")
        if s.get("conclusion"):
            parts.append(f"<p class='verdict'><b>What this story demands:</b> {fmt(s['conclusion'])}</p>")
    parts.append("</main>")
    return "\n".join(parts)


if __name__ == "__main__":
    sys.stdout.write(render())
