//! The lens between a composition and one of its occurrences.
//!
//! `get` (filter): composition commit -> component commit at a path. A total,
//! pure function of Git objects and the path — it does not consult the
//! manifest, so the history of any directory can be derived (extraction is
//! just `get` followed by adoption); derived commits are written to the object store but are
//! only *published* when someone pushes them.
//!
//! `put` (adopt): embed a component commit into the composition as a merge
//! whose extra parent *is* the component commit and whose message records
//! `Clonex-Adopt: <path> <sha>`.
//!
//! Laws (tested in `tests/laws.rs`):
//! - PutGet: `get(put(c, t)) == t` when the adoption leaves no delta.
//! - GetPut: `put(c, get(c))` does not change the composition tree.
//! - Determinism: `get` depends only on objects, never on clock or user.
//! - Locality: rewriting composition commits in ways that do not touch the
//!   occurrence (reorder, committer changes) leaves `get` unchanged.

use crate::git::{Commit, Git, MergeOutcome, Oid};
use crate::manifest::{trailer, Manifest, MANIFEST_PATH};
use anyhow::{bail, Result};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// Composition-side knowledge shared by all lenses over one repository.
pub struct Composition<'g> {
    pub git: &'g Git,
    manifests: RefCell<HashMap<Oid, Manifest>>,
}

impl<'g> Composition<'g> {
    pub fn new(git: &'g Git) -> Self {
        Composition { git, manifests: RefCell::new(HashMap::new()) }
    }

    pub fn manifest_at(&self, commit: &str) -> Result<Manifest> {
        let c = self.git.commit(commit)?;
        if let Some(m) = self.manifests.borrow().get(&c.tree) {
            return Ok(m.clone());
        }
        let m = match self.git.read_blob_at(&c.tree, MANIFEST_PATH)? {
            Some(text) => Manifest::parse(&text)?,
            None => Manifest::default(),
        };
        self.manifests.borrow_mut().insert(c.tree.clone(), m.clone());
        Ok(m)
    }

    /// Valid adoption claims made by `c`: the named commit must be one of
    /// `c`'s parents. Claims that don't hold (e.g. after a squash dropped the
    /// parent) are ignored — the trailer is evidence, the graph is truth.
    pub fn adoptions(&self, c: &Commit) -> Vec<(String, Oid)> {
        trailer::adoptions(&c.message)
            .into_iter()
            .filter(|(_, sha)| c.parents.iter().any(|p| p == sha))
            .collect()
    }

    /// Parents that belong to the composition's own history (not adopted
    /// component commits).
    pub fn own_parents(&self, c: &Commit) -> Vec<Oid> {
        let adopted: HashSet<Oid> = self.adoptions(c).into_iter().map(|(_, s)| s).collect();
        c.parents.iter().filter(|p| !adopted.contains(*p)).cloned().collect()
    }

    pub fn lens(&'g self, path: &str) -> Lens<'g> {
        Lens { comp: self, path: path.trim_matches('/').to_string(), memo: RefCell::new(HashMap::new()) }
    }
}

pub struct Lens<'g> {
    comp: &'g Composition<'g>,
    pub path: String,
    memo: RefCell<HashMap<Oid, Option<Oid>>>,
}

/// Identity of the logical change a commit represents, when one exists that
/// is stable across rewrites: jj's change-id header on composition commits,
/// or the Clonex-Change trailer on derived component commits. Plain-Git
/// commits have none: deriving one from the SHA would make every rebase
/// change every derived component commit (breaking locality).
pub fn change_id(c: &Commit) -> Option<String> {
    c.header("change-id")
        .map(str::to_string)
        .or_else(|| trailer::changes(&c.message).into_iter().next())
}

impl<'g> Lens<'g> {
    fn git(&self) -> &Git {
        self.comp.git
    }

    /// Component commit corresponding to composition commit `c`, or None if
    /// the occurrence doesn't exist there.
    pub fn get(&self, c: &str) -> Result<Option<Oid>> {
        // Iterative post-order over the composition's own history.
        let mut stack: Vec<(Oid, bool)> = vec![(c.to_string(), false)];
        while let Some((oid, expanded)) = stack.pop() {
            if self.memo.borrow().contains_key(&oid) {
                continue;
            }
            let commit = self.git().commit(&oid)?;
            let own = self.comp.own_parents(&commit);
            if !expanded {
                stack.push((oid.clone(), true));
                for p in own.iter().rev() {
                    if !self.memo.borrow().contains_key(p) {
                        stack.push((p.clone(), false));
                    }
                }
                continue;
            }
            let value = self.derive_one(&commit, &own)?;
            self.memo.borrow_mut().insert(oid, value);
        }
        Ok(self.memo.borrow().get(c).cloned().flatten())
    }

    fn derive_one(&self, c: &Commit, own_parents: &[Oid]) -> Result<Option<Oid>> {
        let Some(tree) = self.git().subtree(&c.tree, &self.path)? else {
            return Ok(None);
        };
        let adoptions = self.comp.adoptions(c);
        // Adoptions at this path, or at an ancestor directory (nested
        // compositions: the component commit's own lens answers for the
        // sub-path, so get_{A/Q}(c) = get_Q(adopted at A)).
        let mut adopted: Vec<Oid> = Vec::new();
        for (a_path, sha) in &adoptions {
            if *a_path == self.path {
                adopted.push(sha.clone());
            } else if let Some(rest) = self.path.strip_prefix(&format!("{a_path}/")) {
                if let Some(inner) = self.comp.lens(rest).get(sha)? {
                    adopted.push(inner);
                }
            }
        }
        // Round-trip law: an adoption with no remaining delta *is* the
        // adopted commit (same SHA, signature, statuses).
        if adopted.len() == 1 && self.git().commit(&adopted[0])?.tree == tree {
            return Ok(Some(adopted[0].clone()));
        }
        let mut cand: Vec<Oid> = Vec::new();
        for p in own_parents {
            if let Some(v) = self.memo.borrow().get(p).cloned().flatten() {
                cand.push(v);
            }
        }
        cand.extend(adopted);
        let cand = self.maximal(cand)?;
        if cand.len() == 1 && self.git().commit(&cand[0])?.tree == tree {
            return Ok(Some(cand[0].clone())); // no effect on this occurrence
        }
        // Adoptions *below* this path stay adoptions in the derived commit,
        // re-rooted: the adopted commit becomes a parent there too, so the
        // inner lens of the derived history finds it (L5).
        let mut parents = cand;
        let mut inner_claims = Vec::new();
        for (a_path, sha) in &adoptions {
            if let Some(rest) = a_path.strip_prefix(&format!("{}/", self.path)) {
                if !parents.contains(sha) {
                    parents.push(sha.clone());
                }
                inner_claims.push(format!("{}: {rest} {sha}", trailer::ADOPT));
            }
        }
        // Keep-merges survival rule (git-filter-repo's): a commit joining two
        // distinct component lines survives even if its tree equals one side.
        let mut message = trailer::strip_adoptions(&c.message);
        if !inner_claims.is_empty() {
            message = format!("{}\n\n{}\n", message.trim_end(), inner_claims.join("\n"));
        }
        let message = match c.header("change-id") {
            Some(id) => trailer::with_change(&message, id),
            None => message,
        };
        // committer := author, so rebases (which rewrite committer dates)
        // don't change derived commits.
        let oid = self.git().commit_tree(&tree, &parents, &c.author, &c.author, &message)?;
        Ok(Some(oid))
    }

    /// Reduce to commits not ancestors of another in the set, keeping order.
    fn maximal(&self, mut set: Vec<Oid>) -> Result<Vec<Oid>> {
        let mut seen = HashSet::new();
        set.retain(|s| seen.insert(s.clone()));
        let mut out = Vec::new();
        for (i, a) in set.iter().enumerate() {
            let mut dominated = false;
            for (j, b) in set.iter().enumerate() {
                if i != j && self.git().is_ancestor(a, b)? {
                    dominated = true;
                    break;
                }
            }
            if !dominated {
                out.push(a.clone());
            }
        }
        Ok(out)
    }
}

/// One occurrence's part of an adoption.
pub struct Adoption {
    pub path: String,
    pub commit: Oid,
}

/// `put`: adopt component commits into composition commit `head`.
/// Returns the tree and the parent list for the adoption commit; the caller
/// writes the commit with a human identity.
pub fn plan_adoption(
    comp: &Composition,
    head: &str,
    adoptions: &[Adoption],
) -> Result<Option<(Oid, Vec<Oid>, Vec<(String, Oid)>)>> {
    let git = comp.git;
    let head_commit = git.commit(head)?;
    let mut tree = head_commit.tree.clone();
    let mut parents = vec![head.to_string()];
    let mut claims = Vec::new();
    for a in adoptions {
        let lens = comp.lens(&a.path);
        let theirs = git.commit(&a.commit)?;
        let merged = match lens.get(head)? {
            None => theirs.tree.clone(),
            Some(ours) if git.is_ancestor(&a.commit, &ours)? => continue, // already contained
            Some(ours) if git.is_ancestor(&ours, &a.commit)? => theirs.tree.clone(), // fast-forward
            Some(ours) => match git.merge_commits(&ours, &a.commit)? {
                MergeOutcome::Clean(t) => t,
                MergeOutcome::Conflict { paths } => bail!(
                    "adopting {} into {} conflicts with changes the composition holds:\n  {}",
                    &a.commit[..12],
                    a.path,
                    paths.join("\n  ")
                ),
            },
        };
        tree = git.replace_subtree(&tree, &a.path, Some(&merged))?;
        if !parents.contains(&a.commit) {
            parents.push(a.commit.clone());
        }
        claims.push((a.path.clone(), a.commit.clone()));
    }
    if claims.is_empty() {
        return Ok(None);
    }
    Ok(Some((tree, parents, claims)))
}
