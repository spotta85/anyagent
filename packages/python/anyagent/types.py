# Generated from packages/schema.json by `just types`. Do not edit.

from __future__ import annotations

from typing import Any, Literal, NotRequired, TypeAlias, TypedDict


class Frame1(TypedDict):
    id: int
    cmd: Literal['discover']


class Frame6(TypedDict):
    id: int
    session: str
    text: str
    attachments: NotRequired[list[str]]
    cmd: Literal['prompt']


class Frame7(TypedDict):
    id: int
    session: str
    prompt: str
    cmd: Literal['dequeue']


class Frame11(TypedDict):
    id: int
    session: str
    cmd: Literal['compact']


class Frame12(TypedDict):
    id: int
    session: str
    clear_queue: NotRequired[bool]
    cmd: Literal['cancel']


class Frame13(TypedDict):
    id: int
    session: str
    cmd: Literal['info']


class Frame14(TypedDict):
    id: int
    session: str
    cmd: Literal['close']


class AcpSpec(TypedDict):
    name: str
    path: str
    args: NotRequired[list[str]]


PermissionMode: TypeAlias = Literal['Ask', 'AutoApprove']


class Stdio(TypedDict):
    command: str
    args: list[str]
    env: dict[str, str]


class McpConnection1(TypedDict):
    Stdio: Stdio


class Http(TypedDict):
    url: str
    headers: dict[str, str]


class McpConnection2(TypedDict):
    Http: Http


class Sse(TypedDict):
    url: str
    headers: dict[str, str]


class McpConnection3(TypedDict):
    Sse: Sse


McpConnection: TypeAlias = McpConnection1 | McpConnection2 | McpConnection3


ConfigValue: TypeAlias = str | bool


PermissionChoice: TypeAlias = Literal['AllowOnce', 'AllowAlways', 'DenyOnce', 'DenyAlways']


class QuestionAnswer1(TypedDict):
    Choices: list[str]


class QuestionAnswer2(TypedDict):
    Text: str


QuestionAnswer: TypeAlias = QuestionAnswer1 | QuestionAnswer2


RollbackScope: TypeAlias = Literal['Conversation', 'ConversationAndFiles']


class Line2(TypedDict):
    id: int
    ok: Any


class Line3(TypedDict):
    id: NotRequired[int | None]
    error: Any


class Line5(TypedDict):
    session: str
    error: Any


class Line6(TypedDict):
    closed: str


class Hello(TypedDict):
    protocol: int
    anyagent: str


class SystemTime(TypedDict):
    secs_since_epoch: int
    nanos_since_epoch: int


class TurnContext(TypedDict):
    id: str
    parent_tool_id: NotRequired[str | None]


class TextDelta(TypedDict):
    message_id: str
    text: str


class EventKind2(TypedDict):
    TextDelta: TextDelta


class ReasoningDelta(TypedDict):
    message_id: str
    text: str


class EventKind3(TypedDict):
    ReasoningDelta: ReasoningDelta


class UserMessage(TypedDict):
    message_id: str
    text: str


class EventKind4(TypedDict):
    UserMessage: UserMessage


class MessageEnded(TypedDict):
    message_id: str


class EventKind5(TypedDict):
    MessageEnded: MessageEnded


class ToolOutputDelta(TypedDict):
    tool_id: str
    text: str


class EventKind7(TypedDict):
    ToolOutputDelta: ToolOutputDelta


class RequestClosed(TypedDict):
    request_id: str


class EventKind10(TypedDict):
    RequestClosed: RequestClosed


class ContextUsage(TypedDict):
    used_tokens: int
    window_tokens: NotRequired[int | None]
    cost_usd: NotRequired[float | None]


class EventKind13(TypedDict):
    ContextUsage: ContextUsage


class TurnOrigin1(TypedDict):
    Prompt: str


TurnOrigin: TypeAlias = Literal['Agent'] | TurnOrigin1


class Mcp(TypedDict):
    server: str
    tool: str


class ToolKind1(TypedDict):
    Mcp: Mcp


ToolKind: TypeAlias = Literal['Read', 'Edit', 'Delete', 'Move', 'Search', 'Execute', 'Fetch', 'Think', 'Other'] | ToolKind1 | Literal['Subagent']


ToolStatus: TypeAlias = Literal['Pending', 'Running', 'Completed', 'Failed', 'Cancelled']


class ToolInput1(TypedDict):
    Path: str


class Command(TypedDict):
    command: str
    cwd: NotRequired[str | None]


class ToolInput2(TypedDict):
    Command: Command


class ToolInput3(TypedDict):
    Pattern: str


class ToolInput4(TypedDict):
    Url: str


class ToolInput5(TypedDict):
    Query: str


class ToolInput6(TypedDict):
    Text: str


ToolInput: TypeAlias = Literal['None'] | ToolInput1 | ToolInput2 | ToolInput3 | ToolInput4 | ToolInput5 | ToolInput6


class FileDiff(TypedDict):
    path: str
    old_text: NotRequired[str | None]
    new_text: str


class RawTool(TypedDict):
    name: str
    input: Any


PlanStatus: TypeAlias = Literal['Pending', 'InProgress', 'Completed']


class Choice(TypedDict):
    id: str
    label: str
    description: NotRequired[str | None]


InstallationSource: TypeAlias = Literal['EnvOverride', 'Path', 'LoginShellPath', 'VersionManager', 'KnownLocation', 'Pinned']


class MissingAgent(TypedDict):
    id: str
    name: str
    searched: list[str]
    install_hint: str


class AuthKind1(TypedDict):
    Other: str


AuthKind: TypeAlias = Literal['Subscription', 'ApiKey', 'CloudProvider'] | AuthKind1


class AccountInfo(TypedDict):
    email: NotRequired[str | None]
    plan: NotRequired[str | None]


class Terminal(TypedDict):
    command: list[str]
    env: dict[str, str]
    description: str


class LoginMethod1(TypedDict):
    Terminal: Terminal


class EnvVar(TypedDict):
    name: str


class LoginMethod2(TypedDict):
    EnvVar: EnvVar


LoginMethod: TypeAlias = LoginMethod1 | LoginMethod2


Capability: TypeAlias = Literal['Images', 'Resume', 'Steer', 'Permissions', 'Questions', 'Rollback', 'Fork', 'SlashCommands', 'Plan', 'Subagents', 'ContextUsage', 'PlanUsage'] | Literal['RollbackFiles'] | Literal['Compact']


McpTransport: TypeAlias = Literal['Stdio', 'Http', 'Sse']


class ConfigChoice(TypedDict):
    value: str
    label: str
    description: NotRequired[str | None]


class SlashCommand(TypedDict):
    name: str
    description: str
    input_hint: NotRequired[str | None]


class SessionConfiguration(TypedDict):
    options: dict[str, ConfigValue]


SessionStatus: TypeAlias = Literal['Idle', 'Working', 'NeedsInput']


class UsageWindow(TypedDict):
    label: str
    used_percent: int
    resets_at: NotRequired[SystemTime | None]


DiagnosticLevel: TypeAlias = Literal['Info', 'Warning', 'Error']


class Failed(TypedDict):
    message: str


class StopReason2(TypedDict):
    Failed: Failed


CompletionSource: TypeAlias = Literal['Protocol', 'Inferred']


class ErrorBody(TypedDict):
    kind: str
    message: str


class Started(TypedDict):
    turn_id: str


class DeliveryKind1(TypedDict):
    Started: Started


class Steered(TypedDict):
    turn_id: str


class DeliveryKind2(TypedDict):
    Steered: Steered


class Queued(TypedDict):
    position: int


class DeliveryKind3(TypedDict):
    Queued: Queued


DeliveryKind: TypeAlias = DeliveryKind1 | DeliveryKind2 | DeliveryKind3


class Frame9(TypedDict):
    id: int
    session: str
    option: str
    value: ConfigValue
    cmd: Literal['configure']


class Frame10(TypedDict):
    id: int
    session: str
    turns: int
    scope: RollbackScope
    cmd: Literal['rollback']


class AgentRef1(TypedDict):
    acp: AcpSpec


AgentRef: TypeAlias = str | AgentRef1


class McpServer(TypedDict):
    name: str
    connection: McpConnection


class Answer1(TypedDict):
    Permission: PermissionChoice


class Answer2(TypedDict):
    Question: list[QuestionAnswer]


Answer: TypeAlias = Answer1 | Answer2


class Line1(TypedDict):
    hello: Hello


class TurnStarted(TypedDict):
    origin: TurnOrigin


class EventKind1(TypedDict):
    TurnStarted: TurnStarted


class EventKind12(TypedDict):
    StatusChanged: SessionStatus


class ToolUpdate(TypedDict):
    id: str
    kind: ToolKind
    title: str
    status: ToolStatus
    input: ToolInput
    output: NotRequired[str | None]
    diffs: list[FileDiff]
    locations: list[str]
    raw: NotRequired[RawTool | None]


class PlanEntry(TypedDict):
    text: str
    status: PlanStatus


class PermissionRequest(TypedDict):
    id: str
    tool: ToolUpdate
    options: list[PermissionChoice]
    detail: NotRequired[str | None]


class Question(TypedDict):
    id: str
    text: str
    header: NotRequired[str | None]
    choices: list[Choice]
    multi_select: bool
    allows_free_text: bool


class AgentInstallation(TypedDict):
    id: str
    name: str
    executable_path: str
    source: InstallationSource
    upgrade: NotRequired[MissingAgent | None]
    acp_args: NotRequired[list[str] | None]


class Authenticated(TypedDict):
    kind: AuthKind
    account: NotRequired[AccountInfo | None]


class AuthStatus1(TypedDict):
    Authenticated: Authenticated


class Unauthenticated(TypedDict):
    login: list[LoginMethod]


class AuthStatus2(TypedDict):
    Unauthenticated: Unauthenticated


AuthStatus: TypeAlias = Literal['Unknown'] | AuthStatus1 | AuthStatus2


class Capabilities(TypedDict):
    features: list[Capability]
    mcp_transports: list[McpTransport]


class Select(TypedDict):
    choices: list[ConfigChoice]


class ConfigKind1(TypedDict):
    Select: Select


ConfigKind: TypeAlias = Literal['Boolean'] | ConfigKind1


class PlanUsage(TypedDict):
    plan: NotRequired[str | None]
    windows: list[UsageWindow]
    fetched_at: SystemTime


class Diagnostic(TypedDict):
    level: DiagnosticLevel
    message: str


class Completed(TypedDict):
    source: CompletionSource


class StopReason1(TypedDict):
    Completed: Completed


StopReason: TypeAlias = Literal['Cancelled', 'Refused'] | StopReason1 | StopReason2


class DiscoveryReport(TypedDict):
    agents: list[AgentInstallation]
    missing: list[MissingAgent]
    diagnostics: list[Diagnostic]


class Delivery(TypedDict):
    prompt_id: str
    kind: DeliveryKind


class Frame2(TypedDict):
    id: int
    agent: AgentRef
    cmd: Literal['probe']


class Frame3(TypedDict):
    id: int
    agent: AgentRef
    cmd: Literal['plan_usage']


class Frame4(TypedDict):
    id: int
    agent: AgentRef
    dir: str
    prompt: str
    resume: NotRequired[str | None]
    fork: NotRequired[str | None]
    fork_at: NotRequired[str | None]
    permission_mode: NotRequired[PermissionMode | None]
    mcp_servers: NotRequired[list[McpServer]]
    configure: NotRequired[dict[str, ConfigValue]]
    cmd: Literal['generate']


class Frame5(TypedDict):
    id: int
    agent: AgentRef
    dir: str
    resume: NotRequired[str | None]
    fork: NotRequired[str | None]
    fork_at: NotRequired[str | None]
    permission_mode: NotRequired[PermissionMode | None]
    mcp_servers: NotRequired[list[McpServer]]
    configure: NotRequired[dict[str, ConfigValue]]
    cmd: Literal['open']


class Frame8(TypedDict):
    id: int
    session: str
    request: str
    answer: Answer
    cmd: Literal['answer']


Frame: TypeAlias = Frame1 | Frame2 | Frame3 | Frame4 | Frame5 | Frame6 | Frame7 | Frame8 | Frame9 | Frame10 | Frame11 | Frame12 | Frame13 | Frame14


class EventKind6(TypedDict):
    ToolUpdated: ToolUpdate


class PlanUpdated(TypedDict):
    entries: list[PlanEntry]


class EventKind8(TypedDict):
    PlanUpdated: PlanUpdated


class EventKind14(TypedDict):
    PlanUsageUpdated: PlanUsage


class EventKind15(TypedDict):
    Diagnostic: Diagnostic


class TurnEnded(TypedDict):
    stop: StopReason
    background: list[str]


class EventKind16(TypedDict):
    TurnEnded: TurnEnded


class Request1(TypedDict):
    Permission: PermissionRequest


class QuestionRequest(TypedDict):
    id: str
    questions: list[Question]


class ConfigOption(TypedDict):
    id: str
    name: str
    category: NotRequired[str | None]
    kind: ConfigKind
    current: NotRequired[ConfigValue | None]
    live: bool


class Request2(TypedDict):
    Question: QuestionRequest


Request: TypeAlias = Request1 | Request2


class AgentDetails(TypedDict):
    version: NotRequired[str | None]
    auth: AuthStatus
    capabilities: Capabilities
    config_options: list[ConfigOption]
    commands: list[SlashCommand]


class EventKind9(TypedDict):
    RequestOpened: Request


class SessionInfo(TypedDict):
    id: str
    agent: AgentInstallation
    details: AgentDetails
    configuration: SessionConfiguration
    resume_token: NotRequired[str | None]
    title: NotRequired[str | None]
    status: NotRequired[SessionStatus]


class EventKind11(TypedDict):
    SessionUpdated: SessionInfo


EventKind: TypeAlias = EventKind1 | EventKind2 | EventKind3 | EventKind4 | EventKind5 | EventKind6 | EventKind7 | EventKind8 | EventKind9 | EventKind10 | EventKind11 | EventKind12 | EventKind13 | Literal['ContextCompacted'] | EventKind14 | EventKind15 | EventKind16


class Event(TypedDict):
    sequence: int
    occurred_at: NotRequired[SystemTime]
    session_id: str
    turn_info: NotRequired[TurnContext | None]
    kind: EventKind
    extensions: dict[str, Any]


class Line4(TypedDict):
    event: Event


Line: TypeAlias = Line1 | Line2 | Line3 | Line4 | Line5 | Line6


class Protocol(TypedDict):
    command: Frame
    line: Line
    event: Event
    error: ErrorBody
    discovery: DiscoveryReport
    details: AgentDetails
    plan_usage: PlanUsage
    session_info: SessionInfo
    delivery: Delivery
