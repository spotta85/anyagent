// S1–S11 from ticket 13: the wrapper against `anyagent serve --mock`.
// Needs a mock-enabled binary: `cargo build --features mock` (or ANYAGENT_BIN).

import Foundation
import Testing

@testable import Anyagent

// packages/swift/Tests/AnyagentTests/SubprocessTests.swift -> the repo root.
let root = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
    .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
let bin = ProcessInfo.processInfo.environment["ANYAGENT_BIN"] ?? root.appendingPathComponent("target/debug/anyagent").path
let scripts = root.appendingPathComponent("packages/mock-scripts")
let dir = FileManager.default.temporaryDirectory.appendingPathComponent("anyagent-swift-\(getpid())").path

/// A runtime over a mock script.
func start(_ script: String) async throws -> Runtime {
    try await Runtime.start(bin: bin, mock: scripts.appendingPathComponent("\(script).json").path)
}

/// Reads the stream to its end.
func drain(_ session: Session) async throws -> [Event] {
    var out: [Event] = []
    for try await ev in session.events() { out.append(ev) }
    return out
}

/// Events until `kind` (inclusive); the last one is the match.
func until(_ session: Session, _ kind: String) async throws -> [Event] {
    var seen: [Event] = []
    for try await ev in session.events() {
        seen.append(ev)
        if ev.kind.name == kind { return seen }
    }
    throw AnyagentError(kind: "Test", message: "stream ended before \(kind); saw \(seen.map(\.kind.name).joined(separator: ","))")
}

func texts(_ events: [Event]) -> [String] {
    events.compactMap { if case .textDelta(let d) = $0.kind { d.text } else { nil } }
}

/// Expects `op` to throw an AnyagentError of `kind`.
@discardableResult
func rejects<T>(_ kind: String, _ op: () async throws -> T) async -> AnyagentError? {
    do {
        _ = try await op()
        Issue.record("expected \(kind), got a result")
    } catch let e as AnyagentError {
        #expect(e.kind == kind, "\(e.message)")
        return e
    } catch {
        Issue.record("expected \(kind), got \(error)")
    }
    return nil
}

@Suite(.serialized)
struct Subprocess {
    init() {
        try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
    }

    @Test func s1_open_prompt_answer_the_permission_see_the_turn_end_close() async throws {
        let rt = try await start("turn")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        let info = await session.info
        #expect(info.id == session.id)

        let delivery = try await session.prompt("hi")
        guard case .started = delivery.kind else { throw AnyagentError(kind: "Test", message: "\(delivery)") }

        let opened = try await until(session, "RequestOpened")
        guard case .requestOpened(.permission(let request)) = opened.last!.kind else {
            throw AnyagentError(kind: "Test", message: "\(opened.last!)")
        }
        try await session.answer(request.id, .permission(.allowOnce))

        let rest = try await until(session, "TurnEnded")
        #expect(texts(rest) == ["Done."])
        _ = try await until(session, "StatusChanged")
        let status = await session.status
        #expect(status == .idle)  // W2: live

        try await session.close()
        #expect(try await drain(session).isEmpty, "no events after closed")
        await rt.close()
    }

    @Test func s2_events_buffered_before_the_app_iterates_are_all_delivered() async throws {
        let rt = try await start("turn")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        _ = try await session.prompt("hi")
        try await Task.sleep(for: .milliseconds(200))
        let seen = try await until(session, "RequestOpened")
        #expect(texts(seen) == ["Let me check. "])
        #expect(seen.contains { $0.kind.name == "TurnStarted" })
        await rt.close()
    }

    @Test func s3_a_prompt_after_close_rejects_with_session_closed() async throws {
        let rt = try await start("turn")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        try await session.close()
        await rejects("SessionClosed") { try await session.prompt("x") }
        await rt.close()
    }

    @Test func s4_two_sessions_see_only_their_own_events_in_order() async throws {
        let rt = try await start("chatter")
        async let openA = rt.open("mock", OpenOptions(dir: dir))
        async let openB = rt.open("mock", OpenOptions(dir: dir))
        let (a, b) = try await (openA, openB)
        #expect(a.id != b.id)
        async let promptA = a.prompt("x")
        async let promptB = b.prompt("y")
        _ = try await (promptA, promptB)
        async let eventsA = until(a, "TurnEnded")
        async let eventsB = until(b, "TurnEnded")
        let (ea, eb) = try await (eventsA, eventsB)
        for (session, events) in [(a, ea), (b, eb)] {
            #expect(events.allSatisfy { $0.sessionId == session.id })
            let seqs = events.map(\.sequence)
            #expect(seqs == seqs.sorted())
            #expect(texts(events) == ["one", "two", "three"])
        }
        await rt.close()
    }

    @Test func s5_a_malformed_line_is_answered_with_bad_frame_and_the_runtime_goes_on() async throws {
        let rt = try await start("turn")
        await rt.write(line: "not json")
        let report = try await rt.discover()
        #expect(report.agents[0].id == "mock")
        await rt.close()
    }

    @Test func s6_process_death_rejects_pending_calls_fails_iterators_and_later_calls() async throws {
        let rt = try await start("turn")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        // Killed first: nothing written from here on can be answered.
        kill(await rt.pid, SIGKILL)
        await rejects("ProcessExited") { try await rt.discover() }
        await rejects("ProcessExited") { try await rt.open("mock", OpenOptions(dir: dir)) }
        await rejects("ProcessExited") { try await drain(session) }
        await rejects("ProcessExited") { try await rt.discover() }
        await rt.close()
    }

    /// The turn fails, the stream throws ProcessExited, then ends.
    @Test func s7_the_agent_dying_mid_turn() async throws {
        let rt = try await start("die")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        _ = try await session.prompt("go")
        let ended = try await until(session, "TurnEnded").last!.kind
        guard case .turnEnded(let turn) = ended, case .failed = turn.stop else {
            throw AnyagentError(kind: "Test", message: "\(ended)")
        }
        let e = await rejects("ProcessExited") { try await drain(session) }
        #expect(e?.data["status"] == .string("9"))
        #expect(try await drain(session).isEmpty, "stream restarted")
        await rt.close()
    }

    @Test func s8a_a_20_000_event_flood_arrives_whole_and_in_order() async throws {
        let rt = try await start("flood")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        _ = try await session.prompt("go")
        var deltas = 0
        var last: UInt64 = 0
        for try await ev in session.events() {
            #expect(ev.sequence > last)
            last = ev.sequence
            if case .textDelta = ev.kind { deltas += 1 }
            if case .turnEnded = ev.kind { break }
        }
        #expect(deltas == 20_000)
        await rt.close()
    }

    @Test func s8b_a_consumer_that_stops_reading_gets_consumer_lagged_and_the_runtime_survives() async throws {
        let rt = try await start("flood")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        _ = try await session.prompt("go")
        var n = 0
        for try await _ in session.events() {
            n += 1
            if n == 10 { break }
        }
        try await Task.sleep(for: .milliseconds(2500))  // 4096 events arrive in ~0.8 s at the flood's pace
        await rejects("ConsumerLagged") { try await until(session, "TurnEnded") }
        #expect(try await rt.discover().agents[0].id == "mock")
        await rt.close()
    }

    /// `closed` arrives, the process exits 0 and is gone.
    @Test func s9_close_with_a_session_open() async throws {
        let rt = try await start("turn")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        let pid = await rt.pid
        let code = await rt.close()
        #expect(code == 0)
        #expect(try await drain(session).isEmpty, "closed should end the stream")
        #expect(kill(pid, 0) != 0, "process still there")
    }

    @Test func s10_configure_sends_option_and_session_updated_updates_info() async throws {
        let rt = try await start("configure")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        try await session.configure("model", "opus")
        _ = try await until(session, "SessionUpdated")
        let info = await session.info
        #expect(info.configuration.options["model"] == .string("opus"))
        await rt.close()
    }

    @Test func s11_a_binary_that_does_not_speak_the_protocol_fails_start_with_protocol_failed() async throws {
        // A shell script stands in for the binary: it prints one stdout line, then exits.
        let fake = "\(dir)/fake-anyagent"
        try "#!/bin/sh\necho \"$LINE\"\n".write(toFile: fake, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: fake)
        func speaks(_ line: String) async throws -> Runtime {
            try await Runtime.start(bin: fake, env: ProcessInfo.processInfo.environment.merging(["LINE": line]) { $1 })
        }
        await rejects("ProtocolFailed") { try await speaks("nope") }
        await rejects("ProtocolFailed") { try await speaks(#"{"hello": {"protocol": 99, "anyagent": "x"}}"#) }
    }

    @Test(.timeLimit(.minutes(1))) func cancelling_the_reading_task_ends_its_stream() async throws {
        let rt = try await start("turn")
        let session = try await rt.open("mock", OpenOptions(dir: dir))
        let reader = Task { try await drain(session) }  // nothing prompted: it waits
        try await Task.sleep(for: .milliseconds(100))
        reader.cancel()
        #expect(try await reader.value.isEmpty)
        await rt.close()
    }
}
