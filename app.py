"""ACT component: Python interpreter.

Exposes an `exec` tool that runs arbitrary Python code.
Each call gets a fresh namespace for isolation.
"""

import io
import sys
import traceback

from act_sdk import component, tool


@component
class PythonEval:
    @tool(description="Execute Python code and return stdout/stderr")
    async def exec(self, code: str) -> str:
        old_stdout, old_stderr = sys.stdout, sys.stderr
        capture_out = io.StringIO()
        capture_err = io.StringIO()
        sys.stdout = capture_out
        sys.stderr = capture_err

        result_value = None
        error_text = None
        try:
            try:
                result_value = eval(
                    compile(code, "<act>", "eval"),
                    {"__builtins__": __builtins__},
                )
            except SyntaxError:
                exec(
                    compile(code, "<act>", "exec"),
                    {"__builtins__": __builtins__},
                )
        except Exception:
            error_text = traceback.format_exc()
        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr

        stdout_text = capture_out.getvalue()
        stderr_text = capture_err.getvalue()

        parts = []
        if stdout_text:
            parts.append(stdout_text)
        if result_value is not None:
            parts.append(repr(result_value))
        if stderr_text:
            parts.append(f"[stderr]\n{stderr_text}")
        if error_text:
            parts.append(f"[error]\n{error_text}")

        return "\n".join(parts) if parts else "(no output)"
