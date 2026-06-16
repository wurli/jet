//! Parsing kallichore websocket frames into typed events.
//!
//! Frames come off the websocket as JSON; [`parse_event`] turns them into
//! a typed [`Event`] suitable for any consumer (the CLI renderer, a Lua
//! binding, …). The pure-parsing path lives here so it has no I/O deps.

use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug)]
pub enum Event {
    Stream {
        name: String,
        text: String,
    },
    DisplayData {
        data: Value,
    },
    Error {
        traceback: String,
    },
    Banner {
        text: String,
    },
    Idle {
        parent_id: String,
    },
    /// Kernel asked for stdin via the `stdin` channel (e.g. R `readline()`
    /// or Python `input()`). The REPL prompts the user and replies with
    /// `input_reply` carrying the same parent_id.
    InputRequest {
        prompt: String,
        password: bool,
        parent_id: String,
    },
    /// kallichore reported the kernel has exited. The REPL uses this to
    /// shut down immediately rather than wait for the user to press a key.
    KernelExited,
    Other,
}

/// Forwarded by consumers of [`Event::InputRequest`] when they need to bounce
/// the request to a separate input-handling layer (the REPL prompts via
/// rustyline; the Lua binding surfaces it through its poller).
pub struct InputRequest {
    pub prompt: String,
    pub password: bool,
    pub parent_id: String,
}

/// kallichore wraps every websocket frame in a `{kind, ...}` envelope.
/// Jupyter messages are tagged `kind: "jupyter"` with the standard
/// `channel`/`header`/`parent_header`/`content` fields. Server-side
/// kernel-lifecycle events are tagged `kind: "kernel"` with shapes like
/// `{status: {...}}`, `{exited: <code>}`, `{output: [...]}`,
/// `{resourceUsage: {...}}`. We only act on `exited` and on
/// `status.status == "exited"`; the other kernel-kind frames are noise
/// for a CLI REPL.
#[derive(Deserialize)]
struct IncomingMessage {
    #[serde(default)]
    kind: String,
    // jupyter-kind fields
    #[serde(default)]
    channel: String,
    #[serde(default)]
    header: Header,
    #[serde(default)]
    parent_header: Option<ParentHeader>,
    #[serde(default)]
    content: Value,
    // kernel-kind fields
    #[serde(default)]
    exited: Option<i64>,
    #[serde(default)]
    status: Option<KernelStatus>,
}

#[derive(Deserialize, Default)]
struct Header {
    #[serde(default)]
    msg_type: String,
}

#[derive(Deserialize, Default)]
struct ParentHeader {
    #[serde(default)]
    msg_id: String,
}

#[derive(Deserialize)]
struct KernelStatus {
    #[serde(default)]
    status: String,
}

pub fn parse_event(text: &str) -> Result<Event> {
    let m: IncomingMessage = serde_json::from_str(text)?;
    if m.kind == "kernel" {
        if m.exited.is_some() {
            return Ok(Event::KernelExited);
        }
        if m.status
            .as_ref()
            .map(|s| s.status == "exited")
            .unwrap_or(false)
        {
            return Ok(Event::KernelExited);
        }
        return Ok(Event::Other);
    }
    let event = match (m.channel.as_str(), m.header.msg_type.as_str()) {
        ("iopub", "stream") => {
            let name = m
                .content
                .get("name")
                .and_then(|s| s.as_str())
                .unwrap_or("stdout")
                .to_string();
            let text = m
                .content
                .get("text")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            Event::Stream { name, text }
        }
        ("iopub", "execute_result") | ("iopub", "display_data") => {
            let data = m.content.get("data").cloned().unwrap_or(Value::Null);
            Event::DisplayData { data }
        }
        ("iopub", "error") => {
            // Python kernels put the colorized backtrace in `traceback`;
            // ark/R leaves traceback empty and puts the message in `evalue`.
            // Fall back to `ename: evalue` when traceback is empty/missing.
            let traceback = m
                .content
                .get("traceback")
                .and_then(|t| t.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    let ename = m
                        .content
                        .get("ename")
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    let evalue = m
                        .content
                        .get("evalue")
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    match (ename.is_empty(), evalue.is_empty()) {
                        (false, false) => format!("{ename}: {evalue}"),
                        (true, false) => evalue.to_string(),
                        (false, true) => ename.to_string(),
                        (true, true) => String::new(),
                    }
                });
            Event::Error { traceback }
        }
        ("shell", "kernel_info_reply") => {
            let text = m
                .content
                .get("banner")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            Event::Banner { text }
        }
        ("stdin", "input_request") => {
            let prompt = m
                .content
                .get("prompt")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let password = m
                .content
                .get("password")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            let parent_id = m.parent_header.map(|p| p.msg_id).unwrap_or_default();
            Event::InputRequest {
                prompt,
                password,
                parent_id,
            }
        }
        ("iopub", "status") => {
            let state = m
                .content
                .get("execution_state")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            let parent_id = m.parent_header.map(|p| p.msg_id).unwrap_or_default();
            if state == "idle" && !parent_id.is_empty() {
                Event::Idle { parent_id }
            } else {
                Event::Other
            }
        }
        _ => Event::Other,
    };
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frame(channel: &str, msg_type: &str, parent_id: &str, content: Value) -> String {
        json!({
            "kind": "jupyter",
            "channel": channel,
            "header": {"msg_type": msg_type},
            "parent_header": {"msg_id": parent_id},
            "content": content,
        })
        .to_string()
    }

    #[test]
    fn parse_stream_event() {
        let f = frame(
            "iopub",
            "stream",
            "",
            json!({"name": "stdout", "text": "hi"}),
        );
        match parse_event(&f).unwrap() {
            Event::Stream { name, text } => {
                assert_eq!(name, "stdout");
                assert_eq!(text, "hi");
            }
            other => panic!("expected Stream, got {other:?}"),
        }
    }

    #[test]
    fn parse_display_data_event() {
        let f = frame(
            "iopub",
            "display_data",
            "",
            json!({"data": {"text/plain": "x"}}),
        );
        match parse_event(&f).unwrap() {
            Event::DisplayData { data } => {
                assert_eq!(data["text/plain"], "x");
            }
            other => panic!("expected DisplayData, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_event() {
        let f = frame(
            "iopub",
            "error",
            "",
            json!({"traceback": ["line1", "line2"]}),
        );
        match parse_event(&f).unwrap() {
            Event::Error { traceback } => assert_eq!(traceback, "line1\nline2"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_falls_back_to_evalue_when_traceback_empty() {
        // ark/R sends `traceback: []` and puts the message in `evalue`.
        let f = frame(
            "iopub",
            "error",
            "",
            json!({"ename": "", "evalue": "Error:\n! boom", "traceback": []}),
        );
        match parse_event(&f).unwrap() {
            Event::Error { traceback } => assert_eq!(traceback, "Error:\n! boom"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_with_ename_and_evalue() {
        let f = frame(
            "iopub",
            "error",
            "",
            json!({"ename": "RuntimeError", "evalue": "boom", "traceback": []}),
        );
        match parse_event(&f).unwrap() {
            Event::Error { traceback } => assert_eq!(traceback, "RuntimeError: boom"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn parse_banner_event() {
        let f = frame("shell", "kernel_info_reply", "", json!({"banner": "hello"}));
        match parse_event(&f).unwrap() {
            Event::Banner { text } => assert_eq!(text, "hello"),
            other => panic!("expected Banner, got {other:?}"),
        }
    }

    #[test]
    fn parse_idle_event() {
        let f = frame("iopub", "status", "abc", json!({"execution_state": "idle"}));
        match parse_event(&f).unwrap() {
            Event::Idle { parent_id } => assert_eq!(parent_id, "abc"),
            other => panic!("expected Idle, got {other:?}"),
        }
    }

    #[test]
    fn parse_busy_status_is_other() {
        let f = frame("iopub", "status", "abc", json!({"execution_state": "busy"}));
        assert!(matches!(parse_event(&f).unwrap(), Event::Other));
    }

    #[test]
    fn parse_idle_without_parent_is_other() {
        let f = frame("iopub", "status", "", json!({"execution_state": "idle"}));
        assert!(matches!(parse_event(&f).unwrap(), Event::Other));
    }

    #[test]
    fn parse_unknown_msg_type_is_other() {
        let f = frame("iopub", "comm_msg", "", json!({}));
        assert!(matches!(parse_event(&f).unwrap(), Event::Other));
    }

    #[test]
    fn parse_input_request_event() {
        let f = frame(
            "stdin",
            "input_request",
            "exec-1",
            json!({"prompt": "enter something: ", "password": false}),
        );
        match parse_event(&f).unwrap() {
            Event::InputRequest {
                prompt,
                password,
                parent_id,
            } => {
                assert_eq!(prompt, "enter something: ");
                assert!(!password);
                assert_eq!(parent_id, "exec-1");
            }
            other => panic!("expected InputRequest, got {other:?}"),
        }
    }

    #[test]
    fn parse_kernel_exited_frame() {
        let raw = json!({"kind": "kernel", "exited": 0}).to_string();
        assert!(matches!(parse_event(&raw).unwrap(), Event::KernelExited));
    }

    #[test]
    fn parse_kernel_status_exited_frame() {
        let raw = json!({
            "kind": "kernel",
            "status": {"status": "exited", "reason": "child process exited"},
        })
        .to_string();
        assert!(matches!(parse_event(&raw).unwrap(), Event::KernelExited));
    }

    #[test]
    fn parse_kernel_status_busy_is_other() {
        let raw = json!({
            "kind": "kernel",
            "status": {"status": "busy", "reason": "execute_request"},
        })
        .to_string();
        assert!(matches!(parse_event(&raw).unwrap(), Event::Other));
    }

    #[test]
    fn parse_kernel_resource_usage_is_other() {
        let raw = json!({
            "kind": "kernel",
            "resourceUsage": {"cpu_percent": 0, "memory_bytes": 1234},
        })
        .to_string();
        assert!(matches!(parse_event(&raw).unwrap(), Event::Other));
    }

    #[test]
    fn parse_handles_null_parent_header() {
        let raw = serde_json::json!({
            "channel": "iopub",
            "header": {"msg_type": "status"},
            "parent_header": null,
            "content": {"execution_state": "starting"},
        })
        .to_string();
        assert!(matches!(parse_event(&raw).unwrap(), Event::Other));
    }
}
