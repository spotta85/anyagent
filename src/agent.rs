//! Public types for discovery, sessions, configuration, and input.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps a string as this id.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
            /// The id as text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<T: Into<String>> From<T> for $name {
            fn from(value: T) -> Self {
                Self(value.into())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}
pub(crate) use string_id;

string_id!(AgentId); // Agent catalog id, such as "claude" or "codex".
string_id!(ConfigId); // Session option id, such as "model" or "effort".
string_id!(ResumeToken); // Opaque token owned by the agent.

/// Where discovery found an executable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum InstallationSource {
    EnvOverride,
    Path,
    LoginShellPath,
    VersionManager,
    KnownLocation,
    Pinned,
}

/// One installed agent, as returned by `Runtime::discover`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentInstallation {
    pub id: AgentId,
    pub name: String,
    pub executable_path: PathBuf,
    pub source: InstallationSource,
    /// A richer runtime for this agent that is not installed (Antigravity's
    /// ACP server). `open` works without it with fewer capabilities; the
    /// record says what to install to get the rest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrade: Option<crate::runtime::MissingAgent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) acp_args: Option<Vec<String>>,
}

impl AgentInstallation {
    /// Uses an exact executable for a catalog agent.
    pub fn at(id: impl Into<AgentId>, executable: impl Into<PathBuf>) -> Self {
        let id = id.into();
        Self {
            name: id.to_string(),
            id,
            executable_path: executable.into(),
            source: InstallationSource::Pinned,
            upgrade: None,
            acp_args: None,
        }
    }

    /// Launches an ACP agent that is not yet in the catalog.
    pub fn acp(name: impl Into<String>, executable: impl Into<PathBuf>, args: Vec<String>) -> Self {
        let name = name.into();
        Self {
            id: AgentId::new(&name),
            name,
            executable_path: executable.into(),
            source: InstallationSource::Pinned,
            upgrade: None,
            acp_args: Some(args),
        }
    }
}

/// Whether, and how, an agent is logged in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum AuthStatus {
    Authenticated {
        kind: AuthKind,
        account: Option<AccountInfo>,
    },
    Unauthenticated {
        login: Vec<LoginMethod>,
    },
    Unknown,
}

/// The login kind decides which features exist (plan usage needs a subscription).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum AuthKind {
    Subscription,
    ApiKey,
    CloudProvider,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AccountInfo {
    pub email: Option<String>,
    pub plan: Option<String>,
}

/// A login method the application can show to the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum LoginMethod {
    /// Run this full argv in a terminal the user can see.
    Terminal {
        command: Vec<String>,
        env: BTreeMap<String, String>,
        description: String,
    },
    /// Set this environment variable before `open`.
    EnvVar { name: String },
}

/// Optional actions supported by an agent or session.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum Capability {
    Images,
    Resume,
    Steer,
    Permissions,
    Questions,
    Rollback,
    /// `rollback` may also restore the files the dropped turns changed.
    RollbackFiles,
    Fork,
    /// `compact` can summarize the session's context on demand.
    Compact,
    SlashCommands,
    Plan,
    Subagents,
    ContextUsage,
    PlanUsage,
}

/// What `rollback` rewinds: conversation context only, or also the files
/// the agent changed in the dropped turns (requires `RollbackFiles`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RollbackScope {
    Conversation,
    ConversationAndFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum McpTransport {
    Stdio,
    Http,
    Sse,
}

/// A client-owned MCP server the agent should connect to, forwarded at open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpServer {
    pub(crate) name: String,
    pub(crate) connection: McpConnection,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub(crate) enum McpConnection {
    Stdio {
        command: PathBuf,
        args: Vec<String>,
        env: BTreeMap<String, String>,
    },
    Http {
        url: String,
        headers: BTreeMap<String, String>,
    },
    Sse {
        url: String,
        headers: BTreeMap<String, String>,
    },
}

impl McpServer {
    /// A server the agent launches itself. `command` should be absolute.
    pub fn stdio(
        name: impl Into<String>,
        command: impl Into<PathBuf>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            connection: McpConnection::Stdio {
                command: command.into(),
                args: args.into_iter().map(Into::into).collect(),
                env: BTreeMap::new(),
            },
        }
    }

    /// A server already running over streamable HTTP.
    pub fn http(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            connection: McpConnection::Http {
                url: url.into(),
                headers: BTreeMap::new(),
            },
        }
    }

    /// A server already running over SSE.
    pub fn sse(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            connection: McpConnection::Sse {
                url: url.into(),
                headers: BTreeMap::new(),
            },
        }
    }

    /// Connection metadata: an env var for stdio, an HTTP header otherwise.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        match &mut self.connection {
            McpConnection::Stdio { env, .. } => env.insert(key.into(), value.into()),
            McpConnection::Http { headers, .. } | McpConnection::Sse { headers, .. } => {
                headers.insert(key.into(), value.into())
            }
        };
        self
    }

    /// Which transport this server's connection uses.
    pub(crate) fn transport(&self) -> McpTransport {
        match self.connection {
            McpConnection::Stdio { .. } => McpTransport::Stdio,
            McpConnection::Http { .. } => McpTransport::Http,
            McpConnection::Sse { .. } => McpTransport::Sse,
        }
    }
}

impl std::fmt::Debug for McpConnection {
    /// Env and header names only: their values may be credentials.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpConnection::Stdio { command, args, env } => f
                .debug_struct("Stdio")
                .field("command", command)
                .field("args", args)
                .field("env", &env.keys().collect::<Vec<_>>())
                .finish(),
            McpConnection::Http { url, headers } => f
                .debug_struct("Http")
                .field("url", url)
                .field("headers", &headers.keys().collect::<Vec<_>>())
                .finish(),
            McpConnection::Sse { url, headers } => f
                .debug_struct("Sse")
                .field("url", url)
                .field("headers", &headers.keys().collect::<Vec<_>>())
                .finish(),
        }
    }
}

/// Effective caller actions for one agent or session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Capabilities {
    features: BTreeSet<Capability>,
    pub mcp_transports: Vec<McpTransport>,
}

impl Capabilities {
    /// A set of features with no MCP transports.
    pub fn new(features: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            features: features.into_iter().collect(),
            mcp_transports: Vec::new(),
        }
    }

    /// Whether the agent or session offers this action.
    pub fn supports(&self, cap: Capability) -> bool {
        self.features.contains(&cap)
    }

    /// Every supported action, for listing.
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.features.iter()
    }

    /// Adds an action an adapter discovered after the handshake.
    pub(crate) fn add(&mut self, cap: Capability) {
        self.features.insert(cap);
    }
}

/// A session setting the agent advertises. Well-known ids: `model`, `effort`,
/// `mode`, `sandbox`, `fast` (boolean, lower latency with increased usage).
/// The `mode` choice `plan` is plan mode on every agent that has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConfigOption {
    pub id: ConfigId,
    pub name: String,
    pub category: Option<String>,
    pub kind: ConfigKind,
    pub current: Option<ConfigValue>,
    /// Changeable through `Session::configure`; the adapter may resume internally.
    pub live: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ConfigKind {
    Select { choices: Vec<ConfigChoice> },
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConfigChoice {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    /// For a `model` choice: the options this model offers once selected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ConfigOption>,
}

impl ConfigChoice {
    /// A choice with no options of its own.
    pub(crate) fn new(
        value: impl Into<String>,
        label: impl Into<String>,
        description: Option<String>,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description,
            options: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ConfigValue {
    Text(String),
    Bool(bool),
}

impl From<&str> for ConfigValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for ConfigValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<bool> for ConfigValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub input_hint: Option<String>,
    /// Where the command comes from; absent on the wire means `Builtin`.
    #[serde(default, skip_serializing_if = "CommandSource::is_builtin")]
    pub source: CommandSource,
}

/// Where a slash command comes from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum CommandSource {
    /// Not reported as a skill by the agent.
    #[default]
    Builtin,
    /// A skill on disk. `scope` is the agent's own word for where it lives ("user", "repo").
    Skill {
        path: Option<PathBuf>,
        scope: Option<String>,
    },
}

impl CommandSource {
    /// `Builtin`, the default the wire leaves out.
    fn is_builtin(&self) -> bool {
        *self == Self::Builtin
    }
}

/// What `probe` and `open` learn about an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AgentDetails {
    pub version: Option<String>,
    pub auth: AuthStatus,
    pub capabilities: Capabilities,
    pub config_options: Vec<ConfigOption>,
    pub commands: Vec<SlashCommand>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SessionConfiguration {
    pub options: BTreeMap<ConfigId, ConfigValue>,
}

/// How anyagent handles tool permission requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum PermissionMode {
    /// Forward each request to the application.
    Ask,
    /// Allow file edits once without asking; forward every other request.
    AcceptEdits,
    /// Allow each permission request once without forwarding it, except a
    /// proposed plan's approval.
    AutoApprove,
}

/// Creation-time settings for `Runtime::open`.
#[derive(Debug, Clone)]
pub struct SessionOptions {
    pub(crate) cwd: PathBuf,
    pub(crate) start: SessionStart,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) no_tools: bool,
    /// Never persisted: keeps probes and one-shot generation out of the
    /// user's session history.
    pub(crate) throwaway: bool,
    /// Only the details are wanted (the probes): an adapter may stop before
    /// creating the provider session.
    pub(crate) details_only: bool,
    pub(crate) quiet_window: Option<Duration>,
    pub(crate) stall_after: Option<Duration>,
    pub(crate) mcp_servers: Vec<McpServer>,
    pub(crate) configure: Vec<(ConfigId, ConfigValue)>,
    pub(crate) config_home: Option<PathBuf>,
    pub(crate) record_wire: Option<PathBuf>,
    pub(crate) instructions: Option<String>,
    pub(crate) env: EnvVars,
    pub(crate) args: Vec<String>,
}

/// The caller's env pairs for the agent; `Debug` shows the names, never the values.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct EnvVars(pub(crate) BTreeMap<String, String>);

impl std::fmt::Debug for EnvVars {
    /// The variable names only: values may be credentials.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.0.keys()).finish()
    }
}

/// How `open` binds to a provider session.
#[derive(Debug, Clone)]
pub(crate) enum SessionStart {
    New,
    Resume(ResumeToken),
    Fork {
        from: ResumeToken,
        at: Option<crate::event::MessageId>,
    },
}
impl SessionOptions {
    /// Working directory the agent runs in. Required. A relative path is made
    /// absolute (lexically, no disk access): some agents reject a relative cwd.
    pub fn in_dir(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        Self {
            cwd: std::path::absolute(&cwd).unwrap_or(cwd),
            start: SessionStart::New,
            permission_mode: PermissionMode::Ask,
            no_tools: false,
            throwaway: false,
            details_only: false,
            quiet_window: None,
            stall_after: None,
            mcp_servers: Vec::new(),
            configure: Vec::new(),
            config_home: None,
            record_wire: None,
            instructions: None,
            env: EnvVars::default(),
            args: Vec::new(),
        }
    }

    /// Set an advertised config option at creation — the path for options
    /// with `live: false` (e.g. a non-live `model`). `open` fails with
    /// `InvalidConfiguration` when the agent refuses the value.
    pub fn configure(mut self, id: impl Into<ConfigId>, value: impl Into<ConfigValue>) -> Self {
        self.configure.push((id.into(), value.into()));
        self
    }

    /// Resume a provider session instead of creating a new one.
    pub fn resume(mut self, token: ResumeToken) -> Self {
        self.start = SessionStart::Resume(token);
        self
    }

    /// Branch a provider session into a new one that starts with its
    /// conversation; the original stays untouched. `None` forks at the tip.
    /// `at` cuts the copied history after that message, in the id currency
    /// the adapter documents (claude: the `claude/fork_point` extension on
    /// `MessageEnded`). Requires `Capability::Fork`; `open` fails typed
    /// without it.
    pub fn fork_from(mut self, token: ResumeToken, at: Option<crate::event::MessageId>) -> Self {
        self.start = SessionStart::Fork { from: token, at };
        self
    }

    /// Forward a client-owned MCP server. `open` fails with
    /// `UnsupportedFeature` when the agent lacks the server's transport.
    pub fn mcp_server(mut self, server: McpServer) -> Self {
        self.mcp_servers.push(server);
        self
    }

    /// How tool permission requests are handled; `Ask` by default.
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Point the agent at a separate config directory so one machine can hold
    /// several logins side by side (claude: `CLAUDE_CONFIG_DIR`, codex:
    /// `CODEX_HOME`). This is the supported way to keep logins apart;
    /// credential copying stays the application's job. `open` fails with
    /// `InvalidConfiguration` on an agent that has no known config-home
    /// variable, rather than silently ignoring the request.
    pub fn config_home(mut self, dir: impl Into<PathBuf>) -> Self {
        self.config_home = Some(dir.into());
        self
    }

    /// Tee every raw protocol frame, both directions, to `path` for bug
    /// reports: one JSON object per line, `{"dir":"in"|"out","frame":<frame>}`,
    /// append-only and flushed per line. A recording failure never fails a
    /// turn. Unset (the default) records nothing.
    ///
    /// The file is unredacted — prompts, file contents, command output, and
    /// possibly secrets — except the header and env values of declared MCP
    /// servers, which read `<redacted>`. Treat it as sensitive and delete it after use.
    pub fn record_wire(mut self, path: impl Into<PathBuf>) -> Self {
        self.record_wire = Some(path.into());
        self
    }

    /// Extra instructions for the agent, added to its system prompt.
    pub fn instructions(mut self, text: impl Into<String>) -> Self {
        self.instructions = Some(text.into());
        self
    }

    /// One environment variable for the agent process, over the inherited ones.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.0.insert(key.into(), value.into());
        self
    }

    /// One extra launch argument, placed after anyagent's own.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Overrides the quiet window used to infer turn end on agents whose
    /// wire does not end turns deterministically. Diagnostics only.
    pub fn quiet_window(mut self, window: Duration) -> Self {
        self.quiet_window = Some(window);
        self
    }

    /// How long the agent may stay silent mid-turn before a stall warning
    /// `Diagnostic` (default 120 s). It warns once per silence; never ends
    /// the turn.
    pub fn stall_after(mut self, after: Duration) -> Self {
        self.stall_after = Some(after);
        self
    }

    /// The working directory the agent runs in.
    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }
}

/// One prompt: text plus optional attachments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Input {
    pub(crate) text: String,
    pub(crate) attachments: Vec<PathBuf>,
}

impl Input {
    /// A text-only prompt.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            attachments: Vec::new(),
        }
    }

    /// Attach a file by path. Every attachment is referenced by path in the
    /// prompt text so the agent can open it; images under the inline cap are
    /// additionally sent as image blocks when the wire supports them.
    pub fn attach(mut self, path: impl Into<PathBuf>) -> Self {
        self.attachments.push(path.into());
        self
    }

    /// The prompt text without attachments.
    pub fn as_text(&self) -> &str {
        &self.text
    }
}

impl From<&str> for Input {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

impl From<String> for Input {
    fn from(text: String) -> Self {
        Self::text(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SessionOptions::in_dir absolutizes relative cwd so agents rejecting relative paths work.
    #[test]
    fn in_dir_makes_cwd_absolute() {
        // Agents such as grok reject a relative cwd at `session/new`.
        assert_eq!(
            SessionOptions::in_dir(".").cwd(),
            &std::env::current_dir().unwrap()
        );
        let abs = std::env::temp_dir();
        assert_eq!(SessionOptions::in_dir(&abs).cwd(), &abs);
    }

    /// `Debug` of the options names each env variable but never prints its value.
    #[test]
    fn debug_hides_env_values() {
        let options = SessionOptions::in_dir(".").env("API_KEY", "sk-secret");
        let text = format!("{options:?}");
        assert!(text.contains(r#"env: {"API_KEY"}"#), "{text}");
        assert!(!text.contains("sk-secret"), "{text}");
    }

    /// `Debug` of an MCP server names its env vars and headers, never their values.
    #[test]
    fn mcp_debug_hides_env_and_header_values() {
        let stdio = McpServer::stdio("db", "/bin/db", ["--ro"]).with("DB_TOKEN", "sk-stdio");
        let http = McpServer::http("web", "https://x.test").with("Authorization", "sk-http");
        let sse = McpServer::sse("feed", "https://y.test").with("X-Key", "sk-sse");
        let text = format!("{stdio:?} {http:?} {sse:?}");
        for name in ["DB_TOKEN", "Authorization", "X-Key"] {
            assert!(text.contains(name), "{text}");
        }
        assert!(!text.contains("sk-"), "{text}");
    }

    /// A command stored before `source` existed loads as `Builtin`.
    #[test]
    fn a_command_stored_without_source_loads_as_builtin() {
        let stored =
            serde_json::json!({ "name": "compact", "description": "", "input_hint": null });
        let command: SlashCommand = serde_json::from_value(stored).unwrap();
        assert_eq!(command.source, CommandSource::Builtin);
    }
}
