//! Drive the packed component through `act run --mcp` with a real MCP client.
//!
//! This replaces the python fastmcp/pytest suite that lives next to this file
//! (left in place for reference): the tests observe exactly what an agent
//! observes, over the same client stack (`rmcp`) the host bridge itself is
//! built on.
//!
//! Env: WASM — path to the packed component (default: the component root's
//!      `python-eval.wasm`, what `just build` produces);
//!      ACT  — the act invocation (default `act`; `npx @actcore/act`, the
//!             component justfile's default, also works — whitespace-split,
//!             like the shlex.split the python conftest did).

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex as AsyncMutex;

/// `().serve(transport)` hands back the client-role service running over the
/// child process: role first, the unit client handler second.
type Client = rmcp::service::RunningService<rmcp::service::RoleClient, ()>;

/// Deliberately loose — the same bound the python conftest's CONNECT_TIMEOUT
/// used. `act run --mcp` instantiates the component before it answers
/// `initialize`, so "connect" includes that cost, and for a bundled-python
/// component on a loaded runner it varies. The bound is the diagnostic that
/// fires first: a stalled handshake must not hang the whole cargo test run.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(120);

fn wasm_path() -> PathBuf {
    PathBuf::from(std::env::var("WASM").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../python-eval.wasm").into()
    }))
}

/// The ACT invocation, honouring the same override the component justfile
/// uses. Its default there is `npx @actcore/act` — two words — which cannot
/// be `argv[0]` for a non-shell spawn, so the value is whitespace-split into
/// program + leading args. Quoted paths with spaces are not a form this
/// fleet passes through `ACT`; a full shlex is deliberately not pulled in.
fn act_argv() -> Vec<String> {
    std::env::var("ACT")
        .unwrap_or_else(|_| "act".into())
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

/// Spawn `act run <wasm> --mcp` — with no grants.
///
/// The component DOES declare a `wasi:filesystem` ceiling (act.toml: `**`,
/// rw — Python code the tool runs may itself open files), but `exec` takes
/// only a `code` string: no test drives a file touch, so every assertion is
/// reachable with no grant at all, and the python conftest launched `act run`
/// bare for exactly that reason. Grants are not optional in general — the
/// default policy mode is `ask` and a headless run degrades it to deny —
/// they are simply not this component's load-bearing part.
fn act_command() -> tokio::process::Command {
    let argv = act_argv();
    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.arg("run").arg(wasm_path()).arg("--mcp");
    cmd
}

fn spawn_transport() -> TokioChildProcess {
    TokioChildProcess::new(act_command()).expect("spawn act run --mcp")
}

/// Spawn with stderr captured: the audit trail (refusals, per-call rollup)
/// writes there unconditionally — RUST_LOG never silences it.
fn spawn_with_captured_stderr() -> (TokioChildProcess, Arc<AsyncMutex<String>>) {
    let (transport, stderr) = TokioChildProcess::builder(act_command())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn act run --mcp with piped stderr");

    let captured = Arc::new(AsyncMutex::new(String::new()));
    let sink = captured.clone();
    let mut lines = BufReader::new(stderr.expect("stderr was piped")).lines();
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            sink.lock().await.push_str(&line);
            sink.lock().await.push('\n');
        }
    });

    (transport, captured)
}

/// Poll the captured stderr until `needle` appears — the audit line is
/// flushed before the JSON-RPC reply, but reaching this buffer still crosses
/// a pipe and an async read.
async fn wait_for_stderr(
    captured: &Arc<AsyncMutex<String>>,
    needle: &str,
    timeout: Duration,
) -> bool {
    let start = std::time::Instant::now();
    loop {
        if captured.lock().await.contains(needle) {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn connect() -> Client {
    // Bound the handshake, not the test body — a stalled connect otherwise
    // consumes the whole cargo run with no diagnostic at all, which is
    // precisely how the webdriver-bidi CI hang presented for hours (the
    // python conftest wrapped the same path in asyncio.timeout for that).
    let served = tokio::time::timeout(CONNECT_TIMEOUT, async { ().serve(spawn_transport()).await })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "rmcp handshake did not complete within {CONNECT_TIMEOUT:?}; \
                 act's stderr, if it wrote any, is lost with the child"
            )
        });
    served.expect("rmcp handshake with act run --mcp")
}

fn first_text_block(result: &rmcp::model::CallToolResult) -> &rmcp::model::TextContent {
    match result.content.first() {
        Some(rmcp::model::ContentBlock::Text(t)) => t,
        other => panic!("expected the first content block to be Text, got: {other:?}"),
    }
}

async fn call_tool(client: &Client, tool: &str, args: Value) -> rmcp::model::CallToolResult {
    // `new` takes a Cow<'static, str>: the &str parameter must be owned up.
    let params = CallToolRequestParams::new(tool.to_string())
        .with_arguments(args.as_object().expect("args are an object").clone());
    let result = client.call_tool(params).await.expect("call_tool");
    assert_ne!(result.is_error, Some(true), "{tool} failed: {result:?}");
    result
}

async fn exec_tool(client: &Client, code: &str) -> rmcp::model::CallToolResult {
    call_tool(client, "exec", json!({ "code": code })).await
}

/// The manifest probe from the python test_info.py: the packed artifact
/// must declare its name and a version. Also the fast-fail the python
/// `wasm_path` fixture provided — `just build` alone (componentize-py +
/// `act-build pack`) can leave a stale wasm behind if it was interrupted,
/// and an unpacked artifact declares no capability ceiling, so every grant
/// would be refused as "outside ceiling" and the failures point anywhere but
/// here. The justfile's `test: build` ordering exists so this test finds a
/// packed artifact.
#[test]
fn manifest_reports_name_and_version() {
    let output = {
        let argv = act_argv();
        let mut cmd = std::process::Command::new(&argv[0]);
        cmd.args(&argv[1..]);
        cmd.args(["inspect", "component-manifest"])
            .arg(wasm_path())
            .output()
            .expect("run act inspect component-manifest")
    };
    assert!(
        output.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: Value = serde_json::from_slice(&output.stdout).expect("manifest is JSON");
    assert_eq!(
        manifest["std"]["name"], "python-eval",
        "packed manifest must carry the component name"
    );
    assert!(
        manifest["std"]["version"].is_string(),
        "packed manifest must carry a version, got: {}",
        manifest["std"]["version"]
    );
}

/// python test_tools.py: `list_tools()` must return at least one tool.
#[tokio::test]
async fn component_exposes_its_tools() {
    let client = connect().await;
    let tools = client.list_all_tools().await.expect("list_all_tools");
    assert!(
        tools.len() >= 1,
        "component must expose at least one tool, got: {:?}",
        tools.iter().map(|t| t.name.to_string()).collect::<Vec<_>>()
    );
    client.cancel().await.ok();
}

/// python test_exec.py: `exec` returns a plain string — measured, the text
/// lands in `content[0].text` and `structured_content` is always None — so
/// every case is a before/after on that one string.
async fn exec_exact(code: &str, expected: &str) {
    let client = connect().await;
    let result = exec_tool(&client, code).await;
    assert_eq!(
        first_text_block(&result).text,
        expected,
        "exec({code:?}) must evaluate to exactly {expected:?}"
    );
    client.cancel().await.ok();
}

/// python test_exec.py: the substring cases — exec-path code that prints,
/// plus the two error-handling cases. Both error kinds still come back as a
/// successful call with the traceback/message embedded in the result text:
/// `exec` catches Python-level failures itself rather than surfacing them as
/// an ACT error (app.py formats any Exception into the returned string), so
/// no test here asserts a `dev.actcore/error-kind` — that is contract, not
/// an omission.
async fn exec_contains(code: &str, needle: &str) {
    let client = connect().await;
    let result = exec_tool(&client, code).await;
    let text = &first_text_block(&result).text;
    assert!(
        text.contains(needle),
        "exec({code:?}) → {text:?} must contain {needle:?}"
    );
    client.cancel().await.ok();
}

/// One test per python parametrize case (test_exec.py EXACT_CASES): pytest
/// ran them as six separate tests, so the rust suite keeps that granularity
/// — six fresh `act` processes, matching the function-scoped client fixture.

/// simple expression (eval path)
#[tokio::test]
async fn exec_exact_simple_expression() {
    exec_exact("2 + 2", "4").await
}

/// string expression
#[tokio::test]
async fn exec_exact_string_expression() {
    exec_exact("'hello' + ' ' + 'world'", "'hello world'").await
}

/// list comprehension
#[tokio::test]
async fn exec_exact_list_comprehension() {
    exec_exact("[x**2 for x in range(5)]", "[0, 1, 4, 9, 16]").await
}

/// no output
#[tokio::test]
async fn exec_exact_no_output() {
    exec_exact("x = 42", "(no output)").await
}

/// string methods
#[tokio::test]
async fn exec_exact_string_method() {
    exec_exact("'Hello, World!'.upper()", "'HELLO, WORLD!'").await
}

/// lambda
#[tokio::test]
async fn exec_exact_lambda_map() {
    exec_exact(
        "list(map(lambda x: x*2, [1,2,3,4,5]))",
        "[2, 4, 6, 8, 10]",
    )
    .await
}

/// One test per python parametrize case (test_exec.py CONTAINS_CASES):
/// thirteen fresh `act` processes, as pytest ran them.

/// print (exec path, stdout capture)
#[tokio::test]
async fn exec_print_stdout_capture() {
    exec_contains("print('hello from python')", "hello from python").await
}

/// multi-line code with variables
#[tokio::test]
async fn exec_multiline_with_variables() {
    exec_contains("x = 10\ny = 20\nprint(x + y)", "30").await
}

/// dictionary
#[tokio::test]
async fn exec_dictionary_literal() {
    exec_contains("{'a': 1, 'b': 2}", "'a': 1").await
}

/// function definition and call
#[tokio::test]
async fn exec_function_definition_and_call() {
    exec_contains(
        "def factorial(n):\n    return 1 if n <= 1 else n * factorial(n-1)\nprint(factorial(10))",
        "3628800",
    )
    .await
}

/// import standard library
#[tokio::test]
async fn exec_import_standard_library() {
    exec_contains("import math\nprint(math.pi)", "3.14159").await
}

/// import json
#[tokio::test]
async fn exec_import_json() {
    exec_contains(
        "import json\nprint(json.dumps({'key': 'value'}))",
        r#"{"key": "value"}"#,
    )
    .await
}

/// exception handling — a runtime error is reported as text, not as an ACT
/// error: the traceback lands in the result under `[error]` (app.py).
#[tokio::test]
async fn exec_python_exception_is_reported_as_text() {
    exec_contains("1 / 0", "ZeroDivisionError").await
}

/// syntax error — same path as the exception: text, not an ACT error.
#[tokio::test]
async fn exec_syntax_error_is_reported_as_text() {
    exec_contains("def (", "SyntaxError").await
}

/// loop with accumulation
#[tokio::test]
async fn exec_loop_with_accumulation() {
    exec_contains(
        "total = 0\nfor i in range(1, 101):\n    total += i\nprint(total)",
        "5050",
    )
    .await
}

/// class definition
#[tokio::test]
async fn exec_class_definition() {
    exec_contains(
        // Written as one literal, without a `\`-continuation: that would
        // strip the leading whitespace of the continuation lines and
        // de-indent `def __repr__` out of the class body.
        "class Point:\n    def __init__(self, x, y):\n        self.x = x\n        self.y = y\n    def __repr__(self):\n        return f'Point({self.x}, {self.y})'\np = Point(3, 4)\nprint(p)",
        "Point(3, 4)",
    )
    .await
}

/// regex
#[tokio::test]
async fn exec_regex() {
    exec_contains(
        "import re\nprint(re.findall(r'\\d+', 'abc123def456'))",
        "['123', '456']",
    )
    .await
}

/// datetime
#[tokio::test]
async fn exec_datetime() {
    exec_contains(
        "from datetime import datetime\nprint(type(datetime.now()).__name__)",
        "datetime",
    )
    .await
}

/// collections
#[tokio::test]
async fn exec_collections_counter() {
    exec_contains(
        "from collections import Counter\nprint(Counter('abracadabra').most_common(3))",
        "('a', 5)",
    )
    .await
}

/// python test_exec_stderr_capture: stderr the user code writes lands in the
/// result under a `[stderr]` header, after any stdout/repr (app.py's part
/// ordering).
#[tokio::test]
async fn exec_stderr_capture() {
    let client = connect().await;
    let result = exec_tool(
        &client,
        "import sys\nprint('error msg', file=sys.stderr)",
    )
    .await;
    let text = &first_text_block(&result).text;
    assert!(text.contains("[stderr]"), "stderr header missing: {text:?}");
    assert!(text.contains("error msg"), "stderr payload missing: {text:?}");
    client.cancel().await.ok();
}

/// Beyond python parity (http-client and crypto keep the same extra): an
/// exec call must leave its per-call rollup on the audit trail, and the
/// captured-stderr plumbing must actually see it.
#[tokio::test]
async fn exec_call_is_audited() {
    let (transport, captured) = spawn_with_captured_stderr();
    let client = ().serve(transport).await.expect("rmcp handshake");

    let result = exec_tool(&client, "2 + 2").await;
    assert_ne!(result.is_error, Some(true), "exec failed: {result:?}");

    assert!(
        wait_for_stderr(&captured, "req:", Duration::from_secs(5)).await,
        "expected a per-call rollup line in the audit trail:\n{}",
        captured.lock().await
    );

    client.cancel().await.ok();
}
