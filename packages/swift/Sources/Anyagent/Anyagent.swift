// The anyagent binary as a Swift API: spawn `anyagent serve`, write command
// lines, route reply and event lines. Every rule lives in the binary; this
// file is a pipe (ticket 13, W1–W10).

import Foundation

// ---------------------------------------------------------------------------
// PUBLIC TYPES
// ---------------------------------------------------------------------------

/// What `open` and `generate` accept besides the agent: the `open` command's fields.
public struct OpenOptions: Encodable, Sendable {
    public var dir: String
    public var resume: String?
    public var fork: String?
    public var forkAt: String?
    public var permissionMode: PermissionMode?
    public var mcpServers: [McpServer]?
    public var configure: [String: ConfigValue]?

    public init(
        dir: String, resume: String? = nil, fork: String? = nil, forkAt: String? = nil,
        permissionMode: PermissionMode? = nil, mcpServers: [McpServer]? = nil, configure: [String: ConfigValue]? = nil
    ) {
        self.dir = dir
        self.resume = resume
        self.fork = fork
        self.forkAt = forkAt
        self.permissionMode = permissionMode
        self.mcpServers = mcpServers
        self.configure = configure
    }

    enum CodingKeys: String, CodingKey {
        case dir, resume, fork, configure
        case forkAt = "fork_at"
        case permissionMode = "permission_mode"
        case mcpServers = "mcp_servers"
    }
}

/// `kind`, `message`, and every extra field from the wire in `data` (W8).
public struct AnyagentError: Error, Sendable, CustomStringConvertible {
    public let kind: String
    public let message: String
    public let data: [String: JSONValue]

    public var description: String { "\(kind): \(message)" }

    init(kind: String, message: String, data: [String: JSONValue] = [:]) {
        self.kind = kind
        self.message = message
        self.data = data
    }

    /// From a wire `error` object.
    init(body: [String: JSONValue]) {
        var data = body
        kind = if case .string(let s) = data.removeValue(forKey: "kind") { s } else { "" }
        message = if case .string(let s) = data.removeValue(forKey: "message") { s } else { "" }
        self.data = data
    }
}

// ---------------------------------------------------------------------------
// RUNTIME: one `anyagent serve` process
// ---------------------------------------------------------------------------

/// The wire protocol this package speaks; the binary's hello must match.
let PROTOCOL: UInt32 = 1

/// One line from the binary, untagged: the fields present say what it is.
private struct Line: Decodable {
    var hello: Hello?
    var id: UInt64?
    var ok: JSONValue?
    var error: [String: JSONValue]?
    var event: Event?
    var session: String?
    var closed: String?
}

private struct Hello: Decodable {
    var `protocol`: UInt32
    var anyagent: String
}

/// One `anyagent serve` process. Build it with `Runtime.start`.
public actor Runtime {
    private let process = Process()
    private let stdin = Pipe()
    private var next: UInt64 = 1
    private var pending: [UInt64: Pending] = [:]
    private var sessions: [String: Session] = [:]
    private var dead: AnyagentError?
    private var hello: CheckedContinuation<Void, Error>?
    private var exited: Task<Int32, Never>?

    private struct Pending {
        let cont: CheckedContinuation<Reply, Error>
        let opens: Bool
    }

    private enum Reply {
        case ok(JSONValue)
        case session(Session)
    }

    /// Spawns the binary; returns after its hello line.
    /// `bin`: path to the binary; default `ANYAGENT_BIN`, then the app bundle's, then PATH.
    /// `mock`: a mock script (`packages/mock-scripts/*.json`): no real agents.
    /// `env`: environment for the binary and the agents it spawns; default this process's.
    public static func start(bin: String? = nil, mock: String? = nil, env: [String: String]? = nil) async throws -> Runtime {
        let rt = Runtime()
        try await rt.launch(bin: bin, mock: mock, env: env)
        return rt
    }

    public func discover() async throws -> DiscoveryReport {
        try await call(["cmd": "discover"])
    }

    public func probe(_ agent: AgentRef) async throws -> AgentDetails {
        try await call(["cmd": "probe", "agent": json(agent)])
    }

    public func planUsage(_ agent: AgentRef) async throws -> PlanUsage {
        try await call(["cmd": "plan_usage", "agent": json(agent)])
    }

    /// One-shot text with no session to manage: titles, commit messages.
    public func generate(_ agent: AgentRef, _ opts: OpenOptions, _ prompt: String) async throws -> String {
        try await call(fields(of: opts).merging(["cmd": "generate", "agent": json(agent), "prompt": .string(prompt)]) { $1 })
    }

    /// Opens a session. The Session is registered inside onLine (W1).
    public func open(_ agent: AgentRef, _ opts: OpenOptions) async throws -> Session {
        guard case .session(let session) = try await send(fields(of: opts).merging(["cmd": "open", "agent": json(agent)]) { $1 }, opens: true) else {
            throw AnyagentError(kind: "ProtocolFailed", message: "open replied without a session")
        }
        return session
    }

    /// Graceful and idempotent (W5): close stdin, wait up to 5 s, then kill.
    @discardableResult
    public func close() async -> Int32 {
        guard let exited else { return -1 }
        if dead == nil {
            try? stdin.fileHandleForWriting.close()
            let deadline = Task {
                try? await Task.sleep(for: .seconds(5))
                if !Task.isCancelled { killProcess() }
            }
            let code = await exited.value
            deadline.cancel()
            return code
        }
        return await exited.value
    }

    // Used by Session; not part of the API.
    /// Writes one command line; returns the reply's `ok` as T.
    func call<T: Decodable & Sendable>(_ fields: [String: JSONValue]) async throws -> T {
        guard case .ok(let ok) = try await send(fields) else {
            throw AnyagentError(kind: "ProtocolFailed", message: "unexpected reply shape")
        }
        return try decode(T.self, from: ok)
    }

    /// One raw line on stdin (tests use it for a bad frame).
    func write(line: String) {
        try? stdin.fileHandleForWriting.write(contentsOf: Data((line + "\n").utf8))
    }

    var pid: Int32 { process.processIdentifier }

    /// Writes one command line; the continuation gets the reply's `ok`, or the Session it opened.
    private func send(_ fields: [String: JSONValue], opens: Bool = false) async throws -> Reply {
        if let dead { throw dead }  // W4
        let id = next
        next += 1
        var frame = fields
        frame["id"] = .number(Double(id))
        let data = try JSONEncoder().encode(frame)
        return try await withCheckedThrowingContinuation { cont in
            pending[id] = Pending(cont: cont, opens: opens)
            write(line: String(decoding: data, as: UTF8.self))
        }
    }

    /// Spawns the binary, starts the reader, waits for the hello.
    private func launch(bin: String?, mock: String?, env: [String: String]?) async throws {
        signal(SIGPIPE, SIG_IGN)  // a write after the binary died throws EPIPE instead of ending the app
        let stdout = Pipe()
        process.executableURL = URL(fileURLWithPath: try resolveBinary(bin))
        process.arguments = mock.map { ["serve", "--mock", $0] } ?? ["serve"]
        if let env { process.environment = env }
        process.standardInput = stdin
        process.standardOutput = stdout
        let (status, report) = AsyncStream<Int32>.makeStream()
        process.terminationHandler = { report.yield($0.terminationStatus); report.finish() }
        try process.run()
        exited = Task {
            for await line in Runtime.lines(of: stdout.fileHandleForReading) { await onLine(line) }
            var code: Int32 = -1
            for await s in status { code = s }
            await onExit(code)
            return code
        }
        try await withCheckedThrowingContinuation { hello = $0 }
    }

    /// Routes one stdout line. Registers a session before the next line is read (W1).
    private func onLine(_ text: String) async {
        if dead != nil { return }
        guard let line = try? JSONDecoder().decode(Line.self, from: Data(text.utf8)) else {
            return abort("not a frame: \(text)")
        }
        if let hello = line.hello {
            if hello.protocol != PROTOCOL { return abort("protocol \(hello.protocol), this package speaks \(PROTOCOL)") }
            self.hello?.resume()
            self.hello = nil
        } else if let id = line.id, let p = pending.removeValue(forKey: id) {
            if let error = line.error {
                p.cont.resume(throwing: AnyagentError(body: error))
            } else if p.opens {
                p.cont.resume(with: Result { .session(try register(line.ok ?? .null)) })
            } else {
                p.cont.resume(returning: .ok(line.ok ?? .null))
            }
        } else if let event = line.event {
            await sessions[event.sessionId]?.push(event)
        } else if let id = line.session, let error = line.error {  // W3
            await sessions[id]?.fail(AnyagentError(body: error))
        } else if let id = line.closed {
            await sessions.removeValue(forKey: id)?.end()
        }
    }

    private func register(_ ok: JSONValue) throws -> Session {
        let info = try decode(SessionInfo.self, from: ok)
        let session = Session(self, info)
        sessions[info.id] = session
        return session
    }

    /// A binary that does not speak the protocol (W10): kill it; onExit fails the rest.
    private func abort(_ why: String) {
        dead = AnyagentError(kind: "ProtocolFailed", message: why)
        killProcess()
    }

    /// Process gone: fail everything still waiting (W4).
    private func onExit(_ code: Int32) async {
        let error = dead ?? AnyagentError(
            kind: "ProcessExited", message: "anyagent exited (\(code))", data: ["status": .string("\(code)"), "stderr": ""])
        dead = error
        hello?.resume(throwing: error)
        hello = nil
        for p in pending.values { p.cont.resume(throwing: error) }
        pending = [:]
        for session in sessions.values { await session.fail(error) }
        sessions = [:]
    }

    private func killProcess() {
        if process.isRunning { kill(process.processIdentifier, SIGKILL) }
    }

    /// stdout as lines: a blocking read loop on its own thread feeding a stream, in order.
    private static func lines(of handle: FileHandle) -> AsyncStream<String> {
        let fd = handle.fileDescriptor
        return AsyncStream { cont in
            Thread.detachNewThread {
                let handle = FileHandle(fileDescriptor: fd)
                var buffer = Data()
                while case let chunk = handle.availableData, !chunk.isEmpty {
                    buffer.append(chunk)
                    var start = buffer.startIndex
                    while let newline = buffer[start...].firstIndex(of: UInt8(ascii: "\n")) {
                        var line = buffer[start..<newline]
                        if line.last == UInt8(ascii: "\r") { line = line.dropLast() }
                        cont.yield(String(decoding: line, as: UTF8.self))
                        start = newline + 1
                    }
                    buffer.removeSubrange(buffer.startIndex..<start)
                }
                cont.finish()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SESSION: one open session
// ---------------------------------------------------------------------------

/// Unread events a session may hold before it is closed as lagging (W6).
let CAP = 4096

/// One open session: commands in, an ordered event stream out.
public actor Session {
    public nonisolated let id: String
    /// Live: replaced on every `SessionUpdated` (W2).
    public private(set) var info: SessionInfo
    /// Live: replaced on every `StatusChanged` (W2).
    public private(set) var status: SessionStatus
    private let rt: Runtime
    private var queue: [Event] = []
    private var waiters: [CheckedContinuation<Void, Never>] = []
    private var error: AnyagentError?
    private var done = false

    // Built by Runtime.open; not part of the API.
    init(_ rt: Runtime, _ info: SessionInfo) {
        self.rt = rt
        id = info.id
        self.info = info
        status = info.status ?? .idle
    }

    public func prompt(_ text: String, attachments: [String] = []) async throws -> Delivery {
        try await call(["cmd": "prompt", "text": .string(text), "attachments": .array(attachments.map { .string($0) })])
    }

    public func answer(_ request: String, _ answer: Answer) async throws {
        try await run(["cmd": "answer", "request": .string(request), "answer": json(answer)])
    }

    public func configure(_ option: String, _ value: ConfigValue) async throws {
        try await run(["cmd": "configure", "option": .string(option), "value": json(value)])
    }

    public func cancel(clearQueue: Bool = false) async throws {
        try await run(["cmd": "cancel", "clear_queue": .bool(clearQueue)])
    }

    public func dequeue(_ prompt: String) async throws {
        try await run(["cmd": "dequeue", "prompt": .string(prompt)])
    }

    public func rollback(turns: Int, scope: RollbackScope) async throws {
        try await run(["cmd": "rollback", "turns": .number(Double(turns)), "scope": json(scope)])
    }

    public func compact() async throws {
        try await run(["cmd": "compact"])
    }

    public func close() async throws {
        try await run(["cmd": "close"])
    }

    /// This session's events in order. Ends after `closed` or when the reading
    /// task is cancelled; throws once on a session error, then ends (W3).
    public nonisolated func events() -> AsyncThrowingStream<Event, Error> {
        AsyncThrowingStream { try await self.next() }
    }

    private func call<T: Decodable & Sendable>(_ fields: [String: JSONValue]) async throws -> T {
        var fields = fields
        fields["session"] = .string(id)
        return try await rt.call(fields)
    }

    private func run(_ fields: [String: JSONValue]) async throws {
        let _: JSONValue = try await call(fields)
    }

    /// The next event; nil at the end or once the reading task is cancelled; the error once, when the queue is drained.
    private func next() async throws -> Event? {
        while true {
            if !queue.isEmpty { return queue.removeFirst() }
            if let error {
                self.error = nil
                done = true
                throw error
            }
            if done || Task.isCancelled { return nil }
            await withTaskCancellationHandler {
                await withCheckedContinuation { waiters.append($0) }
            } onCancel: {
                Task { await self.wake() }
            }
        }
    }

    // push, fail and end are called by Runtime.onLine/onExit; not part of the API.
    /// Keeps info and status live (W2), applies the cap (W6).
    func push(_ ev: Event) {
        if error != nil || done { return }
        if case .sessionUpdated(let i) = ev.kind { info = i }
        if case .statusChanged(let s) = ev.kind { status = s }
        queue.append(ev)
        if queue.count > CAP {
            fail(AnyagentError(kind: "ConsumerLagged", message: "\(CAP) events unread"))
            Task { try? await close() }
            return
        }
        wake()
    }

    func fail(_ e: AnyagentError) {
        if error == nil, !done {
            error = e
            wake()
        }
    }

    func end() {
        done = true
        wake()
    }

    /// Resumes every waiting reader; each re-checks the queue, the error, and its own cancellation.
    private func wake() {
        for w in waiters { w.resume() }
        waiters = []
    }
}

// ---------------------------------------------------------------------------
// HELPERS
// ---------------------------------------------------------------------------

/// Any Encodable as a JSON value, for a command field.
func json<T: Encodable>(_ value: T) throws -> JSONValue {
    try JSONDecoder().decode(JSONValue.self, from: JSONEncoder().encode(value))
}

/// An Encodable's fields, for `open` and `generate`.
func fields<T: Encodable>(of value: T) throws -> [String: JSONValue] {
    if case .object(let o) = try json(value) { return o }
    return [:]
}

/// A reply's `ok` as the type the command returns.
func decode<T: Decodable & Sendable>(_ type: T.Type, from value: JSONValue) throws -> T {
    try JSONDecoder().decode(type, from: JSONEncoder().encode(value))
}

// ---------------------------------------------------------------------------
// INTERNAL: finding the binary
// ---------------------------------------------------------------------------

/// `bin`, then `ANYAGENT_BIN`, then the app bundle's auxiliary executable, then PATH.
func resolveBinary(_ bin: String?) throws -> String {
    let env = ProcessInfo.processInfo.environment
    if let explicit = bin ?? env["ANYAGENT_BIN"] { return explicit }
    #if os(macOS)
    if let bundled = Bundle.main.url(forAuxiliaryExecutable: "anyagent") { return bundled.path }
    #endif
    for dir in (env["PATH"] ?? "").split(separator: ":") where FileManager.default.isExecutableFile(atPath: "\(dir)/anyagent") {
        return "\(dir)/anyagent"
    }
    throw AnyagentError(
        kind: "NotInstalled",
        message: "no anyagent binary: bundle it in the app (Contents/MacOS/anyagent), set ANYAGENT_BIN, or put it on PATH",
        data: ["agent": "anyagent"])
}
