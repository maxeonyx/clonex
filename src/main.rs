//! `clonex`: repositories that contain other repositories' real histories.
//!
//! Every command is a thin shell over `ops`; `--json` gives agents the
//! complete structured result.

use anyhow::Result;
use clap::{Parser, Subcommand};
use clonex::git::Git;
use clonex::lens::Composition;
use clonex::manifest::display_name;
use clonex::ops::{self, PublishOutcome};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "clonex", version, about = "Repositories that contain other repositories' real histories.")]
struct Cli {
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,
    /// Repository to operate on.
    #[arg(short = 'C', long, global = true, default_value = ".")]
    repo: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Make a history part of this repository at PATH: fetch it and adopt it,
    /// or convert a submodule there, or (--extract) turn an existing
    /// directory's history into its own history.
    Declare {
        path: String,
        /// Where this history is exchanged (any Git URL).
        #[arg(long)]
        remote: Option<String>,
        /// Branch to follow on sync and publish to by default; omit to freeze.
        #[arg(long)]
        follow: Option<String>,
        /// Derive the history from this repository's own history of PATH.
        #[arg(long)]
        extract: bool,
        /// Adopt this commit instead of the followed tip.
        #[arg(long)]
        at: Option<String>,
    },
    /// Show every occurrence: which component commit the current state is,
    /// what the composition has that the component doesn't, and vice versa.
    Status {
        #[arg(default_value = "HEAD")]
        rev: String,
    },
    /// Fetch followed branches and adopt them (one merge commit).
    Sync {
        /// Occurrences (path or last component); default all that follow.
        only: Vec<String>,
        /// Adopt this branch instead (e.g. a PR branch a CI bot wrote to).
        #[arg(long)]
        branch: Option<String>,
    },
    /// Push each occurrence's derived history to its remote branch.
    Publish {
        only: Vec<String>,
        /// Component branch to publish to; default: this repository's branch.
        #[arg(long)]
        branch: Option<String>,
        /// Show what would be pushed.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the component commit an occurrence has at REV (plumbing).
    Get {
        path: String,
        #[arg(default_value = "HEAD")]
        rev: String,
    },
    /// Show an occurrence's history as the component sees it.
    Log {
        path: String,
        #[arg(default_value = "HEAD")]
        rev: String,
        #[arg(short = 'n', default_value_t = 20)]
        max: usize,
    },
    /// Where does a logical change (jj change-id prefix) appear?
    Where {
        change: String,
        #[arg(default_value = "HEAD")]
        rev: String,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("clonex: {e:#}");
        std::process::exit(1);
    }
}

fn emit<T: Serialize>(json: bool, value: &T, text: impl FnOnce(&T) -> String) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        print!("{}", text(value));
    }
    Ok(())
}

fn short(oid: &str) -> &str {
    &oid[..oid.len().min(10)]
}

fn run(cli: Cli) -> Result<()> {
    let git = Git::open(&cli.repo)?;
    match cli.cmd {
        Cmd::Declare { path, remote, follow, extract, at } => {
            let commit = ops::declare(&git, ops::Declare { path: path.clone(), remote, follow, extract, at })?;
            emit(cli.json, &serde_json::json!({ "path": path, "commit": commit }), |_| {
                format!("{path} is now contained here ({})\n", short(&commit))
            })
        }
        Cmd::Status { rev } => {
            let st = ops::status(&git, &rev)?;
            emit(cli.json, &st, |st| {
                let mut out = String::new();
                for o in st {
                    let follow = o.follow.as_deref().map(|f| format!("follows {f}")).unwrap_or("frozen".into());
                    let at = o.derived.as_deref().map(short).unwrap_or("-");
                    out.push_str(&format!("{} ({}) at {at}, {follow}\n", o.path, display_name(&o.path)));
                    if o.ahead.is_empty() && o.behind == 0 {
                        if o.remote_tip.is_some() {
                            out.push_str("  in step with the followed branch\n");
                        }
                    }
                    if !o.ahead.is_empty() {
                        out.push_str(&format!("  {} change(s) not on the followed branch:\n", o.ahead.len()));
                        for c in &o.ahead {
                            out.push_str(&format!("    {} {}\n", short(&c.commit), c.subject));
                        }
                    }
                    if o.behind > 0 {
                        out.push_str(&format!("  {} commit(s) on the followed branch not adopted (clonex sync)\n", o.behind));
                    }
                }
                out
            })
        }
        Cmd::Sync { only, branch } => {
            let r = ops::sync(&git, ops::SyncRequest { only, branch })?;
            emit(cli.json, &r, |r| match &r.commit {
                None => "everything followed is already contained\n".into(),
                Some(c) => {
                    let mut s = format!("adopted in {}:\n", short(c));
                    for (p, sha) in &r.adopted {
                        s.push_str(&format!("  {p} <- {}\n", short(sha)));
                    }
                    s
                }
            })
        }
        Cmd::Publish { only, branch, dry_run } => {
            let rs = ops::publish(&git, ops::PublishRequest { only, branch, dry_run })?;
            let failed = rs.iter().any(|r| matches!(r.outcome, PublishOutcome::Diverged { .. } | PublishOutcome::Rejected { .. }));
            emit(cli.json, &rs, |rs| {
                let mut s = String::new();
                for r in rs {
                    let line = match &r.outcome {
                        PublishOutcome::UpToDate => "up to date".to_string(),
                        PublishOutcome::NoRemote => "no remote declared".to_string(),
                        PublishOutcome::Pushed { to, commits, replaced, .. } => format!(
                            "{} {commits} commit(s), now {}",
                            if *replaced { "replaced with" } else { "pushed" },
                            short(to)
                        ),
                        PublishOutcome::WouldPush { to, commits, replaced, .. } => format!(
                            "would {} {commits} commit(s), ending at {}",
                            if *replaced { "replace with" } else { "push" },
                            short(to)
                        ),
                        PublishOutcome::Diverged { remote, .. } => format!(
                            "remote has commits not adopted here (tip {}); run `clonex sync {} --branch {}` first",
                            short(remote),
                            display_name(&r.path),
                            r.branch
                        ),
                        PublishOutcome::Rejected { reason } => format!("rejected: {reason}"),
                    };
                    s.push_str(&format!("{} -> {}: {line}\n", r.path, r.branch));
                }
                s
            })?;
            if failed {
                std::process::exit(2);
            }
            Ok(())
        }
        Cmd::Get { path, rev } => {
            let head = git.rev_parse(&rev)?;
            let comp = Composition::new(&git);
            let d = comp.lens(&path).get(&head)?;
            emit(cli.json, &d, |d| format!("{}\n", d.as_deref().unwrap_or("")))
        }
        Cmd::Log { path, rev, max } => {
            let head = git.rev_parse(&rev)?;
            let comp = Composition::new(&git);
            let Some(d) = comp.lens(&path).get(&head)? else {
                anyhow::bail!("{path} does not exist at {rev}");
            };
            let out = git.run(&["log", "--graph", "--format=%h %an %ad %s", "--date=short", "-n", &max.to_string(), &d])?;
            emit(cli.json, &d, |_| out)
        }
        Cmd::Where { change, rev } => {
            let r = ops::where_change(&git, &rev, &change)?;
            emit(cli.json, &r, |r| {
                r.iter().map(|a| format!("{:<24} {} {}\n", a.place, short(&a.commit), a.subject)).collect()
            })
        }
    }
}
