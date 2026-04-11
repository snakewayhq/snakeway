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
    cargo build --release -p snakeway --features static_files
    @echo "Profiling with Samply (Ctrl+C to stop)"
    samply record target/release/snakeway > /dev/null

# Start this profile recipe, then run "tokio-console" at the cli
profile-tokio:
    RUSTFLAGS="--cfg tokio_unstable" TOKIO_CONSOLE=1 RUST_LOG=error cargo run -r -p snakeway --features static_files > /dev/null

# Generate meaningful profiling data against an upstream.
run-load-against-upstream:
    hey -n 400000 -c 256 http://127.0.0.1:8080/api/users/1

# Generate meaningful profiling data against a static file.
run-load-against-static:
    hey -n 400000 -c 256 http://127.0.0.1:8080/assets/index.html

# Generate some spoofed traffic for the identity device
run-spoofed-traffic:
    k6 run --vus 10 --duration 30s ./k6/spoof-traffic.js

run-load-test:
    k6 run --vus 10 --duration 30s ./k6/load-test.js

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
