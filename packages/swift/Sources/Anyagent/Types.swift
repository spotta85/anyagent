// Generated from packages/schema.json by `just types`. Do not edit.

import Foundation

/// A catalog id like `"claude"`, or an ACP agent the catalog does not know.
public enum AgentRef: Codable, Sendable, Equatable {
    case string(String)
    case acp(AcpSpec)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "string", …
    public var name: String {
        switch self {
        case .string: "string"
        case .acp: "acp"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .string(s); return }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "acp": self = .acp(try c.decode(AcpSpec.self, forKey: Key("acp")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .string(let v): try encoder.raw(v)
        case .acp(let v): try encoder.tagged("acp", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

extension AgentRef: ExpressibleByStringLiteral {
    public init(stringLiteral v: String) { self = .string(v) }
}

public struct AcpSpec: Codable, Sendable, Equatable {
    public var name: String
    public var path: String
    public var args: [String]?

    public init(name: String, path: String, args: [String]? = nil) {
        self.name = name
        self.path = path
        self.args = args
    }
}

/// How anyagent handles tool permission requests.
public enum PermissionMode: String, Codable, Sendable, Equatable {
    case ask = "Ask"
    case autoApprove = "AutoApprove"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

/// A client-owned MCP server the agent should connect to, forwarded at open.
public struct McpServer: Codable, Sendable, Equatable {
    public var name: String
    public var connection: McpConnection

    public init(name: String, connection: McpConnection) {
        self.name = name
        self.connection = connection
    }
}

public struct Stdio: Codable, Sendable, Equatable {
    public var command: String
    public var args: [String]
    public var env: [String: String]

    public init(command: String, args: [String], env: [String: String]) {
        self.command = command
        self.args = args
        self.env = env
    }
}

public struct Http: Codable, Sendable, Equatable {
    public var url: String
    public var headers: [String: String]

    public init(url: String, headers: [String: String]) {
        self.url = url
        self.headers = headers
    }
}

public struct Sse: Codable, Sendable, Equatable {
    public var url: String
    public var headers: [String: String]

    public init(url: String, headers: [String: String]) {
        self.url = url
        self.headers = headers
    }
}

public enum McpConnection: Codable, Sendable, Equatable {
    case stdio(Stdio)
    case http(Http)
    case sse(Sse)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Stdio", …
    public var name: String {
        switch self {
        case .stdio: "Stdio"
        case .http: "Http"
        case .sse: "Sse"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .unrecognized(s); return }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Stdio": self = .stdio(try c.decode(Stdio.self, forKey: Key("Stdio")))
        case "Http": self = .http(try c.decode(Http.self, forKey: Key("Http")))
        case "Sse": self = .sse(try c.decode(Sse.self, forKey: Key("Sse")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .stdio(let v): try encoder.tagged("Stdio", v)
        case .http(let v): try encoder.tagged("Http", v)
        case .sse(let v): try encoder.tagged("Sse", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public enum ConfigValue: Codable, Sendable, Equatable {
    case string(String)
    case bool(Bool)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "string", …
    public var name: String {
        switch self {
        case .string: "string"
        case .bool: "bool"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .string(s); return }
        if let v = try? Bool(from: decoder) { self = .bool(v); return }
        self = .unrecognized("?")
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .string(let v): try encoder.raw(v)
        case .bool(let v): try encoder.raw(v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

extension ConfigValue: ExpressibleByStringLiteral {
    public init(stringLiteral v: String) { self = .string(v) }
}

extension ConfigValue: ExpressibleByBooleanLiteral {
    public init(booleanLiteral v: Bool) { self = .bool(v) }
}

public enum Answer: Codable, Sendable, Equatable {
    case permission(PermissionChoice)
    case question([QuestionAnswer])
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Permission", …
    public var name: String {
        switch self {
        case .permission: "Permission"
        case .question: "Question"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .unrecognized(s); return }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Permission": self = .permission(try c.decode(PermissionChoice.self, forKey: Key("Permission")))
        case "Question": self = .question(try c.decode([QuestionAnswer].self, forKey: Key("Question")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .permission(let v): try encoder.tagged("Permission", v)
        case .question(let v): try encoder.tagged("Question", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public enum PermissionChoice: String, Codable, Sendable, Equatable {
    case allowOnce = "AllowOnce"
    case allowAlways = "AllowAlways"
    case denyOnce = "DenyOnce"
    case denyAlways = "DenyAlways"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

public enum QuestionAnswer: Codable, Sendable, Equatable {
    case choices([String])
    case text(String)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Choices", …
    public var name: String {
        switch self {
        case .choices: "Choices"
        case .text: "Text"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .unrecognized(s); return }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Choices": self = .choices(try c.decode([String].self, forKey: Key("Choices")))
        case "Text": self = .text(try c.decode(String.self, forKey: Key("Text")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .choices(let v): try encoder.tagged("Choices", v)
        case .text(let v): try encoder.tagged("Text", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

/// What `rollback` rewinds: conversation context only, or also the files
public enum RollbackScope: String, Codable, Sendable, Equatable {
    case conversation = "Conversation"
    case conversationAndFiles = "ConversationAndFiles"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

/// One normalized event produced by anyagent.
public struct Event: Codable, Sendable, Equatable {
    public var sequence: UInt64
    public var occurredAt: SystemTime?
    public var sessionId: String
    public var turnInfo: TurnContext?
    public var kind: EventKind
    public var extensions: [String: JSONValue]

    public init(sequence: UInt64, occurredAt: SystemTime? = nil, sessionId: String, turnInfo: TurnContext? = nil, kind: EventKind, extensions: [String: JSONValue]) {
        self.sequence = sequence
        self.occurredAt = occurredAt
        self.sessionId = sessionId
        self.turnInfo = turnInfo
        self.kind = kind
        self.extensions = extensions
    }

    enum CodingKeys: String, CodingKey {
        case sequence
        case occurredAt = "occurred_at"
        case sessionId = "session_id"
        case turnInfo = "turn_info"
        case kind
        case extensions
    }
}

public struct SystemTime: Codable, Sendable, Equatable {
    public var secsSinceEpoch: UInt64
    public var nanosSinceEpoch: UInt32

    public init(secsSinceEpoch: UInt64, nanosSinceEpoch: UInt32) {
        self.secsSinceEpoch = secsSinceEpoch
        self.nanosSinceEpoch = nanosSinceEpoch
    }

    enum CodingKeys: String, CodingKey {
        case secsSinceEpoch = "secs_since_epoch"
        case nanosSinceEpoch = "nanos_since_epoch"
    }
}

public struct TurnContext: Codable, Sendable, Equatable {
    public var id: String
    public var parentToolId: String?

    public init(id: String, parentToolId: String? = nil) {
        self.id = id
        self.parentToolId = parentToolId
    }

    enum CodingKeys: String, CodingKey {
        case id
        case parentToolId = "parent_tool_id"
    }
}

public struct TurnStarted: Codable, Sendable, Equatable {
    public var origin: TurnOrigin

    public init(origin: TurnOrigin) {
        self.origin = origin
    }
}

public struct TextDelta: Codable, Sendable, Equatable {
    public var messageId: String
    public var text: String

    public init(messageId: String, text: String) {
        self.messageId = messageId
        self.text = text
    }

    enum CodingKeys: String, CodingKey {
        case messageId = "message_id"
        case text
    }
}

public struct ReasoningDelta: Codable, Sendable, Equatable {
    public var messageId: String
    public var text: String

    public init(messageId: String, text: String) {
        self.messageId = messageId
        self.text = text
    }

    enum CodingKeys: String, CodingKey {
        case messageId = "message_id"
        case text
    }
}

public struct UserMessage: Codable, Sendable, Equatable {
    public var messageId: String
    public var text: String

    public init(messageId: String, text: String) {
        self.messageId = messageId
        self.text = text
    }

    enum CodingKeys: String, CodingKey {
        case messageId = "message_id"
        case text
    }
}

public struct MessageEnded: Codable, Sendable, Equatable {
    public var messageId: String

    public init(messageId: String) {
        self.messageId = messageId
    }

    enum CodingKeys: String, CodingKey {
        case messageId = "message_id"
    }
}

public struct ToolOutputDelta: Codable, Sendable, Equatable {
    public var toolId: String
    public var text: String

    public init(toolId: String, text: String) {
        self.toolId = toolId
        self.text = text
    }

    enum CodingKeys: String, CodingKey {
        case toolId = "tool_id"
        case text
    }
}

public struct PlanUpdated: Codable, Sendable, Equatable {
    public var entries: [PlanEntry]

    public init(entries: [PlanEntry]) {
        self.entries = entries
    }
}

public struct RequestClosed: Codable, Sendable, Equatable {
    public var requestId: String

    public init(requestId: String) {
        self.requestId = requestId
    }

    enum CodingKeys: String, CodingKey {
        case requestId = "request_id"
    }
}

public struct ContextUsage: Codable, Sendable, Equatable {
    public var usedTokens: UInt64
    public var windowTokens: UInt64?
    public var costUsd: Double?

    public init(usedTokens: UInt64, windowTokens: UInt64? = nil, costUsd: Double? = nil) {
        self.usedTokens = usedTokens
        self.windowTokens = windowTokens
        self.costUsd = costUsd
    }

    enum CodingKeys: String, CodingKey {
        case usedTokens = "used_tokens"
        case windowTokens = "window_tokens"
        case costUsd = "cost_usd"
    }
}

public struct TurnEnded: Codable, Sendable, Equatable {
    public var stop: StopReason
    public var background: [String]

    public init(stop: StopReason, background: [String]) {
        self.stop = stop
        self.background = background
    }
}

public enum EventKind: Codable, Sendable, Equatable {
    case turnStarted(TurnStarted)
    case textDelta(TextDelta)
    case reasoningDelta(ReasoningDelta)
    case userMessage(UserMessage)
    case messageEnded(MessageEnded)
    case toolUpdated(ToolUpdate)
    case toolOutputDelta(ToolOutputDelta)
    case planUpdated(PlanUpdated)
    case requestOpened(Request)
    case requestClosed(RequestClosed)
    case sessionUpdated(SessionInfo)
    case statusChanged(SessionStatus)
    case contextUsage(ContextUsage)
    case contextCompacted
    case planUsageUpdated(PlanUsage)
    case diagnostic(Diagnostic)
    case turnEnded(TurnEnded)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "TurnStarted", …
    public var name: String {
        switch self {
        case .turnStarted: "TurnStarted"
        case .textDelta: "TextDelta"
        case .reasoningDelta: "ReasoningDelta"
        case .userMessage: "UserMessage"
        case .messageEnded: "MessageEnded"
        case .toolUpdated: "ToolUpdated"
        case .toolOutputDelta: "ToolOutputDelta"
        case .planUpdated: "PlanUpdated"
        case .requestOpened: "RequestOpened"
        case .requestClosed: "RequestClosed"
        case .sessionUpdated: "SessionUpdated"
        case .statusChanged: "StatusChanged"
        case .contextUsage: "ContextUsage"
        case .contextCompacted: "ContextCompacted"
        case .planUsageUpdated: "PlanUsageUpdated"
        case .diagnostic: "Diagnostic"
        case .turnEnded: "TurnEnded"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "ContextCompacted": self = .contextCompacted
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "TurnStarted": self = .turnStarted(try c.decode(TurnStarted.self, forKey: Key("TurnStarted")))
        case "TextDelta": self = .textDelta(try c.decode(TextDelta.self, forKey: Key("TextDelta")))
        case "ReasoningDelta": self = .reasoningDelta(try c.decode(ReasoningDelta.self, forKey: Key("ReasoningDelta")))
        case "UserMessage": self = .userMessage(try c.decode(UserMessage.self, forKey: Key("UserMessage")))
        case "MessageEnded": self = .messageEnded(try c.decode(MessageEnded.self, forKey: Key("MessageEnded")))
        case "ToolUpdated": self = .toolUpdated(try c.decode(ToolUpdate.self, forKey: Key("ToolUpdated")))
        case "ToolOutputDelta": self = .toolOutputDelta(try c.decode(ToolOutputDelta.self, forKey: Key("ToolOutputDelta")))
        case "PlanUpdated": self = .planUpdated(try c.decode(PlanUpdated.self, forKey: Key("PlanUpdated")))
        case "RequestOpened": self = .requestOpened(try c.decode(Request.self, forKey: Key("RequestOpened")))
        case "RequestClosed": self = .requestClosed(try c.decode(RequestClosed.self, forKey: Key("RequestClosed")))
        case "SessionUpdated": self = .sessionUpdated(try c.decode(SessionInfo.self, forKey: Key("SessionUpdated")))
        case "StatusChanged": self = .statusChanged(try c.decode(SessionStatus.self, forKey: Key("StatusChanged")))
        case "ContextUsage": self = .contextUsage(try c.decode(ContextUsage.self, forKey: Key("ContextUsage")))
        case "PlanUsageUpdated": self = .planUsageUpdated(try c.decode(PlanUsage.self, forKey: Key("PlanUsageUpdated")))
        case "Diagnostic": self = .diagnostic(try c.decode(Diagnostic.self, forKey: Key("Diagnostic")))
        case "TurnEnded": self = .turnEnded(try c.decode(TurnEnded.self, forKey: Key("TurnEnded")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .turnStarted(let v): try encoder.tagged("TurnStarted", v)
        case .textDelta(let v): try encoder.tagged("TextDelta", v)
        case .reasoningDelta(let v): try encoder.tagged("ReasoningDelta", v)
        case .userMessage(let v): try encoder.tagged("UserMessage", v)
        case .messageEnded(let v): try encoder.tagged("MessageEnded", v)
        case .toolUpdated(let v): try encoder.tagged("ToolUpdated", v)
        case .toolOutputDelta(let v): try encoder.tagged("ToolOutputDelta", v)
        case .planUpdated(let v): try encoder.tagged("PlanUpdated", v)
        case .requestOpened(let v): try encoder.tagged("RequestOpened", v)
        case .requestClosed(let v): try encoder.tagged("RequestClosed", v)
        case .sessionUpdated(let v): try encoder.tagged("SessionUpdated", v)
        case .statusChanged(let v): try encoder.tagged("StatusChanged", v)
        case .contextUsage(let v): try encoder.tagged("ContextUsage", v)
        case .contextCompacted: try encoder.raw("ContextCompacted")
        case .planUsageUpdated(let v): try encoder.tagged("PlanUsageUpdated", v)
        case .diagnostic(let v): try encoder.tagged("Diagnostic", v)
        case .turnEnded(let v): try encoder.tagged("TurnEnded", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public enum TurnOrigin: Codable, Sendable, Equatable {
    case agent
    case prompt(String)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Agent", …
    public var name: String {
        switch self {
        case .agent: "Agent"
        case .prompt: "Prompt"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "Agent": self = .agent
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Prompt": self = .prompt(try c.decode(String.self, forKey: Key("Prompt")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .agent: try encoder.raw("Agent")
        case .prompt(let v): try encoder.tagged("Prompt", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

/// Cumulative snapshot of one tool call.
public struct ToolUpdate: Codable, Sendable, Equatable {
    public var id: String
    public var kind: ToolKind
    public var title: String
    public var status: ToolStatus
    public var input: ToolInput
    public var output: String?
    public var diffs: [FileDiff]
    public var locations: [String]
    public var raw: RawTool?

    public init(id: String, kind: ToolKind, title: String, status: ToolStatus, input: ToolInput, output: String? = nil, diffs: [FileDiff], locations: [String], raw: RawTool? = nil) {
        self.id = id
        self.kind = kind
        self.title = title
        self.status = status
        self.input = input
        self.output = output
        self.diffs = diffs
        self.locations = locations
        self.raw = raw
    }
}

public struct Mcp: Codable, Sendable, Equatable {
    public var server: String
    public var tool: String

    public init(server: String, tool: String) {
        self.server = server
        self.tool = tool
    }
}

public enum ToolKind: Codable, Sendable, Equatable {
    case read
    case edit
    case delete
    case move
    case search
    case execute
    case fetch
    case think
    case other
    case mcp(Mcp)
    case subagent
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Read", …
    public var name: String {
        switch self {
        case .read: "Read"
        case .edit: "Edit"
        case .delete: "Delete"
        case .move: "Move"
        case .search: "Search"
        case .execute: "Execute"
        case .fetch: "Fetch"
        case .think: "Think"
        case .other: "Other"
        case .mcp: "Mcp"
        case .subagent: "Subagent"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "Read": self = .read
            case "Edit": self = .edit
            case "Delete": self = .delete
            case "Move": self = .move
            case "Search": self = .search
            case "Execute": self = .execute
            case "Fetch": self = .fetch
            case "Think": self = .think
            case "Other": self = .other
            case "Subagent": self = .subagent
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Mcp": self = .mcp(try c.decode(Mcp.self, forKey: Key("Mcp")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .read: try encoder.raw("Read")
        case .edit: try encoder.raw("Edit")
        case .delete: try encoder.raw("Delete")
        case .move: try encoder.raw("Move")
        case .search: try encoder.raw("Search")
        case .execute: try encoder.raw("Execute")
        case .fetch: try encoder.raw("Fetch")
        case .think: try encoder.raw("Think")
        case .other: try encoder.raw("Other")
        case .mcp(let v): try encoder.tagged("Mcp", v)
        case .subagent: try encoder.raw("Subagent")
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public enum ToolStatus: String, Codable, Sendable, Equatable {
    case pending = "Pending"
    case running = "Running"
    case completed = "Completed"
    case failed = "Failed"
    case cancelled = "Cancelled"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

public struct Command: Codable, Sendable, Equatable {
    public var command: String
    public var cwd: String?

    public init(command: String, cwd: String? = nil) {
        self.command = command
        self.cwd = cwd
    }
}

public enum ToolInput: Codable, Sendable, Equatable {
    case `none`
    case path(String)
    case command(Command)
    case pattern(String)
    case url(String)
    case query(String)
    case text(String)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "None", …
    public var name: String {
        switch self {
        case .`none`: "None"
        case .path: "Path"
        case .command: "Command"
        case .pattern: "Pattern"
        case .url: "Url"
        case .query: "Query"
        case .text: "Text"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "None": self = .`none`
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Path": self = .path(try c.decode(String.self, forKey: Key("Path")))
        case "Command": self = .command(try c.decode(Command.self, forKey: Key("Command")))
        case "Pattern": self = .pattern(try c.decode(String.self, forKey: Key("Pattern")))
        case "Url": self = .url(try c.decode(String.self, forKey: Key("Url")))
        case "Query": self = .query(try c.decode(String.self, forKey: Key("Query")))
        case "Text": self = .text(try c.decode(String.self, forKey: Key("Text")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .`none`: try encoder.raw("None")
        case .path(let v): try encoder.tagged("Path", v)
        case .command(let v): try encoder.tagged("Command", v)
        case .pattern(let v): try encoder.tagged("Pattern", v)
        case .url(let v): try encoder.tagged("Url", v)
        case .query(let v): try encoder.tagged("Query", v)
        case .text(let v): try encoder.tagged("Text", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public struct FileDiff: Codable, Sendable, Equatable {
    public var path: String
    public var oldText: String?
    public var newText: String

    public init(path: String, oldText: String? = nil, newText: String) {
        self.path = path
        self.oldText = oldText
        self.newText = newText
    }

    enum CodingKeys: String, CodingKey {
        case path
        case oldText = "old_text"
        case newText = "new_text"
    }
}

public struct RawTool: Codable, Sendable, Equatable {
    public var name: String
    public var input: JSONValue

    public init(name: String, input: JSONValue) {
        self.name = name
        self.input = input
    }
}

public struct PlanEntry: Codable, Sendable, Equatable {
    public var text: String
    public var status: PlanStatus

    public init(text: String, status: PlanStatus) {
        self.text = text
        self.status = status
    }
}

public enum PlanStatus: String, Codable, Sendable, Equatable {
    case pending = "Pending"
    case inProgress = "InProgress"
    case completed = "Completed"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

/// Something the agent is waiting on the caller for. Answer once with
public enum Request: Codable, Sendable, Equatable {
    case permission(PermissionRequest)
    case question(QuestionRequest)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Permission", …
    public var name: String {
        switch self {
        case .permission: "Permission"
        case .question: "Question"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .unrecognized(s); return }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Permission": self = .permission(try c.decode(PermissionRequest.self, forKey: Key("Permission")))
        case "Question": self = .question(try c.decode(QuestionRequest.self, forKey: Key("Question")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .permission(let v): try encoder.tagged("Permission", v)
        case .question(let v): try encoder.tagged("Question", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public struct PermissionRequest: Codable, Sendable, Equatable {
    public var id: String
    public var tool: ToolUpdate
    public var options: [PermissionChoice]
    public var detail: String?

    public init(id: String, tool: ToolUpdate, options: [PermissionChoice], detail: String? = nil) {
        self.id = id
        self.tool = tool
        self.options = options
        self.detail = detail
    }
}

public struct QuestionRequest: Codable, Sendable, Equatable {
    public var id: String
    public var questions: [Question]

    public init(id: String, questions: [Question]) {
        self.id = id
        self.questions = questions
    }
}

public struct Question: Codable, Sendable, Equatable {
    public var id: String
    public var text: String
    public var header: String?
    public var choices: [Choice]
    public var multiSelect: Bool
    public var allowsFreeText: Bool

    public init(id: String, text: String, header: String? = nil, choices: [Choice], multiSelect: Bool, allowsFreeText: Bool) {
        self.id = id
        self.text = text
        self.header = header
        self.choices = choices
        self.multiSelect = multiSelect
        self.allowsFreeText = allowsFreeText
    }

    enum CodingKeys: String, CodingKey {
        case id
        case text
        case header
        case choices
        case multiSelect = "multi_select"
        case allowsFreeText = "allows_free_text"
    }
}

public struct Choice: Codable, Sendable, Equatable {
    public var id: String
    public var label: String
    public var description: String?

    public init(id: String, label: String, description: String? = nil) {
        self.id = id
        self.label = label
        self.description = description
    }
}

/// Snapshot of a live session. Also carried by `EventKind::SessionUpdated`.
public struct SessionInfo: Codable, Sendable, Equatable {
    public var id: String
    public var agent: AgentInstallation
    public var details: AgentDetails
    public var configuration: SessionConfiguration
    public var resumeToken: String?
    public var title: String?
    public var status: SessionStatus?

    public init(id: String, agent: AgentInstallation, details: AgentDetails, configuration: SessionConfiguration, resumeToken: String? = nil, title: String? = nil, status: SessionStatus? = nil) {
        self.id = id
        self.agent = agent
        self.details = details
        self.configuration = configuration
        self.resumeToken = resumeToken
        self.title = title
        self.status = status
    }

    enum CodingKeys: String, CodingKey {
        case id
        case agent
        case details
        case configuration
        case resumeToken = "resume_token"
        case title
        case status
    }
}

/// One installed agent, as returned by `Runtime::discover`.
public struct AgentInstallation: Codable, Sendable, Equatable {
    public var id: String
    public var name: String
    public var executablePath: String
    public var source: InstallationSource
    public var upgrade: MissingAgent?
    public var acpArgs: [String]?

    public init(id: String, name: String, executablePath: String, source: InstallationSource, upgrade: MissingAgent? = nil, acpArgs: [String]? = nil) {
        self.id = id
        self.name = name
        self.executablePath = executablePath
        self.source = source
        self.upgrade = upgrade
        self.acpArgs = acpArgs
    }

    enum CodingKeys: String, CodingKey {
        case id
        case name
        case executablePath = "executable_path"
        case source
        case upgrade
        case acpArgs = "acp_args"
    }
}

/// Where discovery found an executable.
public enum InstallationSource: String, Codable, Sendable, Equatable {
    case envOverride = "EnvOverride"
    case path = "Path"
    case loginShellPath = "LoginShellPath"
    case versionManager = "VersionManager"
    case knownLocation = "KnownLocation"
    case pinned = "Pinned"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

public struct MissingAgent: Codable, Sendable, Equatable {
    public var id: String
    public var name: String
    public var searched: [String]
    public var installHint: String

    public init(id: String, name: String, searched: [String], installHint: String) {
        self.id = id
        self.name = name
        self.searched = searched
        self.installHint = installHint
    }

    enum CodingKeys: String, CodingKey {
        case id
        case name
        case searched
        case installHint = "install_hint"
    }
}

/// What `probe` and `open` learn about an agent.
public struct AgentDetails: Codable, Sendable, Equatable {
    public var version: String?
    public var auth: AuthStatus
    public var capabilities: Capabilities
    public var configOptions: [ConfigOption]
    public var commands: [SlashCommand]

    public init(version: String? = nil, auth: AuthStatus, capabilities: Capabilities, configOptions: [ConfigOption], commands: [SlashCommand]) {
        self.version = version
        self.auth = auth
        self.capabilities = capabilities
        self.configOptions = configOptions
        self.commands = commands
    }

    enum CodingKeys: String, CodingKey {
        case version
        case auth
        case capabilities
        case configOptions = "config_options"
        case commands
    }
}

public struct Authenticated: Codable, Sendable, Equatable {
    public var kind: AuthKind
    public var account: AccountInfo?

    public init(kind: AuthKind, account: AccountInfo? = nil) {
        self.kind = kind
        self.account = account
    }
}

public struct Unauthenticated: Codable, Sendable, Equatable {
    public var login: [LoginMethod]

    public init(login: [LoginMethod]) {
        self.login = login
    }
}

/// Whether, and how, an agent is logged in.
public enum AuthStatus: Codable, Sendable, Equatable {
    case unknown
    case authenticated(Authenticated)
    case unauthenticated(Unauthenticated)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Unknown", …
    public var name: String {
        switch self {
        case .unknown: "Unknown"
        case .authenticated: "Authenticated"
        case .unauthenticated: "Unauthenticated"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "Unknown": self = .unknown
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Authenticated": self = .authenticated(try c.decode(Authenticated.self, forKey: Key("Authenticated")))
        case "Unauthenticated": self = .unauthenticated(try c.decode(Unauthenticated.self, forKey: Key("Unauthenticated")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .unknown: try encoder.raw("Unknown")
        case .authenticated(let v): try encoder.tagged("Authenticated", v)
        case .unauthenticated(let v): try encoder.tagged("Unauthenticated", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

/// The login kind decides which features exist (plan usage needs a subscription).
public enum AuthKind: Codable, Sendable, Equatable {
    case subscription
    case apiKey
    case cloudProvider
    case other(String)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Subscription", …
    public var name: String {
        switch self {
        case .subscription: "Subscription"
        case .apiKey: "ApiKey"
        case .cloudProvider: "CloudProvider"
        case .other: "Other"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "Subscription": self = .subscription
            case "ApiKey": self = .apiKey
            case "CloudProvider": self = .cloudProvider
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Other": self = .other(try c.decode(String.self, forKey: Key("Other")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .subscription: try encoder.raw("Subscription")
        case .apiKey: try encoder.raw("ApiKey")
        case .cloudProvider: try encoder.raw("CloudProvider")
        case .other(let v): try encoder.tagged("Other", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public struct AccountInfo: Codable, Sendable, Equatable {
    public var email: String?
    public var plan: String?

    public init(email: String? = nil, plan: String? = nil) {
        self.email = email
        self.plan = plan
    }
}

public struct Terminal: Codable, Sendable, Equatable {
    public var command: [String]
    public var env: [String: String]
    public var description: String

    public init(command: [String], env: [String: String], description: String) {
        self.command = command
        self.env = env
        self.description = description
    }
}

public struct EnvVar: Codable, Sendable, Equatable {
    public var name: String

    public init(name: String) {
        self.name = name
    }
}

/// A login method the application can show to the user.
public enum LoginMethod: Codable, Sendable, Equatable {
    case terminal(Terminal)
    case envVar(EnvVar)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Terminal", …
    public var name: String {
        switch self {
        case .terminal: "Terminal"
        case .envVar: "EnvVar"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .unrecognized(s); return }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Terminal": self = .terminal(try c.decode(Terminal.self, forKey: Key("Terminal")))
        case "EnvVar": self = .envVar(try c.decode(EnvVar.self, forKey: Key("EnvVar")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .terminal(let v): try encoder.tagged("Terminal", v)
        case .envVar(let v): try encoder.tagged("EnvVar", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

/// Effective caller actions for one agent or session.
public struct Capabilities: Codable, Sendable, Equatable {
    public var features: [Capability]
    public var mcpTransports: [McpTransport]

    public init(features: [Capability], mcpTransports: [McpTransport]) {
        self.features = features
        self.mcpTransports = mcpTransports
    }

    enum CodingKeys: String, CodingKey {
        case features
        case mcpTransports = "mcp_transports"
    }
}

/// Optional actions supported by an agent or session.
public enum Capability: String, Codable, Sendable, Equatable {
    case images = "Images"
    case resume = "Resume"
    case steer = "Steer"
    case permissions = "Permissions"
    case questions = "Questions"
    case rollback = "Rollback"
    case fork = "Fork"
    case slashCommands = "SlashCommands"
    case plan = "Plan"
    case subagents = "Subagents"
    case contextUsage = "ContextUsage"
    case planUsage = "PlanUsage"
    case rollbackFiles = "RollbackFiles"
    case compact = "Compact"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

public enum McpTransport: String, Codable, Sendable, Equatable {
    case stdio = "Stdio"
    case http = "Http"
    case sse = "Sse"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

/// A session setting the agent advertises. Well-known ids: `model`, `effort`,
public struct ConfigOption: Codable, Sendable, Equatable {
    public var id: String
    public var name: String
    public var category: String?
    public var kind: ConfigKind
    public var current: ConfigValue?
    public var live: Bool

    public init(id: String, name: String, category: String? = nil, kind: ConfigKind, current: ConfigValue? = nil, live: Bool) {
        self.id = id
        self.name = name
        self.category = category
        self.kind = kind
        self.current = current
        self.live = live
    }
}

public struct Select: Codable, Sendable, Equatable {
    public var choices: [ConfigChoice]

    public init(choices: [ConfigChoice]) {
        self.choices = choices
    }
}

public enum ConfigKind: Codable, Sendable, Equatable {
    case boolean
    case select(Select)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Boolean", …
    public var name: String {
        switch self {
        case .boolean: "Boolean"
        case .select: "Select"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "Boolean": self = .boolean
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Select": self = .select(try c.decode(Select.self, forKey: Key("Select")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .boolean: try encoder.raw("Boolean")
        case .select(let v): try encoder.tagged("Select", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public struct ConfigChoice: Codable, Sendable, Equatable {
    public var value: String
    public var label: String
    public var description: String?

    public init(value: String, label: String, description: String? = nil) {
        self.value = value
        self.label = label
        self.description = description
    }
}

public struct SlashCommand: Codable, Sendable, Equatable {
    public var name: String
    public var description: String
    public var inputHint: String?

    public init(name: String, description: String, inputHint: String? = nil) {
        self.name = name
        self.description = description
        self.inputHint = inputHint
    }

    enum CodingKeys: String, CodingKey {
        case name
        case description
        case inputHint = "input_hint"
    }
}

public struct SessionConfiguration: Codable, Sendable, Equatable {
    public var options: [String: ConfigValue]

    public init(options: [String: ConfigValue]) {
        self.options = options
    }
}

/// What a UI should show for the session right now. Changes arrive as
public enum SessionStatus: String, Codable, Sendable, Equatable {
    case idle = "Idle"
    case working = "Working"
    case needsInput = "NeedsInput"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

/// Plan quota windows for the logged-in account.
public struct PlanUsage: Codable, Sendable, Equatable {
    public var plan: String?
    public var windows: [UsageWindow]
    public var fetchedAt: SystemTime

    public init(plan: String? = nil, windows: [UsageWindow], fetchedAt: SystemTime) {
        self.plan = plan
        self.windows = windows
        self.fetchedAt = fetchedAt
    }

    enum CodingKeys: String, CodingKey {
        case plan
        case windows
        case fetchedAt = "fetched_at"
    }
}

public struct UsageWindow: Codable, Sendable, Equatable {
    public var label: String
    public var usedPercent: UInt8
    public var resetsAt: SystemTime?

    public init(label: String, usedPercent: UInt8, resetsAt: SystemTime? = nil) {
        self.label = label
        self.usedPercent = usedPercent
        self.resetsAt = resetsAt
    }

    enum CodingKeys: String, CodingKey {
        case label
        case usedPercent = "used_percent"
        case resetsAt = "resets_at"
    }
}

public struct Diagnostic: Codable, Sendable, Equatable {
    public var level: DiagnosticLevel
    public var message: String

    public init(level: DiagnosticLevel, message: String) {
        self.level = level
        self.message = message
    }
}

public enum DiagnosticLevel: String, Codable, Sendable, Equatable {
    case info = "Info"
    case warning = "Warning"
    case error = "Error"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

public struct Completed: Codable, Sendable, Equatable {
    public var source: CompletionSource

    public init(source: CompletionSource) {
        self.source = source
    }
}

public struct Failed: Codable, Sendable, Equatable {
    public var message: String

    public init(message: String) {
        self.message = message
    }
}

public enum StopReason: Codable, Sendable, Equatable {
    case cancelled
    case refused
    case completed(Completed)
    case failed(Failed)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Cancelled", …
    public var name: String {
        switch self {
        case .cancelled: "Cancelled"
        case .refused: "Refused"
        case .completed: "Completed"
        case .failed: "Failed"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) {
            switch s {
            case "Cancelled": self = .cancelled
            case "Refused": self = .refused
            default: self = .unrecognized(s)
            }
            return
        }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Completed": self = .completed(try c.decode(Completed.self, forKey: Key("Completed")))
        case "Failed": self = .failed(try c.decode(Failed.self, forKey: Key("Failed")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .cancelled: try encoder.raw("Cancelled")
        case .refused: try encoder.raw("Refused")
        case .completed(let v): try encoder.tagged("Completed", v)
        case .failed(let v): try encoder.tagged("Failed", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}

public enum CompletionSource: String, Codable, Sendable, Equatable {
    case `protocol` = "Protocol"
    case inferred = "Inferred"
    /// A value this package does not know (a newer binary).
    case unrecognized

    public init(from decoder: Decoder) throws {
        self = Self(rawValue: try String(from: decoder)) ?? .unrecognized
    }
}

/// What `discover` found and what it could not read.
public struct DiscoveryReport: Codable, Sendable, Equatable {
    public var agents: [AgentInstallation]
    public var missing: [MissingAgent]
    public var diagnostics: [Diagnostic]

    public init(agents: [AgentInstallation], missing: [MissingAgent], diagnostics: [Diagnostic]) {
        self.agents = agents
        self.missing = missing
        self.diagnostics = diagnostics
    }
}

/// Immediate result of submitting a prompt.
public struct Delivery: Codable, Sendable, Equatable {
    public var promptId: String
    public var kind: DeliveryKind

    public init(promptId: String, kind: DeliveryKind) {
        self.promptId = promptId
        self.kind = kind
    }

    enum CodingKeys: String, CodingKey {
        case promptId = "prompt_id"
        case kind
    }
}

public struct Started: Codable, Sendable, Equatable {
    public var turnId: String

    public init(turnId: String) {
        self.turnId = turnId
    }

    enum CodingKeys: String, CodingKey {
        case turnId = "turn_id"
    }
}

public struct Steered: Codable, Sendable, Equatable {
    public var turnId: String

    public init(turnId: String) {
        self.turnId = turnId
    }

    enum CodingKeys: String, CodingKey {
        case turnId = "turn_id"
    }
}

public struct Queued: Codable, Sendable, Equatable {
    public var position: UInt32

    public init(position: UInt32) {
        self.position = position
    }
}

public enum DeliveryKind: Codable, Sendable, Equatable {
    case started(Started)
    case steered(Steered)
    case queued(Queued)
    /// A variant this package does not know (a newer binary): its wire name.
    case unrecognized(String)

    /// The variant's wire name: "Started", …
    public var name: String {
        switch self {
        case .started: "Started"
        case .steered: "Steered"
        case .queued: "Queued"
        case .unrecognized(let tag): tag
        }
    }

    public init(from decoder: Decoder) throws {
        if let s = try? String(from: decoder) { self = .unrecognized(s); return }
        let c = try decoder.container(keyedBy: Key.self)
        switch c.allKeys.first?.stringValue {
        case "Started": self = .started(try c.decode(Started.self, forKey: Key("Started")))
        case "Steered": self = .steered(try c.decode(Steered.self, forKey: Key("Steered")))
        case "Queued": self = .queued(try c.decode(Queued.self, forKey: Key("Queued")))
        default: self = .unrecognized(c.allKeys.first?.stringValue ?? "?")
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .started(let v): try encoder.tagged("Started", v)
        case .steered(let v): try encoder.tagged("Steered", v)
        case .queued(let v): try encoder.tagged("Queued", v)
        case .unrecognized(let tag): try encoder.raw(tag)
        }
    }
}
