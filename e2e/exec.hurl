# Simple expression (eval path)
POST {{baseurl}}/tools/exec
{"arguments": {"code": "2 + 2"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "4"

# String expression
POST {{baseurl}}/tools/exec
{"arguments": {"code": "'hello' + ' ' + 'world'"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "'hello world'"

# Print statement (exec path, stdout capture)
POST {{baseurl}}/tools/exec
{"arguments": {"code": "print('hello from python')"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "hello from python"

# Multi-line code with variables
POST {{baseurl}}/tools/exec
{"arguments": {"code": "x = 10\ny = 20\nprint(x + y)"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "30"

# List comprehension
POST {{baseurl}}/tools/exec
{"arguments": {"code": "[x**2 for x in range(5)]"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "[0, 1, 4, 9, 16]"

# Dictionary
POST {{baseurl}}/tools/exec
{"arguments": {"code": "{'a': 1, 'b': 2}"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "'a': 1"

# Function definition and call
POST {{baseurl}}/tools/exec
{"arguments": {"code": "def factorial(n):\n    return 1 if n <= 1 else n * factorial(n-1)\nprint(factorial(10))"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "3628800"

# Import standard library
POST {{baseurl}}/tools/exec
{"arguments": {"code": "import math\nprint(math.pi)"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "3.14159"

# Import json
POST {{baseurl}}/tools/exec
{"arguments": {"code": "import json\nprint(json.dumps({'key': 'value'}))"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "{\"key\": \"value\"}"

# Exception handling
POST {{baseurl}}/tools/exec
{"arguments": {"code": "1 / 0"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "ZeroDivisionError"

# Syntax error
POST {{baseurl}}/tools/exec
{"arguments": {"code": "def ("}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "SyntaxError"

# Stderr capture
POST {{baseurl}}/tools/exec
{"arguments": {"code": "import sys\nprint('error msg', file=sys.stderr)"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "[stderr]"
jsonpath "$.content[0].data" contains "error msg"

# No output
POST {{baseurl}}/tools/exec
{"arguments": {"code": "x = 42"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "(no output)"

# Loop with accumulation
POST {{baseurl}}/tools/exec
{"arguments": {"code": "total = 0\nfor i in range(1, 101):\n    total += i\nprint(total)"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "5050"

# String methods
POST {{baseurl}}/tools/exec
{"arguments": {"code": "'Hello, World!'.upper()"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "'HELLO, WORLD!'"

# Lambda
POST {{baseurl}}/tools/exec
{"arguments": {"code": "list(map(lambda x: x*2, [1,2,3,4,5]))"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" == "[2, 4, 6, 8, 10]"

# Class definition
POST {{baseurl}}/tools/exec
{"arguments": {"code": "class Point:\n    def __init__(self, x, y):\n        self.x = x\n        self.y = y\n    def __repr__(self):\n        return f'Point({self.x}, {self.y})'\np = Point(3, 4)\nprint(p)"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "Point(3, 4)"

# Regex
POST {{baseurl}}/tools/exec
{"arguments": {"code": "import re\nprint(re.findall(r'\\d+', 'abc123def456'))"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "['123', '456']"

# Datetime
POST {{baseurl}}/tools/exec
{"arguments": {"code": "from datetime import datetime\nprint(type(datetime.now()).__name__)"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "datetime"

# Collections
POST {{baseurl}}/tools/exec
{"arguments": {"code": "from collections import Counter\nprint(Counter('abracadabra').most_common(3))"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "('a', 5)"
