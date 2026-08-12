wasm := "python-eval.wasm"

act := env("ACT", "npx @actcore/act")
actbuild := env("ACT_BUILD", "npx @actcore/act-build")
act-build := env("ACT_BUILD", "npx @actcore/act-build")
registry := env("OCI_REGISTRY", "actpkg.dev/library")

# Fetch WIT deps from the registry (ghcr.io/actcore) into wit/deps/.
# wkg-registry.toml maps the act namespace -> actcore.dev (well-known -> ghcr.io/actcore).
init:
    WKG_CONFIG_FILE=wkg-registry.toml wkg wit fetch --type wit

setup: init
    prek install

build:
    uv run componentize-py -d wit -w component-world componentize app -o {{wasm}}
    {{act-build}} pack {{wasm}}

test:
    ACT="{{act}}" uv run --project e2e pytest e2e/ -v

publish:
    #!/usr/bin/env bash
    set -euo pipefail
    INFO=$({{act}} inspect component-manifest {{wasm}})
    NAME=$(echo "$INFO" | jq -r .std.name)
    VERSION=$(echo "$INFO" | jq -r .std.version)
    OUTPUT=$({{actbuild}} push {{wasm}} "{{registry}}/$NAME:$VERSION" \
      --skip-if-exists \
      --also-tag latest 2>&1) || { echo "$OUTPUT" >&2; exit 1; }
    echo "$OUTPUT"
    DIGEST=$(echo "$OUTPUT" | grep "^Digest:" | awk '{print $2}' || true)
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
      echo "image={{registry}}/$NAME" >> "$GITHUB_OUTPUT"
      echo "digest=$DIGEST" >> "$GITHUB_OUTPUT"
    fi
