//! Translate Jupyter wire messages into typed events for the renderer / Lua
//! binding.
//!
//! The renderer (`crates/cli/src/render`) consumes [`Event`] without caring
//! about the underlying wire format, so the previous kallichore-WebSocket
//! variant of this module had the same enum. Now the source is
//! [`JupyterMessage`] from `jupyter-protocol`, fed in from each ZMQ channel.
//!
//! Per Jupyter spec, the channel matters: an `input_request` is only valid
//! on `stdin`, `kernel_info_reply` is on `shell`, and the rest live on
//! `iopub`. Callers thread the [`Channel`] in so we can replicate that
//! routing without depending on the optional `JupyterMessage::channel`
//! field (which is `None` for ZMQ transports).

use jupyter_protocol::{
    ExecutionState, IsCompleteReplyStatus, JupyterMessage, JupyterMessageContent, MediaType, Stdio,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct Event {
    pub parent_session: Option<String>,
    pub data: EventData,
}

#[derive(Debug)]
pub enum EventData {
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
        parent_id: Option<String>,
    },
    Busy {
        parent_id: Option<String>,
    },
    InputRequest {
        prompt: String,
        password: bool,
        parent_id: Option<String>,
    },
    ExecuteInput {
        code: String,
    },
    IsComplete {
        parent_id: Option<String>,
        status: IsCompleteReplyStatus,
        indent: String,
    },
    /// Shell `execute_reply` matching a request the caller sent. Fires alongside
    /// (and often after) the iopub `status: idle`, so consumers that need
    /// "execute is fully done" must gate on both — the Jupyter spec explicitly
    /// warns that the two arrive over different sockets and may reorder.
    ExecuteReply {
        parent_id: Option<String>,
    },
    /// The kernel has gone away. Emitted by the reader task when its socket
    /// returns an error or the child process exits — not from a wire frame.
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

/// Forwarded by consumers of [`EventData::IsComplete`] so the REPL can
/// match the reply against the request it sent and decide whether to
/// execute or keep accumulating input.
pub struct IsCompleteReplyMsg {
    pub parent_id: String,
    pub status: IsCompleteReplyStatus,
    pub indent: String,
}

/// Which ZMQ channel a message arrived on. We keep our own enum rather
/// than reuse `jupyter_protocol::Channel` so callers don't need to import
/// runtimed types just to feed events in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Shell,
    IoPub,
    Stdin,
    Control,
}

impl Channel {
    /// Lowercase wire name, matching how the Jupyter spec refers to each
    /// channel. Used for filter-by-name in API surfaces (e.g. Lua's
    /// `jet.listen({channel="iopub"})`).
    pub fn name(self) -> &'static str {
        match self {
            Channel::Shell => "shell",
            Channel::IoPub => "iopub",
            Channel::Stdin => "stdin",
            Channel::Control => "control",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "shell" => Some(Channel::Shell),
            "iopub" => Some(Channel::IoPub),
            "stdin" => Some(Channel::Stdin),
            "control" => Some(Channel::Control),
            _ => None,
        }
    }
}

/// Convert a single message into an [`Event`].
pub fn from_message(channel: Channel, msg: &JupyterMessage) -> Event {
    let parent_session = msg.parent_header.as_ref().map(|h| h.session.clone());
    let parent_id = msg.parent_header.as_ref().map(|h| h.msg_id.clone());

    let event_data = match (&channel, &msg.content) {
        (Channel::IoPub, JupyterMessageContent::StreamContent(sc)) => EventData::Stream {
            name: match sc.name {
                Stdio::Stdout => "stdout".into(),
                Stdio::Stderr => "stderr".into(),
            },
            text: sc.text.clone(),
        },
        (Channel::IoPub, JupyterMessageContent::DisplayData(dd)) => EventData::DisplayData {
            data: media_to_value(&dd.data.content),
        },
        (Channel::IoPub, JupyterMessageContent::ExecuteResult(er)) => EventData::DisplayData {
            data: media_to_value(&er.data.content),
        },
        (Channel::IoPub, JupyterMessageContent::ExecuteInput(ei)) => EventData::ExecuteInput {
            code: ei.code.clone(),
        },
        (Channel::IoPub, JupyterMessageContent::ErrorOutput(err)) => {
            // Note 1 (iopub vs shell):
            // Kernels can omit error info on both the shell and IoPub. Shell only goes back to the
            // client that ran the code, but IoPub is broadcast to all clients. For this reason it
            // seems that the conventional behaviour for clients is to ignore the shell reply and
            // only display the IoPub error.
            // https://github.com/posit-dev/positron/issues/1053

            // Note 2 (traceback vs ename/evalue):
            // JupyterLab ignores the ename and evalue if the traceback is present. This is probably
            // influenced by ipykernel, since here there is overlap in traceback and ename/evalue
            // content. Other kernels seem to do this too, e.g. Ark rolls ename and evalue into the
            // traceback if you use `--session-mode notebook` (but not otherwise, leading to omitted
            // info in the error message).
            let mut traceback = "".to_string();

            if !err.traceback.is_empty() {
                traceback.push_str(&err.traceback.join("\n"));
            } else {
                if !err.ename.is_empty() {
                    traceback.push_str(&err.ename);
                    traceback.push_str(": ");
                }

                if !err.evalue.is_empty() {
                    traceback.push_str("\n");
                    traceback.push_str(&err.evalue);
                }

                if !err.traceback.is_empty() {
                    traceback.push_str("\n");
                    traceback.push_str(&err.traceback.join("\n"));
                }
            }

            EventData::Error { traceback }
        }
        (Channel::Shell, JupyterMessageContent::KernelInfoReply(reply)) => EventData::Banner {
            text: reply.banner.clone(),
        },
        (Channel::Shell, JupyterMessageContent::IsCompleteReply(reply)) => EventData::IsComplete {
            parent_id,
            status: reply.status.clone(),
            indent: reply.indent.clone(),
        },
        (Channel::Stdin, JupyterMessageContent::InputRequest(req)) => EventData::InputRequest {
            prompt: req.prompt.clone(),
            password: req.password,
            parent_id,
        },
        (Channel::IoPub, JupyterMessageContent::Status(s)) => match s.execution_state {
            ExecutionState::Idle if parent_id.is_some() => EventData::Idle { parent_id },
            ExecutionState::Busy => EventData::Busy { parent_id },
            _ => EventData::Other,
        },
        (Channel::Shell, JupyterMessageContent::ExecuteReply(_)) => {
            EventData::ExecuteReply { parent_id }
        }
        _ => EventData::Other,
    };

    Event {
        parent_session,
        data: event_data,
    }
}

/// Re-encode a `Vec<MediaType>` back into the original Jupyter media bundle
/// shape (`{ "image/png": ..., "text/plain": ..., ... }`) so the existing
/// renderer (which works on `serde_json::Value`) doesn't need to learn the
/// runtimed type.
fn media_to_value(content: &[MediaType]) -> Value {
    let mut map = Map::new();
    for mt in content {
        match mt {
            MediaType::Plain(s) => {
                map.insert("text/plain".into(), Value::String(s.clone()));
            }
            MediaType::Html(s) => {
                map.insert("text/html".into(), Value::String(s.clone()));
            }
            MediaType::Latex(s) => {
                map.insert("text/latex".into(), Value::String(s.clone()));
            }
            MediaType::Javascript(s) => {
                map.insert("application/javascript".into(), Value::String(s.clone()));
            }
            MediaType::Markdown(s) => {
                map.insert("text/markdown".into(), Value::String(s.clone()));
            }
            MediaType::Svg(s) => {
                map.insert("image/svg+xml".into(), Value::String(s.clone()));
            }
            MediaType::Png(s) => {
                map.insert("image/png".into(), Value::String(s.clone()));
            }
            MediaType::Jpeg(s) => {
                map.insert("image/jpeg".into(), Value::String(s.clone()));
            }
            MediaType::Gif(s) => {
                map.insert("image/gif".into(), Value::String(s.clone()));
            }
            MediaType::Json(o)
            | MediaType::GeoJson(o)
            | MediaType::Plotly(o)
            | MediaType::WidgetView(o)
            | MediaType::WidgetState(o)
            | MediaType::VegaLiteV2(o)
            | MediaType::VegaLiteV3(o)
            | MediaType::VegaLiteV4(o)
            | MediaType::VegaLiteV5(o)
            | MediaType::VegaLiteV6(o)
            | MediaType::VegaV3(o)
            | MediaType::VegaV4(o)
            | MediaType::VegaV5(o)
            | MediaType::Vdom(o) => {
                map.insert(mt.mime_type().into(), o.clone());
            }
            MediaType::DataTable(boxed) => {
                if let Ok(v) = serde_json::to_value(boxed) {
                    map.insert(mt.mime_type().into(), v);
                }
            }
            MediaType::Other((mime, value)) => {
                map.insert(mime.clone(), value.clone());
            }
            // MediaType is #[non_exhaustive]; ignore variants we don't
            // recognize. The renderer only knows about a handful anyway.
            _ => {}
        }
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jupyter_protocol::{
        DisplayData, ErrorOutput, InputRequest as JpInputRequest, JupyterMessage, KernelInfoReply,
        LanguageInfo, Media, MediaType, Status, StreamContent,
    };

    fn with_parent(mut m: JupyterMessage, parent_id: &str) -> JupyterMessage {
        let mut header = m.header.clone();
        header.msg_id = parent_id.to_string();
        m.parent_header = Some(header);
        m
    }

    #[test]
    fn stream_event() {
        let msg: JupyterMessage = StreamContent {
            name: Stdio::Stdout,
            text: "hi".into(),
        }
        .into();
        match from_message(Channel::IoPub, &msg).data {
            EventData::Stream { name, text } => {
                assert_eq!(name, "stdout");
                assert_eq!(text, "hi");
            }
            other => panic!("expected Stream, got {other:?}"),
        }
    }

    #[test]
    fn display_data_event() {
        let msg: JupyterMessage = DisplayData {
            data: Media {
                content: vec![MediaType::Plain("x".into())],
            },
            metadata: Default::default(),
            transient: None,
        }
        .into();
        match from_message(Channel::IoPub, &msg).data {
            EventData::DisplayData { data } => {
                assert_eq!(data["text/plain"], "x");
            }
            other => panic!("expected DisplayData, got {other:?}"),
        }
    }

    #[test]
    fn error_event_uses_traceback() {
        let msg: JupyterMessage = ErrorOutput {
            ename: "RuntimeError".into(),
            evalue: "boom".into(),
            traceback: vec!["line1".into(), "line2".into()],
        }
        .into();
        match from_message(Channel::IoPub, &msg).data {
            EventData::Error { traceback } => assert_eq!(traceback, "line1\nline2"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn error_event_falls_back_to_evalue() {
        let msg: JupyterMessage = ErrorOutput {
            ename: "".into(),
            evalue: "Error:\n! boom".into(),
            traceback: vec![],
        }
        .into();
        match from_message(Channel::IoPub, &msg).data {
            EventData::Error { traceback } => assert_eq!(traceback, "\nError:\n! boom"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn error_event_combines_ename_evalue() {
        let msg: JupyterMessage = ErrorOutput {
            ename: "RuntimeError".into(),
            evalue: "boom".into(),
            traceback: vec![],
        }
        .into();
        match from_message(Channel::IoPub, &msg).data {
            EventData::Error { traceback } => assert_eq!(traceback, "RuntimeError: \nboom"),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn banner_event() {
        let reply = KernelInfoReply {
            status: Default::default(),
            protocol_version: "5.3".into(),
            implementation: "ipykernel".into(),
            implementation_version: "0".into(),
            language_info: LanguageInfo {
                name: "python".into(),
                version: "3".into(),
                mimetype: None,
                file_extension: None,
                pygments_lexer: None,
                codemirror_mode: None,
                nbconvert_exporter: None,
            },
            banner: "hello".into(),
            help_links: vec![],
            debugger: false,
            error: None,
        };
        let msg: JupyterMessage = reply.into();
        match from_message(Channel::Shell, &msg).data {
            EventData::Banner { text } => assert_eq!(text, "hello"),
            other => panic!("expected Banner, got {other:?}"),
        }
    }

    #[test]
    fn idle_event() {
        let msg: JupyterMessage = Status::idle().into();
        let msg = with_parent(msg, "abc");
        match from_message(Channel::IoPub, &msg).data {
            EventData::Idle { parent_id } => assert_eq!(parent_id, Some("abc".into())),
            other => panic!("expected Idle, got {other:?}"),
        }
    }

    #[test]
    fn busy_status_event() {
        let msg: JupyterMessage = Status::busy().into();
        let msg = with_parent(msg, "abc");
        match from_message(Channel::IoPub, &msg).data {
            EventData::Busy { parent_id } => assert_eq!(parent_id, Some("abc".into())),
            other => panic!("expected Busy, got {other:?}"),
        }
    }

    #[test]
    fn idle_without_parent_is_other() {
        let msg: JupyterMessage = Status::idle().into();
        assert!(matches!(
            from_message(Channel::IoPub, &msg).data,
            EventData::Other
        ));
    }

    #[test]
    fn input_request_event() {
        let msg: JupyterMessage = JpInputRequest {
            prompt: "enter: ".into(),
            password: false,
        }
        .into();
        let msg = with_parent(msg, "exec-1");
        match from_message(Channel::Stdin, &msg).data {
            EventData::InputRequest {
                prompt,
                password,
                parent_id,
            } => {
                assert_eq!(prompt, "enter: ");
                assert!(!password);
                assert_eq!(parent_id, Some("exec-1".into()));
            }
            other => panic!("expected InputRequest, got {other:?}"),
        }
    }
}
