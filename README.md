# python-interpreter

ACT component that executes arbitrary Python code.

## Tools

### `exec`

Execute Python code and return stdout/stderr.

| Parameter | Type   | Required | Description             |
|-----------|--------|----------|-------------------------|
| `code`    | string | yes      | Python code to execute  |

- Expressions (e.g. `2 + 2`) return their value
- Statements (e.g. `print("hello")`) return captured stdout
- Errors return a Python traceback
- Each call runs in a fresh namespace

## Build

```sh
pip install cbor2 -t lib
rm -f lib/*.so  # remove native extensions (not needed in WASM)
uvx componentize-py -d wit -w act-world componentize -p . -p lib app -o python-interpreter.wasm
```

## Usage

```sh
act-host call python-interpreter.wasm exec --args '{"code": "print(2 + 2)"}'
act-host call python-interpreter.wasm exec --args '{"code": "import math; math.pi"}'
act-host call python-interpreter.wasm exec --args '{"code": "for i in range(5): print(i)"}'
```
