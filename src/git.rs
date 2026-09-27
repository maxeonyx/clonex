//! Thin, deterministic access to one Git repository through Git's own
//! plumbing. Everything CloneX computes is an ordinary Git object; this
//! module is the only place that talks to `git`.

use anyhow::{bail, Context, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub type Oid = String;

pub const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// A parsed commit object. `author`/`committer` are the raw Git ident
/// lines (`Name <email> 1790550749 +1300`), kept verbatim so derived
/// commits reproduce them byte for byte.
#[derive(Clone, Debug)]
pub struct Commit {
    pub oid: Oid,
    pub tree: Oid,
    pub parents: Vec<Oid>,
    pub author: String,
    pub committer: String,
    /// Extra headers such as jj's `change-id`, in order.
    pub extra_headers: Vec<(String, String)>,
    pub message: String,
}

impl Commit {
    pub fn header(&self, key: &str) -> Option<&str> {
        self.extra_headers
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn subject(&self) -> &str {
        self.message.lines().next().unwrap_or("")
    }
}

#[derive(Clone, Debug)]
pub struct TreeEntry {
    pub mode: String,
    pub kind: String,
    pub oid: Oid,
    pub name: String,
}

pub struct Git {
    dir: PathBuf,
    commits: RefCell<HashMap<Oid, Commit>>,
    trees: RefCell<HashMap<Oid, Vec<TreeEntry>>>,
    ancestry: RefCell<HashMap<(Oid, Oid), bool>>,
    batch: RefCell<Option<Batch>>,
}

/// One long-lived `git cat-file --batch` for object reads.
struct Batch {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for Git {
    fn drop(&mut self) {
        if let Some(mut b) = self.batch.borrow_mut().take() {
            drop(b.stdin);
            let _ = b._child.wait();
        }
    }
}

pub enum MergeOutcome {
    Clean(Oid),
    Conflict { paths: Vec<String> },
}

impl Git {
    pub fn open(dir: impl AsRef<Path>) -> Result<Git> {
        let dir = dir.as_ref().to_path_buf();
        let git = Git {
            dir,
            commits: RefCell::new(HashMap::new()),
            trees: RefCell::new(HashMap::new()),
            ancestry: RefCell::new(HashMap::new()),
            batch: RefCell::new(None),
        };
        git.run(&["rev-parse", "--git-dir"])
            .context("not inside a Git repository")?;
        Ok(git)
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.dir).args(args);
        // Derived objects must not depend on the caller's configuration.
        cmd.env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
            .env("GIT_CONFIG_VALUE_0", "false");
        cmd
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        self.run_with(args, None, &[])
    }

    pub fn run_with(
        &self,
        args: &[&str],
        stdin: Option<&[u8]>,
        env: &[(&str, &str)],
    ) -> Result<String> {
        let (ok, stdout, stderr) = self.try_run(args, stdin, env)?;
        if !ok {
            bail!("git {} failed: {}", args.join(" "), stderr.trim());
        }
        Ok(stdout)
    }

    pub fn try_run(
        &self,
        args: &[&str],
        stdin: Option<&[u8]>,
        env: &[(&str, &str)],
    ) -> Result<(bool, String, String)> {
        let mut cmd = self.command(args);
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        let mut child = cmd.spawn().context("failed to run git")?;
        if let Some(input) = stdin {
            child
                .stdin
                .take()
                .expect("piped stdin")
                .write_all(input)?;
        }
        let out = child.wait_with_output()?;
        Ok((
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ))
    }

    /// Read a raw object through the persistent batch process.
    fn read_object(&self, oid: &str) -> Result<Option<(String, Vec<u8>)>> {
        let mut slot = self.batch.borrow_mut();
        if slot.is_none() {
            let mut child = self
                .command(&["cat-file", "--batch"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .context("failed to start git cat-file")?;
            let stdin = child.stdin.take().expect("piped");
            let stdout = BufReader::new(child.stdout.take().expect("piped"));
            *slot = Some(Batch { _child: child, stdin, stdout });
        }
        let b = slot.as_mut().expect("started");
        writeln!(b.stdin, "{oid}")?;
        b.stdin.flush()?;
        let mut header = String::new();
        b.stdout.read_line(&mut header)?;
        let header = header.trim_end();
        if header.ends_with(" missing") || header.ends_with(" ambiguous") {
            return Ok(None);
        }
        let mut parts = header.split(' ');
        let _oid = parts.next();
        let kind = parts.next().context("bad cat-file header")?.to_string();
        let size: usize = parts.next().context("bad cat-file header")?.parse()?;
        let mut data = vec![0u8; size + 1]; // + trailing newline
        b.stdout.read_exact(&mut data)?;
        data.pop();
        Ok(Some((kind, data)))
    }

    pub fn rev_parse(&self, rev: &str) -> Result<Oid> {
        let out = self
            .run(&["rev-parse", "--verify", "--quiet", &format!("{rev}^{{commit}}")])
            .with_context(|| format!("unknown revision {rev}"))?;
        Ok(out.trim().to_string())
    }

    pub fn try_rev_parse(&self, rev: &str) -> Option<Oid> {
        self.rev_parse(rev).ok()
    }

    pub fn commit(&self, oid: &str) -> Result<Commit> {
        if let Some(c) = self.commits.borrow().get(oid) {
            return Ok(c.clone());
        }
        let raw = match self.read_object(oid)? {
            Some((kind, data)) if kind == "commit" => String::from_utf8_lossy(&data).into_owned(),
            Some((kind, _)) => bail!("{oid} is a {kind}, not a commit"),
            None => bail!("commit {oid} is not in this repository"),
        };
        let c = parse_commit(oid, &raw)?;
        self.commits.borrow_mut().insert(oid.to_string(), c.clone());
        Ok(c)
    }

    pub fn ls_tree(&self, tree: &str) -> Result<Vec<TreeEntry>> {
        if let Some(t) = self.trees.borrow().get(tree) {
            return Ok(t.clone());
        }
        let data = match self.read_object(tree)? {
            Some((kind, data)) if kind == "tree" => data,
            _ => bail!("{tree} is not a tree"),
        };
        let mut entries = Vec::new();
        let mut rest = &data[..];
        while !rest.is_empty() {
            let sp = rest.iter().position(|b| *b == b' ').context("bad tree")?;
            let nul = rest.iter().position(|b| *b == 0).context("bad tree")?;
            let mode = String::from_utf8_lossy(&rest[..sp]).into_owned();
            let name = String::from_utf8_lossy(&rest[sp + 1..nul]).into_owned();
            let oid: String = rest[nul + 1..nul + 21].iter().map(|b| format!("{b:02x}")).collect();
            rest = &rest[nul + 21..];
            let (mode, kind) = match mode.as_str() {
                "40000" => ("040000".to_string(), "tree"),
                "160000" => (mode, "commit"),
                _ => (mode, "blob"),
            };
            entries.push(TreeEntry { mode, kind: kind.into(), oid, name });
        }
        self.trees.borrow_mut().insert(tree.to_string(), entries.clone());
        Ok(entries)
    }

    /// The entry at `path` inside `tree` (path uses `/`, empty = the tree).
    pub fn entry(&self, tree: &str, path: &str) -> Result<Option<TreeEntry>> {
        let mut current = TreeEntry {
            mode: "040000".into(),
            kind: "tree".into(),
            oid: tree.to_string(),
            name: String::new(),
        };
        for part in path.split('/').filter(|p| !p.is_empty()) {
            if current.kind != "tree" {
                return Ok(None);
            }
            match self.ls_tree(&current.oid)?.into_iter().find(|e| e.name == part) {
                Some(e) => current = e,
                None => return Ok(None),
            }
        }
        Ok(Some(current))
    }

    /// The subtree at `path`, if `path` is a directory.
    pub fn subtree(&self, tree: &str, path: &str) -> Result<Option<Oid>> {
        Ok(self
            .entry(tree, path)?
            .filter(|e| e.kind == "tree")
            .map(|e| e.oid))
    }

    pub fn read_blob_at(&self, tree: &str, path: &str) -> Result<Option<String>> {
        match self.entry(tree, path)? {
            Some(e) if e.kind == "blob" => Ok(self
                .read_object(&e.oid)?
                .map(|(_, data)| String::from_utf8_lossy(&data).into_owned())),
            _ => Ok(None),
        }
    }

    pub fn write_blob(&self, content: &[u8]) -> Result<Oid> {
        Ok(self
            .run_with(&["hash-object", "-w", "--stdin"], Some(content), &[])?
            .trim()
            .to_string())
    }

    fn mktree(&self, entries: &[TreeEntry]) -> Result<Oid> {
        let mut input = Vec::new();
        for e in entries {
            input.extend_from_slice(format!("{} {} {}\t{}\0", e.mode, e.kind, e.oid, e.name).as_bytes());
        }
        Ok(self
            .run_with(&["mktree", "-z"], Some(&input), &[])?
            .trim()
            .to_string())
    }

    /// `tree` with the entry at `path` replaced by `new` (None deletes it).
    /// Missing intermediate directories are created.
    pub fn replace_at(&self, tree: &str, path: &str, new: Option<(&str, &str, &str)>) -> Result<Oid> {
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return match new {
                Some((_, "tree", oid)) => Ok(oid.to_string()),
                _ => bail!("the root can only be replaced by a tree"),
            };
        }
        self.replace_parts(tree, &parts, new)
    }

    fn replace_parts(&self, tree: &str, parts: &[&str], new: Option<(&str, &str, &str)>) -> Result<Oid> {
        let mut entries = if tree == EMPTY_TREE { Vec::new() } else { self.ls_tree(tree)? };
        let name = parts[0];
        let existing = entries.iter().position(|e| e.name == name);
        let replacement = if parts.len() == 1 {
            new.map(|(mode, kind, oid)| TreeEntry {
                mode: mode.into(),
                kind: kind.into(),
                oid: oid.into(),
                name: name.into(),
            })
        } else {
            let child = match existing {
                Some(i) if entries[i].kind == "tree" => entries[i].oid.clone(),
                _ => EMPTY_TREE.to_string(),
            };
            let sub = self.replace_parts(&child, &parts[1..], new)?;
            (sub != EMPTY_TREE).then(|| TreeEntry {
                mode: "040000".into(),
                kind: "tree".into(),
                oid: sub,
                name: name.into(),
            })
        };
        match (existing, replacement) {
            (Some(i), Some(r)) => entries[i] = r,
            (Some(i), None) => {
                entries.remove(i);
            }
            (None, Some(r)) => entries.push(r),
            (None, None) => {}
        }
        if entries.is_empty() {
            return Ok(EMPTY_TREE.to_string());
        }
        self.mktree(&entries)
    }

    pub fn replace_subtree(&self, tree: &str, path: &str, subtree: Option<&str>) -> Result<Oid> {
        self.replace_at(tree, path, subtree.map(|oid| ("040000", "tree", oid)))
    }

    /// Write a commit with exactly the given identity lines. Two calls with
    /// equal arguments produce the same object id.
    pub fn commit_tree(
        &self,
        tree: &str,
        parents: &[Oid],
        author: &str,
        committer: &str,
        message: &str,
    ) -> Result<Oid> {
        let (an, ae, ad) = split_ident(author)?;
        let (cn, ce, cd) = split_ident(committer)?;
        let mut args: Vec<&str> = vec!["commit-tree", "--no-gpg-sign", tree];
        for p in parents {
            args.push("-p");
            args.push(p);
        }
        let env = [
            ("GIT_AUTHOR_NAME", an.as_str()),
            ("GIT_AUTHOR_EMAIL", ae.as_str()),
            ("GIT_AUTHOR_DATE", ad.as_str()),
            ("GIT_COMMITTER_NAME", cn.as_str()),
            ("GIT_COMMITTER_EMAIL", ce.as_str()),
            ("GIT_COMMITTER_DATE", cd.as_str()),
        ];
        Ok(self
            .run_with(&args, Some(message.as_bytes()), &env)?
            .trim()
            .to_string())
    }

    /// The caller's own identity, for commits a person makes (adoptions).
    pub fn user_ident(&self) -> Result<String> {
        let out = self.run(&["var", "GIT_COMMITTER_IDENT"])?;
        Ok(out.trim().to_string())
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        if ancestor == descendant {
            return Ok(true);
        }
        let key = (ancestor.to_string(), descendant.to_string());
        if let Some(v) = self.ancestry.borrow().get(&key) {
            return Ok(*v);
        }
        let (ok, _, _) = self.try_run(&["merge-base", "--is-ancestor", ancestor, descendant], None, &[])?;
        self.ancestry.borrow_mut().insert(key, ok);
        Ok(ok)
    }

    /// Git's own 3-way merge of two commits (merge base computed by Git,
    /// criss-cross handled by ort's virtual base).
    pub fn merge_commits(&self, ours: &str, theirs: &str) -> Result<MergeOutcome> {
        let (ok, out, err) = self.try_run(
            &["merge-tree", "--write-tree", "--name-only", "--no-messages", ours, theirs],
            None,
            &[],
        )?;
        let mut lines = out.lines();
        let tree = lines.next().unwrap_or_default().trim().to_string();
        if ok {
            return Ok(MergeOutcome::Clean(tree));
        }
        if tree.len() != 40 {
            bail!("git merge-tree failed: {}", err.trim());
        }
        Ok(MergeOutcome::Conflict {
            paths: lines.take_while(|l| !l.is_empty()).map(str::to_string).collect(),
        })
    }

    pub fn rev_list(&self, args: &[&str]) -> Result<Vec<Oid>> {
        let mut full = vec!["rev-list"];
        full.extend_from_slice(args);
        Ok(self.run(&full)?.lines().map(str::to_string).collect())
    }

    pub fn update_ref(&self, name: &str, oid: &str) -> Result<()> {
        self.run(&["update-ref", name, oid])?;
        Ok(())
    }

    pub fn current_branch(&self) -> Option<String> {
        self.run(&["symbolic-ref", "--quiet", "--short", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string())
    }
}

fn parse_commit(oid: &str, raw: &str) -> Result<Commit> {
    let (head, message) = raw.split_once("\n\n").unwrap_or((raw, ""));
    let mut c = Commit {
        oid: oid.to_string(),
        tree: String::new(),
        parents: Vec::new(),
        author: String::new(),
        committer: String::new(),
        extra_headers: Vec::new(),
        message: message.to_string(),
    };
    let mut last_key: Option<String> = None;
    for line in head.lines() {
        if let Some(cont) = line.strip_prefix(' ') {
            // Continuation line of a multi-line header (e.g. gpgsig).
            if let Some((_, v)) = c.extra_headers.last_mut() {
                v.push('\n');
                v.push_str(cont);
            }
            continue;
        }
        let (k, v) = line.split_once(' ').unwrap_or((line, ""));
        match k {
            "tree" => c.tree = v.to_string(),
            "parent" => c.parents.push(v.to_string()),
            "author" => c.author = v.to_string(),
            "committer" => c.committer = v.to_string(),
            _ => c.extra_headers.push((k.to_string(), v.to_string())),
        }
        last_key = Some(k.to_string());
    }
    let _ = last_key;
    if c.tree.is_empty() {
        bail!("commit {oid} has no tree");
    }
    Ok(c)
}

/// Split `Name <email> 1790550749 +1300` into (name, email, "1790550749 +1300").
pub fn split_ident(ident: &str) -> Result<(String, String, String)> {
    let lt = ident.find('<').context("bad ident")?;
    let gt = ident.rfind('>').context("bad ident")?;
    Ok((
        ident[..lt].trim().to_string(),
        ident[lt + 1..gt].to_string(),
        ident[gt + 1..].trim().to_string(),
    ))
}
