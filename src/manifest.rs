//! `.clonex.toml`: the composition's declarations. It holds intent only
//! (what lives where and where to exchange it), never revisions — which
//! component commit an occurrence contains is recorded by the commit graph
//! itself (see `lens`).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MANIFEST_PATH: &str = ".clonex.toml";

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// Keyed by path inside the composition.
    #[serde(default, rename = "occurrence")]
    pub occurrences: BTreeMap<String, Occurrence>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Occurrence {
    /// Where this history may be exchanged. Optional: an occurrence with no
    /// remote is a history born here that has not been published yet. Local
    /// mirrors/paths are Git's job (`url.<base>.insteadOf`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    /// The remote branch this occurrence follows on sync and publishes to by
    /// default. `None` = frozen: never adopted automatically (a pinned
    /// fixture, a vendored copy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow: Option<String>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest> {
        toml::from_str(text).context("invalid .clonex.toml")
    }

    pub fn render(&self) -> String {
        let mut out = String::from(
            "# CloneX composition manifest: which histories this repository contains.\n\
             # Revisions are not recorded here; they are part of the commit graph.\n\n",
        );
        for (path, occ) in &self.occurrences {
            out.push_str(&format!("[occurrence.\"{}\"]\n", path));
            if let Some(r) = &occ.remote {
                out.push_str(&format!("remote = {}\n", toml_str(r)));
            }
            if let Some(f) = &occ.follow {
                out.push_str(&format!("follow = {}\n", toml_str(f)));
            }
            out.push('\n');
        }
        out
    }

    /// Find by path, or by the last path component ("trunc" for tools/trunc).
    pub fn find(&self, key: &str) -> Option<(&String, &Occurrence)> {
        let key = key.trim_matches('/');
        self.occurrences
            .iter()
            .find(|(p, _)| p.as_str() == key)
            .or_else(|| self.occurrences.iter().find(|(p, _)| display_name(p) == key))
    }
}

/// Display label for an occurrence path: its last component.
pub fn display_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn toml_str(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// Trailers CloneX writes. They are parsed from anywhere in the message so a
/// squash that pushes them mid-body still yields them; every claim is
/// validated against the graph before use.
pub mod trailer {
    pub const ADOPT: &str = "Clonex-Adopt";
    pub const CHANGE: &str = "Clonex-Change";

    /// `Clonex-Adopt: <path> <sha>` lines.
    pub fn adoptions(message: &str) -> Vec<(String, String)> {
        message
            .lines()
            .filter_map(|l| l.trim().strip_prefix(ADOPT)?.strip_prefix(':'))
            .filter_map(|rest| {
                let mut it = rest.split_whitespace();
                let path = it.next()?.trim_end_matches('/').to_string();
                let sha = it.next()?.to_string();
                (sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())).then_some((path, sha))
            })
            .collect()
    }

    pub fn changes(message: &str) -> Vec<String> {
        message
            .lines()
            .filter_map(|l| l.trim().strip_prefix(CHANGE)?.strip_prefix(':'))
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect()
    }

    /// Remove composition-only trailers, so a derived component commit does
    /// not claim adoptions that only make sense in the composition.
    pub fn strip_adoptions(message: &str) -> String {
        let kept: Vec<&str> = message
            .lines()
            .filter(|l| !l.trim().starts_with(&format!("{ADOPT}:")))
            .collect();
        let mut s = kept.join("\n");
        while s.ends_with("\n\n") {
            s.pop();
        }
        s
    }

    /// Ensure the message carries `Clonex-Change: id` in its trailer block.
    pub fn with_change(message: &str, id: &str) -> String {
        if changes(message).iter().any(|c| c == id) {
            let mut m = message.to_string();
            if !m.ends_with('\n') {
                m.push('\n');
            }
            return m;
        }
        let body = message.trim_end();
        let last_para = body.rsplit("\n\n").next().unwrap_or("");
        let in_trailer_block = body.contains("\n\n")
            && last_para
                .lines()
                .all(|l| l.split_once(": ").map(|(k, _)| !k.contains(' ') && !k.is_empty()).unwrap_or(false));
        if in_trailer_block {
            format!("{body}\n{CHANGE}: {id}\n")
        } else {
            format!("{body}\n\n{CHANGE}: {id}\n")
        }
    }
}
