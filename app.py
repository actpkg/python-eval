"""ACT component: Python interpreter.

Exposes an `exec` tool that runs arbitrary Python code.
Each call gets a fresh namespace for isolation.
"""

import io
import json
import sys
import traceback

import cbor2
import componentize_py_async_support
import wit_world
from wit_world import exports
from wit_world.imports.types import (
    ComponentInfo,
    ContentPart,
    ListToolsResponse,
    StreamEvent_Content,
    StreamEvent_Error,
    ToolCall,
    ToolDefinition,
    ToolError,
)

EXEC_SCHEMA = json.dumps({
    "type": "object",
    "properties": {
        "code": {
            "type": "string",
            "description": "Python code to execute",
        },
    },
    "required": ["code"],
})


class ToolProvider(exports.ToolProvider):
    def get_info(self) -> ComponentInfo:
        return ComponentInfo(
            name="python-interpreter",
            version="0.1.0",
            default_language="en",
            description=[("en", "Executes arbitrary Python code")],
            capabilities=[],
            metadata=[],
        )

    def get_config_schema(self):
        return None

    async def list_tools(self, config):
        return ListToolsResponse(
            metadata=[],
            tools=[
                ToolDefinition(
                    name="exec",
                    description=[("en", "Execute Python code and return stdout/stderr")],
                    parameters_schema=EXEC_SCHEMA,
                    metadata=[],
                ),
            ],
        )

    async def call_tool(self, config, call: ToolCall):
        writer, reader = wit_world.types_stream_event_stream()

        async def produce():
            try:
                if call.name != "exec":
                    await writer.write([StreamEvent_Error(ToolError(
                        kind="std:not-found",
                        message=[("en", f"Unknown tool: {call.name}")],
                        metadata=[],
                    ))])
                    return

                # Decode CBOR arguments
                args = cbor2.loads(bytes(call.arguments))
                code = args.get("code", "")

                # Capture stdout and stderr
                old_stdout, old_stderr = sys.stdout, sys.stderr
                capture_out = io.StringIO()
                capture_err = io.StringIO()
                sys.stdout = capture_out
                sys.stderr = capture_err

                result_value = None
                error_text = None
                try:
                    # Try eval first (expression), fall back to exec (statements)
                    try:
                        result_value = eval(compile(code, "<act>", "eval"), {"__builtins__": __builtins__})
                    except SyntaxError:
                        exec(compile(code, "<act>", "exec"), {"__builtins__": __builtins__})
                except Exception:
                    error_text = traceback.format_exc()
                finally:
                    sys.stdout = old_stdout
                    sys.stderr = old_stderr

                stdout_text = capture_out.getvalue()
                stderr_text = capture_err.getvalue()

                # Build output text
                parts = []
                if stdout_text:
                    parts.append(stdout_text)
                if result_value is not None:
                    parts.append(repr(result_value))
                if stderr_text:
                    parts.append(f"[stderr]\n{stderr_text}")
                if error_text:
                    parts.append(f"[error]\n{error_text}")

                output = "\n".join(parts) if parts else "(no output)"

                await writer.write([StreamEvent_Content(ContentPart(
                    data=output.encode("utf-8"),
                    mime_type="text/plain",
                    metadata=[],
                ))])

            except Exception:
                await writer.write([StreamEvent_Error(ToolError(
                    kind="std:internal",
                    message=[("en", traceback.format_exc())],
                    metadata=[],
                ))])

        componentize_py_async_support.spawn(produce())
        return reader
