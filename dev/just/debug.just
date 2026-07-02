# Show max vs used file descriptors
debug-file-descriptors:
    @echo "Max file descriptors:"
    @ulimit -n
    @echo "\nCurrent descriptors in use:"
    @lsof -p $(pgrep snakeway) | wc -l

dump-config:
    cargo run -q --all-features -- config dump|jq

# Report how how often a pooled connection is reused.
diagnose-upstream-reuse requests="5000" concurrency="16": build-jwt-auth-device
    #!/usr/bin/env bash
    # Diagnose upstream connection reuse. Runs a low-volume load with pingora connector debug
    # logging and reports, from the logs, how often a pooled connection is reused vs the
    # try_unwrap race fires ("failed to acquire reusable stream") vs a cold miss (pool empty,
    # "No reusable connection found"). Also probes whether the origin honors keepalive and
    # samples the live upstream connection count. Run at a few concurrency levels (1, 16, 128)
    # to see whether the race scales with concurrency. Keep `requests` modest: connector debug
    # logging is verbose. Prerequisites: docker origin up, snakeway.test -> 127.0.0.1, hey.
    set -euo pipefail
    mkdir -p target/profiling
    log="target/profiling/reuse-c{{ concurrency }}.log"

    echo ">>> Building snakeway (release, no hotpath)"
    cargo build --release -p snakeway

    echo ">>> Origin keepalive probe (two requests over one curl connection)"
    conns="$(curl -s -o /dev/null -w '%{num_connects}' --max-time 5 \
      http://snakeway.test:4000/api/users/1 http://snakeway.test:4000/api/users/1 || true)"
    if [ "${conns}" = "1" ]; then
      echo "    origin: keepalive OK (1 connection served both requests)"
    else
      echo "    origin: no reuse (curl opened ${conns:-?} connections for 2 requests)"
    fi

    echo ">>> Starting snakeway (connectors=debug; logs -> ${log})"
    RUST_LOG="warn,pingora_core::connectors=debug" \
      ./target/release/snakeway run --config ./config >"${log}" 2>&1 &
    server_pid=$!
    cleanup() { kill -KILL "${server_pid}" 2>/dev/null || true; wait "${server_pid}" 2>/dev/null || true; }
    trap cleanup EXIT INT TERM

    echo ">>> Waiting for snakeway to accept requests"
    ready=0
    for _ in $(seq 1 40); do
      code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 1 -H 'Authorization: Bearer probe' http://snakeway.test:8080/api/users/1 || true)"
      if [ -n "${code}" ] && [ "${code}" != "000" ]; then echo "    ready (HTTP ${code})"; ready=1; break; fi
      sleep 0.5
    done
    if [ "${ready}" -ne 1 ]; then echo "!!! snakeway did not become ready" >&2; cat "${log}" >&2; exit 1; fi

    # Valid HS256 token for the JWT device (key "secret", matching config/device.d/wasm.hcl).
    h=$(printf '{"alg":"HS256","typ":"JWT"}' | openssl base64 -A | tr '+/' '-_' | tr -d '=')
    p=$(printf '{"sub":"user-1","iss":"https://auth.example.com","aud":"https://api.example.com","exp":4102444800}' | openssl base64 -A | tr '+/' '-_' | tr -d '=')
    s=$(printf '%s' "$h.$p" | openssl dgst -sha256 -hmac 'secret' -binary | openssl base64 -A | tr '+/' '-_' | tr -d '=')
    token="$h.$p.$s"

    echo ">>> Load: {{ requests }} requests at concurrency {{ concurrency }}"
    hey -n {{ requests }} -c {{ concurrency }} -H "Authorization: Bearer ${token}" \
      http://snakeway.test:8080/api/users/1 >/dev/null 2>&1 &
    load_pid=$!
    peak_conns=0
    while kill -0 "${load_pid}" 2>/dev/null; do
      n=$(lsof -nP -iTCP:4000 -sTCP:ESTABLISHED 2>/dev/null | grep -c snakeway || true)
      if [ "${n:-0}" -gt "${peak_conns}" ]; then peak_conns="${n}"; fi
      sleep 0.5
    done
    wait "${load_pid}" 2>/dev/null || true

    kill -INT "${server_pid}" 2>/dev/null || true
    for _ in $(seq 1 20); do kill -0 "${server_pid}" 2>/dev/null || break; sleep 0.5; done
    kill -KILL "${server_pid}" 2>/dev/null || true
    wait "${server_pid}" 2>/dev/null || true
    trap - EXIT INT TERM

    candidates=$(grep -c "find reusable stream" "${log}" || true)
    misses=$(grep -c "No reusable connection found" "${log}" || true)
    races=$(grep -c "failed to acquire reusable stream" "${log}" || true)
    reused=$(( candidates - races )); if [ "${reused}" -lt 0 ]; then reused=0; fi

    echo ""
    echo "================ upstream reuse summary (c={{ concurrency }}) ================"
    echo "requests sent               : {{ requests }}"
    echo "pool candidates (Some)      : ${candidates}"
    echo "  reused  (try_unwrap ok)   : ~${reused}"
    echo "  race    (try_unwrap err)  : ${races}   <- 'failed to acquire reusable stream (0 is desired)'"
    echo "cold misses (pool empty)    : ${misses}   <- 'No reusable connection found (should match worker count)'"
    echo "peak upstream conns (:4000) : ${peak_conns}"
    echo "log: ${log}"
