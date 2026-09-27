//! Line-delimited JSON over a child's stdio, shared by the stdio adapters
//! (claude, codex, acp, pi), plus the optional raw-frame recorder.
//!
//! High level: `LineWire::over` takes the child's pipes and starts the
//! reader task; `write` sends one frame; `frames` receives them.

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::adapter::Emitter;
use crate::agent::SessionOptions;
use crate::event::DiagnosticLevel;
use crate::process::{Child, SharedStdin};

/// Frames buffered between the reader task and the drive task.
pub(crate) const FRAME_BUFFER: usize = 64;

/// Where frames carry declared MCP servers: claude's `mcp_set_servers`, ACP's
/// `session/new` and `session/load`, opencode's `POST /mcp` body.
const MCP_SERVERS_AT: [&str; 3] = ["/request/servers", "/params/mcpServers", "/body/config"];

/// One JSON object per line each way. Adapters add their own request ids
/// and response matching on top.
pub(crate) struct LineWire {
    /// Shared with the child so `shutdown` can close it (EOF).
    stdin: SharedStdin,
    /// Every frame the reader task parsed, in order; closes at stdout EOF.
    pub frames: mpsc::Receiver<Value>,
    recorder: Option<WireRecorder>,
}

impl LineWire {
    /// Takes the child's stdio and starts the line-reader task. Unparseable
    /// lines are skipped.
    pub(crate) fn over(child: &mut Child, recorder: Option<WireRecorder>) -> Self {
        let stdin = child.stdin.clone();
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, frames) = mpsc::channel(FRAME_BUFFER);
        let reader_recorder = recorder.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(frame) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if let Some(recorder) = &reader_recorder {
                    recorder.record("in", &frame);
                }
                if tx.send(frame).await.is_err() {
                    break;
                }
            }
        });
        Self {
            stdin,
            frames,
            recorder,
        }
    }

    /// Writes one frame as a line, recording it first. Fails with
    /// `BrokenPipe` once shutdown has closed stdin.
    pub(crate) async fn write(&mut self, frame: Value) -> std::io::Result<()> {
        if let Some(recorder) = &self.recorder {
            recorder.record("out", &frame);
        }
        let mut line = frame.to_string();
        line.push('\n');
        match self.stdin.lock().await.as_mut() {
            Some(stdin) => stdin.write_all(line.as_bytes()).await,
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        }
    }
}

/// Tees raw protocol frames to a JSONL file when `record_wire` is set: one
/// `{"dir":"in"|"out","frame":<frame>}` per line, append-only and flushed
/// per line. Unredacted except declared MCP servers' header and env values and
/// codex's config; unbounded: a local debug artifact. A write failure is reported once as a
/// `Diagnostic`; recording never fails a turn.
#[derive(Clone)]
pub(crate) struct WireRecorder {
    lines: mpsc::UnboundedSender<Vec<u8>>,
}

impl WireRecorder {
    /// The session's recorder when `record_wire` is set; `None` otherwise.
    /// An open failure is one diagnostic and recording stays off.
    pub(crate) async fn for_session(options: &SessionOptions, events: &Emitter) -> Option<Self> {
        let path = options.record_wire.as_deref()?;
        let events = events.clone();
        let file = match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
        {
            Ok(file) => file,
            Err(e) => {
                let _ = events
                    .diagnostic(
                        DiagnosticLevel::Warning,
                        format!("wire recording is off: {e}"),
                    )
                    .await;
                return None;
            }
        };
        let (lines, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            let mut file = file;
            while let Some(bytes) = rx.recv().await {
                if let Err(e) = append(&mut file, &bytes).await {
                    let _ = events
                        .diagnostic(
                            DiagnosticLevel::Warning,
                            format!("wire recording stopped: {e}"),
                        )
                        .await;
                    break;
                }
            }
        });
        Some(Self { lines })
    }

    /// Records one frame in the given direction, secrets redacted. Never
    /// blocks or errors; a gone writer just loses the frame.
    pub(crate) fn record(&self, dir: &'static str, frame: &Value) {
        let mut entry = json!({ "dir": dir, "frame": frame });
        for at in MCP_SERVERS_AT {
            if let Some(servers) = entry["frame"].pointer_mut(at) {
                redact_mcp_servers(servers);
            }
        }
        // codex's `config/read` reply is the user's whole config file.
        if let Some(config) = entry["frame"].pointer_mut("/result/config") {
            *config = json!("<redacted>");
        }
        let mut line = entry.to_string();
        line.push('\n');
        let _ = self.lines.send(line.into_bytes());
    }
}

/// Replaces every header and env value in declared MCP servers' JSON with
/// `<redacted>`, in any adapter's shape; names, URLs, commands and args stay.
fn redact_mcp_servers(servers: &mut Value) {
    match servers {
        Value::Object(fields) => {
            for (key, value) in fields {
                if !matches!(key.as_str(), "headers" | "env" | "environment") {
                    redact_mcp_servers(value);
                    continue;
                }
                // A name-to-value map, or ACP's `[{name, value}]` list.
                let values: Vec<&mut Value> = match value {
                    Value::Object(map) => map.values_mut().collect(),
                    Value::Array(pairs) => pairs
                        .iter_mut()
                        .filter_map(|p| p.get_mut("value"))
                        .collect(),
                    _ => Vec::new(),
                };
                for secret in values {
                    *secret = json!("<redacted>");
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact_mcp_servers),
        _ => {}
    }
}

/// Appends and flushes one line.
async fn append(file: &mut tokio::fs::File, bytes: &[u8]) -> std::io::Result<()> {
    file.write_all(bytes).await?;
    file.flush().await
}
