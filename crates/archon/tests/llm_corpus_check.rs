//! harmonia#682 — `_llm/current_state.toml` must stay a live pointer, not a
//! restated snapshot. Guards the two structural failure modes the file
//! actually shipped with: an unresolved `<...>` placeholder locator, and a
//! sentinel `issue = 0` standing in for a real tracker reference.

use std::path::Path;

fn current_state_path() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../_llm/current_state.toml"
    ))
}

#[test]
fn current_state_has_no_unresolved_placeholder() {
    let raw = std::fs::read_to_string(current_state_path()).expect("read _llm/current_state.toml");
    assert!(
        !raw.contains('<') || !raw.contains('>'),
        "_llm/current_state.toml contains an unresolved '<...>' placeholder \
         (e.g. a locator like <canonical-state-doc>); every source_docs entry \
         must resolve to a real path"
    );
}

#[test]
fn current_state_has_no_sentinel_issue_id() {
    let raw = std::fs::read_to_string(current_state_path()).expect("read _llm/current_state.toml");
    let value: toml::Value =
        toml::from_str(&raw).expect("_llm/current_state.toml must be valid TOML");

    if let Some(threads) = value.get("open_threads").and_then(|v| v.as_array()) {
        for thread in threads {
            if let Some(issue) = thread.get("issue").and_then(toml::Value::as_integer) {
                assert_ne!(
                    issue, 0,
                    "_llm/current_state.toml carries a sentinel `issue = 0` open \
                     thread; a real thread needs a real issue number, and one \
                     that has none should not be restated here at all"
                );
            }
        }
    }
}

#[test]
fn current_state_does_not_restate_volatile_tracker_snapshots() {
    let raw = std::fs::read_to_string(current_state_path()).expect("read _llm/current_state.toml");
    let value: toml::Value =
        toml::from_str(&raw).expect("_llm/current_state.toml must be valid TOML");

    // WHY: `[[recent]]` (dated work log) and per-thread `[[open_threads]]`
    // entries are exactly the two shapes that went stale in harmonia#682 —
    // this file must point at git/tracker authorities instead of restating
    // their contents.
    assert!(
        value.get("recent").is_none(),
        "_llm/current_state.toml must not carry a dated [[recent]] work log; \
         point at `git log` / CHANGELOG.md instead"
    );

    let table_threads = value
        .get("open_threads")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|t| t.get("issue").is_some()))
        .unwrap_or(false);
    assert!(
        !table_threads,
        "_llm/current_state.toml must not restate per-issue open-thread \
         entries; point at `gh issue list --repo forkwright/harmonia` instead"
    );
}
