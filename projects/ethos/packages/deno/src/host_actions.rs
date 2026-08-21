//! A generic, host-agnostic way for a script to tell its host "here's a real message" without the
//! host having to interpret the script's return value to find out. Real bug this replaces: a
//! sentinel-string convention (`escher-anvil`'s own `QUIT_SENTINEL`/`CLEAR_SENTINEL`) where a
//! command script's *display text* doubled as a signal the host greps for after the fact — fragile
//! (a script that wants to show that exact text for an unrelated reason silently triggers the
//! action too), and backwards from how a script should ask a host to do something: call a real
//! function for it, the same way `fetch()` or `console.log` are real functions, not thread a
//! magic string back through the one channel meant for user-facing output.
//!
//! One generic op carrying a real structured message, not a specialized op per action. A script
//! calls `globalThis.__ethosHostAction(message)` with an actual JSON-serializable object (or a
//! host-provided `postMessage(message)` wrapper — a deliberately familiar public shape, matching
//! `Worker`/`BroadcastChannel` conventions script authors already know, even though the mechanism
//! underneath is this direct op, not real cross-realm message passing), never a bare type name
//! with everything else thrown away. Specialized ops (`op_anvil_quit()`, one per action) would
//! give up on this generality for a small type-safety win that doesn't matter here: actions are
//! host-interpreted strings either way, and a new action shouldn't need a new op plus a new JS
//! wrapper every time — just a new `type` value the host recognizes. Deliberately doesn't know
//! what any message *means* — interpreting `"quit"`/`"clear"` (or anything else) is entirely the
//! host's own business; this crate only carries the signal.
//!
//! Why `globalThis.__ethosHostAction` and not the raw op directly: `deno_runtime`'s own bootstrap
//! (`99_main.js`) deletes every op not on its own hardcoded CLI allowlist from `Deno.core.ops`
//! before a command script ever runs, and separately replaces the whole `Deno` namespace object
//! (so anything hung off the pre-bootstrap `Deno` is lost too). `host_actions_init.js` is a
//! classic (non-module) extension script that runs eagerly at extension setup, before either of
//! those things happen, and grabs a direct reference to the op function while it's still there.

use std::sync::Arc;
use std::sync::Mutex;

use deno_core::extension;
use deno_core::op2;
use deno_core::Extension;
use deno_core::OpState;

/// A real message a script posted to its host — `type` is the one field every message needs (what
/// the host matches on to decide what happened), `data` is whatever else the script wants to send
/// along, if anything. Mirrors the shape a `postMessage({ type, ...data })` convention already
/// implies, rather than reducing a message down to only its type and dropping the rest.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HostMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(flatten)]
    pub data: serde_json::Map<String, serde_json::Value>,
}

/// Shared with the host: every message a script has posted, oldest first. The host drains this
/// after a command finishes running — see `escher-anvil`'s own `run_js_command` caller for the
/// pattern.
pub type HostActions = Arc<Mutex<Vec<HostMessage>>>;

#[op2]
fn op_host_action(state: &mut OpState, #[serde] message: HostMessage) {
    if let Some(actions) = state.try_borrow_mut::<HostActions>() {
        actions.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(message);
    }
}

extension!(
    host_actions_ext,
    ops = [op_host_action],
    js = [ dir "src", "host_actions_init.js" ],
    options = { actions: HostActions },
    state = |state, options| {
        state.put(options.actions);
    },
);

/// Builds the extension a `run_module_command` caller passes in to let the script it runs post
/// real messages to its host. `actions` is the same handle the caller keeps and drains after the
/// run completes — this function only wires it into the worker, it doesn't own or interpret it.
pub fn host_action_extension(actions: HostActions) -> Extension {
    host_actions_ext::init(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::command::run_module_command;
    use deno_runtime::deno_io::Stdio;

    /// Drives the real `run_module_command`/`bootstrap_main_worker` path end to end — not a
    /// reimplementation — with a real (temp-file) script calling the real op, the same way
    /// `escher-anvil`'s `commands/quit.js`/`commands/clear.js` do. This is the actual mechanism
    /// under test, not a mock of it. Covers a message with extra data alongside `type`, not just
    /// the bare-type case, since carrying real data through is the whole point of this design.
    #[test]
    fn script_posting_messages_reaches_the_shared_actions_list_with_data_intact() {
        let dir = std::env::temp_dir().join(format!("ethos-deno-host-actions-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let script_path = dir.join("script.js");
        std::fs::write(
            &script_path,
            r#"
            function postMessage(message) {
                globalThis.__ethosHostAction(message);
            }
            export const run = async () => {
                postMessage({ type: "quit" });
                postMessage({ type: "openUrl", url: "https://example.com" });
                return "done";
            };
            "#,
        )
        .expect("write script");

        let actions: HostActions = Default::default();
        let result = run_module_command(&script_path, &dir, "", Stdio::default(), vec![host_action_extension(actions.clone())], "run");

        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(result, Ok("done".to_string()), "script should have run to completion and returned its display text");
        let recorded = actions.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(recorded.len(), 2, "both postMessage calls should have reached the shared actions list");
        assert_eq!(recorded[0].message_type, "quit");
        assert!(recorded[0].data.is_empty(), "a bare {{ type }} message shouldn't gain phantom data fields");
        assert_eq!(recorded[1].message_type, "openUrl");
        assert_eq!(recorded[1].data.get("url").and_then(serde_json::Value::as_str), Some("https://example.com"), "extra fields alongside type must survive, not just type itself");
    }
}
