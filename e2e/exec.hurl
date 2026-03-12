# Basic expression evaluation
POST {{host}}/tools/exec
{"arguments": {"code": "2 + 3"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "5"

# Print statement
POST {{host}}/tools/exec
{"arguments": {"code": "print(\"hello world\")"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "hello world"

# String operations
POST {{host}}/tools/exec
{"arguments": {"code": "\"hello\".upper()"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "HELLO"

# List comprehension
POST {{host}}/tools/exec
{"arguments": {"code": "[x**2 for x in range(5)]"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "[0, 1, 4, 9, 16]"

# Multi-line code
POST {{host}}/tools/exec
{"arguments": {"code": "x = 10\ny = 20\nprint(x + y)"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "30"

# Math import
POST {{host}}/tools/exec
{"arguments": {"code": "import math\nprint(math.sqrt(16))"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "4.0"

# Exception handling - division by zero
POST {{host}}/tools/exec
{"arguments": {"code": "1/0"}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "[error]"

# Syntax error
POST {{host}}/tools/exec
{"arguments": {"code": "def f("}}
HTTP 200
[Asserts]
jsonpath "$.content[0].data" contains "[error]"
