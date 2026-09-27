//! User-level operations. Each is a small composition of the lens and
//! ordinary Git transport; none keeps state outside Git objects and refs.

use crate::git::{Git, Oid};
use crate::lens::{change_id, plan_adoption, Adoption, Composition};
use crate::manifest::{display_name, trailer, Manifest, Occurrence, MANIFEST_PATH};
use anyhow::{bail, Context, Result};
use serde::Serialize;

/// Remote-tracking ref for an occurrence's branch. Keyed by path, since the
/// path is the occurrence's only identity inside a composition. It lives
/// under `refs/remotes/` so jj imports it as an untracked remote bookmark
/// (`tools/trunc/main@clonex`), which jj treats as immutable: component
/// commits can't be rewritten into composition-shaped commits by a jj rebase.
fn remote_ref(path: &str, branch: &str) -> String {
    format!("refs/remotes/clonex/{path}/{branch}")
}

/// Last tip this repository itself pushed to an occurrence's branch.
fn published_ref(path: &str, branch: &str) -> String {
    format!("refs/clonex/published/{path}/{branch}")
}

/// Fetch `branch` of an occurrence's remote. Returns its tip, or None if the
/// branch doesn't exist there.
pub fn fetch(git: &Git, path: &str, occ: &Occurrence, branch: &str) -> Result<Option<Oid>> {
    let url = occ.remote.clone().with_context(|| format!("{path} has no remote"))?;
    let refspec = format!("+refs/heads/{branch}:{}", remote_ref(path, branch));
    let (ok, _, err) = git.try_run(&["fetch", "--quiet", "--no-tags", &url, &refspec], None, &[])?;
    if !ok {
        if err.contains("couldn't find remote ref") {
            return Ok(None);
        }
        bail!("fetching {path} from {url}: {}", err.trim());
    }
    Ok(git.try_rev_parse(&remote_ref(path, branch)))
}

fn fetch_commit(git: &Git, path: &str, occ: &Occurrence, sha: &str) -> Result<()> {
    if git.commit(sha).is_ok() {
        return Ok(());
    }
    let url = occ.remote.clone().with_context(|| format!("{path} has no remote"))?;
    git.run(&["fetch", "--quiet", "--no-tags", &url, sha])
        .with_context(|| format!("fetching {sha} of {path}"))?;
    Ok(())
}

/// Write a composition commit on top of HEAD and move the current branch and
/// working tree to it (refusing if that would overwrite local edits).
fn advance_head(git: &Git, tree: &str, parents: &[Oid], message: &str) -> Result<Oid> {
    let me = git.user_ident()?;
    let head = git.rev_parse("HEAD")?;
    let new = git.commit_tree(tree, parents, &me, &me, message)?;
    let bare = git.run(&["rev-parse", "--is-bare-repository"])?.trim() == "true";
    if !bare {
        git.run(&["read-tree", "-m", "-u", &head, &new])
            .context("the working tree has local changes where the new commit changes files; commit or move them first")?;
    }
    let target = git.current_branch().map(|b| format!("refs/heads/{b}")).unwrap_or_else(|| "HEAD".into());
    git.run(&["update-ref", "-m", "clonex", &target, &new, &head])?;
    Ok(new)
}

fn adoption_message(git: &Git, comp: &Composition, head: &str, claims: &[(String, Oid)], title: &str) -> Result<String> {
    let mut body = String::new();
    for (path, sha) in claims {
        let name = display_name(path);
        let ours = comp.lens(path).get(head)?;
        let range = match &ours {
            Some(o) => format!("{o}..{sha}"),
            None => sha.clone(),
        };
        let subjects = git.run(&["log", "--format=%s", "--first-parent", "-n", "11", &range])?;
        let subjects: Vec<&str> = subjects.lines().collect();
        body.push_str(&format!("{name} ({path}):\n"));
        for s in subjects.iter().take(10) {
            body.push_str(&format!("  - {s}\n"));
        }
        if subjects.len() > 10 {
            body.push_str("  - …\n");
        }
    }
    let trailers: Vec<String> = claims.iter().map(|(p, s)| format!("{}: {p} {s}", trailer::ADOPT)).collect();
    Ok(format!("{title}\n\n{body}\n{}\n", trailers.join("\n")))
}

// ---------------------------------------------------------------- declare

pub struct Declare {
    pub path: String,
    pub remote: Option<String>,
    pub follow: Option<String>,
    /// Derive the history from this repository's own history of `path`
    /// (extracting a directory into a new history).
    pub extract: bool,
    /// Adopt this commit (or ref of the remote) instead of the followed tip.
    pub at: Option<String>,
}

pub fn declare(git: &Git, d: Declare) -> Result<Oid> {
    let path = d.path.trim_matches('/').to_string();
    let head = git.rev_parse("HEAD")?;
    let comp = Composition::new(git);
    let mut manifest = comp.manifest_at(&head)?;
    if manifest.occurrences.contains_key(&path) {
        bail!("{path} is already declared");
    }
    let occ = Occurrence { remote: d.remote.clone(), follow: d.follow.clone() };
    let head_tree = git.commit(&head)?.tree;
    let existing = git.entry(&head_tree, &path)?;

    let component: Oid = if d.extract {
        if existing.as_ref().map(|e| e.kind.as_str()) != Some("tree") {
            bail!("--extract needs an existing directory at {path}");
        }
        // The directory's own history, derived by the same lens as always.
        comp.lens(&path).get(&head)?.with_context(|| format!("{path} has no history"))?
    } else if let Some(at) = &d.at {
        if let Some(sha) = git.try_rev_parse(at) {
            sha
        } else {
            fetch_commit(git, &path, &occ, at)?;
            git.rev_parse(at)?
        }
    } else if let Some(e) = existing.as_ref().filter(|e| e.kind == "commit") {
        // A submodule gitlink: the pointer becomes an adoption of that commit.
        fetch_commit(git, &path, &occ, &e.oid)?;
        if let Some(f) = &occ.follow {
            let _ = fetch(git, &path, &occ, f);
        }
        e.oid.clone()
    } else {
        let branch = occ.follow.clone().unwrap_or_else(|| "main".into());
        fetch(git, &path, &occ, &branch)?.with_context(|| format!("{path}: remote has no branch {branch}"))?
    };

    if existing.as_ref().map(|e| e.kind == "commit").unwrap_or(false) {
        release_submodule_checkout(git, &path)?;
    }
    let component_tree = git.commit(&component)?.tree;
    if let Some(e) = &existing {
        if e.kind == "tree" && e.oid != component_tree && !d.extract {
            bail!(
                "{path} already has content that differs from {}; declare with --extract to derive its history from this repository, or remove it first",
                &component[..12]
            );
        }
    }
    let mut tree = git.replace_subtree(&head_tree, &path, Some(&component_tree))?;
    if existing.as_ref().map(|e| e.kind == "commit").unwrap_or(false) {
        tree = drop_gitmodules_entry(git, &tree, &path)?;
    }
    manifest.occurrences.insert(path.clone(), occ);
    let blob = git.write_blob(manifest.render().as_bytes())?;
    tree = git.replace_at(&tree, MANIFEST_PATH, Some(("100644", "blob", &blob)))?;
    let name = display_name(&path);
    let msg = format!(
        "Contain {name} at {path}\n\n{name}'s history is now part of this repository's history.\n\n{}: {path} {component}\n",
        trailer::ADOPT
    );
    advance_head(git, &tree, &[head, component], &msg)
}

/// A submodule's working directory is a separate repository; its files are
/// about to become ordinary files of this one. Refuse if it has any work
/// that isn't committed, then release it.
fn release_submodule_checkout(git: &Git, path: &str) -> Result<()> {
    let (ok, out, _) = git.try_run(&["-C", path, "status", "--porcelain", "--ignored=no"], None, &[])?;
    if !ok {
        return Ok(()); // not checked out
    }
    if !out.trim().is_empty() {
        bail!("the submodule checkout at {path} has uncommitted changes; commit or remove them first:\n{out}");
    }
    git.run(&["submodule", "deinit", "--quiet", "--force", "--", path])?;
    Ok(())
}

fn drop_gitmodules_entry(git: &Git, tree: &str, path: &str) -> Result<Oid> {
    let Some(text) = git.read_blob_at(tree, ".gitmodules")? else {
        return Ok(tree.to_string());
    };
    let mut out = String::new();
    let mut skip = false;
    let mut current = String::new();
    let mut sections: Vec<(String, bool)> = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with('[') {
            if !current.is_empty() {
                sections.push((std::mem::take(&mut current), skip));
            }
            skip = false;
        }
        if line.trim().strip_prefix("path").map(|r| r.trim_start().trim_start_matches('=').trim() == path).unwrap_or(false) {
            skip = true;
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.is_empty() {
        sections.push((current, skip));
    }
    for (s, skip) in sections {
        if !skip {
            out.push_str(&s);
        }
    }
    if out.trim().is_empty() {
        git.replace_at(tree, ".gitmodules", None)
    } else {
        let blob = git.write_blob(out.as_bytes())?;
        git.replace_at(tree, ".gitmodules", Some(("100644", "blob", &blob)))
    }
}

// ---------------------------------------------------------------- sync

pub struct SyncRequest {
    /// Occurrence names or paths; empty = every occurrence that follows a branch.
    pub only: Vec<String>,
    /// Adopt this branch instead of each occurrence's followed branch
    /// (e.g. a PR branch where a CI bot added commits).
    pub branch: Option<String>,
}

#[derive(Serialize)]
pub struct SyncReport {
    pub adopted: Vec<(String, Oid)>,
    pub commit: Option<Oid>,
}

pub fn sync(git: &Git, req: SyncRequest) -> Result<SyncReport> {
    let head = git.rev_parse("HEAD")?;
    let comp = Composition::new(git);
    let manifest = comp.manifest_at(&head)?;
    let mut adoptions = Vec::new();
    let mut branches = std::collections::HashMap::new();
    for (path, occ) in selected(&manifest, &req.only)? {
        let branch = match (&req.branch, &occ.follow) {
            (Some(b), _) => b.clone(),
            (None, Some(f)) => f.clone(),
            (None, None) if !req.only.is_empty() => bail!("{path} is frozen (no follow branch); pass --branch"),
            (None, None) => continue,
        };
        if let Some(tip) = fetch(git, path, occ, &branch)? {
            branches.insert(path.clone(), branch.clone());
            adoptions.push(Adoption { path: path.clone(), commit: tip });
        }
    }
    let Some((tree, parents, claims)) = plan_adoption(&comp, &head, &adoptions)? else {
        return Ok(SyncReport { adopted: vec![], commit: None });
    };
    let names: Vec<String> = claims
        .iter()
        .map(|(p, s)| format!("{} {} ({})", display_name(p), branches[p], &s[..10]))
        .collect();
    // Worded as an ordinary upstream merge: when the composition holds
    // unpublished work, this commit is also derived into the component's
    // own history, where "Merge dotsync main (…)" reads naturally.
    let msg = adoption_message(git, &comp, &head, &claims, &format!("Merge {}", names.join(", ")))?;
    let commit = advance_head(git, &tree, &parents, &msg)?;
    Ok(SyncReport { adopted: claims, commit: Some(commit) })
}

fn selected<'m>(manifest: &'m Manifest, only: &[String]) -> Result<Vec<(&'m String, &'m Occurrence)>> {
    if only.is_empty() {
        return Ok(manifest.occurrences.iter().collect());
    }
    only.iter()
        .map(|n| manifest.find(n).with_context(|| format!("no occurrence {n}")))
        .collect()
}

// ---------------------------------------------------------------- status

#[derive(Serialize)]
pub struct OccurrenceStatus {
    pub path: String,
    pub follow: Option<String>,
    pub remote: Option<String>,
    /// The component commit this composition's current state corresponds to.
    pub derived: Option<Oid>,
    /// Last fetched tip of the followed branch.
    pub remote_tip: Option<Oid>,
    /// Derived commits the followed branch doesn't have (unpublished or unlanded).
    pub ahead: Vec<Change>,
    /// Commits on the followed branch the composition hasn't adopted.
    pub behind: usize,
}

#[derive(Serialize)]
pub struct Change {
    pub commit: Oid,
    pub change: Option<String>,
    pub subject: String,
}

pub fn status(git: &Git, rev: &str) -> Result<Vec<OccurrenceStatus>> {
    let head = git.rev_parse(rev)?;
    let comp = Composition::new(git);
    let manifest = comp.manifest_at(&head)?;
    let mut out = Vec::new();
    for (path, occ) in &manifest.occurrences {
        let derived = comp.lens(path).get(&head)?;
        let remote_tip = occ.follow.as_ref().and_then(|b| git.try_rev_parse(&remote_ref(path, b)));
        let (ahead, behind) = match (&derived, &remote_tip) {
            (Some(d), Some(r)) => (changes_in(git, &format!("{r}..{d}"))?, git.rev_list(&["--count", &format!("{d}..{r}")])?[0].parse()?),
            (Some(d), None) => (changes_in(git, d)?, 0),
            _ => (vec![], 0),
        };
        out.push(OccurrenceStatus {
            path: path.clone(),
            follow: occ.follow.clone(),
            remote: occ.remote.clone(),
            derived,
            remote_tip,
            ahead,
            behind,
        });
    }
    Ok(out)
}

fn changes_in(git: &Git, range: &str) -> Result<Vec<Change>> {
    git.rev_list(&["--topo-order", range])?
        .into_iter()
        .map(|oid| {
            let c = git.commit(&oid)?;
            Ok(Change { change: change_id(&c), subject: c.subject().to_string(), commit: oid })
        })
        .collect()
}

// ---------------------------------------------------------------- publish

pub struct PublishRequest {
    pub only: Vec<String>,
    /// Target branch in every component; default: the composition's current branch.
    pub branch: Option<String>,
    pub dry_run: bool,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum PublishOutcome {
    UpToDate,
    Pushed { from: Option<Oid>, to: Oid, commits: usize, replaced: bool },
    WouldPush { from: Option<Oid>, to: Oid, commits: usize, replaced: bool },
    /// The remote branch has commits the composition hasn't adopted (another
    /// person, a CI bot, a GitHub merge): adopt them first (`sync --branch`).
    Diverged { remote: Oid, local: Oid },
    Rejected { reason: String },
    NoRemote,
}

#[derive(Serialize)]
pub struct PublishResult {
    pub path: String,
    pub branch: String,
    #[serde(flatten)]
    pub outcome: PublishOutcome,
}

/// Publication is convergence, not a transaction: for each occurrence, make
/// the remote branch contain the derived commit. Re-running after a partial
/// failure only does what is still missing.
pub fn publish(git: &Git, req: PublishRequest) -> Result<Vec<PublishResult>> {
    let head = git.rev_parse("HEAD")?;
    let comp = Composition::new(git);
    let manifest = comp.manifest_at(&head)?;
    let branch = match &req.branch {
        Some(b) => b.clone(),
        None => git.current_branch().context("HEAD is detached; pass --branch")?,
    };
    let mut results = Vec::new();
    for (path, occ) in selected(&manifest, &req.only)? {
        // A frozen occurrence (pinned fixture, vendored copy) is published
        // only when asked for by name: its line is not the branch's line.
        if req.only.is_empty() && occ.follow.is_none() {
            continue;
        }
        let Some(derived) = comp.lens(path).get(&head)? else { continue };
        let outcome = publish_one(git, path, occ, &derived, &branch, req.dry_run)?;
        results.push(PublishResult { path: path.clone(), branch: branch.clone(), outcome });
    }
    Ok(results)
}

fn publish_one(git: &Git, path: &str, occ: &Occurrence, derived: &str, branch: &str, dry_run: bool) -> Result<PublishOutcome> {
    let Some(url) = occ.remote.clone() else { return Ok(PublishOutcome::NoRemote) };
    let tip = fetch(git, path, occ, branch)?;
    // A new branch starts from the followed branch: nothing of ours to
    // publish if that already contains the derived commit, and only the
    // commits beyond it count as ours.
    let mut new_branch_base = None;
    if tip.is_none() {
        if let Some(f) = occ.follow.as_ref().filter(|f| f.as_str() != branch) {
            if let Some(ft) = fetch(git, path, occ, f)? {
                if git.is_ancestor(derived, &ft)? {
                    return Ok(PublishOutcome::UpToDate);
                }
                new_branch_base = Some(ft);
            }
        }
    }
    // Replacing a branch is safe exactly when its tip is what this repository
    // last published there: every commit on it is then one of ours, so the
    // new derivation supersedes it (a rewritten change on a PR branch). If
    // anyone else added commits, they must be adopted first.
    let ours_last = git.try_rev_parse(&published_ref(path, branch));
    let (commits, replaced) = match &tip {
        Some(t) if git.is_ancestor(derived, t)? => return Ok(PublishOutcome::UpToDate),
        Some(t) if git.is_ancestor(t, derived)? => (count(git, &format!("{t}..{derived}"))?, false),
        Some(t) if ours_last.as_deref() == Some(t.as_str()) => (count(git, &format!("{t}..{derived}"))?, true),
        Some(t) => return Ok(PublishOutcome::Diverged { remote: t.clone(), local: derived.to_string() }),
        None => match &new_branch_base {
            Some(base) => (count(git, &format!("{base}..{derived}"))?, false),
            None => (count(git, derived)?, false),
        },
    };
    if dry_run {
        return Ok(PublishOutcome::WouldPush { from: tip, to: derived.to_string(), commits, replaced });
    }
    let refspec = format!("{derived}:refs/heads/{branch}");
    let lease = tip.as_ref().map(|t| format!("--force-with-lease=refs/heads/{branch}:{t}"));
    let mut args = vec!["push", "--quiet", "--no-verify"];
    if replaced {
        args.push(lease.as_deref().expect("replacement implies a tip"));
    }
    args.push(&url);
    args.push(&refspec);
    let (ok, _, err) = git.try_run(&args, None, &[])?;
    if !ok {
        return Ok(PublishOutcome::Rejected { reason: err.trim().to_string() });
    }
    git.update_ref(&remote_ref(path, branch), derived)?;
    git.update_ref(&published_ref(path, branch), derived)?;
    Ok(PublishOutcome::Pushed { from: tip, to: derived.to_string(), commits, replaced })
}

fn count(git: &Git, range: &str) -> Result<usize> {
    Ok(git.rev_list(&["--count", range])?[0].parse()?)
}

// ---------------------------------------------------------------- queries

/// Where does a logical change appear? Composition commits carrying it and,
/// per occurrence, derived component commits carrying it.
#[derive(Serialize)]
pub struct Appearance {
    pub place: String,
    pub commit: Oid,
    pub subject: String,
}

pub fn where_change(git: &Git, rev: &str, change: &str) -> Result<Vec<Appearance>> {
    let head = git.rev_parse(rev)?;
    let comp = Composition::new(git);
    let manifest = comp.manifest_at(&head)?;
    let mut out = Vec::new();
    for oid in own_history(&comp, &head)? {
        let c = git.commit(&oid)?;
        if change_id(&c).is_some_and(|id| id.starts_with(change)) {
            out.push(Appearance { place: "composition".into(), commit: oid, subject: c.subject().into() });
        }
    }
    for path in manifest.occurrences.keys() {
        let Some(d) = comp.lens(path).get(&head)? else { continue };
        for oid in git.rev_list(&["--topo-order", &d])? {
            let c = git.commit(&oid)?;
            if trailer::changes(&c.message).iter().any(|id| id.starts_with(change)) {
                out.push(Appearance { place: path.clone(), commit: oid, subject: c.subject().into() });
            }
        }
    }
    Ok(out)
}

/// The composition's own commits reachable from `head` (not adopted
/// component commits).
pub fn own_history(comp: &Composition, head: &str) -> Result<Vec<Oid>> {
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![head.to_string()];
    let mut out = Vec::new();
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid.clone()) {
            continue;
        }
        let c = comp.git.commit(&oid)?;
        stack.extend(comp.own_parents(&c));
        out.push(oid);
    }
    Ok(out)
}
