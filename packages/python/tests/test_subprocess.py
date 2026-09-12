"""S1–S11 from ticket 13: the wrapper against `anyagent serve --mock`.
Needs a mock-enabled binary: `cargo build --features mock` (or ANYAGENT_BIN)."""

import asyncio
import json
import os
import sys
from collections.abc import Awaitable, Callable
from pathlib import Path

import pytest

from anyagent import AnyagentError, Runtime, Session, kind_of
from anyagent.types import Event

ROOT = Path(__file__).resolve().parents[3]
BIN = os.environ.get("ANYAGENT_BIN", str(ROOT / "target/debug/anyagent"))
SCRIPTS = ROOT / "packages/mock-scripts"

Start = Callable[[str], Awaitable[Runtime]]


@pytest.fixture
async def start() -> Start:
    """A runtime over a mock script, closed when the test ends however it ends."""
    started: list[Runtime] = []

    async def start(script: str) -> Runtime:
        rt = await Runtime.start(bin=BIN, mock=str(SCRIPTS / f"{script}.json"))
        started.append(rt)
        return rt

    yield start  # type: ignore[misc]
    for rt in started:
        await rt.close()


@pytest.fixture(scope="session")
def dir(tmp_path_factory: pytest.TempPathFactory) -> str:
    return str(tmp_path_factory.mktemp("anyagent-py-"))


async def drain(session: Session) -> list[Event]:
    """Reads the stream to its end."""
    return [ev async for ev in session.events()]


async def until(session: Session, kind: str) -> list[Event]:
    """Events until `kind` (inclusive); the last one is the match."""
    seen: list[Event] = []
    async for ev in session.events():
        seen.append(ev)
        if kind_of(ev) == kind:
            return seen
    raise AssertionError(f"stream ended before {kind}; saw {','.join(map(kind_of, seen))}")


def texts(events: list[Event]) -> list[str]:
    return [ev["kind"]["TextDelta"]["text"] for ev in events if kind_of(ev) == "TextDelta"]  # type: ignore[index]


async def rejects(aw: Awaitable[object], kind: str) -> AnyagentError:
    with pytest.raises(AnyagentError) as e:
        await aw
    assert e.value.kind == kind, e.value.message
    return e.value


async def test_s1_open_prompt_answer_the_permission_see_the_turn_end_close(start: Start, dir: str):
    rt = await start("turn")
    session = await rt.open("mock", dir=dir)
    assert session.info["id"] == session.id

    delivery = await session.prompt("hi")
    assert isinstance(delivery["kind"], dict) and "Started" in delivery["kind"], delivery

    opened = await until(session, "RequestOpened")
    last = opened[-1]["kind"]["RequestOpened"]  # type: ignore[index]
    await session.answer(last["Permission"]["id"], {"Permission": "AllowOnce"})

    rest = await until(session, "TurnEnded")
    assert texts(rest) == ["Done."]
    await until(session, "StatusChanged")
    assert session.status == "Idle"  # W2: live

    await session.close()
    assert await drain(session) == [], "no events after closed"
    await rt.close()


async def test_s2_events_buffered_before_the_app_iterates_are_all_delivered(start: Start, dir: str):
    rt = await start("turn")
    session = await rt.open("mock", dir=dir)
    await session.prompt("hi")
    await asyncio.sleep(0.2)
    seen = await until(session, "RequestOpened")
    assert texts(seen) == ["Let me check. "]
    assert any(kind_of(ev) == "TurnStarted" for ev in seen)
    await rt.close()


async def test_s3_a_prompt_after_close_rejects_with_session_closed(start: Start, dir: str):
    rt = await start("turn")
    session = await rt.open("mock", dir=dir)
    await session.close()
    await rejects(session.prompt("x"), "SessionClosed")
    await rt.close()


async def test_s4_two_sessions_see_only_their_own_events_in_order(start: Start, dir: str):
    rt = await start("chatter")
    a, b = await asyncio.gather(rt.open("mock", dir=dir), rt.open("mock", dir=dir))
    assert a.id != b.id
    await asyncio.gather(a.prompt("x"), b.prompt("y"))
    ea, eb = await asyncio.gather(until(a, "TurnEnded"), until(b, "TurnEnded"))
    for session, events in ((a, ea), (b, eb)):
        assert all(ev["session_id"] == session.id for ev in events)
        seqs = [ev["sequence"] for ev in events]
        assert seqs == sorted(seqs)
        assert texts(events) == ["one", "two", "three"]
    await rt.close()


async def test_s5_a_malformed_line_is_answered_with_bad_frame_and_the_runtime_goes_on(start: Start):
    rt = await start("turn")
    rt._proc.stdin.write(b"not json\n")  # type: ignore[union-attr]
    report = await rt.discover()
    assert report["agents"][0]["id"] == "mock"
    await rt.close()


async def test_s6_process_death_rejects_pending_calls_fails_iterators_and_later_calls(start: Start, dir: str):
    rt = await start("turn")
    session = await rt.open("mock", dir=dir)
    # Killed first: nothing written from here on can be answered.
    rt._proc.kill()
    for pending in (rt.discover(), rt.open("mock", dir=dir)):
        await rejects(pending, "ProcessExited")
    await rejects(drain(session), "ProcessExited")
    await rejects(rt.discover(), "ProcessExited")
    await rt.close()


async def test_s7_the_agent_dying_mid_turn(start: Start, dir: str):
    """The turn fails, the stream raises ProcessExited, then ends."""
    rt = await start("die")
    session = await rt.open("mock", dir=dir)
    await session.prompt("go")
    ended = (await until(session, "TurnEnded"))[-1]["kind"]["TurnEnded"]  # type: ignore[index]
    assert isinstance(ended["stop"], dict) and "Failed" in ended["stop"], ended
    e = await rejects(drain(session), "ProcessExited")
    assert e.data["status"] == "9"
    assert await drain(session) == [], "stream restarted"
    await rt.close()


async def test_s8a_a_20_000_event_flood_arrives_whole_and_in_order(start: Start, dir: str):
    rt = await start("flood")
    session = await rt.open("mock", dir=dir)
    await session.prompt("go")
    deltas = 0
    last = 0
    async for ev in session.events():
        assert ev["sequence"] > last
        last = ev["sequence"]
        if kind_of(ev) == "TextDelta":
            deltas += 1
        if kind_of(ev) == "TurnEnded":
            break
    assert deltas == 20_000
    await rt.close()


async def test_s8b_a_consumer_that_stops_reading_gets_consumer_lagged_and_the_runtime_survives(start: Start, dir: str):
    rt = await start("flood")
    session = await rt.open("mock", dir=dir)
    await session.prompt("go")
    n = 0
    async for _ in session.events():
        n += 1
        if n == 10:
            break
    await asyncio.sleep(2.5)  # 4096 events arrive in ~0.8 s at the flood's pace
    await rejects(until(session, "TurnEnded"), "ConsumerLagged")
    assert (await rt.discover())["agents"][0]["id"] == "mock"
    await rt.close()


async def test_s9_close_with_a_session_open(start: Start, dir: str):
    """`closed` arrives, the process exits 0 and is gone."""
    rt = await start("turn")
    session = await rt.open("mock", dir=dir)
    pid = rt._proc.pid
    code = await rt.close()
    assert code == 0
    assert await drain(session) == [], "closed should end the stream"
    if os.name != "nt":
        with pytest.raises(ProcessLookupError):
            os.kill(pid, 0)


async def test_s10_configure_sends_option_and_session_updated_updates_info(start: Start, dir: str):
    rt = await start("configure")
    session = await rt.open("mock", dir=dir)
    await session.configure("model", "opus")
    await until(session, "SessionUpdated")
    assert session.info["configuration"]["options"]["model"] == "opus"
    await rt.close()


async def test_s11_a_binary_that_does_not_speak_the_protocol_fails_start_with_protocol_failed(tmp_path: Path):
    # python stands in for the binary: sitecustomize prints one stdout line, then exits.
    (tmp_path / "sitecustomize.py").write_text(
        "import os, sys\nsys.stdout.write(os.environ['LINE'] + '\\n')\nsys.stdout.flush()\nos._exit(0)\n"
    )

    def speaks(line: str) -> dict[str, object]:
        return {"bin": sys.executable, "env": {**os.environ, "PYTHONPATH": str(tmp_path), "LINE": line}}

    await rejects(Runtime.start(**speaks("nope")), "ProtocolFailed")  # type: ignore[arg-type]
    await rejects(Runtime.start(**speaks(json.dumps({"hello": {"protocol": 99, "anyagent": "x"}}))), "ProtocolFailed")  # type: ignore[arg-type]
