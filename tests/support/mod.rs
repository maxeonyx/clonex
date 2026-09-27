//! A small world of real Git repositories for black-box tests: bare
//! "hosted" remotes, plain-Git clones standing in for other people, and
//! composition repositories driven through the `clonex` binary.
#![allow(dead_code)]

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::process::Command;
use tempfile::TempDir;

pub struct World {
    pub dir: TempDir,
    clock: Rc<Cell<u64>>,
}

pub struct Repo {
    pub path: PathBuf,
    clock: Rc<Cell<u64>>,
}

pub struct Out {
    pub ok: bool,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl World {
    pub fn new() -> World {
        World { dir: tempfile::tempdir().unwrap(), clock: Rc::new(Cell::new(1_790_000_000)) }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// A hosted remote (bare repo) with an initial commit on `main`.
    pub fn remote(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let bare = self.path(&format!("{name}.git"));
        run_ok(self.dir.path(), "git", &["init", "-q", "--bare", "-b", "main", bare.to_str().unwrap()]);
        let seed = self.clone_of(&bare, &format!("{name}-seed"));
        seed.write_all(files);
        seed.commit(&format!("Start {name}"));
        seed.git(&["push", "-q", "origin", "HEAD:main"]);
        bare
    }

    pub fn init(&self, name: &str, files: &[(&str, &str)]) -> Repo {
        let path = self.path(name);
        run_ok(self.dir.path(), "git", &["init", "-q", "-b", "main", path.to_str().unwrap()]);
        let r = self.repo(path);
        r.configure();
        r.write_all(files);
        r.commit(&format!("Start {name}"));
        r
    }

    pub fn clone_of(&self, url: &Path, name: &str) -> Repo {
        let path = self.path(name);
        run_ok(self.dir.path(), "git", &["clone", "-q", url.to_str().unwrap(), path.to_str().unwrap()]);
        let r = self.repo(path);
        r.configure();
        r
    }

    fn repo(&self, path: PathBuf) -> Repo {
        Repo { path, clock: self.clock.clone() }
    }
}

impl Repo {
    fn tick(&self) -> String {
        // Deterministic, strictly increasing timestamps.
        let t = self.clock.get() + 60;
        self.clock.set(t);
        format!("{t} +1200")
    }

    fn configure(&self) {
        self.git(&["config", "user.name", "Tester"]);
        self.git(&["config", "user.email", "tester@example.com"]);
        self.git(&["config", "commit.gpgsign", "false"]);
    }

    pub fn git(&self, args: &[&str]) -> String {
        let date = self.tick();
        let out = Command::new("git")
            .current_dir(&self.path)
            .args(args)
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {} failed in {}:\n{}{}",
            args.join(" "),
            self.path.display(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    pub fn try_git(&self, args: &[&str]) -> Out {
        let out = Command::new("git").current_dir(&self.path).args(args).output().unwrap();
        to_out(out)
    }

    pub fn write(&self, path: &str, content: &str) {
        let p = self.path.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    pub fn write_all(&self, files: &[(&str, &str)]) {
        for (p, c) in files {
            self.write(p, c);
        }
    }

    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.path.join(path)).unwrap()
    }

    pub fn commit(&self, msg: &str) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "--allow-empty", "-m", msg]);
        self.head()
    }

    pub fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"])
    }

    pub fn rev(&self, r: &str) -> String {
        self.git(&["rev-parse", r])
    }

    pub fn tree_of(&self, rev: &str) -> String {
        self.git(&["rev-parse", &format!("{rev}^{{tree}}")])
    }

    /// Run the clonex binary in this repository.
    pub fn cx(&self, args: &[&str]) -> Out {
        let date = self.tick();
        let out = Command::new(env!("CARGO_BIN_EXE_clonex"))
            .current_dir(&self.path)
            .args(args)
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        to_out(out)
    }

    pub fn cx_ok(&self, args: &[&str]) -> String {
        let o = self.cx(args);
        assert!(o.ok, "clonex {} failed ({}):\n{}{}", args.join(" "), o.code, o.stdout, o.stderr);
        o.stdout
    }

    pub fn cx_json(&self, args: &[&str]) -> serde_json::Value {
        let mut full = vec!["--json"];
        full.extend_from_slice(args);
        let o = self.cx(&full);
        assert!(o.code == 0 || o.code == 2, "clonex {} failed:\n{}{}", args.join(" "), o.stdout, o.stderr);
        serde_json::from_str(&o.stdout).unwrap()
    }

    /// `clonex get`: the component commit an occurrence is at.
    pub fn get(&self, path: &str, rev: &str) -> String {
        self.cx_ok(&["get", path, rev]).trim().to_string()
    }
}

pub fn remote_rev(remote: &Path, rev: &str) -> String {
    run_ok(remote, "git", &["rev-parse", rev]).trim().to_string()
}

pub fn remote_git(remote: &Path, args: &[&str]) -> String {
    run_ok(remote, "git", args).trim().to_string()
}

fn run_ok(dir: &Path, prog: &str, args: &[&str]) -> String {
    let out = Command::new(prog).current_dir(dir).args(args).output().unwrap();
    assert!(out.status.success(), "{prog} {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn to_out(out: std::process::Output) -> Out {
    Out {
        ok: out.status.success(),
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}
