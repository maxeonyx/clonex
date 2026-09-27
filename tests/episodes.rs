//! Episodes from design/episodes.md, replayed against real Git repositories
//! through the clonex binary. Each test names the episode it replays.

mod support;
use support::*;

/// U contains trunc at tools/trunc, following main of a hosted remote.
fn umbrella_with_trunc(w: &World) -> (Repo, std::path::PathBuf) {
    let t = w.remote("trunc", &[("src/lib.rs", "fn trunc() {}\n"), ("AGENTS.md", "trunc agents\n")]);
    let u = w.init("umbrella", &[("README.md", "umbrella\n")]);
    u.cx_ok(&["declare", "tools/trunc", "--remote", t.to_str().unwrap(), "--follow", "main"]);
    (u, t)
}

#[test]
fn a2_contained_history_is_the_real_component_history() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    // PutGet: the occurrence *is* trunc's main, same SHA.
    assert_eq!(u.get("tools/trunc", "HEAD"), remote_rev(&t, "main"));
    // The content is really there (no submodule init).
    assert_eq!(u.read("tools/trunc/src/lib.rs"), "fn trunc() {}\n");
    // And a plain clone of U contains trunc's commit objects.
    let plain = w.clone_of(&u.path, "plain-u");
    plain.git(&["cat-file", "-e", &remote_rev(&t, "main")]);
}

#[test]
fn b2_change_from_the_composition_becomes_an_ordinary_component_commit() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    u.write("tools/trunc/src/lib.rs", "fn trunc() { /* faster */ }\n");
    u.commit("Make trunc faster");
    let derived = u.get("tools/trunc", "HEAD");
    let r = u.cx_json(&["publish"]);
    assert_eq!(r[0]["outcome"], "pushed", "{r}");
    assert_eq!(remote_rev(&t, "main"), derived);
    let msg = remote_git(&t, &["log", "-1", "--format=%s|%an|%P", "main"]);
    let base = remote_git(&t, &["rev-parse", "main~1"]);
    assert_eq!(msg, format!("Make trunc faster|Tester|{base}"));
    // The component commit contains only the component's files.
    assert_eq!(remote_git(&t, &["ls-tree", "-r", "--name-only", "main"]), "AGENTS.md\nsrc/lib.rs");
    // Nothing left unpublished, nothing to adopt.
    let st = u.cx_json(&["status"]);
    assert_eq!(st[0]["ahead"].as_array().unwrap().len(), 0, "{st}");
    assert!(u.cx_ok(&["sync"]).contains("already contained"));
}

#[test]
fn b3_one_change_across_several_components_and_the_composition() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    let d = w.remote("dotsync", &[("src/main.rs", "fn main() {}\n"), ("AGENTS.md", "dotsync agents\n")]);
    u.cx_ok(&["declare", "tools/dotsync", "--remote", d.to_str().unwrap(), "--follow", "main"]);
    // "Alias AGENTS.md as CLAUDE.md" everywhere, plus the umbrella's own check.
    for p in ["tools/trunc", "tools/dotsync"] {
        u.write(&format!("{p}/CLAUDE.md"), "@AGENTS.md\n");
    }
    u.write("crates/standards/claude_alias.rs", "// check\n");
    let before = u.head();
    u.commit("Alias AGENTS.md as CLAUDE.md");
    let r = u.cx_json(&["publish"]);
    assert!(r.as_array().unwrap().iter().all(|x| x["outcome"] == "pushed"), "{r}");
    // One composition commit; one ordinary commit in each component.
    assert_eq!(u.git(&["rev-list", "--count", &format!("{before}..HEAD"), "--first-parent"]), "1");
    for remote in [&t, &d] {
        assert_eq!(remote_git(remote, &["log", "-1", "--format=%s", "main"]), "Alias AGENTS.md as CLAUDE.md");
        assert_eq!(remote_git(remote, &["show", "main:CLAUDE.md"]), "@AGENTS.md");
    }
    // No pointer bump exists anywhere: U's history already contains the content.
    assert_eq!(u.get("tools/trunc", "HEAD"), remote_rev(&t, "main"));
    assert_eq!(u.get("tools/dotsync", "HEAD"), remote_rev(&d, "main"));
}

#[test]
fn b1_outsider_commit_in_component_is_adopted_with_its_real_sha() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    let outsider = w.clone_of(&t, "outsider");
    outsider.write("src/lib.rs", "fn trunc() { /* outsider */ }\n");
    let theirs = outsider.commit("Outsider fix");
    outsider.git(&["push", "-q", "origin", "HEAD:main"]);
    let out = u.cx_ok(&["sync"]);
    assert!(out.contains("tools/trunc"), "{out}");
    assert_eq!(u.get("tools/trunc", "HEAD"), theirs, "the occurrence is exactly the outsider's commit");
    assert_eq!(u.read("tools/trunc/src/lib.rs"), "fn trunc() { /* outsider */ }\n");
    // The adoption is a merge whose extra parent is the component commit.
    assert_eq!(u.rev("HEAD^2"), theirs);
}

#[test]
fn f6_publication_after_someone_else_moved_the_component() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    // Composition-side change, unpublished.
    u.write("tools/trunc/AGENTS.md", "trunc agents v2\n");
    u.commit("Update trunc guidance");
    // Meanwhile an outsider pushes to trunc main.
    let outsider = w.clone_of(&t, "outsider");
    outsider.write("src/lib.rs", "fn trunc() { /* outsider */ }\n");
    let theirs = outsider.commit("Outsider fix");
    outsider.git(&["push", "-q", "origin", "HEAD:main"]);
    // Publishing refuses to overwrite and says what to do.
    let r = u.cx_json(&["publish"]);
    assert_eq!(r[0]["outcome"], "diverged", "{r}");
    // Adopt, then publish: trunc gets our change merged with theirs, nothing lost.
    u.cx_ok(&["sync"]);
    let r = u.cx_json(&["publish"]);
    assert_eq!(r[0]["outcome"], "pushed", "{r}");
    let main = remote_rev(&t, "main");
    assert_eq!(remote_git(&t, &["show", "main:AGENTS.md"]), "trunc agents v2");
    assert_eq!(remote_git(&t, &["show", "main:src/lib.rs"]), "fn trunc() { /* outsider */ }");
    assert!(remote_git(&t, &["rev-list", &main]).contains(&theirs));
    // Converged: the composition's occurrence is exactly trunc main.
    assert_eq!(u.get("tools/trunc", "HEAD"), main);
}

#[test]
fn k14_ci_bot_commits_on_a_pr_branch_are_adopted_under_further_work() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    u.git(&["switch", "-q", "-c", "ratchet-feature"]);
    // Red: the test first.
    u.write("tools/trunc/tests/new.rs", "#[test] fn new() { panic!() }\n");
    u.commit("Add failing test for new behaviour");
    assert_eq!(u.cx_json(&["publish"])[0]["outcome"], "pushed");
    // The trusted ledger bot records `pending` on the PR branch.
    let bot = w.clone_of(&t, "bot");
    bot.git(&["fetch", "-q", "origin", "ratchet-feature"]);
    bot.git(&["switch", "-q", "-c", "ratchet-feature", "FETCH_HEAD"]);
    bot.write(".test-status.json", "{\"new\": \"pending\"}\n");
    let bot_commit = bot.commit("chore: record tdd-ratchet status");
    bot.git(&["push", "-q", "origin", "HEAD:ratchet-feature"]);
    // Green work continues in the composition.
    u.write("tools/trunc/src/lib.rs", "fn trunc() {} fn new() {}\n");
    u.commit("Implement new behaviour");
    // Publishing sees the bot's commit and refuses to drop it…
    assert_eq!(u.cx_json(&["publish"])[0]["outcome"], "diverged");
    // …adopting the PR branch brings it in, and the green commit follows it.
    u.cx_ok(&["sync", "trunc", "--branch", "ratchet-feature"]);
    assert_eq!(u.cx_json(&["publish"])[0]["outcome"], "pushed");
    let log = remote_git(&t, &["log", "--format=%s", "ratchet-feature"]);
    let order: Vec<&str> = log.lines().collect();
    let pos = |s: &str| order.iter().position(|l| *l == s).unwrap_or_else(|| panic!("{s} missing in\n{log}"));
    assert!(pos("Add failing test for new behaviour") > pos("chore: record tdd-ratchet status"));
    assert!(remote_git(&t, &["rev-list", "ratchet-feature"]).contains(&bot_commit));
    assert_eq!(remote_git(&t, &["show", "ratchet-feature:.test-status.json"]), "{\"new\": \"pending\"}");
}

#[test]
fn d2_rewriting_an_unmerged_published_change_replaces_the_pr_branch() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    u.git(&["switch", "-q", "-c", "typo"]);
    u.write("tools/trunc/AGENTS.md", "trunc agnets\n");
    u.commit("Fix guidance");
    assert_eq!(u.cx_json(&["publish"])[0]["outcome"], "pushed");
    // Amend in the composition (review feedback).
    u.write("tools/trunc/AGENTS.md", "trunc agents, fixed\n");
    u.git(&["add", "-A"]);
    u.git(&["commit", "-q", "--amend", "--no-edit"]);
    let r = u.cx_json(&["publish"]);
    assert_eq!(r[0]["outcome"], "pushed");
    assert_eq!(r[0]["replaced"], true, "only our own commits were on the branch: {r}");
    assert_eq!(remote_git(&t, &["show", "typo:AGENTS.md"]), "trunc agents, fixed");
    assert_eq!(remote_git(&t, &["rev-list", "--count", "main..typo"]), "1");
}

#[test]
fn d6_reordering_composition_commits_leaves_component_commits_identical() {
    let w = World::new();
    let (u, _t) = umbrella_with_trunc(&w);
    let base = u.head();
    u.write("tools/trunc/src/lib.rs", "fn trunc() { 1 }\n");
    u.commit("Trunc change");
    u.write("README.md", "umbrella v2\n");
    u.commit("Umbrella-only change");
    let derived = u.get("tools/trunc", "HEAD");
    // Reorder with plain git (new committer dates, new SHAs).
    let trunc_change = u.rev("HEAD~1");
    u.git(&["reset", "-q", "--hard", &base]);
    u.write("README.md", "umbrella v2\n");
    u.commit("Umbrella-only change");
    u.git(&["cherry-pick", &trunc_change]);
    assert_ne!(u.rev("HEAD"), trunc_change);
    assert_eq!(u.get("tools/trunc", "HEAD"), derived, "locality: same component commit");
}

#[test]
fn c2_identical_edits_to_identical_occurrences_fuse_by_content() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    let tip = remote_rev(&t, "main");
    u.cx_ok(&["declare", "fixtures/trunc-copy", "--remote", t.to_str().unwrap(), "--at", &tip]);
    for p in ["tools/trunc", "fixtures/trunc-copy"] {
        u.write(&format!("{p}/src/lib.rs"), "fn trunc() { /* same */ }\n");
    }
    u.commit("Same fix in both copies");
    assert_eq!(u.get("tools/trunc", "HEAD"), u.get("fixtures/trunc-copy", "HEAD"));
}

#[test]
fn c3_one_change_to_two_revisions_splits_into_two_component_commits() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    let old = remote_rev(&t, "main");
    let outsider = w.clone_of(&t, "outsider");
    outsider.write("src/lib.rs", "fn trunc() {} // v2\n");
    outsider.commit("v2");
    outsider.git(&["push", "-q", "origin", "HEAD:main"]);
    u.cx_ok(&["sync"]);
    // A frozen fixture of the old revision.
    u.cx_ok(&["declare", "fixtures/trunc-old", "--remote", t.to_str().unwrap(), "--at", &old]);
    assert_eq!(u.get("fixtures/trunc-old", "HEAD"), old);
    // CVE fix applied to both revisions as one composition commit.
    for p in ["tools/trunc", "fixtures/trunc-old"] {
        let f = format!("{p}/SECURITY.md");
        u.write(&f, "fixed CVE-1\n");
    }
    u.commit("Fix CVE-1 everywhere");
    let latest = u.get("tools/trunc", "HEAD");
    let fixture = u.get("fixtures/trunc-old", "HEAD");
    assert_ne!(latest, fixture);
    assert_eq!(u.rev(&format!("{latest}^")), remote_rev(&t, "main"));
    assert_eq!(u.rev(&format!("{fixture}^")), old, "the fixture's fix sits on the old revision");
    // The frozen occurrence is not advanced by sync, and publish doesn't
    // push it anywhere it doesn't follow.
    let st = u.cx_json(&["status"]);
    let fx = st.as_array().unwrap().iter().find(|o| o["path"] == "fixtures/trunc-old").unwrap();
    assert!(fx["follow"].is_null());
}

#[test]
fn c12_upgrading_a_frozen_occurrence_is_a_fast_forward_not_a_copy() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    let old = remote_rev(&t, "main");
    u.cx_ok(&["declare", "fixtures/trunc-old", "--remote", t.to_str().unwrap(), "--at", &old]);
    let outsider = w.clone_of(&t, "outsider");
    outsider.write("src/lib.rs", "fn trunc() {} // v2\n");
    let v2 = outsider.commit("v2");
    outsider.git(&["push", "-q", "origin", "HEAD:main"]);
    u.cx_ok(&["sync", "fixtures/trunc-old", "--branch", "main"]);
    assert_eq!(u.get("fixtures/trunc-old", "HEAD"), v2, "same commit as trunc main, not a synthetic jump");
}

#[test]
fn a3_submodule_pointer_becomes_contained_history() {
    let w = World::new();
    let t = w.remote("trunc", &[("src/lib.rs", "fn trunc() {}\n")]);
    let u = w.init("umbrella", &[("README.md", "umbrella\n")]);
    u.git(&["-c", "protocol.file.allow=always", "submodule", "add", "-q", t.to_str().unwrap(), "tools/trunc"]);
    u.commit("Add trunc submodule");
    let pinned = remote_rev(&t, "main");
    u.cx_ok(&["declare", "tools/trunc", "--remote", t.to_str().unwrap(), "--follow", "main"]);
    assert_eq!(u.get("tools/trunc", "HEAD"), pinned);
    assert!(u.try_git(&["show", "HEAD:.gitmodules"]).ok == false, ".gitmodules entry removed");
    let ls = u.git(&["ls-tree", "HEAD", "tools/trunc"]);
    assert!(ls.starts_with("040000 tree"), "gitlink replaced by content: {ls}");
}

#[test]
fn a5_extracting_a_directory_derives_its_history_from_the_composition() {
    let w = World::new();
    let u = w.init("umbrella", &[("README.md", "umbrella\n")]);
    u.write("crates/standards/lib.rs", "v1\n");
    u.commit("Standards v1");
    u.write("README.md", "umbrella v2\n");
    u.commit("Umbrella only");
    u.write("crates/standards/lib.rs", "v2\n");
    u.write("README.md", "umbrella v3\n");
    u.commit("Standards v2 and README");
    let crosscut = w.path("crosscut.git");
    u.git(&["init", "-q", "--bare", crosscut.to_str().unwrap()]);
    u.cx_ok(&["declare", "crates/standards", "--extract", "--remote", crosscut.to_str().unwrap(), "--follow", "main"]);
    let head = u.get("crates/standards", "HEAD");
    let log = u.git(&["log", "--format=%s", &head]);
    assert_eq!(log, "Standards v2 and README\nStandards v1");
    assert_eq!(u.git(&["ls-tree", "--name-only", &head]), "lib.rs");
    assert_eq!(u.cx_json(&["publish", "--branch", "main"])[0]["outcome"], "pushed");
    assert_eq!(remote_rev(&crosscut, "main"), head);
}

#[test]
fn l5_nested_compositions_compose_their_lenses() {
    let w = World::new();
    let (u, t) = umbrella_with_trunc(&w);
    let u_remote = w.path("umbrella.git");
    u.git(&["init", "-q", "--bare", u_remote.to_str().unwrap()]);
    u.git(&["push", "-q", u_remote.to_str().unwrap(), "HEAD:main"]);
    let m = w.init("mega", &[("site/index.html", "<h1>site</h1>\n")]);
    m.cx_ok(&["declare", "at", "--remote", u_remote.to_str().unwrap(), "--follow", "main"]);
    m.write("at/tools/trunc/src/lib.rs", "fn trunc() { /* from mega */ }\n");
    m.write("site/index.html", "<h1>site v2</h1>\n");
    m.commit("Change trunc and the site together");
    let u_derived = m.get("at", "HEAD");
    // get_{tools/trunc}(get_{at}(M)) == get_{at/tools/trunc}(M)
    let direct = m.get("at/tools/trunc", "HEAD");
    // Evaluate the inner lens inside U's object store after fetching M's derived U commit.
    u.git(&["fetch", "-q", m.path.to_str().unwrap(), "HEAD"]);
    u.git(&["fetch", "-q", m.path.to_str().unwrap(), &u_derived]);
    let via_u = u.get("tools/trunc", &u_derived);
    assert_eq!(direct, via_u);
    assert_eq!(u.rev(&format!("{direct}^")), remote_rev(&t, "main"));
}
