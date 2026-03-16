wasm := "python-interpreter.wasm"
act := env("ACT", "act")
port := "3456"
addr := "[::1]:" + port
name := `uv run toml get --toml-path pyproject.toml project.name`
version := `uv version --short`
description := `uv run toml get --toml-path pyproject.toml project.description`

build:
    uv run componentize-py -d wit -w component-world componentize app -o {{wasm}}
    wasm-tools metadata add --name "{{name}}" --version "{{version}}" {{wasm}} -o {{wasm}}
    uv run python3 -c "import cbor2,sys; sys.stdout.buffer.write(cbor2.dumps({'std:name':'{{name}}','std:version':'{{version}}','std:description':'{{description}}'}))" | wasm-custom-section {{wasm}} add act:component
    mv {{wasm}}.out {{wasm}}

test: build
    #!/usr/bin/env bash
    {{act}} serve {{wasm}} --listen "{{addr}}" &
    trap "kill $!" EXIT
    npx wait-on http://[::1]:{{port}}/info
    hurl --test --variable "port={{port}}" e2e/*.hurl
