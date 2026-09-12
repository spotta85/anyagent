"""The anyagent binary as a Python API: spawn `anyagent serve`, write
command lines, route reply and event lines. Every rule lives in the
binary; this file is a pipe (ticket 13, W1–W10)."""

from __future__ import annotations

import asyncio
import json
import os
import sys
import sysconfig
from asyncio.subprocess import PIPE
from collections.abc import AsyncIterator, Callable, Mapping, Sequence
from typing import Any, Required, TypedDict, Unpack

from .types import (
    AgentDetails,
    AgentRef,
    Answer,
    ConfigValue,
    Delivery,
    DiscoveryReport,
    ErrorBody,
    Event,
    McpServer,
    PermissionMode,
    PlanUsage,
    RollbackScope,
    SessionInfo,
    SessionStatus,
)

__all__ = ["AnyagentError", "OpenOptions", "Runtime", "Session", "kind_of"]

# ---------------------------------------------------------------------------
# PUBLIC TYPES
# ---------------------------------------------------------------------------


class OpenOptions(TypedDict, total=False):
    """What `open` and `generate` accept besides the agent: the `open` command's fields."""

    dir: Required[str]
    resume: str | None
    fork: str | None
    fork_at: str | None
    permission_mode: PermissionMode | None
    mcp_servers: list[McpServer]
    configure: dict[str, ConfigValue]


# ---------------------------------------------------------------------------
# RUNTIME: one `anyagent serve` process
# ---------------------------------------------------------------------------

Settle = Callable[[Any], Any]
Pending = tuple["asyncio.Future[Any]", Settle | None]

# The wire protocol this package speaks; the binary's hello must match.
PROTOCOL = 1
# Longest stdout line accepted; a tool result carrying a big diff is one line.
LINE_LIMIT = 64 << 20


class Runtime:
    """One `anyagent serve` process. Build it with `Runtime.start`."""

    _proc: asyncio.subprocess.Process
    _hello: asyncio.Future[None]
    _exited: asyncio.Task[int | None]

    def __init__(self) -> None:
        self._next = 1
        self._pending: dict[int, Pending] = {}
        self._sessions: dict[str, Session] = {}
        self._dead: AnyagentError | None = None

    @classmethod
    async def start(
        cls, *, bin: str | None = None, mock: str | None = None, env: Mapping[str, str] | None = None
    ) -> Runtime:
        """Spawns the binary; returns after its hello line.

        `bin`: path to the binary; default `ANYAGENT_BIN`, then the wheel's.
        `mock`: a mock script (`packages/mock-scripts/*.json`): no real agents.
        `env`: environment for the binary and the agents it spawns; default this process's.
        """
        rt = cls()
        args = ["serve", "--mock", mock] if mock else ["serve"]
        rt._proc = await asyncio.create_subprocess_exec(
            resolve_binary(bin), *args, stdin=PIPE, stdout=PIPE, env=env, limit=LINE_LIMIT
        )
        rt._hello = asyncio.get_running_loop().create_future()
        rt._exited = asyncio.create_task(rt._read())  # resolves the hello, or fails it on exit (W4)
        await rt._hello
        return rt

    async def discover(self) -> DiscoveryReport:
        return await self.call({"cmd": "discover"})

    async def probe(self, agent: AgentRef) -> AgentDetails:
        return await self.call({"cmd": "probe", "agent": agent})

    async def plan_usage(self, agent: AgentRef) -> PlanUsage:
        return await self.call({"cmd": "plan_usage", "agent": agent})

    async def generate(self, agent: AgentRef, prompt: str, **opts: Unpack[OpenOptions]) -> str:
        """One-shot text with no session to manage: titles, commit messages."""
        return await self.call({"cmd": "generate", "agent": agent, "prompt": prompt, **opts})

    async def open(self, agent: AgentRef, **opts: Unpack[OpenOptions]) -> Session:
        """Opens a session. The Session is registered inside _on_line (W1)."""

        def settle(info: SessionInfo) -> Session:
            session = Session(self, info)
            self._sessions[info["id"]] = session
            return session

        return await self.call({"cmd": "open", "agent": agent, **opts}, settle)

    async def close(self) -> int | None:
        """Graceful and idempotent (W5): close stdin, wait up to 5 s, then kill."""
        if not self._dead:
            self._proc.stdin.close()  # type: ignore[union-attr]
            try:
                await asyncio.wait_for(asyncio.shield(self._exited), 5)
            except TimeoutError:
                self._kill()
        return await self._exited

    # Used by Session; not part of the API.
    def call(self, cmd: dict[str, Any], settle: Settle | None = None) -> asyncio.Future[Any]:
        """Writes one command line; the future gets the reply's `ok`, or `settle(ok)`."""
        fut: asyncio.Future[Any] = asyncio.get_running_loop().create_future()
        if self._dead:
            fut.set_exception(self._dead)  # W4
            return fut
        id = self._next
        self._next += 1
        self._pending[id] = (fut, settle)
        self._proc.stdin.write((json.dumps({"id": id, **cmd}) + "\n").encode())  # type: ignore[union-attr]
        return fut

    async def _read(self) -> int | None:
        """The reader task: routes every stdout line, then reports the exit."""
        async for raw in self._proc.stdout:  # type: ignore[union-attr]
            self._on_line(raw.decode(errors="replace").rstrip("\r\n"))
        code = await self._proc.wait()
        self._on_exit(code)
        return code

    def _on_line(self, line: str) -> None:
        """Routes one stdout line. Synchronous on purpose: W1 depends on it."""
        if self._dead:
            return
        try:
            msg = json.loads(line)
        except ValueError:
            msg = None
        if not isinstance(msg, dict):
            return self._abort(f"not a frame: {line}")
        if "hello" in msg:
            hello = msg["hello"]
            if not isinstance(hello, dict) or hello.get("protocol") != PROTOCOL:
                return self._abort(f"protocol {hello}, this package speaks {PROTOCOL}")
            if not self._hello.done():
                self._hello.set_result(None)
            return
        if isinstance(msg.get("id"), int):
            fut, settle = self._pending.pop(msg["id"], (None, None))
            if fut is None or fut.done():
                return
            if "error" in msg:
                fut.set_exception(AnyagentError(msg["error"]))
            else:
                fut.set_result(settle(msg["ok"]) if settle else msg["ok"])
            return
        if "event" in msg:
            session = self._sessions.get(msg["event"]["session_id"])
            if session:
                session._push(msg["event"])
        elif "session" in msg:  # W3
            session = self._sessions.get(msg["session"])
            if session:
                session._fail(AnyagentError(msg["error"]))
        elif "closed" in msg:
            session = self._sessions.pop(msg["closed"], None)
            if session:
                session._end()

    def _abort(self, why: str) -> None:
        """A binary that does not speak the protocol (W10): kill it; _on_exit fails the rest."""
        self._dead = AnyagentError({"kind": "ProtocolFailed", "message": why})
        self._kill()

    def _on_exit(self, code: int | None) -> None:
        """Process gone: fail everything still waiting (W4)."""
        self._dead = self._dead or AnyagentError(
            {"kind": "ProcessExited", "message": f"anyagent exited ({code})", "status": str(code), "stderr": ""}
        )
        if not self._hello.done():
            self._hello.set_exception(self._dead)
        for fut, _ in self._pending.values():
            if not fut.done():
                fut.set_exception(self._dead)
        self._pending.clear()
        for session in self._sessions.values():
            session._fail(self._dead)
        self._sessions.clear()

    def _kill(self) -> None:
        try:
            self._proc.kill()
        except ProcessLookupError:
            pass  # already gone


# ---------------------------------------------------------------------------
# SESSION: one open session
# ---------------------------------------------------------------------------

# Unread events a session may hold before it is closed as lagging (W6).
CAP = 4096


class Session:
    """One open session: commands in, an ordered event stream out."""

    # Built by Runtime.open; not part of the API.
    def __init__(self, rt: Runtime, info: SessionInfo) -> None:
        self._rt = rt
        self.id: str = info["id"]
        #: Live: replaced on every `SessionUpdated` (W2).
        self.info: SessionInfo = info
        #: Live: replaced on every `StatusChanged` (W2).
        self.status: SessionStatus = info.get("status", "Idle")
        # Events, then one Exception (session error) or None (closed).
        self._queue: asyncio.Queue[Event | Exception | None] = asyncio.Queue()
        self._ended = False
        self._closing: asyncio.Task[None] | None = None

    async def prompt(self, text: str, attachments: Sequence[str] = ()) -> Delivery:
        return await self._call({"cmd": "prompt", "text": text, "attachments": list(attachments)})

    async def answer(self, request: str, answer: Answer) -> None:
        await self._call({"cmd": "answer", "request": request, "answer": answer})

    async def configure(self, option: str, value: ConfigValue) -> None:
        await self._call({"cmd": "configure", "option": option, "value": value})

    async def cancel(self, clear_queue: bool = False) -> None:
        await self._call({"cmd": "cancel", "clear_queue": clear_queue})

    async def dequeue(self, prompt: str) -> None:
        await self._call({"cmd": "dequeue", "prompt": prompt})

    async def rollback(self, turns: int, scope: RollbackScope) -> None:
        await self._call({"cmd": "rollback", "turns": turns, "scope": scope})

    async def compact(self) -> None:
        await self._call({"cmd": "compact"})

    async def close(self) -> None:
        await self._call({"cmd": "close"})

    async def events(self) -> AsyncIterator[Event]:
        """This session's events in order. Ends after `closed`; raises once on a
        session error, then ends (W3)."""
        while not (self._ended and self._queue.empty()):
            item = await self._queue.get()
            if item is None:
                return
            if isinstance(item, Exception):
                raise item
            yield item

    def _call(self, cmd: dict[str, Any]) -> asyncio.Future[Any]:
        return self._rt.call({**cmd, "session": self.id})

    # _push, _fail and _end are called by Runtime._on_line/_on_exit.
    def _push(self, ev: Event) -> None:
        """Keeps info and status live (W2), applies the cap (W6)."""
        if self._ended:
            return
        kind = ev["kind"]
        if isinstance(kind, dict):
            if "SessionUpdated" in kind:
                self.info = kind["SessionUpdated"]
            if "StatusChanged" in kind:
                self.status = kind["StatusChanged"]
        self._queue.put_nowait(ev)
        if self._queue.qsize() > CAP:
            self._fail(AnyagentError({"kind": "ConsumerLagged", "message": f"{CAP} events unread"}))
            self._closing = asyncio.create_task(self._close_quietly())

    def _fail(self, error: Exception) -> None:
        if not self._ended:
            self._ended = True
            self._queue.put_nowait(error)

    def _end(self) -> None:
        if not self._ended:
            self._ended = True
            self._queue.put_nowait(None)

    async def _close_quietly(self) -> None:
        try:
            await self.close()
        except AnyagentError:
            pass


# ---------------------------------------------------------------------------
# HELPERS
# ---------------------------------------------------------------------------


def kind_of(ev: Event) -> str:
    """The variant name of `event["kind"]`, for both `{"TextDelta": {..}}` and `"ContextCompacted"` (W7)."""
    kind = ev["kind"]
    return kind if isinstance(kind, str) else next(iter(kind))


class AnyagentError(Exception):
    """`kind`, `message`, and every extra field from the wire in `data` (W8)."""

    def __init__(self, body: ErrorBody) -> None:
        super().__init__(body["message"])
        self.kind: str = body["kind"]
        self.message: str = body["message"]
        self.data: dict[str, Any] = {k: v for k, v in body.items() if k not in ("kind", "message")}


# ---------------------------------------------------------------------------
# INTERNAL: finding the binary
# ---------------------------------------------------------------------------


def resolve_binary(bin: str | None) -> str:
    """`bin`, then `ANYAGENT_BIN`, then the binary the wheel put on this environment's scripts path."""
    explicit = bin or os.environ.get("ANYAGENT_BIN")
    if explicit:
        return explicit
    exe = "anyagent" + (sysconfig.get_config_var("EXE") or "")
    # The venv or system scripts dir, then `pip install --user`'s.
    for scheme in (sysconfig.get_default_scheme(), f"{os.name}_user"):
        path = os.path.join(sysconfig.get_path("scripts", scheme), exe)
        if os.path.isfile(path):
            return path
    raise FileNotFoundError(f"no anyagent binary next to {sys.executable}: pip install anyagent-py, or set ANYAGENT_BIN")
