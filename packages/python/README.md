# anyagent

One API over the coding agents installed on a machine: Claude Code, Codex,
Cursor, opencode, Kiro, Grok, Hermes, Qwen, pi, Antigravity. This package
ships the `anyagent` binary inside the wheel (it lands on your venv's PATH),
spawns it, and talks to it over JSON lines. Every rule lives in the binary;
the package is a thin, typed pipe.

```bash
pip install anyagent-py
```

```python
import asyncio, os
from anyagent import Runtime, kind_of

async def main():
    rt = await Runtime.start()
    session = await rt.open("claude", dir=os.getcwd())
    await session.prompt("explain this repo")
    async for ev in session.events():
        kind = kind_of(ev)
        if kind == "TextDelta":
            print(ev["kind"]["TextDelta"]["text"], end="", flush=True)
        if kind == "RequestOpened" and "Permission" in ev["kind"]["RequestOpened"]:
            await session.answer(ev["kind"]["RequestOpened"]["Permission"]["id"], {"Permission": "AllowOnce"})
        if kind == "TurnEnded":
            break
    await session.close()
    await rt.close()

asyncio.run(main())
```

`kind_of(ev)` is the variant name of `ev["kind"]`, for both the object form
`{"TextDelta": {...}}` and the bare-string form `"ContextCompacted"`.

`session.info` and `session.status` stay current. A session error
(`AuthRequired`, `ProcessExited`) raises from the `async for`; a reader that
falls 4096 events behind gets `ConsumerLagged` and its session is closed.

One-shot text, no session: `await rt.generate("codex", "one-line title for this diff", dir=dir)`.

Test your app without agents: build the binary with `--features mock` and
pass `Runtime.start(bin=..., mock="packages/mock-scripts/turn.json")`.

Docs: https://anyagent.mintlify.site/sidecar · Types: `anyagent.types` is
generated from the binary's JSON schema, so commands and events are one contract.
