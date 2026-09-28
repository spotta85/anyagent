# Round 3: account actions

Closes the account rows of [gaps.md](gaps.md) that anyagent can serve from the agent's own process.

```
app ── Runtime::login(agent) ──► agent's own login request ──► url / code ──► done | failed
app ── Runtime::redeem_reset(agent) ──► codex's own request ──► updated usage
```

| # | Task | Agents | Size |
|---|---|---|---|
| 1 | In-app login and logout | codex, claude (where the CLI has a request for it) | ~120 lines |
| 2 | Redeem a banked reset | codex | ~40 lines |

Not built: feedback upload (no app need), claude banked resets (needs the login token; anyagent never reads credentials).

## Global constraints

The round 2 constraints hold. Added for this round:

1. anyagent never sees or stores a password, token or key. It passes on the URL or code the agent prints and reports the outcome.
2. Live tests never touch the user's real login: they run with `config_home` set to an empty temp dir, start a login, read the URL or code, and cancel. No live test calls logout on the real home, completes a login, or redeems a reset.
3. `redeem_reset` spends something that cannot be given back. It is never called by anyagent on its own, and its doc says so.

## Task 1: login and logout

```rust
// src/runtime.rs
/// Starts the agent's own login and reports its steps. Dropping the stream cancels it.
pub async fn login(&self, agent: &AgentInstallation, options: SessionOptions) -> Result<LoginEvents, AgentError>
/// Signs the agent out.
pub async fn logout(&self, agent: &AgentInstallation, options: SessionOptions) -> Result<(), AgentError>

// src/event.rs
pub enum LoginEvent {
    /// Open this URL in a browser; `code` is what to type there, when the agent gives one.
    Open { url: String, code: Option<String> },
    Done,
    Failed { message: String },
}
```

Capability `Login` on agents that serve it; others fail with `UnsupportedFeature("login")` and keep `LoginMethod::Terminal`. Sidecar: `login` (streams `login_event` lines), `login_cancel`, `logout`. Node wrapper: matching methods.

Tests: fixture tests per agent for start, done, failed, cancel. Live: constraint 2.

## Task 2: redeem a banked reset

```rust
// src/runtime.rs
/// Spends one banked reset. Cannot be undone. Returns the usage after it.
pub async fn redeem_reset(&self, agent: &AgentInstallation, options: SessionOptions) -> Result<PlanUsage, AgentError>
```

codex only, through its own request; `InvalidRequest` when no reset is banked (checked from `plan_usage` first, so the request is not sent). Others: `UnsupportedFeature("redeem reset")`. Capability `RedeemReset`. Sidecar `redeem_reset`. Fixture tests only; never live.
