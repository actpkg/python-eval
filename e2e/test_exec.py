"""exec: run arbitrary Python and return combined stdout/stderr/result/traceback.

`exec` returns a plain string (measured: `structured_content` is always
`None`, the text lands in `content[0].text`), so every case here is a
before/after on that one string.
"""

import pytest

# Cases whose result is asserted for exact equality.
EXACT_CASES = [
    ("2 + 2", "4"),  # simple expression (eval path)
    ("'hello' + ' ' + 'world'", "'hello world'"),  # string expression
    ("[x**2 for x in range(5)]", "[0, 1, 4, 9, 16]"),  # list comprehension
    ("x = 42", "(no output)"),  # no output
    ("'Hello, World!'.upper()", "'HELLO, WORLD!'"),  # string methods
    ("list(map(lambda x: x*2, [1,2,3,4,5]))", "[2, 4, 6, 8, 10]"),  # lambda
]


@pytest.mark.parametrize("code,expected", EXACT_CASES)
async def test_exec_exact_result(client, code, expected):
    result = await client.call_tool("exec", {"code": code})
    assert result.content[0].text == expected


# Cases whose result is asserted by substring — mostly exec-path code that
# prints, plus the two error-handling cases (an exception and a syntax
# error), both of which still return HTTP 200 with the traceback/message text
# embedded in the result: `exec` catches Python-level failures itself rather
# than surfacing them as an ACT error (see `app.py`).
CONTAINS_CASES = [
    (
        "print('hello from python')",
        "hello from python",
    ),  # print (exec path, stdout capture)
    ("x = 10\ny = 20\nprint(x + y)", "30"),  # multi-line code with variables
    ("{'a': 1, 'b': 2}", "'a': 1"),  # dictionary
    (
        "def factorial(n):\n    return 1 if n <= 1 else n * factorial(n-1)\nprint(factorial(10))",
        "3628800",
    ),  # function definition and call
    ("import math\nprint(math.pi)", "3.14159"),  # import standard library
    (
        "import json\nprint(json.dumps({'key': 'value'}))",
        '{"key": "value"}',
    ),  # import json
    ("1 / 0", "ZeroDivisionError"),  # exception handling
    ("def (", "SyntaxError"),  # syntax error
    (
        "total = 0\nfor i in range(1, 101):\n    total += i\nprint(total)",
        "5050",
    ),  # loop with accumulation
    (
        "class Point:\n    def __init__(self, x, y):\n        self.x = x\n        self.y = y\n"
        "    def __repr__(self):\n        return f'Point({self.x}, {self.y})'\n"
        "p = Point(3, 4)\nprint(p)",
        "Point(3, 4)",
    ),  # class definition
    (
        "import re\nprint(re.findall(r'\\d+', 'abc123def456'))",
        "['123', '456']",
    ),  # regex
    (
        "from datetime import datetime\nprint(type(datetime.now()).__name__)",
        "datetime",
    ),  # datetime
    (
        "from collections import Counter\nprint(Counter('abracadabra').most_common(3))",
        "('a', 5)",
    ),  # collections
]


@pytest.mark.parametrize("code,substring", CONTAINS_CASES)
async def test_exec_result_contains(client, code, substring):
    result = await client.call_tool("exec", {"code": code})
    assert substring in result.content[0].text


async def test_exec_stderr_capture(client):
    result = await client.call_tool(
        "exec", {"code": "import sys\nprint('error msg', file=sys.stderr)"}
    )
    text = result.content[0].text
    assert "[stderr]" in text
    assert "error msg" in text
