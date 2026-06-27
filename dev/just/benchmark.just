# Run Criterion microbenchmarks for snakeway-core.
bench:
    cargo bench -p snakeway-core

# Use wrk and hey to test various upstream configurations.
benchmark-proxy:
    @echo "No TLS (wrk)"
    wrk -t4 -c128 -d10s http://localhost:8080/api/users/1
    @echo "With TLS (wrk)"
    wrk -t4 -c128 -d10s https://localhost:8443/api/users/1
    @echo "Raw upstream (wrk)"
    wrk -t4 -c128 -d10s http://localhost:4000/api/users/1
    @echo "NO TLS (hey)"
    hey -n 20000 -c 128 http://127.0.0.1:8080/api/users/1
    @echo "Raw upstream (hey)"
    hey -n 20000 -c 128 http://127.0.0.1:4000/api/users/1

# Run hey to test out various static file request configs.
benchmark-static-files:
    @echo "No compression..."
    hey -n 20000 -c 128 -H "Accept-Encoding: gzip" http://127.0.0.1:8080/assets/
    @echo "Gzip..."
    hey -n 20000 -c 128 -H "Accept-Encoding: gzip" http://127.0.0.1:8080/assets/1kb.html
    @echo "Brotli..."
    hey -n 20000 -c 128 -H "Accept-Encoding: br" http://127.0.0.1:8080/assets/6kb.html
    @echo "Range request..."
    hey -n 5000 -c 128 -H "Range: bytes=0-99" http://127.0.0.1:8080/assets/images/1mb.png
    @echo "Head request..."
    hey -n 20000 -c 128 -m HEAD -H "Accept-Encoding: gzip" http://127.0.0.1:8080/assets/index.html

# Start this profile recipe, then run run-load. When this command exits, the results should be displayed.
profile:
    @echo "Building Snakeway (release, with symbols)"
    cargo build --release -p snakeway
    @echo "Profiling with Samply (Ctrl+C to stop)"
    samply record target/release/snakeway > /dev/null

# Start this profile recipe, then run "tokio-console" at the cli
profile-tokio:
    RUSTFLAGS="--cfg tokio_unstable" TOKIO_CONSOLE=1 RUST_LOG=error cargo run -r -p snakeway > /dev/null

# Generate meaningful profiling data against an upstream.
run-load-against-upstream:
    hey -n 400000 -c 256 http://127.0.0.1:8080/api/users/1

# Generate meaningful profiling data against a static file.
run-load-against-static:
    hey -n 400000 -c 256 http://127.0.0.1:8080/assets/index.html

# Generate some spoofed traffic for the identity device
run-spoofed-traffic:
    k6 run --vus 10 --duration 30s ./dev/k6/spoof-traffic.js

run-load-test:
    k6 run --vus 10 --duration 30s ./dev/k6/load-test.js

# Check all origin protocols (TCP + UDS)
sanity-check-origin:
    @echo "TCP ================"

    @echo "\nHTTP:"
    @curl -s http://localhost:4000/

    @echo "\nHTTPS:"
    @curl -s --cacert crates/snakeway-tests/certs/origin-ca.pem https://localhost:4443/

    @echo "\nWS:"
    @(echo "Hello, websocket." | wscat -c ws://localhost:4000/ws)

    @echo "\nWSS:"
    @(echo "Hello, secure websocket." | NODE_EXTRA_CA_CERTS=crates/snakeway-tests/certs/origin-ca.pem wscat -c wss://localhost:4443/ws)

    @echo "\ngRPC:"
    @grpcurl \
    	-cacert crates/snakeway-tests/certs/origin-ca.pem \
    	-d '{"id":"123"}' \
    	localhost:5051 \
    	users.UserService/GetUser

    @echo "\n UDS ================"

    @echo "\nHTTP (plaintext) over UDS:"
    @curl -s --unix-socket /tmp/snakeway-http-0.sock http://localhost/

    @echo "\nHTTPS (TLS) over UDS:"
    @curl -s \
    	--unix-socket /tmp/snakeway-https-0.sock \
    	--cacert crates/snakeway-tests/certs/origin-ca.pem \
    	https://localhost/

sanity-check-snakeway:
    @echo "\n\nStatic file over HTTPS ================"
    curl -s --cacert crates/snakeway-tests/certs/origin-ca.pem https://127.0.0.1:8443/assets/index.html
    @echo "\n\nService over HTTPS ================"
    curl -s --cacert crates/snakeway-tests/certs/origin-ca.pem https://127.0.0.1:8443/api/users/1
    @echo "\n\nAdmin stats ================"
    curl -s --cacert crates/snakeway-tests/certs/origin-ca.pem https://127.0.0.1:8440/admin/stats

# Profile WASM with hotpath
profile-wasm-run features="hotpath" duration="60s" vus="200": build-jwt-auth-device
    #!/usr/bin/env bash
    # Hotpath profiling core: build snakeway with the given feature(s), run the JWT load
    # test, then SIGINT the server so its hotpath report prints. Used by profile-wasm-timing
    # and profile-wasm-alloc. Prerequisites: docker origin up, snakeway.test -> 127.0.0.1,
    # k6 installed, and the JWT device enabled in config/device.d/wasm.hcl.
    set -euo pipefail
    features="{{ features }}"
    mkdir -p target/profiling
    log="target/profiling/hotpath-${features//,/-}.log"

    echo ">>> Building snakeway (release, --features ${features})"
    cargo build --release -p snakeway --features "${features}"

    echo ">>> Checking the origin is reachable"
    if ! curl -s -o /dev/null --max-time 2 http://snakeway.test:4000/; then
      echo "!!! origin http://snakeway.test:4000 is unreachable; start the docker compose origin first." >&2
      exit 1
    fi

    # RUST_LOG keeps startup diagnostics while silencing the per-request connection-reuse flood
    echo ">>> Starting snakeway (config ./config; logs -> ${log})"
    RUST_LOG="warn,pingora_core=off" \
      ./target/release/snakeway run --config ./config >"${log}" 2>&1 &
    server_pid=$!

    cleanup() {
      if kill -0 "${server_pid}" 2>/dev/null; then
        # Force-kill on abort: pingora can busy-loop on shutdown, so do not wait gracefully here.
        kill -KILL "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
      fi
    }
    trap cleanup EXIT INT TERM

    echo ">>> Waiting for snakeway to accept requests"
    ready=0
    for _ in $(seq 1 40); do
      code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 1 \
        -H 'Authorization: Bearer probe' \
        http://snakeway.test:8080/api/users/1 || true)"
      if [ -n "${code}" ] && [ "${code}" != "000" ]; then
        echo "    ready (HTTP ${code})"
        ready=1
        break
      fi
      sleep 0.5
    done
    if [ "${ready}" -ne 1 ]; then
      echo "!!! snakeway did not become ready (re-run with RUST_LOG=info for details):" >&2
      cat "${log}" >&2
      exit 1
    fi

    echo ">>> Running k6 (vus={{ vus }}, duration={{ duration }})"
    k6 run --vus {{ vus }} --duration {{ duration }} dev/k6/jwt-load.js || true

    echo ">>> Stopping snakeway (SIGINT) to flush the hotpath report"
    kill -INT "${server_pid}" 2>/dev/null || true
    # Wait up to 20s for a clean exit (report flushes on shutdown), then force-kill so a
    # pingora shutdown accept-spin cannot hang the recipe forever.
    for _ in $(seq 1 40); do kill -0 "${server_pid}" 2>/dev/null || break; sleep 0.5; done
    if kill -0 "${server_pid}" 2>/dev/null; then
      echo ">>> server still up after 20s (pingora shutdown spin); forcing kill, report may be incomplete" >&2
      kill -KILL "${server_pid}" 2>/dev/null || true
    fi
    wait "${server_pid}" 2>/dev/null || true
    trap - EXIT INT TERM

    echo ""
    echo "hotpath report (--features ${features}):"
    cat "${log}"

# Hotpath TIMING run: produces a timeing table
profile-wasm-timing duration="60s" vus="200":
    @just profile-wasm-run hotpath {{ duration }} {{ vus }}

# Hotpath ALLOCATION run: produces alloc table (its timings are inflated by the allocator wrapper and should be ignored)
profile-wasm-alloc duration="60s" vus="200":
    @just profile-wasm-run hotpath,hotpath-alloc {{ duration }} {{ vus }}

# Foreground hotpath server (manual). Ctrl-C prints the report. Run `just profile-wasm-load` from another shell while this is up. `features` is "hotpath" or "hotpath,hotpath-alloc".
profile-wasm-serve features="hotpath": build-jwt-auth-device
    cargo build --release -p snakeway --features {{ features }}
    @curl -s -o /dev/null --max-time 2 http://snakeway.test:4000/ || { echo "origin http://snakeway.test:4000 unreachable; start the docker compose origin." >&2; exit 1; }
    @echo "snakeway up (--features {{ features }}). Run 'just profile-wasm-load' elsewhere; Ctrl-C here for the report."
    RUST_LOG="warn,pingora_core=off" ./target/release/snakeway run --config ./config

# k6 JWT load against an already-running server (pairs with profile-wasm-serve).
profile-wasm-load duration="60s" vus="200" url="http://snakeway.test:8080/api/users/1":
    k6 run --vus {{ vus }} --duration {{ duration }} -e TARGET_URL={{ url }} dev/k6/jwt-load.js

# Uninstrumented throughput baseline: plain release build (no hotpath), JWT load, k6
# summary only. Use this for true req/sec and thread-scaling numbers, since the hotpath
# builds distort throughput. Set threads in config/snakeway.hcl to compare 1 vs 20.
profile-wasm-baseline duration="60s" vus="200": build-jwt-auth-device
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p target/profiling
    log="target/profiling/baseline.log"

    echo ">>> Building snakeway (release, no hotpath features)"
    cargo build --release -p snakeway

    echo ">>> Checking the origin is reachable"
    if ! curl -s -o /dev/null --max-time 2 http://snakeway.test:4000/; then
      echo "!!! origin http://snakeway.test:4000 is unreachable; start the docker compose origin first." >&2
      exit 1
    fi

    # RUST_LOG keeps startup diagnostics while silencing the per-request connection-reuse
    echo ">>> Starting snakeway (config ./config; logs -> ${log})"
    RUST_LOG="warn,pingora_core=off" \
      ./target/release/snakeway run --config ./config >"${log}" 2>&1 &
    server_pid=$!

    cleanup() {
      if kill -0 "${server_pid}" 2>/dev/null; then
        # Force-kill on abort: pingora can busy-loop on shutdown, so do not wait gracefully here.
        kill -KILL "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
      fi
    }
    trap cleanup EXIT INT TERM

    echo ">>> Waiting for snakeway to accept requests"
    ready=0
    for _ in $(seq 1 40); do
      code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 1 \
        -H 'Authorization: Bearer probe' \
        http://snakeway.test:8080/api/users/1 || true)"
      if [ -n "${code}" ] && [ "${code}" != "000" ]; then
        echo "    ready (HTTP ${code})"
        ready=1
        break
      fi
      sleep 0.5
    done
    if [ "${ready}" -ne 1 ]; then
      echo "!!! snakeway did not become ready (re-run with RUST_LOG=info for details):" >&2
      cat "${log}" >&2
      exit 1
    fi

    echo ">>> Running k6 (vus={{ vus }}, duration={{ duration }}); k6 summary is the result"
    k6 run --vus {{ vus }} --duration {{ duration }} dev/k6/jwt-load.js
