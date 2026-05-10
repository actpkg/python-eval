# python-eval

> Sandboxed Python interpreter for AI agents. One `.wasm`, no cloud, no Docker.

A self-contained CPython 3.14 interpreter packaged as a WebAssembly component.
Runs untrusted Python code inside a wasmtime sandbox with capability-based
filesystem and network access controlled by the host — not the guest code.

**Slot:** untrusted text processing, parsing, regex, transformations,
config evaluation, plugin systems, agent-generated control logic.
Pair with [`act`](https://github.com/actcore/act-cli) and any MCP-compatible agent.

<!-- TODO: replace with asciinema embed once recorded -->

```bash
$ act call python-eval.wasm exec --args '{"code":"print(sum(range(100)))"}'
4950

$ act run python-eval.wasm --mcp     # serve over MCP for any agent
$ act run python-eval.wasm --http    # serve over ACT-HTTP
```

## Honest limitations

| What works | What doesn't |
|------------|--------------|
| Pure-Python stdlib (`re`, `json`, `datetime`, `itertools`, `csv`, ...) | NumPy, Pandas, SciPy, Matplotlib, scikit-learn |
| Custom pure-Python packages | Any package with C extensions (`cryptography`, `lxml`, `pillow`, ...) |
| File I/O via `wasi:filesystem` (host-mounted paths only) | `dlopen` / shared library loading |
| HTTP via `wasi:http` (host-mediated) | Native threading, `fork`, subprocess |
| Capability-bounded sandbox enforced by host | Persistent state across `exec()` calls (each call = fresh namespace) |

If your agent needs `numpy.array(csv).describe()`, this is the wrong tool —
use [E2B](https://e2b.dev) or [Daytona](https://daytona.io) instead. If your
agent needs to evaluate untrusted text-processing or control-flow code without
spinning up cloud infrastructure, this is exactly what it's for.

**Why no NumPy?** Python C extensions need a working `dlopen` in WASI; that
hasn't landed yet. The NumPy upstream WASI build issue
([numpy/numpy#25859](https://github.com/numpy/numpy/issues/25859)) has been
open and dormant since February 2024, and the only third-party effort
([wasi-wheels](https://github.com/dicej/wasi-wheels)) was abandoned in
December 2023. When upstream support lands, this section gets shorter.

## Tools

| Tool | Description |
|------|-------------|
| `exec` | Execute Python code, return combined stdout/stderr/result/traceback |

Each call gets a fresh namespace — there is no persistent state between calls.
For session-scoped state, see the planned [`act:sessions`](https://github.com/actcore/act-spec/blob/main/spec/ACT-SESSIONS.md) integration.

## Capability model

The component declares `wasi:filesystem` (read/write, all paths). The host
controls what is actually mounted via `--allow-dir guest:host`. The default
when running `act call` / `act run` is **no filesystem access**.

```bash
# No filesystem
act call python-eval.wasm exec --args '{"code":"print(open(\"/etc/passwd\").read())"}'
# → traceback, file not found

# Mount a single host directory
act call python-eval.wasm exec \
  --args '{"code":"import os; print(os.listdir(\"/work\"))"}' \
  --allow-dir /work:./input
```

## Use with LLM agents

Three ready-to-paste system-prompt fragments:

### Strict pure-Python (recommended starting point)

```
You have an `exec(code: str)` tool that runs Python in a sandbox. The sandbox
has Python stdlib only — NO numpy, pandas, scipy, sklearn, matplotlib, or any
package with C extensions. No networking unless the host has explicitly
granted it. State does NOT persist between calls — each exec() runs in a
fresh namespace. Use `import` for stdlib modules (re, json, datetime,
itertools, csv, etc). Print final results; the tool returns combined
stdout/stderr/repr.
```

### Text-processing focus

```
You have an `exec(code: str)` tool for evaluating untrusted Python. Use it
for: regex, string parsing, JSON/CSV/XML transformation, date math, small
algorithmic tasks. Each call is independent — do not assume variables persist.
If the user asks for data analysis with numpy/pandas, explain this tool is
stdlib-only and either (a) suggest manual loops + the csv module, or
(b) decline and recommend a numerical sandbox.
```

### Plugin / DSL evaluator

```
You have an `exec(code: str)` tool that runs Python in a hermetic sandbox.
Use this when the user provides a small Python expression or function to
evaluate against data. Wrap the user's expression in a print() call to
return the result. Do not introduce side effects; the sandbox cannot reach
the network or filesystem unless the host explicitly allows it.
```

## Build

```bash
just init   # fetch WIT deps
just build  # build wasm component
just test   # run e2e tests
```

## License

MIT OR Apache-2.0
