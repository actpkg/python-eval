wasm := "python-interpreter.wasm"
act := env("ACT", "act")
port := `python3 -c 'import socket; s=socket.socket(socket.AF_INET, socket.SOCK_STREAM); s.bind(("", 0)); print(s.getsockname()[1]); s.close()'`
addr := "[::1]:" + port
baseurl := "http://" + addr
name := `uv run toml get --toml-path pyproject.toml project.name`
version := `uv version --short`
description := `uv run toml get --toml-path pyproject.toml project.description`

build:
    uv run componentize-py -d wit -w component-world componentize app -o {{wasm}}
    wasm-tools metadata add --name "{{name}}" --version "{{version}}" {{wasm}} -o {{wasm}}
    uv run python3 -c "import cbor2,sys; sys.stdout.buffer.write(cbor2.dumps({'std:name':'{{name}}','std:version':'{{version}}','std:description':'{{description}}'}))" | wasm-custom-section {{wasm}} add act:component
    mv {{wasm}}.out {{wasm}}

test:
    #!/usr/bin/env bash
    {{act}} serve {{wasm}} --listen "{{addr}}" &
    trap "kill $!" EXIT
    npx wait-on {{baseurl}}/info
    hurl --test --variable "baseurl={{baseurl}}" e2e/*.hurl
