//! The native Codex adapter driven end to end through the public interface,
//! against the fixture agent (tests/fixtures/codex/fixture.mjs; needs `node`).
//! A wrapper script pins the catalog's `codex` id to the fixture.

use std::path::PathBuf;
use std::time::Duration;

use futures::StreamExt;

use anyagent::{
    AgentError, AgentInstallation, Answer, AuthKind, AuthStatus, Capability, CommandSource,
    ConfigId, ConfigKind, ConfigValue, DeliveryKind, DiagnosticLevel, Event, EventKind, Events,
    Input, LoginMethod, McpServer, PermissionChoice, PermissionRequest, PlanStatus, QuestionAnswer,
    Request, Runtime, Session, SessionOptions, StopReason, ToolInput, ToolKind, ToolStatus,
    ToolUpdate, TurnUsage,
};

mod common;

/// A `codex` stand-in: a script that execs the fixture with scenario flags,
/// ignoring the real launch args appended after them.
fn wrapper(name: &str, flags: &str) -> PathBuf {
    common::wrapper("codex", "codex", name, flags)
}

async fn open_with(
    name: &str,
    flags: &str,
    options: SessionOptions,
) -> Result<(Session, Events), AgentError> {
    let runtime = Runtime::new();
    let agent = AgentInstallation::at("codex", wrapper(name, flags));
    runtime.open(&agent, options).await
}

async fn open(name: &str, flags: &str) -> (Session, Events) {
    open_with(name, flags, SessionOptions::in_dir(std::env::temp_dir()))
        .await
        .unwrap()
}

async fn next(events: &mut Events) -> Event {
    tokio::time::timeout(Duration::from_secs(10), events.next())
        .await
        .expect("timed out waiting for an event")
        .expect("stream ended")
        .expect("stream error")
}

/// Drives one turn to its end: answers permissions with `answer`, collects
/// text, and returns it.
async fn complete_turn(session: &Session, events: &mut Events, answer: PermissionChoice) -> String {
    complete_turn_usage(session, events, answer).await.0
}

/// `complete_turn`, also returning the usage the turn ended with.
async fn complete_turn_usage(
    session: &Session,
    events: &mut Events,
    answer: PermissionChoice,
) -> (String, Option<TurnUsage>) {
    let mut text = String::new();
    loop {
        match next(events).await.kind {
            EventKind::TextDelta { text: t, .. } => text.push_str(&t),
            EventKind::RequestOpened(Request::Permission(request)) => {
                session
                    .answer(request.id, Answer::Permission(answer))
                    .await
                    .unwrap();
            }
            EventKind::TurnEnded { usage, .. } => return (text, usage),
            _ => {}
        }
    }
}

/// The fixture turn's one model call.
const TURN_USAGE: TurnUsage = TurnUsage {
    input_tokens: 1100,
    cached_input_tokens: 600,
    output_tokens: 100,
};

/// Drains events through the first one `stop` matches, returning the text
/// of every diagnostic on the way.
async fn diagnostics_until(events: &mut Events, stop: impl Fn(&EventKind) -> bool) -> Vec<String> {
    let mut seen = Vec::new();
    loop {
        let kind = next(events).await.kind;
        if let EventKind::Diagnostic(d) = &kind {
            seen.push(d.message.clone());
        }
        if stop(&kind) {
            return seen;
        }
    }
}

/// Runs one MCP-approval turn, answering each permission with `answer` (`None`
/// cancels the turn). Returns the text, the requests, the MCP tool snapshots, and the stop.
async fn mcp_turn(
    session: &Session,
    events: &mut Events,
    prompt: &str,
    answer: Option<Answer>,
) -> (String, Vec<PermissionRequest>, Vec<ToolUpdate>, StopReason) {
    session.prompt(prompt).await.unwrap();
    let (mut text, mut requests, mut states) = (String::new(), Vec::new(), Vec::new());
    loop {
        match next(events).await.kind {
            EventKind::TextDelta { text: t, .. } => text.push_str(&t),
            EventKind::ToolUpdated(tool) if matches!(tool.kind, ToolKind::Mcp { .. }) => {
                states.push(tool)
            }
            EventKind::RequestOpened(Request::Permission(request)) => {
                let id = request.id.clone();
                requests.push(request);
                match &answer {
                    Some(answer) => session.answer(id, answer.clone()).await,
                    None => session.cancel(false).await,
                }
                .unwrap();
            }
            EventKind::TurnEnded { stop, .. } => return (text, requests, states, stop),
            _ => {}
        }
    }
}

fn text_option(session: &anyagent::SessionInfo, id: &str) -> Option<String> {
    session.configuration.options.iter().find_map(|(k, v)| {
        (k.as_str() == id).then(|| match v {
            ConfigValue::Text(t) => t.clone(),
            ConfigValue::Bool(b) => b.to_string(),
        })
    })
}

/// Handshake reports auth, version 0.147.0, capabilities, token, and deduped
/// enabled skills (with path and scope) as commands.
#[tokio::test]
async fn handshake_reports_auth_version_options_and_token() {
    let (session, mut events) = open("handshake", "").await;
    // Skills arrive after open as a `SessionUpdated` (the fetch is async so
    // opens stay fast); wait for the list before reading the snapshot.
    while session.info().details.commands.is_empty() {
        next(&mut events).await;
    }
    let info = session.info();
    assert_eq!(info.details.version.as_deref(), Some("0.147.0"));
    let AuthStatus::Authenticated { kind, account } = &info.details.auth else {
        panic!(
            "expected an authenticated login, got {:?}",
            info.details.auth
        );
    };
    assert_eq!(*kind, AuthKind::Subscription);
    let account = account.as_ref().unwrap();
    assert_eq!(account.email.as_deref(), Some("user@example.com"));
    assert_eq!(account.plan.as_deref(), Some("edu"));
    // The resume token exists at open: `thread/start` returns the id.
    assert_eq!(info.resume_token.as_ref().unwrap().as_str(), "th-1");

    let caps = &info.details.capabilities;
    for cap in [
        Capability::Steer,
        Capability::Fork,
        Capability::PlanUsage,
        Capability::Rollback,
        Capability::Images,
        Capability::Questions,
    ] {
        assert!(caps.supports(cap.clone()), "missing {cap:?}");
    }
    assert!(
        !caps.supports(Capability::RollbackFiles),
        "over-advertised RollbackFiles"
    );

    // Hidden models stay hidden; effort defaults to the model's default.
    let model = info
        .details
        .config_options
        .iter()
        .find(|o| o.id.as_str() == "model")
        .unwrap();
    let ConfigKind::Select { choices } = &model.kind else {
        panic!("model is a select");
    };
    assert_eq!(
        choices.iter().map(|c| c.value.as_str()).collect::<Vec<_>>(),
        vec!["gpt-6", "gpt-6-mini"]
    );
    assert!(model.live);
    assert_eq!(text_option(&info, "model").as_deref(), Some("gpt-6"));
    assert_eq!(text_option(&info, "effort").as_deref(), Some("medium"));
    assert_eq!(text_option(&info, "mode").as_deref(), Some("on-request"));
    assert_eq!(text_option(&info, "sandbox").as_deref(), Some("read-only"));

    // Skills are the commands: deduped, junk and disabled dropped, short description first.
    let skill = |path: &str, scope: &str| CommandSource::Skill {
        path: Some(PathBuf::from(path)),
        scope: Some(scope.into()),
    };
    let commands: Vec<_> = info
        .details
        .commands
        .iter()
        .map(|c| (c.name.as_str(), c.description.as_str(), c.source.clone()))
        .collect();
    assert_eq!(
        commands,
        vec![
            (
                "review",
                "Review a diff.",
                skill("/repo/.codex/skills/review/SKILL.md", "repo")
            ),
            (
                "release",
                "Cut a release.",
                skill("/home/skills/release/SKILL.md", "user")
            ),
        ]
    );
    session.close().await.unwrap();
}

/// Probe returns commands in <2s without waiting full timeout.
#[tokio::test]
async fn probe_reads_commands_without_waiting_them_out() {
    let runtime = Runtime::new();
    let agent = AgentInstallation::at("codex", wrapper("probe", ""));
    let started = std::time::Instant::now();
    let details = runtime.probe(&agent).await.unwrap();
    assert!(!details.commands.is_empty());
    // An empty command list would cost the probe its full 2 s wait.
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

/// A probe sends no `thread/start` (it starts the user's MCP servers and
/// hooks): current values come from `config/read`, commands from `skills/list`.
#[tokio::test]
async fn a_probe_opens_no_thread() {
    let dir = tempfile::tempdir().unwrap();
    let wire = dir.path().join("wire.jsonl");
    let agent = AgentInstallation::at("codex", wrapper("probe-no-thread", ""));
    let options = SessionOptions::in_dir(dir.path()).record_wire(&wire);
    let details = Runtime::new().probe_with(&agent, options).await.unwrap();

    // Frames are written in order: once `skills/list` is there, so is any `thread/start`.
    common::sent_frames(&wire, 1, |f| f["method"] == "skills/list").await;
    let thread = common::sent_frames(&wire, 0, |f| f["method"] == "thread/start").await;
    assert!(thread.is_empty(), "{thread:?}");
    common::sent_frames(&wire, 1, |f| f["method"] == "config/read").await;
    assert_eq!(details.commands.len(), 2);
    let current = |id: &str| {
        let option = details.config_options.iter().find(|o| o.id.as_str() == id);
        match option.and_then(|o| o.current.clone()) {
            Some(ConfigValue::Text(value)) => value,
            other => panic!("{id}: {other:?}"),
        }
    };
    // The config's model with its default effort; unset approval is codex's default.
    assert_eq!(current("model"), "gpt-6-mini");
    assert_eq!(current("effort"), "low");
    assert_eq!(current("mode"), "on-request");
    assert_eq!(current("sandbox"), "workspace-write");
}

/// A probe's recording redacts the `config/read` reply: it is the user's codex config.
#[tokio::test]
async fn a_probe_recording_redacts_the_codex_config() {
    let dir = tempfile::tempdir().unwrap();
    let wire = dir.path().join("wire.jsonl");
    let agent = AgentInstallation::at("codex", wrapper("probe-redact", ""));
    let options = SessionOptions::in_dir(dir.path()).record_wire(&wire);
    Runtime::new().probe_with(&agent, options).await.unwrap();
    // `skills/list` goes out after the config reply came in: both are written.
    common::sent_frames(&wire, 1, |f| f["method"] == "skills/list").await;
    let recording = std::fs::read_to_string(&wire).unwrap();
    assert!(
        recording.contains(r#""config":"<redacted>""#),
        "{recording}"
    );
    assert!(!recording.contains("sandbox_mode"), "{recording}");
}

/// A probe reports the caller's `configure` choices over the config file's, as
/// `thread/start` would echo them.
#[tokio::test]
async fn a_probe_reports_the_configured_choices() {
    let agent = AgentInstallation::at("codex", wrapper("probe-configured", ""));
    let options = SessionOptions::in_dir(std::env::temp_dir())
        .configure("model", "gpt-6")
        .configure("effort", "high")
        .configure("sandbox", "danger-full-access")
        .configure("mode", "never");
    let details = Runtime::new().probe_with(&agent, options).await.unwrap();
    let current = |id: &str| {
        let option = details.config_options.iter().find(|o| o.id.as_str() == id);
        option.and_then(|o| o.current.clone())
    };
    assert_eq!(current("model"), Some("gpt-6".into()));
    assert_eq!(current("effort"), Some("high".into()));
    assert_eq!(current("sandbox"), Some("danger-full-access".into()));
    assert_eq!(current("mode"), Some("never".into()));
}

/// A codex that dies while the probe reads its config fails the probe; it never reports defaults.
#[tokio::test]
async fn a_probe_fails_when_codex_dies_at_the_config_read() {
    let agent = AgentInstallation::at("codex", wrapper("probe-dies", "--config-read-dies"));
    let failed = Runtime::new().probe(&agent).await;
    assert!(
        matches!(failed, Err(AgentError::ProtocolFailed(_))),
        "{failed:?}"
    );
}

/// An output schema rides every `turn/start` as `outputSchema`: it holds for one turn only.
#[tokio::test]
async fn an_output_schema_rides_every_turn() {
    let dir = tempfile::tempdir().unwrap();
    let wire = dir.path().join("wire.jsonl");
    let schema = serde_json::json!({ "type": "object", "properties": { "title": { "type": "string" } }, "required": ["title"] });
    let options = SessionOptions::in_dir(dir.path())
        .output_schema(schema.clone())
        .record_wire(&wire);
    let (session, mut events) = open_with("schema", "", options).await.unwrap();
    let capabilities = session.info().details.capabilities;
    assert!(capabilities.supports(Capability::OutputSchema));
    for _ in 0..2 {
        session.prompt("hi").await.unwrap();
        complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    }
    let sent: Vec<_> = common::sent_frames(&wire, 2, |f| f["method"] == "turn/start")
        .await
        .into_iter()
        .map(|f| f["params"]["outputSchema"].clone())
        .collect();
    assert_eq!(sent, [schema.clone(), schema]);
    session.close().await.unwrap();
}

/// A subagent's child thread runs a whole turn inside the parent's: its
/// content must ride the subagent tool and its bookkeeping must not touch the
/// parent turn.
/// Subagent child thread's deltas/tool updates are attributed via parent_tool_id and don't settle parent turn.
#[tokio::test]
async fn a_subagent_child_thread_never_settles_the_parent_turn() {
    let (session, mut events) = open("subagent", "").await;
    session.prompt("subagent please").await.unwrap();

    let mut turns_started = 0;
    let mut turns_ended = 0;
    let mut child_text = Vec::new();
    let mut subagents = Vec::new();
    let mut usage = Vec::new();
    let mut plans = Vec::new();
    let mut diagnostics = Vec::new();
    loop {
        let event = next(&mut events).await;
        let parent = event
            .turn_info
            .as_ref()
            .and_then(|t| t.parent_tool_id.clone());
        match event.kind {
            EventKind::TurnStarted { .. } => turns_started += 1,
            EventKind::TextDelta { text, .. } if parent.is_some() => {
                child_text.push((text, parent.unwrap()));
            }
            EventKind::ToolUpdated(tool) if tool.kind == ToolKind::Subagent => {
                subagents.push(tool);
            }
            EventKind::ContextUsage { used_tokens, .. } => usage.push(used_tokens),
            EventKind::PlanUpdated { entries } => plans.push(entries),
            EventKind::Diagnostic(d) => diagnostics.push(d.message),
            EventKind::TurnEnded { .. } => {
                turns_ended += 1;
                break;
            }
            _ => {}
        }
    }
    assert_eq!((turns_started, turns_ended), (1, 1));
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    // The `subAgentActivity` tool is the child thread: Running while the child
    // works, then with the child's token total, Completed once its turn ends.
    let activity = subagents
        .iter()
        .find(|t| t.title.contains("reviewer.md"))
        .expect("a subagent tool for the child thread")
        .id
        .clone();
    let states: Vec<_> = subagents
        .iter()
        .filter(|t| t.id == activity)
        .map(|t| (t.status, t.subagent.as_ref().and_then(|s| s.tokens)))
        .collect();
    assert_eq!(
        states,
        vec![
            (ToolStatus::Running, None),
            (ToolStatus::Running, Some(77)),
            (ToolStatus::Completed, Some(77)),
        ]
    );
    // The child's content is attributed to it.
    assert_eq!(child_text, vec![("child text".to_owned(), activity)]);
    // The collab call is a subagent tool too, carrying its prompt.
    assert!(
        subagents
            .iter()
            .any(|t| t.input == ToolInput::Text("review the diff".into())),
        "{subagents:?}"
    );
    // Neither the child's usage nor its plan reaches the parent's.
    assert_eq!(usage, vec![1200]);
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0][0].text, "step 1");
    session.close().await.unwrap();
}

/// Recording 13 (0.154.0): the child's finish arrives as a second activity item
/// under a new id. The subagent tool ends Completed once; no second tool appears.
#[tokio::test]
async fn a_live_shaped_subagent_ends_completed_once() {
    let (session, mut events) = open("spawn-live", "").await;
    session.prompt("spawn-live please").await.unwrap();
    let mut activity = Vec::new();
    loop {
        match next(&mut events).await.kind {
            EventKind::ToolUpdated(tool)
                if tool
                    .raw
                    .as_ref()
                    .is_some_and(|r| r.name == "subAgentActivity") =>
            {
                let tokens = tool.subagent.and_then(|s| s.tokens);
                activity.push((tool.id.as_str().to_owned(), tool.status, tokens));
            }
            EventKind::TurnEnded { .. } => break,
            _ => {}
        }
    }
    let spawn = || "call_spawn".to_owned();
    assert_eq!(
        activity,
        vec![
            (spawn(), ToolStatus::Running, None),
            (spawn(), ToolStatus::Running, Some(22059)),
            (spawn(), ToolStatus::Completed, Some(22059)),
        ]
    );
    session.close().await.unwrap();
}

/// Failed child turn marks its subagent tool as Failed but parent still completes.
#[tokio::test]
async fn a_failed_child_turn_fails_its_subagent_tool() {
    let (session, mut events) = open("subagent-fail", "").await;
    session.prompt("subagent-fails now").await.unwrap();
    let mut failed = None;
    loop {
        match next(&mut events).await.kind {
            EventKind::ToolUpdated(tool)
                if tool.kind == ToolKind::Subagent && tool.status == ToolStatus::Failed =>
            {
                failed = Some(tool);
            }
            // The parent turn still completes normally.
            EventKind::TurnEnded { stop, .. } => {
                assert!(matches!(stop, StopReason::Completed { .. }), "{stop:?}");
                break;
            }
            _ => {}
        }
    }
    let failed = failed.expect("the failed child marks its subagent tool Failed");
    assert_eq!(failed.output.as_deref(), Some("child blew up"));
    session.close().await.unwrap();
}

/// Full turn maps text/reasoning/execute tool, plan, usage, quota, and fork_point extension.
#[tokio::test]
async fn a_full_turn_maps_every_frame_kind() {
    let (session, mut events) = open("full", "").await;
    session.prompt("hi").await.unwrap();

    let mut text = String::new();
    let mut thoughts = String::new();
    let mut tool_states = Vec::new();
    let mut plan = Vec::new();
    let mut usage = None;
    let mut quota = None;
    let mut fork_point = None;
    loop {
        let event = next(&mut events).await;
        if let Some(point) = event.extensions.get("codex/fork_point") {
            fork_point = point.as_str().map(str::to_owned);
        }
        match event.kind {
            EventKind::TextDelta { text: t, .. } => text.push_str(&t),
            EventKind::ReasoningDelta { text: t, .. } => thoughts.push_str(&t),
            EventKind::ToolUpdated(tool) => tool_states.push(tool),
            EventKind::PlanUpdated { entries } => plan = entries,
            EventKind::ContextUsage {
                used_tokens,
                window_tokens,
                cost_usd,
            } => usage = Some((used_tokens, window_tokens, cost_usd)),
            EventKind::PlanUsageUpdated(u) => quota = Some(u),
            EventKind::UserMessage { .. } => panic!("own prompt echoed back"),
            EventKind::TurnEnded { stop, .. } => {
                assert_eq!(
                    stop,
                    StopReason::Completed {
                        source: anyagent::CompletionSource::Protocol
                    }
                );
                break;
            }
            _ => {}
        }
    }
    assert!(
        text.starts_with("Hello ") && text.ends_with("done"),
        "{text}"
    );
    assert_eq!(thoughts, "thinking…");
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].text, "step 1");
    assert_eq!(plan[0].status, PlanStatus::InProgress);
    assert_eq!(usage, Some((1200, Some(258_400), None)));
    // The wire turn id rides `MessageEnded` as the fork anchor.
    assert_eq!(fork_point.as_deref(), Some("turn-0"));

    let quota = quota.unwrap();
    assert_eq!(quota.plan.as_deref(), Some("edu"));
    assert_eq!(quota.windows[0].label, "Session");
    assert_eq!(quota.windows[0].used_percent, 5);
    assert_eq!(quota.windows[1].label, "Week");
    assert_eq!(quota.windows[1].used_percent, 4);

    // The command: running, then completed with its output.
    let exec: Vec<_> = tool_states
        .iter()
        .filter(|t| t.kind == ToolKind::Execute)
        .collect();
    assert_eq!(exec[0].status, ToolStatus::Running);
    let done = exec.last().unwrap();
    assert_eq!(done.status, ToolStatus::Completed);
    assert_eq!(done.output.as_deref(), Some("PEAR\n"));
    session.close().await.unwrap();
}

/// Model+effort ride every turn; live model switch applies to next turn without wire call.
#[tokio::test]
async fn model_and_effort_ride_every_turn_and_switch_live() {
    let (session, mut events) = open_with(
        "per-turn",
        "",
        SessionOptions::in_dir(std::env::temp_dir())
            .configure("model", "gpt-6-mini")
            .configure("effort", "medium"),
    )
    .await
    .unwrap();
    session.prompt("hi").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("model=gpt-6-mini effort=medium"), "{text}");

    // A live switch needs no wire call: the next turn carries it.
    session.configure("model", "gpt-6").await.unwrap();
    loop {
        if let EventKind::SessionUpdated(info) = next(&mut events).await.kind {
            assert_eq!(text_option(&info, "model").as_deref(), Some("gpt-6"));
            break;
        }
    }
    session.prompt("again").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("model=gpt-6 effort=medium"), "{text}");
    session.close().await.unwrap();
}

/// Effort falls back to model's default when new model lacks previous level.
#[tokio::test]
async fn service_tier_rides_turns_and_default_is_omitted() {
    // Configured at creation like Comet does; every turn also opts into
    // reasoning summaries (`summary: "auto"`).
    let (session, mut events) = open_with(
        "tier",
        "",
        SessionOptions::in_dir(std::env::temp_dir()).configure("serviceTier", "priority"),
    )
    .await
    .unwrap();
    let info = session.info();
    let tier = info
        .details
        .config_options
        .iter()
        .find(|o| o.id.as_str() == "serviceTier")
        .unwrap();
    assert!(tier.live);
    let ConfigKind::Select { choices } = &tier.kind else {
        panic!("serviceTier is a select");
    };
    assert_eq!(
        choices.iter().map(|c| c.value.as_str()).collect::<Vec<_>>(),
        vec!["default", "priority"]
    );
    session.prompt("hi").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("tier=priority summary=auto"), "{text}");

    // Back to Standard: "default" never reaches the wire.
    session.configure("serviceTier", "default").await.unwrap();
    loop {
        if let EventKind::SessionUpdated(info) = next(&mut events).await.kind {
            assert_eq!(
                text_option(&info, "serviceTier").as_deref(),
                Some("default")
            );
            break;
        }
    }
    session.prompt("again").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("tier=unset"), "{text}");
    session.close().await.unwrap();
}

#[tokio::test]
async fn failed_and_aborted_turn_notifications_still_end_the_turn() {
    let (session, mut events) = open("turn-ends", "").await;
    for (prompt, expected) in [
        (
            "end-failed",
            StopReason::Failed {
                message: "wire failed".into(),
            },
        ),
        ("end-aborted", StopReason::Cancelled),
    ] {
        session.prompt(prompt).await.unwrap();
        loop {
            if let EventKind::TurnEnded { stop, .. } = next(&mut events).await.kind {
                assert_eq!(stop, expected);
                break;
            }
        }
    }
    session.close().await.unwrap();
}

#[tokio::test]
async fn effort_falls_back_when_the_new_model_lacks_it() {
    let (session, mut events) = open_with(
        "effort-fallback",
        "",
        SessionOptions::in_dir(std::env::temp_dir()).configure("effort", "high"),
    )
    .await
    .unwrap();
    assert_eq!(
        text_option(&session.info(), "effort").as_deref(),
        Some("high")
    );
    // Each model choice carries its own effort (at its default) and fast.
    let model = session
        .info()
        .details
        .config_options
        .into_iter()
        .find(|o| o.id.as_str() == "model")
        .unwrap();
    let ConfigKind::Select { choices } = model.kind else {
        panic!("model is a select");
    };
    let nested = |value: &str| {
        choices
            .iter()
            .find(|c| c.value == value)
            .unwrap()
            .options
            .clone()
    };
    let gpt6 = nested("gpt-6");
    let ids: Vec<&str> = gpt6.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["effort", "fast"]);
    assert_eq!(gpt6[0].current, Some(ConfigValue::from("medium")));
    let ConfigKind::Select { choices: levels } = &gpt6[0].kind else {
        panic!("effort is a select");
    };
    assert!(levels.iter().any(|c| c.value == "high"));
    // gpt-6-mini has no "high": the effort falls back to its default.
    session.configure("model", "gpt-6-mini").await.unwrap();
    loop {
        if let EventKind::SessionUpdated(info) = next(&mut events).await.kind {
            assert_eq!(text_option(&info, "effort").as_deref(), Some("low"));
            let effort = info
                .details
                .config_options
                .iter()
                .find(|o| o.id.as_str() == "effort")
                .unwrap();
            let ConfigKind::Select { choices } = &effort.kind else {
                panic!("effort is a select");
            };
            assert_eq!(
                choices.iter().map(|c| c.value.as_str()).collect::<Vec<_>>(),
                vec!["low", "medium"]
            );
            break;
        }
    }
    // Selecting gpt-6 at its default level makes the live option its nested one.
    session.configure("effort", "medium").await.unwrap();
    session.configure("model", "gpt-6").await.unwrap();
    loop {
        if let EventKind::SessionUpdated(info) = next(&mut events).await.kind
            && text_option(&info, "model").as_deref() == Some("gpt-6")
        {
            let effort = info
                .details
                .config_options
                .iter()
                .find(|o| o.id.as_str() == "effort");
            assert_eq!(effort, gpt6.first());
            break;
        }
    }
    session.close().await.unwrap();
}

/// An effort codex reports that the model's levels omit stays the option's
/// current, in agreement with the configuration.
#[tokio::test]
async fn an_unlisted_thread_effort_stays_current() {
    let (session, _events) = open("unlisted-effort", "--xhigh-effort").await;
    let info = session.info();
    let effort = info
        .details
        .config_options
        .iter()
        .find(|o| o.id.as_str() == "effort")
        .unwrap();
    assert_eq!(effort.current, Some(ConfigValue::from("xhigh")));
    assert_eq!(
        info.configuration.options.get(&ConfigId::new("effort")),
        effort.current.as_ref()
    );
    session.close().await.unwrap();
}

/// Invalid model/effort at creation validated locally before reaching wire.
#[tokio::test]
async fn creation_config_is_validated_before_the_wire_sees_it() {
    // The wire would accept the model and fail the turn later; open refuses.
    let err = open_with(
        "bad-model",
        "",
        SessionOptions::in_dir(std::env::temp_dir()).configure("model", "nope"),
    )
    .await
    .err()
    .unwrap();
    assert!(matches!(err, AgentError::InvalidConfiguration(_)), "{err}");

    let err = open_with(
        "bad-effort",
        "",
        SessionOptions::in_dir(std::env::temp_dir()).configure("effort", "ultra"),
    )
    .await
    .err()
    .unwrap();
    assert!(matches!(err, AgentError::InvalidConfiguration(_)), "{err}");
}

/// Mode/sandbox are creation-only; mid-session configure refused typed.
#[tokio::test]
async fn mode_and_sandbox_are_live_and_ride_every_turn() {
    let (session, mut events) = open_with(
        "mode",
        "",
        SessionOptions::in_dir(std::env::temp_dir())
            .configure("mode", "untrusted")
            .configure("sandbox", "workspace-write"),
    )
    .await
    .unwrap();
    let info = session.info();
    assert_eq!(text_option(&info, "mode").as_deref(), Some("untrusted"));
    assert_eq!(
        text_option(&info, "sandbox").as_deref(),
        Some("workspace-write")
    );
    session.prompt("hi").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(
        text.contains("policy=untrusted sandbox=workspaceWrite"),
        "{text}"
    );
    // Live: `turn/start` carries `approvalPolicy` and `sandboxPolicy`.
    session.configure("mode", "never").await.unwrap();
    session.configure("sandbox", "read-only").await.unwrap();
    loop {
        if let EventKind::SessionUpdated(info) = next(&mut events).await.kind
            && text_option(&info, "sandbox").as_deref() == Some("read-only")
        {
            assert_eq!(text_option(&info, "mode").as_deref(), Some("never"));
            break;
        }
    }
    session.prompt("again").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("policy=never sandbox=readOnly"), "{text}");
    session.close().await.unwrap();
}

/// Plan mode rides `turn/start` as a collaboration mode with no approval
/// policy, its `plan` item is `PlanProposed`, and leaving it sends `default` once.
#[tokio::test]
async fn plan_mode_rides_turn_start_and_proposes_the_plan_item() {
    let (session, mut events) = open_with(
        "plan",
        "",
        SessionOptions::in_dir(std::env::temp_dir()).configure("mode", "plan"),
    )
    .await
    .unwrap();
    assert_eq!(
        text_option(&session.info(), "mode").as_deref(),
        Some("plan")
    );
    let collab = |mode: &str| {
        format!(
            r#"collab={{"mode":"{mode}","settings":{{"model":"gpt-6","reasoning_effort":"medium","developer_instructions":null}}}}"#
        )
    };

    let (text, plans) = plan_turn(&session, &mut events, "plan a README").await;
    assert!(text.contains("policy=unset"), "{text}");
    assert!(text.contains(&collab("plan")), "{text}");
    assert_eq!(plans, vec!["# Plan\n\n1. Add README.md"]);
    let (_, plans) = plan_turn(&session, &mut events, "no-plan").await;
    assert!(plans.is_empty(), "an empty plan item proposed {plans:?}");

    // A policy ends plan mode: `default` goes out until codex accepts a turn.
    session.configure("mode", "never").await.unwrap();
    while text_option(&session.info(), "mode").as_deref() != Some("never") {
        next(&mut events).await;
    }
    plan_turn(&session, &mut events, "refuse-start").await;
    let (text, plans) = plan_turn(&session, &mut events, "plan a README").await;
    assert!(text.contains("policy=never"), "{text}");
    assert!(text.contains(&collab("default")), "{text}");
    assert!(plans.is_empty());
    let (text, _) = plan_turn(&session, &mut events, "plan a README").await;
    assert!(text.contains("collab=null"), "{text}");

    // Selected live, plan drops the policy again.
    session.configure("mode", "plan").await.unwrap();
    while text_option(&session.info(), "mode").as_deref() != Some("plan") {
        next(&mut events).await;
    }
    let (text, plans) = plan_turn(&session, &mut events, "plan a README").await;
    assert!(text.contains("policy=unset"), "{text}");
    assert!(text.contains(&collab("plan")), "{text}");
    assert_eq!(plans.len(), 1);
    session.close().await.unwrap();

    // A refused first plan turn still sends `default` when plan mode ends.
    let (session, mut events) = open_with(
        "plan-refused",
        "",
        SessionOptions::in_dir(std::env::temp_dir()).configure("mode", "plan"),
    )
    .await
    .unwrap();
    plan_turn(&session, &mut events, "refuse-start").await;
    session.configure("mode", "never").await.unwrap();
    while text_option(&session.info(), "mode").as_deref() != Some("never") {
        next(&mut events).await;
    }
    let (text, _) = plan_turn(&session, &mut events, "plan a README").await;
    assert!(text.contains(&collab("default")), "{text}");
    session.close().await.unwrap();
}

/// Runs one turn and returns its text and proposed plans; plan deltas must
/// not surface as diagnostics.
async fn plan_turn(session: &Session, events: &mut Events, prompt: &str) -> (String, Vec<String>) {
    session.prompt(prompt).await.unwrap();
    let (mut text, mut plans) = (String::new(), Vec::new());
    loop {
        match next(events).await.kind {
            EventKind::TextDelta { text: t, .. } => text.push_str(&t),
            EventKind::PlanProposed { markdown } => plans.push(markdown),
            EventKind::Diagnostic(d) => panic!("unexpected diagnostic: {}", d.message),
            EventKind::TurnEnded { .. } => return (text, plans),
            _ => {}
        }
    }
}

/// Approval accept completes tool with write=accept; decline marks tool Cancelled and turn continues.
#[tokio::test]
async fn approvals_map_accept_and_decline() {
    let (session, mut events) = open("approve", "").await;

    session.prompt("write-file please").await.unwrap();
    let mut text = String::new();
    let mut change_states = Vec::new();
    loop {
        match next(&mut events).await.kind {
            EventKind::TextDelta { text: t, .. } => text.push_str(&t),
            EventKind::ToolUpdated(tool) if tool.kind == ToolKind::Edit => change_states.push(tool),
            EventKind::RequestOpened(Request::Permission(request)) => {
                // The request names only the item; the tool snapshot carries
                // the diff from the preceding `item/started`.
                assert_eq!(request.tool.kind, ToolKind::Edit);
                assert_eq!(request.tool.diffs[0].new_text, "PEAR\n");
                assert_eq!(request.tool.locations, vec![PathBuf::from("fruit.txt")]);
                assert_eq!(
                    request.options,
                    vec![
                        PermissionChoice::AllowOnce,
                        PermissionChoice::AllowAlways,
                        PermissionChoice::DenyOnce,
                    ]
                );
                session
                    .answer(request.id, Answer::Permission(PermissionChoice::AllowOnce))
                    .await
                    .unwrap();
            }
            EventKind::TurnEnded { .. } => break,
            _ => {}
        }
    }
    assert!(text.contains("write=accept"), "{text}");
    assert_eq!(change_states.last().unwrap().status, ToolStatus::Completed);

    // Decline: the item ends `declined` and the turn continues.
    session.prompt("write-file again").await.unwrap();
    let mut declined = None;
    let text = loop {
        let mut text = String::new();
        match next(&mut events).await.kind {
            EventKind::TextDelta { text: t, .. } => text.push_str(&t),
            EventKind::ToolUpdated(tool) if tool.kind == ToolKind::Edit => {
                declined = Some(tool.status)
            }
            EventKind::RequestOpened(Request::Permission(request)) => {
                session
                    .answer(request.id, Answer::Permission(PermissionChoice::DenyOnce))
                    .await
                    .unwrap();
            }
            EventKind::TurnEnded { .. } => break text,
            _ => {}
        }
    };
    let _ = text;
    assert_eq!(declined, Some(ToolStatus::Cancelled));
    session.close().await.unwrap();
}

/// `Deny` declines (the wire takes no message); `Cancel` sends `cancel`, which
/// ends the turn interrupted, and a cancelled question sends no answers.
#[tokio::test]
async fn deny_declines_and_cancel_withdraws() {
    let deny = Answer::Deny {
        message: "not now".into(),
    };
    for (name, flags, prompt, answer, reply, cancelled) in [
        ("deny-why", "", "write-file", deny, "write=decline ", false),
        (
            "cancel-write",
            "",
            "write-file",
            Answer::Cancel,
            "write=cancel ",
            true,
        ),
        (
            "cancel-q",
            "--question",
            "hi",
            Answer::Cancel,
            "answer=none ",
            false,
        ),
    ] {
        let (session, mut events) = open(name, flags).await;
        session.prompt(prompt).await.unwrap();
        let mut text = String::new();
        let stop = loop {
            match next(&mut events).await.kind {
                EventKind::TextDelta { text: t, .. } => text.push_str(&t),
                EventKind::RequestOpened(request) => {
                    session.answer(request.id(), answer.clone()).await.unwrap()
                }
                EventKind::TurnEnded { stop, .. } => break stop,
                _ => {}
            }
        };
        assert!(text.contains(reply), "{name}: {text}");
        assert_eq!(stop == StopReason::Cancelled, cancelled, "{name}: {stop:?}");
        session.close().await.unwrap();
    }
}

/// An MCP tool-call elicitation asks on the tracked MCP tool: allow completes
/// it (the session form rides `persist`), deny fails it and the turn goes on.
#[tokio::test]
async fn an_mcp_tool_approval_maps_to_a_permission() {
    let (session, mut events) = open("mcp-approve", "").await;
    let mcp = ToolKind::Mcp {
        server: "probe".into(),
        tool: "secret_word".into(),
    };
    for (choice, reply, status) in [
        (
            PermissionChoice::AllowOnce,
            "mcpcall=accept ",
            ToolStatus::Completed,
        ),
        (
            PermissionChoice::AllowAlways,
            "mcpcall=accept/session ",
            ToolStatus::Completed,
        ),
        (
            PermissionChoice::DenyOnce,
            "mcpcall=decline ",
            ToolStatus::Failed,
        ),
    ] {
        let answer = Some(Answer::Permission(choice));
        let (text, requests, states, stop) =
            mcp_turn(&session, &mut events, "mcp-tool please", answer).await;
        // The request names no item; its tool is the call `item/started` opened.
        assert_eq!(requests[0].tool.id, states[0].id);
        assert_eq!(requests[0].tool.kind, mcp);
        assert_eq!(
            requests[0].options,
            vec![
                PermissionChoice::AllowOnce,
                PermissionChoice::AllowAlways,
                PermissionChoice::DenyOnce,
            ]
        );
        assert_eq!(
            requests[0].detail.as_deref(),
            Some("Allow the probe MCP server to run tool \"secret_word\"?")
        );
        assert!(text.contains(reply), "{text}");
        assert_eq!(states.last().unwrap().status, status);
        assert!(matches!(stop, StopReason::Completed { .. }), "{stop:?}");
    }
    // Only the persistent "always" form offered: no choice maps to it.
    let (_, requests, _, _) = mcp_turn(
        &session,
        &mut events,
        "mcp-always please",
        Some(Answer::Permission(PermissionChoice::AllowOnce)),
    )
    .await;
    assert_eq!(
        requests[0].options,
        vec![PermissionChoice::AllowOnce, PermissionChoice::DenyOnce]
    );
    session.close().await.unwrap();
}

/// On an MCP approval `Deny` sends action `decline` (no message) and `Cancel`
/// sends action `cancel`; both fail the call.
#[tokio::test]
async fn an_mcp_approval_takes_deny_and_cancel() {
    let (session, mut events) = open("mcp-deny-cancel", "").await;
    let deny = Answer::Deny {
        message: "not now".into(),
    };
    for (answer, reply) in [
        (deny, "mcpcall=decline "),
        (Answer::Cancel, "mcpcall=cancel "),
    ] {
        let (text, _, states, _) =
            mcp_turn(&session, &mut events, "mcp-tool please", Some(answer)).await;
        assert!(text.contains(reply), "{text}");
        assert_eq!(states.last().unwrap().status, ToolStatus::Failed);
    }
    session.close().await.unwrap();
}

/// Two in-flight calls of one tool: each approval lands on the call whose
/// arguments its `tool_params` carries.
#[tokio::test]
async fn an_mcp_approval_picks_the_call_by_its_arguments() {
    let (session, mut events) = open("mcp-two", "").await;
    let (_, requests, states, _) = mcp_turn(
        &session,
        &mut events,
        "mcp-two please",
        Some(Answer::Permission(PermissionChoice::AllowOnce)),
    )
    .await;
    let asked: Vec<_> = requests.iter().map(|r| &r.tool.id).collect();
    assert_eq!(asked, vec![&states[0].id, &states[1].id]);
    session.close().await.unwrap();
}

/// Cancel with an MCP approval open replies `{action: cancel}` and the turn ends Cancelled.
#[tokio::test]
async fn cancel_with_an_mcp_approval_open_replies_cancel() {
    let (session, mut events) = open("mcp-cancel", "").await;
    let (text, requests, _, stop) = mcp_turn(&session, &mut events, "mcp-tool please", None).await;
    assert_eq!(requests.len(), 1);
    assert!(text.contains("mcpcall=cancel "), "{text}");
    assert_eq!(stop, StopReason::Cancelled);
    session.close().await.unwrap();
}

/// MCP progress of the running call, the turn diff and a reroute each map to
/// their event; progress of an unknown item is dropped, and the reroute is no warning.
#[tokio::test]
async fn progress_diff_and_reroute_are_events() {
    let (session, mut events) = open("live-events", "").await;
    session
        .prompt("write-file mcp-tool rerouted please")
        .await
        .unwrap();
    let (mut calls, mut progress, mut diffs, mut reroutes, mut diagnostics) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut turn = None;
    loop {
        let event = next(&mut events).await;
        match event.kind {
            EventKind::TurnStarted { .. } => turn = event.turn_info,
            EventKind::ToolUpdated(tool) if matches!(tool.kind, ToolKind::Mcp { .. }) => {
                calls.push(tool.id)
            }
            EventKind::ToolProgress {
                tool_id, message, ..
            } => progress.push((tool_id, message)),
            EventKind::TurnDiff { unified } => diffs.push((unified, event.turn_info)),
            reroute @ EventKind::ModelRerouted { .. } => reroutes.push(reroute),
            EventKind::Diagnostic(d) => diagnostics.push(d.message),
            EventKind::RequestOpened(request) => session
                .answer(
                    request.id(),
                    Answer::Permission(PermissionChoice::AllowOnce),
                )
                .await
                .unwrap(),
            EventKind::TurnEnded { .. } => break,
            _ => {}
        }
    }
    assert_eq!(progress, [(calls[0].clone(), Some("halfway there".into()))]);
    assert_eq!(diffs.len(), 1);
    assert!(
        diffs[0]
            .0
            .contains("+++ b/fruit.txt\n@@ -0,0 +1 @@\n+PEAR\n")
    );
    assert!(turn.is_some());
    assert_eq!(diffs[0].1, turn, "the diff rides the prompted turn");
    let reroute = EventKind::ModelRerouted {
        from: "gpt-6".into(),
        to: "gpt-6-mini".into(),
        reason: Some("highRiskCyberActivity".into()),
    };
    assert_eq!(reroutes, [reroute]);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    session.close().await.unwrap();
}

/// A diff and a reroute that trail `turn/completed` are dropped: no agent turn
/// opens that no `turn/completed` would end, and the session stays idle.
#[tokio::test]
async fn a_late_diff_and_reroute_open_no_turn() {
    let (session, mut events) = open("late-events", "").await;
    session.prompt("late-events please").await.unwrap();
    complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    let mut late = Vec::new();
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_millis(300), events.next()).await
    {
        late.push(event.unwrap().kind);
    }
    assert!(
        late.iter()
            .all(|k| matches!(k, EventKind::StatusChanged(_))),
        "{late:?}"
    );
    assert_eq!(session.status(), anyagent::SessionStatus::Idle);
    session.close().await.unwrap();
}

/// Steer sent before turn/started is held until accepted and folded via Steered delivery.
#[tokio::test]
async fn a_steer_folds_into_the_running_turn() {
    let (session, mut events) = open("steer", "").await;
    session.prompt("hi").await.unwrap();
    // Sent before `turn/started` arrives: the adapter holds it until the
    // wire will accept it (an early `turn/steer` is refused).
    let delivery = session.prompt("extra instructions").await.unwrap();
    assert!(
        matches!(delivery.kind, DeliveryKind::Steered { .. }),
        "{delivery:?}"
    );
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("steered=extra instructions"), "{text}");
    session.close().await.unwrap();
}

/// Cancel interrupts running command, marks tool Cancelled, and ends turn as Cancelled; idle cancel is no-op.
#[tokio::test]
async fn cancel_interrupts_and_cancels_inflight_tools() {
    let (session, mut events) = open("cancel", "").await;
    session.prompt("sleep forever").await.unwrap();
    // Wait for the command to be running, then interrupt.
    loop {
        if let EventKind::ToolUpdated(tool) = next(&mut events).await.kind {
            assert_eq!(tool.status, ToolStatus::Running);
            break;
        }
    }
    session.cancel(false).await.unwrap();
    let mut cancelled_tool = false;
    loop {
        match next(&mut events).await.kind {
            // No `item/completed` comes for it; the adapter cancels it.
            EventKind::ToolUpdated(tool) => {
                assert_eq!(tool.status, ToolStatus::Cancelled);
                cancelled_tool = true;
            }
            EventKind::TurnEnded { stop, .. } => {
                assert_eq!(stop, StopReason::Cancelled);
                break;
            }
            _ => {}
        }
    }
    assert!(cancelled_tool);
    // Idle cancel is a no-op, not an error.
    session.cancel(false).await.unwrap();
    session.close().await.unwrap();
}

/// An isolated config home rides the login command's env, so the login
/// lands where the session looks.
#[tokio::test]
async fn login_methods_carry_the_config_home() {
    let home =
        std::env::temp_dir().join(format!("anyagent-codex-login-home-{}", std::process::id()));
    let (session, _events) = open_with(
        "logged-out-home",
        "--logged-out",
        SessionOptions::in_dir(std::env::temp_dir()).config_home(&home),
    )
    .await
    .unwrap();
    let AuthStatus::Unauthenticated { login } = session.info().details.auth else {
        panic!("expected Unauthenticated");
    };
    let LoginMethod::Terminal { env, .. } = &login[0] else {
        panic!("expected a terminal login method");
    };
    assert_eq!(
        env.get("CODEX_HOME").map(String::as_str),
        Some(home.to_string_lossy().as_ref())
    );
    session.close().await.unwrap();
}

/// Logged-out reported at handshake; first model call surfaces AuthRequired without retries.
#[tokio::test]
async fn logged_out_is_reported_and_the_first_turn_surfaces_auth_required() {
    let (session, mut events) = open("logged-out", "--logged-out").await;
    let AuthStatus::Unauthenticated { login } = session.info().details.auth else {
        panic!("expected Unauthenticated");
    };
    assert!(!login.is_empty());

    // The server accepts the turn; the 401 appears at the first model call
    // and surfaces as AuthRequired without waiting out the retries.
    session.prompt("hi").await.unwrap();
    let mut failed = false;
    loop {
        match tokio::time::timeout(Duration::from_secs(10), events.next())
            .await
            .expect("timed out")
            .expect("stream ended")
        {
            Ok(event) => {
                if let EventKind::TurnEnded { stop, .. } = event.kind {
                    assert!(matches!(stop, StopReason::Failed { .. }), "{stop:?}");
                    failed = true;
                }
            }
            Err(AgentError::AuthRequired { login }) => {
                assert!(!login.is_empty());
                break;
            }
            Err(other) => panic!("unexpected stream error: {other}"),
        }
    }
    assert!(failed);
}

/// Resume keeps the thread id and its turn history (a rollback can cut into
/// it); fork at fork_point creates a new id with the right anchor. The usage
/// frame each bind replays counts toward no turn.
#[tokio::test]
async fn resume_keeps_the_thread_and_fork_cuts_at_the_anchor() {
    let (session, mut events) = open("resume-src", "").await;
    session.prompt("hi").await.unwrap();
    complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    let token = session.info().resume_token.unwrap();
    session.close().await.unwrap();

    let (resumed, mut events) = open_with(
        "resume",
        "",
        SessionOptions::in_dir(std::env::temp_dir()).resume(token.clone()),
    )
    .await
    .unwrap();
    assert_eq!(resumed.info().resume_token.unwrap(), token);
    resumed
        .rollback(
            std::num::NonZeroU32::new(1).unwrap(),
            anyagent::RollbackScope::Conversation,
        )
        .await
        .unwrap();
    loop {
        if let EventKind::SessionUpdated(_) = next(&mut events).await.kind {
            break;
        }
    }
    resumed.prompt("hi").await.unwrap();
    let (text, usage) =
        complete_turn_usage(&resumed, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("rolled=1"), "{text}");
    assert_eq!(usage, Some(TURN_USAGE));
    // The thread may have been left in plan mode; its first turn says which.
    assert!(text.contains(r#"collab={"mode":"default""#), "{text}");
    resumed.close().await.unwrap();

    // Fork at a wire turn id (the `codex/fork_point` extension currency).
    let (fork, mut events) = open_with(
        "fork",
        "",
        SessionOptions::in_dir(std::env::temp_dir())
            .fork_from(token, Some(anyagent::MessageId::new("turn-0"))),
    )
    .await
    .unwrap();
    assert_eq!(fork.info().resume_token.unwrap().as_str(), "th-fork-1");
    fork.prompt("hi").await.unwrap();
    let (text, usage) = complete_turn_usage(&fork, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("fork=turn-0"), "{text}");
    assert_eq!(usage, Some(TURN_USAGE));
    assert!(text.contains(r#"collab={"mode":"default""#), "{text}");
    fork.close().await.unwrap();
}

/// Only an unknown thread is `ResumeFailed`; a malformed id stays a
/// protocol failure.
#[tokio::test]
async fn only_an_unknown_thread_fails_the_resume() {
    for (token, resume_failed) in [("th-gone", true), ("not-a-uuid", false)] {
        let options = SessionOptions::in_dir(std::env::temp_dir()).resume(token.into());
        let err = open_with("resume-gone", "", options).await.err().unwrap();
        assert_eq!(
            matches!(err, AgentError::ResumeFailed(_)),
            resume_failed,
            "{token}: {err}"
        );
    }
}

/// Plan usage probe reads Session/Week windows with resets; logged-out typed AuthRequired.
#[tokio::test]
async fn plan_usage_probe_reads_the_windows() {
    let runtime = Runtime::new();
    let agent = AgentInstallation::at("codex", wrapper("usage", ""));
    let usage = runtime.plan_usage(&agent).await.unwrap();
    assert_eq!(usage.plan.as_deref(), Some("edu"));
    assert_eq!(usage.windows.len(), 2);
    assert_eq!(usage.windows[0].label, "Session");
    assert_eq!(usage.windows[0].used_percent, 5);
    assert!(usage.windows[0].resets_at.is_some());
    assert_eq!(usage.windows[1].label, "Week");

    // Logged out, the refusal is typed as a login problem.
    let runtime = Runtime::new();
    let agent = AgentInstallation::at("codex", wrapper("usage-out", "--logged-out"));
    let err = runtime.plan_usage(&agent).await.err().unwrap();
    assert!(matches!(err, AgentError::AuthRequired { .. }), "{err}");
}

/// `plan_usage_with` launches with the options' config home, env and args,
/// and caches per login: the same options hit the cache, a change probes again.
#[tokio::test]
async fn plan_usage_with_applies_the_options_and_caches_per_login() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("argv.jsonl");
    let runtime = Runtime::new();
    let agent = AgentInstallation::at("codex", wrapper("usage-with", ""));
    let base = SessionOptions::in_dir(dir.path()).env("FIXTURE_ARGV_LOG", log.to_string_lossy());
    let home = dir.path().join("home");
    let calls = [
        base.clone().arg("--extra-flag"),
        base.clone().arg("--extra-flag"),
        base.clone().arg("--other-flag"),
        base.clone().config_home(&home),
        base.clone().env("OTHER", "1"),
    ];
    for options in calls {
        runtime.plan_usage_with(&agent, options).await.unwrap();
    }
    let argv = common::logged_args(&log);
    assert_eq!(argv.len(), 4, "only the repeat hit the cache: {argv:?}");
    assert_eq!(argv[0], ["app-server", "--extra-flag"]);
    assert!(home.is_dir(), "CODEX_HOME is created before the spawn");
}

/// requestUserInput question translates both ways even though capability not advertised.
#[tokio::test]
async fn a_question_request_translates_both_ways() {
    // `requestUserInput` fires only for clients that opt into the
    // experimental API on `initialize` (the fixture answers `noapi`
    // otherwise) and, live, only in collaboration mode — so the capability
    // stays off while the translation is exercised here.
    let (session, mut events) = open("question", "--question").await;
    session.prompt("hi").await.unwrap();
    let mut text = String::new();
    loop {
        match next(&mut events).await.kind {
            EventKind::TextDelta { text: t, .. } => text.push_str(&t),
            EventKind::RequestOpened(Request::Question(request)) => {
                assert_eq!(request.questions.len(), 1);
                let question = &request.questions[0];
                assert_eq!(question.text, "Which color?");
                assert_eq!(question.header.as_deref(), Some("Color"));
                assert_eq!(question.choices.len(), 2);
                session
                    .answer(
                        request.id,
                        Answer::Question(vec![QuestionAnswer::Choices(vec!["Red".into()])]),
                    )
                    .await
                    .unwrap();
            }
            EventKind::TurnEnded { .. } => break,
            _ => {}
        }
    }
    assert!(text.contains("answer=Red"), "{text}");
    session.close().await.unwrap();
}

/// MCP servers ride `-c mcp_servers.…` launch overrides; SSE is refused.
#[tokio::test]
async fn mcp_servers_ride_the_launch_config() {
    let (session, mut events) = open_with(
        "mcp",
        "",
        SessionOptions::in_dir(std::env::temp_dir())
            .mcp_server(McpServer::http("docs", "http://localhost:1").with("X-Key", "k"))
            .mcp_server(McpServer::stdio("tool", "/bin/tool", ["--serve"])),
    )
    .await
    .unwrap();
    assert_eq!(
        session.info().details.capabilities.mcp_transports,
        vec![anyagent::McpTransport::Stdio, anyagent::McpTransport::Http]
    );
    session.prompt("hi").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("mcp=docs,tool"), "{text}");
    session.close().await.unwrap();

    let err = open_with(
        "mcp-sse",
        "",
        SessionOptions::in_dir(std::env::temp_dir())
            .mcp_server(McpServer::sse("voice", "http://localhost:2")),
    )
    .await
    .err()
    .unwrap();
    assert!(matches!(err, AgentError::UnsupportedFeature(_)), "{err}");
}

/// MCP header and stdio env values stay out of argv: the overrides name env
/// vars, and the server process receives the values in its env.
#[tokio::test]
async fn mcp_values_ride_the_env_not_argv() {
    let dir = tempfile::tempdir().unwrap();
    let (log, received) = (dir.path().join("argv.jsonl"), dir.path().join("mcp.jsonl"));
    let options = SessionOptions::in_dir(dir.path())
        .env("FIXTURE_ARGV_LOG", log.to_string_lossy())
        .env("FIXTURE_MCP_LOG", received.to_string_lossy())
        .mcp_server(
            McpServer::http("t3-code", "http://localhost:1")
                .with("Authorization", "Bearer tok-s3cret")
                .with("X-Team", "team-s3cret"),
        )
        .mcp_server(
            McpServer::stdio("tool", "/bin/tool", ["--serve"]).with("TOOL_KEY", "key-s3cret"),
        );
    let (session, _events) = open_with("mcp-env", "", options).await.unwrap();
    let argv = common::logged_args(&log)[0].join(" ");
    assert!(!argv.contains("s3cret"), "a value in argv: {argv}");
    for named in [
        r#"mcp_servers.t3-code.bearer_token_env_var="ANYAGENT_MCP_T3_CODE_TOKEN""#,
        r#"mcp_servers.t3-code.env_http_headers={"X-Team"="ANYAGENT_MCP_T3_CODE_HEADER_0"}"#,
        r#"mcp_servers.tool.env_vars=["TOOL_KEY"]"#,
    ] {
        assert!(argv.contains(named), "{named} missing: {argv}");
    }
    let received = std::fs::read_to_string(&received).unwrap();
    let received: serde_json::Value = serde_json::from_str(received.trim()).unwrap();
    assert_eq!(
        received,
        serde_json::json!({
            "ANYAGENT_MCP_T3_CODE_TOKEN": "tok-s3cret",
            "ANYAGENT_MCP_T3_CODE_HEADER_0": "team-s3cret",
            "TOOL_KEY": "key-s3cret",
        })
    );
    session.close().await.unwrap();
}

/// A stdio env name the launch env holds with another value, or one spawn sets
/// itself (`PATH`), fails the open, naming it but not the value.
#[tokio::test]
async fn a_conflicting_stdio_env_name_is_refused() {
    let server = |name: &str, var: &str, value: &str| {
        McpServer::stdio(name, "/bin/tool", ["--serve"]).with(var, value)
    };
    let base = SessionOptions::in_dir(std::env::temp_dir());
    let same = base
        .clone()
        .env("TOOL_KEY", "v-one")
        .mcp_server(server("a", "TOOL_KEY", "v-one"));
    let (session, _events) = open_with("mcp-same", "", same).await.unwrap();
    session.close().await.unwrap();
    let path = std::env::var("PATH").unwrap();
    for (var, options) in [
        (
            "TOOL_KEY",
            base.clone()
                .env("TOOL_KEY", "v-one")
                .mcp_server(server("a", "TOOL_KEY", "v-two")),
        ),
        (
            "TOOL_KEY",
            base.clone()
                .mcp_server(server("b", "TOOL_KEY", "v-one"))
                .mcp_server(server("a", "TOOL_KEY", "v-two")),
        ),
        (
            "HOME",
            base.clone().mcp_server(server("a", "HOME", "v-two")),
        ),
        ("PATH", base.clone().mcp_server(server("a", "PATH", &path))),
    ] {
        let err = open_with("mcp-conflict", "", options).await.err().unwrap();
        let AgentError::InvalidConfiguration(message) = &err else {
            panic!("{err}");
        };
        assert!(
            message.contains(&format!("`{var}`")) && message.contains("`a`"),
            "{message}"
        );
        assert!(
            !message.contains("v-") && !message.contains(&path),
            "{message}"
        );
    }
}

/// `thread/revert {beforeTurnId}` cuts the conversation before the kept
/// turn; `SessionUpdated` confirms it, a cut deeper than the history is
/// `InvalidRequest`, and the files scope stays refused.
#[tokio::test]
async fn rollback_drops_turns_and_confirms_with_session_updated() {
    use std::num::NonZeroU32;
    let (session, mut events) = open("rollback", "").await;
    for prompt in ["one", "two"] {
        session.prompt(prompt).await.unwrap();
        complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    }
    session
        .rollback(
            NonZeroU32::new(1).unwrap(),
            anyagent::RollbackScope::Conversation,
        )
        .await
        .unwrap();
    loop {
        if let EventKind::SessionUpdated(_) = next(&mut events).await.kind {
            break;
        }
    }
    session.prompt("three").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("rolled=1"), "{text}");
    let err = session
        .rollback(
            NonZeroU32::new(9).unwrap(),
            anyagent::RollbackScope::Conversation,
        )
        .await
        .err()
        .unwrap();
    assert!(
        matches!(&err, AgentError::InvalidRequest(r) if r.starts_with("rollback(9) rejected")),
        "{err}"
    );
    let err = session
        .rollback(
            NonZeroU32::new(1).unwrap(),
            anyagent::RollbackScope::ConversationAndFiles,
        )
        .await
        .err()
        .unwrap();
    assert!(matches!(err, AgentError::UnsupportedFeature(_)), "{err}");
    session.close().await.unwrap();
}

/// Our launch flag, a revert's echo of the session's own model, and the
/// settings, revert, completed-hook and summary-part notifications stay quiet; a real
/// model change or a host-enabled feature still surfaces.
#[tokio::test]
async fn warnings_about_our_own_flag_and_revert_stay_quiet() {
    let one = std::num::NonZeroU32::new(1).unwrap();
    let scope = anyagent::RollbackScope::Conversation;
    let turn_ended = |k: &EventKind| matches!(k, EventKind::TurnEnded { .. });
    let updated = |k: &EventKind| matches!(k, EventKind::SessionUpdated(_));
    let options = SessionOptions::in_dir(std::env::temp_dir()).configure("model", "gpt-6-mini");
    let (session, mut events) = open_with("quiet", "", options).await.unwrap();
    let mut seen = Vec::new();
    for prompt in ["one", "two"] {
        session.prompt(prompt).await.unwrap();
        seen.extend(diagnostics_until(&mut events, turn_ended).await);
    }
    session.rollback(one, scope).await.unwrap();
    seen.extend(diagnostics_until(&mut events, updated).await);
    assert!(seen.is_empty(), "{seen:?}");

    session.configure("model", "gpt-6").await.unwrap();
    diagnostics_until(&mut events, updated).await;
    session.rollback(one, scope).await.unwrap();
    let seen = diagnostics_until(&mut events, updated).await;
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(seen[0].starts_with(
        "This session was recorded with model `gpt-6-mini` but is resuming with `gpt-6`."
    ));
    session.close().await.unwrap();

    let (session, mut events) = open("quiet-host", "--host-feature").await;
    session.prompt("one").await.unwrap();
    let seen = diagnostics_until(&mut events, turn_ended).await;
    assert!(
        seen.iter().any(|d| d.starts_with(
            "Under-development features enabled: current_time_reminder, default_mode_request_user_input."
        )),
        "{seen:?}"
    );
    session.close().await.unwrap();
}

/// A user hook that blocks the prompt is a warning carrying its own text; a
/// completed hook stays quiet (see the test above).
#[tokio::test]
async fn a_blocked_hook_is_a_warning() {
    let (session, mut events) = open("hook", "").await;
    session.prompt("hook-blocked").await.unwrap();
    let mut diagnostics = Vec::new();
    loop {
        match next(&mut events).await.kind {
            EventKind::Diagnostic(d) => diagnostics.push((d.level, d.message)),
            EventKind::TurnEnded { .. } => break,
            _ => {}
        }
    }
    let blocked = "hook userPromptSubmit blocked: no secrets in prompts".to_owned();
    assert_eq!(diagnostics, [(DiagnosticLevel::Warning, blocked)]);
}

/// The server's own rename lands as the session title.
#[tokio::test]
async fn a_thread_rename_updates_the_title() {
    let (session, mut events) = open("rename", "--rename").await;
    assert_eq!(session.info().title, None);
    session.prompt("hi").await.unwrap();
    complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    loop {
        if let EventKind::SessionUpdated(info) = next(&mut events).await.kind
            && info.title.is_some()
        {
            assert_eq!(info.title.as_deref(), Some("Pear talk"));
            break;
        }
    }
    session.close().await.unwrap();
}

/// Config_home creates directory and reaches child as CODEX_HOME; turn echoes cfg path.
#[tokio::test]
async fn config_home_reaches_the_child_and_is_created() {
    let home = std::env::temp_dir().join(format!("anyagent-codex-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let (session, mut events) = open_with(
        "config-home",
        "--echo-config-home",
        SessionOptions::in_dir(std::env::temp_dir()).config_home(&home),
    )
    .await
    .unwrap();
    // The adapter creates the directory: CODEX_HOME must exist at spawn.
    assert!(home.is_dir());
    session.prompt("hi").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains(&format!("cfg={}", home.display())), "{text}");
    session.close().await.unwrap();
}

/// `instructions` ride thread start, resume and fork as `developerInstructions`.
#[tokio::test]
async fn instructions_ride_every_thread_bind() {
    let dir = tempfile::tempdir().unwrap();
    let token = anyagent::ResumeToken::new("th-9");
    let binds = [
        ("thread/start", SessionOptions::in_dir(dir.path())),
        (
            "thread/resume",
            SessionOptions::in_dir(dir.path()).resume(token.clone()),
        ),
        (
            "thread/fork",
            SessionOptions::in_dir(dir.path()).fork_from(token, None),
        ),
    ];
    for (i, (method, options)) in binds.into_iter().enumerate() {
        let log = dir.path().join(format!("wire-{i}.jsonl"));
        let options = options.instructions("Be brief.").record_wire(&log);
        let (session, _events) = open_with("instructions", "", options).await.unwrap();
        let bind = common::sent_frames(&log, 1, |f| f["method"] == method).await;
        assert_eq!(
            bind[0]["params"]["developerInstructions"], "Be brief.",
            "{method}"
        );
        session.close().await.unwrap();
    }
}

/// `env` reaches the server; `arg` lands after `app-server` and anyagent's
/// own overrides.
#[tokio::test]
async fn env_and_args_reach_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("argv.jsonl");
    let options = SessionOptions::in_dir(dir.path())
        .env("FIXTURE_ARGV_LOG", log.to_string_lossy())
        .arg("--extra-flag");
    let (session, _events) = open_with("env-args", "", options).await.unwrap();
    let argv = common::logged_args(&log);
    assert_eq!(
        argv[0],
        [
            "app-server",
            "-c",
            "features.default_mode_request_user_input=true",
            "--extra-flag"
        ],
        "{argv:?}"
    );
    session.close().await.unwrap();
}

/// Every attachment rides as a path ref; images also ride as `localImage`.
#[tokio::test]
async fn attachments_ride_as_path_refs_and_images_as_local_image_items() {
    let dir = std::env::temp_dir().join(format!("anyagent-codex-att-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("report.pdf"), b"%PDF-1.7 data").unwrap();
    std::fs::write(dir.join("shot.png"), b"\x89PNG\r\n\x1a\ndata").unwrap();
    let (session, mut events) = open("attach", "").await;
    session
        .prompt(
            Input::text("look at this")
                .attach(dir.join("report.pdf"))
                .attach(dir.join("shot.png")),
        )
        .await
        .unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("ref=1"), "{text}");
    assert!(text.contains("images=1"), "{text}");
    session.close().await.unwrap();
}

/// Dead agent surfaces ProcessExited status 3 and stderr boom.
#[tokio::test]
async fn a_dead_agent_surfaces_the_exit_and_stderr() {
    let (session, mut events) = open("die", "").await;
    session.prompt("die now").await.unwrap();
    loop {
        match tokio::time::timeout(Duration::from_secs(10), events.next())
            .await
            .expect("timed out")
            .expect("stream ended")
        {
            Ok(_) => {}
            Err(AgentError::ProcessExited { status, stderr }) => {
                assert!(status.contains('3'), "{status}");
                assert!(stderr.contains("boom"), "{stderr}");
                break;
            }
            Err(other) => panic!("unexpected stream error: {other}"),
        }
    }
    drop(session);
}

/// Codex compacts through `thread/compact/start`, which runs a turn of its
/// own without the prompted turn's tokens; a refusal never starts one, so the
/// adapter ends the turn itself.
#[tokio::test]
async fn compact_reports_the_compaction_as_an_agent_turn() {
    let (session, mut events) = open("compact", "").await;
    assert!(
        session
            .info()
            .details
            .capabilities
            .supports(Capability::Compact)
    );
    session.prompt("hi").await.unwrap();
    complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    session.compact().await.unwrap();
    let mut kinds = Vec::new();
    while !matches!(kinds.last(), Some(EventKind::TurnEnded { .. })) {
        kinds.push(next(&mut events).await.kind);
    }
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, EventKind::ContextCompacted)),
        "{kinds:?}"
    );
    assert!(
        matches!(kinds.last(), Some(EventKind::TurnEnded { usage: Some(u), .. }) if u.input_tokens == 0),
        "{kinds:?}"
    );

    let (session, mut events) = open("compact-refused", "--compact-refuses").await;
    session.compact().await.unwrap();
    let mut kinds = Vec::new();
    while !matches!(kinds.last(), Some(EventKind::TurnEnded { .. })) {
        kinds.push(next(&mut events).await.kind);
    }
    assert!(
        kinds.iter().any(
            |k| matches!(k, EventKind::Diagnostic(d) if d.message.contains("nothing to compact"))
        ),
        "{kinds:?}"
    );
}

/// Fast uses the catalog's tier, changes live, and clears on unsupported models.
#[tokio::test]
async fn fast_mode_follows_the_model_catalog_and_turns() {
    let (session, mut events) = open_with(
        "fast",
        "--no-mini-fast",
        SessionOptions::in_dir(std::env::temp_dir()).configure("fast", true),
    )
    .await
    .unwrap();
    let fast = session
        .info()
        .details
        .config_options
        .into_iter()
        .find(|o| o.id.as_str() == "fast")
        .unwrap();
    assert_eq!(fast.kind, ConfigKind::Boolean);
    assert_eq!(fast.current, Some(ConfigValue::Bool(true)));
    assert!(fast.live);
    // `default` is omitted from the turn params (probed): the fixture echoes
    // it as `unset`.
    for (id, value, tier) in [
        ("fast", ConfigValue::Bool(true), "priority"),
        ("fast", ConfigValue::Bool(false), "unset"),
        ("fast", ConfigValue::Bool(true), "priority"),
        ("model", ConfigValue::from("gpt-6-mini"), "unset"),
        ("model", ConfigValue::from("gpt-6"), "unset"),
    ] {
        session.configure(id, value).await.unwrap();
        session.prompt("hi").await.unwrap();
        let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
        assert!(text.contains(&format!("tier={tier}")), "{text}");
        if id == "model" && tier == "unset" && text.contains("model=gpt-6-mini ") {
            assert!(
                !session
                    .info()
                    .details
                    .config_options
                    .iter()
                    .any(|o| o.id.as_str() == "fast")
            );
            assert!(session.configure("fast", true).await.is_err());
        }
    }
    session.close().await.unwrap();
}

/// Inherited Fast and the older catalog both retain the correct wire tier.
#[tokio::test]
async fn fast_mode_preserves_defaults_and_legacy_tiers() {
    let (session, mut events) = open_with(
        "fast-default",
        "--default-fast",
        SessionOptions::in_dir(std::env::temp_dir()),
    )
    .await
    .unwrap();
    assert_eq!(
        session
            .info()
            .configuration
            .options
            .get(&ConfigId::new("fast")),
        Some(&ConfigValue::Bool(true))
    );
    session.configure("model", "gpt-6-mini").await.unwrap();
    session.prompt("hi").await.unwrap();
    let text = complete_turn(&session, &mut events, PermissionChoice::AllowOnce).await;
    assert!(text.contains("tier=fast"), "{text}");
    session.close().await.unwrap();
}

/// Invalid Fast selections fail before a turn reaches the provider.
#[tokio::test]
async fn invalid_fast_configuration_is_rejected() {
    for (name, model, value) in [
        ("fast-type", "gpt-6", ConfigValue::from("true")),
        ("fast-unsupported", "gpt-6-mini", ConfigValue::Bool(true)),
    ] {
        let result = open_with(
            name,
            "--no-mini-fast",
            SessionOptions::in_dir(std::env::temp_dir())
                .configure("model", model)
                .configure("fast", value),
        )
        .await;
        assert!(matches!(result, Err(AgentError::InvalidConfiguration(_))));
    }
}
