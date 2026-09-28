#!/usr/bin/env python3
"""Render CloneX stories as per-timestep commit DAGs (no refs).

Each story is data: repositories, and steps that add commits, parent
edges, "is" lines and "rep" lines. Every step draws the cumulative DAG, so
nothing can be skipped by accident. Output: one HTML page (mermaid).

    python3 design/tools/storydags.py > design/story-dags.html

Conventions (see AGENTS.md):
- circle = commit; dashed box = repository; arrow = parent -> child
- dotted "is": mapping this composition commit down gives that tool commit
- thick "rep": this composition commit is THE one that stands for that tool
  commit (every tool commit has exactly one); rep implies is. In the spec,
  "made" marks a rep the tool commit was derived from (W14), "rep" one
  brought in from outside (must be built on its tool parents' reps)
- grey = no longer reachable from any line anyone is working on
"""
import html
import sys

from stories_spec import STORIES


def mermaid(story, upto):
    repos = story["repos"]
    nodes = {r: [] for r, _ in repos}
    edges, is_lines, rep_lines, dim = [], [], [], set()
    for step in story["steps"][: upto + 1]:
        for repo, nid, label in step.get("commits", []):
            nodes[repo].append((nid, label))
        edges += step.get("edges", [])
        is_lines += step.get("is", [])
        rep_lines += step.get("rep", []) + step.get("made", [])
        dim |= set(step.get("dim", []))
    out = ["flowchart LR"]
    for repo, title in repos:
        out.append(f"  subgraph {repo}[{title}]")
        for nid, label in nodes[repo]:
            out.append(f'    {nid}(("{label}"))')
        out.append("  end")
    for a, b in edges:
        out.append(f"  {a} --> {b}")
    reps = set(rep_lines)
    for a, b in rep_lines:
        out.append(f"  {a} == rep === {b}")
    for a, b in is_lines:
        if (a, b) not in reps:
            out.append(f"  {a} -. is .- {b}")
    for repo, _ in repos:
        out.append(f"  style {repo} fill:none,stroke-dasharray: 6 4")
    if dim:
        out.append("  classDef dim opacity:0.35")
        out.append("  class " + ",".join(sorted(dim)) + " dim")
    return "\n".join(out)


def table(story, upto):
    labels, is_map, order, comp_repos = {}, {}, [], set(story.get("compositions", []))
    tool_repos = [r for r, _ in story["repos"] if r not in comp_repos]
    repo_of = {}
    for step in story["steps"][: upto + 1]:
        for repo, nid, label in step.get("commits", []):
            labels[nid] = label
            repo_of[nid] = repo
            if repo in comp_repos:
                order.append(nid)
        for a, b in step.get("is", []) + step.get("rep", []) + step.get("made", []):
            is_map.setdefault(a, {})[repo_of[b]] = labels[b]
    if not order:
        return ""
    titles = dict(story["repos"])
    head = "".join(f"<th>{html.escape(titles[r])} is</th>" for r in tool_repos)
    rows = []
    for nid in order:
        cells = "".join(
            f"<td><code>{html.escape(is_map.get(nid, {}).get(r, '—'))}</code></td>" for r in tool_repos
        )
        rows.append(f"<tr><td><code>{html.escape(labels[nid])}</code></td>{cells}</tr>")
    return (
        '<div class="tablewrap"><table><tr><th>composition commit</th>'
        + head
        + "</tr>"
        + "".join(rows)
        + "</table></div>"
    )


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
.diagram,.tablewrap{overflow-x:auto}
table{border-collapse:collapse;font-size:.88rem}th,td{text-align:left;padding:3px 14px 3px 0;border-bottom:1px solid var(--rule)}th{color:var(--muted);font-weight:600}
.verdict{border-left:3px solid var(--ok);padding-left:12px}.hazard{border-left:3px solid var(--warn);padding-left:12px}
"""


def check(story):
    """Refuse to draw a story that breaks the model's own bookkeeping."""
    seen, repo_of, reps, is_of = [], {}, {}, {}
    comps = set(story.get("compositions", []))
    for i, step in enumerate(story["steps"]):
        where = f"{story['id']} step {i}"
        for repo, nid, _ in step.get("commits", []):
            assert nid not in repo_of, f"{where}: {nid} added twice"
            repo_of[nid] = repo
        for a, b in step.get("edges", []):
            assert a in repo_of and b in repo_of, f"{where}: edge {a}->{b} names a missing commit"
            assert repo_of[a] == repo_of[b], f"{where}: parent edge {a}->{b} crosses repositories"
        for kind in ("is", "rep", "made"):
            for a, b in step.get(kind, []):
                assert a in repo_of and b in repo_of, f"{where}: {kind} {a}-{b} names a missing commit"
                assert repo_of[a] in comps and repo_of[b] not in comps, f"{where}: {kind} must go composition -> tool"
                prev = is_of.get((a, repo_of[b]))
                assert prev in (None, b), f"{where}: {a} is both {prev} and {b} in {repo_of[b]}"
                is_of[(a, repo_of[b])] = b
        for kind in ("rep", "made"):
            for a, b in step.get(kind, []):
                assert b not in reps, f"{where}: {b} has two reps ({reps[b][0]}, {a})"
                reps[b] = (a, kind)
        edges_so_far = [e for st in story["steps"][: i + 1] for e in st.get("edges", [])]
        for b, (a, kind) in reps.items():  # a is the rep of tool commit b
            tool_parents = [p for p, c in edges_so_far if c == b]
            comp_parents = [p for p, c in edges_so_far if c == a]
            for tp in tool_parents:
                if kind == "rep" and tp in reps:
                    # Brought in: built on the rep of each tool parent (C″).
                    assert reps[tp][0] in comp_parents, (
                        f"{where}: rep {a} of {b} is not built on {tp}'s rep {reps[tp][0]}"
                    )
                else:
                    # Made here (or parent's rep not drawn): some parent must BE the tool parent.
                    assert any(is_of.get((cp, repo_of[b])) == tp for cp in comp_parents), (
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
        "<div class='key'><span>○ circle = a commit · dashed box = a repository with its own commit DAG · arrow = parent → child</span>"
        "<span>dotted <b>is</b> = mapping this composition commit down to the tool gives exactly that commit</span>"
        "<span>thick <b>rep</b> = this is <em>the</em> composition commit that stands for that tool commit; every tool commit has exactly one</span>"
        "<span>faded = no longer part of any line of work (e.g. replaced by a rebase)</span></div>",
        "<nav>" + "".join(f"<a href='#{s['id']}'>{s['id']} {html.escape(s['title'])}</a>" for s in STORIES) + "</nav></header>",
    ]
    for s in STORIES:
        parts.append(f"<h2 id='{s['id']}'>{s['id']} · {html.escape(s['title'])}</h2>")
        parts.append(f"<p>{s['setup']}</p>")
        for i, step in enumerate(s["steps"]):
            parts.append("<section>")
            parts.append(f"<div class='step'>Step {i}</div><h3>{step['title']}</h3>")
            if step.get("cmds"):
                parts.append("<pre class='cmd'>" + html.escape("\n".join(step["cmds"])) + "</pre>")
            if step.get("text"):
                parts.append(f"<p>{step['text']}</p>")
            parts.append(f"<div class='diagram'><pre class='mermaid'>\n{mermaid(s, i)}\n</pre></div>")
            parts.append(table(s, i))
            if step.get("verdict"):
                parts.append(f"<p class='verdict'>{step['verdict']}</p>")
            if step.get("hazard"):
                parts.append(f"<p class='hazard'>{step['hazard']}</p>")
            parts.append("</section>")
        if s.get("conclusion"):
            parts.append(f"<p class='verdict'><b>What this story demands:</b> {s['conclusion']}</p>")
    parts.append("</main>")
    return "\n".join(parts)


if __name__ == "__main__":
    sys.stdout.write(render())
