"""Shared fixtures for the MCP-driven e2e suite.

The suite drives the packed component through `act run --mcp` over stdio with
a real MCP client, so what the tests observe is what an agent observes.
"""

import json
import os
import shlex
import subprocess
import pytest
from pathlib import Path

from fastmcp import Client
from fastmcp.client.transports import StdioTransport

# Measured in docs/specs/2026-08-08-e2e-harness-findings.md, question 1.
from mcp.shared.exceptions import McpError

WASM = "python-eval.wasm"

# ACT's audit trail writes to stderr unconditionally — it is not governed by
# RUST_LOG — so it is redirected to a file rather than left to flood pytest.
LOG_FILE = Path(".pytest-act-stderr.log")


@pytest.fixture(scope="session")
def act_command() -> list[str]:
    """The ACT invocation, honouring the same override the justfile uses.

    Parsed with shlex, not treated as a single path: the justfile's own
    default for its `act` variable is `npx @actcore/act` — two words — which
    cannot be `argv[0]` for a non-shell `subprocess.run`/`StdioTransport`
    call. A bare `os.environ.get("ACT", "act")` string breaks that default;
    splitting it is what makes both forms ("act" on PATH, and the npx
    two-word default) actually spawn.
    """
    return shlex.split(os.environ.get("ACT", "act"))


@pytest.fixture(scope="session")
def wasm_path(act_command: list[str]) -> Path:
    """The packed component.

    Existence is not enough and neither is a fresh mtime: `just build` alone
    (componentize-py + `act-build pack`) can leave a stale wasm behind if it
    was interrupted, and an unpacked artifact declares no capability ceiling,
    so every grant is refused as "outside ceiling" and the failures point
    anywhere but here. This has already bitten repeatedly in this workspace,
    so the fixture checks the section rather than the file.
    """
    path = Path(WASM)
    if not path.exists():
        pytest.fail(f"{path} is missing — run `just build` first")
    probe = subprocess.run(
        [*act_command, "inspect", "component-manifest", str(path)],
        capture_output=True,
        text=True,
    )
    name = json.loads(probe.stdout or "{}").get("std", {}).get("name", "unknown")
    if name in ("", "unknown"):
        pytest.fail(f"{path} is built but not packed — run `just build` again")
    return path


@pytest.fixture
async def client(act_command: list[str], wasm_path: Path):
    """An ungranted MCP client, one `act` process per test.

    No grant is passed, deliberately: every one of the old hurl suite's 24
    assertions is reachable without one — `exec` takes only a `code` string,
    no `path`/`data` source argument, so none of them touch the
    `wasi:filesystem` grant the component declares (full read-write, for
    Python code that itself opens files). Each call also gets a fresh
    Python namespace on the guest side (see `app.py`), so a fresh `act`
    process per test is not load-bearing for isolation the way it is for a
    stateful component — it is still the safe default, matching
    crypto/filesystem.
    """
    transport = StdioTransport(
        command=act_command[0],
        args=[*act_command[1:], "run", str(wasm_path), "--mcp"],
        keep_alive=False,
        log_file=LOG_FILE,
    )
    async with Client(transport) as connected:
        yield connected


@pytest.fixture
def expect_error():
    """Assert a call fails with a specific ACT error kind.

    Exposed as a fixture rather than a plain function so tests never have to
    import from `conftest` — that import only resolves when the test
    directory happens to be on `sys.path`, which is not something to rely on.

    Measured, not assumed. `call-tool` in `act:tools` returns a bare
    `tool-result` with NO `result<>` wrapper — only `list-tools` has one — so
    a guest reporting a failed tool call can only do it through
    `tool-event::error`, which arrives as a result with `is_error` set and the
    kind in `_meta`. **That is the path a tool test will take.**

    The JSON-RPC error path exists for failures that are not the guest's tool
    body: `list-tools`, the session operations, a wasmtime trap, an
    unreachable actor. It raises `mcp.shared.exceptions.McpError` with the
    payload at `exc.error.data`. No tool test in this suite is expected to
    reach it — `exec` reports Python exceptions as text in its result rather
    than as an ACT error (see `app.py`: any `Exception` is caught and
    formatted into the returned string) — but both are handled here so
    callers need not care.
    """

    async def _expect(client, tool: str, arguments: dict, kind: str):
        try:
            result = await client.call_tool(tool, arguments, raise_on_error=False)
        except McpError as exc:
            data = getattr(getattr(exc, "error", None), "data", None) or {}
            assert data.get("dev.actcore/error-kind") == kind, (
                f"expected {kind} on the JSON-RPC error path, got {data!r}"
            )
            return

        assert result.is_error, f"expected {tool} to fail, got {result!r}"
        meta = result.meta or {}
        assert meta.get("dev.actcore/error-kind") == kind, (
            f"expected {kind} on the isError path, got {meta!r}"
        )

    return _expect
