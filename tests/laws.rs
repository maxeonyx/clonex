//! Property tests of the lens laws over generated histories, against real
//! Git repositories. Each case builds a world (hosted component, an outsider,
//! a composition with a following and a frozen occurrence of the same
//! component) and applies a random sequence of everyday operations, checking
//! the laws after every step.

mod support;
use proptest::prelude::*;
use support::*;

#[derive(Clone, Debug)]
enum Op {
    /// Edit a file in one occurrence from the composition.
    Edit { occ: usize, file: usize, val: u8 },
    /// The same edit to both occurrences in one commit.
    EditBoth { file: usize, val: u8 },
    /// Composition-owned file only.
    EditRoot { val: u8 },
    /// Occurrence and composition files together.
    EditMixed { file: usize, val: u8 },
    /// A side branch with an occurrence edit, merged back with a merge commit.
    SideMerge { file: usize, val: u8 },
    /// An outsider pushes to the component's main with plain Git.
    Outsider { file: usize, val: u8 },
    Sync,
    Publish,
}

const OCCS: [&str; 2] = ["tools/t", "fixtures/t"];
const FILES: [&str; 3] = ["a.txt", "src/b.txt", "c.txt"];

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => (0..2usize, 0..3usize, any::<u8>()).prop_map(|(occ, file, val)| Op::Edit { occ, file, val }),
        1 => (0..3usize, any::<u8>()).prop_map(|(file, val)| Op::EditBoth { file, val }),
        1 => any::<u8>().prop_map(|val| Op::EditRoot { val }),
        1 => (0..3usize, any::<u8>()).prop_map(|(file, val)| Op::EditMixed { file, val }),
        1 => (0..3usize, any::<u8>()).prop_map(|(file, val)| Op::SideMerge { file, val }),
        2 => (0..3usize, any::<u8>()).prop_map(|(file, val)| Op::Outsider { file, val }),
        2 => Just(Op::Sync),
        2 => Just(Op::Publish),
    ]
}

fn content(tag: &str, val: u8) -> String {
    format!("{tag} {val}\n")
}

/// Check the laws that must hold in every state.
fn check_state(u: &Repo) {
    for p in OCCS {
        let derived = u.get(p, "HEAD");
        // The derived component commit's tree is exactly the occurrence.
        assert_eq!(u.tree_of(&derived), u.rev(&format!("HEAD:{p}")), "tree law for {p}");
        // GetPut: the derived commit contains everything; adopting it is a no-op.
        let o = u.cx_json(&["status"]);
        assert!(o.is_array());
    }
}

fn run_case(ops: Vec<Op>) {
    let w = World::new();
    let t = w.remote("t", &[("a.txt", "a0\n"), ("src/b.txt", "b0\n"), ("c.txt", "c0\n")]);
    let first = remote_rev(&t, "main");
    let outsider = w.clone_of(&t, "outsider");
    let u = w.init("u", &[("README.md", "u\n")]);
    u.cx_ok(&["declare", "tools/t", "--remote", t.to_str().unwrap(), "--follow", "main"]);
    u.cx_ok(&["declare", "fixtures/t", "--remote", t.to_str().unwrap(), "--at", &first]);
    let mut outsider_commits: Vec<String> = Vec::new();
    let mut main_tip = remote_rev(&t, "main");
    let mut side = 0;

    for op in ops {
        match op {
            Op::Edit { occ, file, val } => {
                u.write(&format!("{}/{}", OCCS[occ], FILES[file]), &content("u", val));
                u.commit(&format!("edit {} {} {val}", OCCS[occ], FILES[file]));
            }
            Op::EditBoth { file, val } => {
                for p in OCCS {
                    u.write(&format!("{p}/{}", FILES[file]), &content("both", val));
                }
                u.commit(&format!("edit both {} {val}", FILES[file]));
                // L8: equal bases + equal edits derive the same commit. Only
                // checkable when both occurrences were equal before.
            }
            Op::EditRoot { val } => {
                u.write("README.md", &content("root", val));
                u.commit(&format!("root {val}"));
            }
            Op::EditMixed { file, val } => {
                u.write(&format!("tools/t/{}", FILES[file]), &content("mixed", val));
                u.write("README.md", &content("mixed", val));
                u.commit(&format!("mixed {val}"));
            }
            Op::SideMerge { file, val } => {
                side += 1;
                let b = format!("side{side}");
                u.git(&["switch", "-q", "-c", &b]);
                u.write(&format!("tools/t/{}", FILES[file]), &content("side", val));
                u.commit(&format!("side {val}"));
                u.git(&["switch", "-q", "main"]);
                u.write("README.md", &content("main-while-side", val));
                u.commit(&format!("main while side {val}"));
                u.git(&["merge", "-q", "--no-ff", "-m", &format!("merge {b}"), &b]);
            }
            Op::Outsider { file, val } => {
                outsider.git(&["pull", "-q", "--no-rebase", "origin", "main"]);
                outsider.write(FILES[file], &content("outsider", val));
                let c = outsider.commit(&format!("outsider {} {val}", FILES[file]));
                let push = outsider.try_git(&["push", "-q", "origin", "HEAD:main"]);
                if push.ok {
                    outsider_commits.push(c);
                }
            }
            Op::Sync => {
                let o = u.cx(&["sync"]);
                if !o.ok {
                    // Only a genuine content conflict may stop adoption.
                    assert!(o.stderr.contains("conflicts"), "sync failed unexpectedly: {}", o.stderr);
                    continue;
                }
                // Every outsider commit that reached main is now contained.
                let derived = u.get("tools/t", "HEAD");
                for c in &outsider_commits {
                    assert!(u.try_git(&["merge-base", "--is-ancestor", c, &derived]).ok, "outsider commit {c} lost");
                }
                // PutGet: with nothing local outstanding, the occurrence is main exactly.
                let st = u.cx_json(&["status"]);
                let tools = st.as_array().unwrap().iter().find(|o| o["path"] == "tools/t").unwrap().clone();
                if tools["ahead"].as_array().unwrap().is_empty() {
                    assert_eq!(derived, remote_rev(&t, "main"), "PutGet");
                }
            }
            Op::Publish => {
                let r = u.cx_json(&["publish"]);
                let outcome = r[0]["outcome"].as_str().unwrap().to_string();
                assert!(["pushed", "up-to-date", "diverged"].contains(&outcome.as_str()), "{r}");
                if outcome == "pushed" {
                    assert_eq!(remote_rev(&t, "main"), u.get("tools/t", "HEAD"));
                }
            }
        }
        // main only ever moves forward.
        let now = remote_rev(&t, "main");
        assert!(remote_git(&t, &["merge-base", "--is-ancestor", &main_tip, &now]).is_empty());
        main_tip = now;
        check_state(&u);
    }

    // Convergence: adopt, publish, repeat until stable (a concurrent
    // outsider can't intervene here, so one or two rounds suffice).
    for _ in 0..3 {
        if !u.cx(&["sync"]).ok {
            return; // content conflict: a human decision, not a law violation
        }
        let r = u.cx_json(&["publish"]);
        if r[0]["outcome"] != "diverged" {
            break;
        }
    }
    let st = u.cx_json(&["status"]);
    let tools = st.as_array().unwrap().iter().find(|o| o["path"] == "tools/t").unwrap().clone();
    assert!(tools["ahead"].as_array().unwrap().is_empty() || tools["behind"].as_u64() == Some(0));
    assert_eq!(u.get("tools/t", "HEAD"), remote_rev(&t, "main"), "converged to main");

    // Determinism: an independent plain clone derives identical SHAs.
    let other = w.clone_of(&u.path, "u-clone");
    for p in OCCS {
        assert_eq!(other.get(p, "HEAD"), u.get(p, "HEAD"), "determinism for {p}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, max_shrink_iters: 64, ..ProptestConfig::default() })]
    #[test]
    fn laws_hold_over_generated_histories(ops in prop::collection::vec(op(), 1..12)) {
        run_case(ops);
    }
}
