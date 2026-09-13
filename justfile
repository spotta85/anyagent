# anyagent dev commands — run `just` to list them.

# List recipes.
default:
    @just --list --unsorted

# Fast offline checks: format, lint, unit + fixture tests.
check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# List the live feature tests (each is one feature, run via `just live`).
features:
    @grep -A2 '#\[ignore' tests/live.rs | grep -oE 'async fn [a-z_]+' | cut -d' ' -f3

# harness: claude|codex|opencode|hermes|kiro|pi|antigravity|all — feature: substring from `just features`, empty = all
live harness feature='':
    ANYAGENT_LIVE={{harness}} cargo test --test live {{feature}} -- --ignored --nocapture --test-threads=1

# Regenerate packages/schema.json after a wire type changes.
schema:
    cargo run -q --example schema --features schema > packages/schema.json

# Regenerate the Python types from packages/schema.json (the node ones: npm run types).
types:
    uv run --directory packages/python datamodel-codegen --input ../schema.json --input-file-type jsonschema \
        --output anyagent/types.py --output-model-type typing.TypedDict --target-python-version 3.11 \
        --no-use-closed-typed-dict --use-union-operator --formatters builtin \
        --custom-file-header '# Generated from packages/schema.json by `just types`. Do not edit.'

# Discover installed agents and probe what each can do.
list:
    cargo run -- list

# Interactive chat with one agent.
chat harness='claude':
    cargo run -- chat {{harness}}

# Several concurrent sessions, close and resume.
sessions harness='claude':
    cargo run --example sessions -- {{harness}}
