# Crates excluded from host unit tests: the binary, the integration suite, and wasm device crates.
# Every other workspace member is a library and is picked up automatically by `--workspace`.
[private]
_skip_non_unit_test_crates := "--exclude snakeway-tests --exclude snakeway-jwt-auth-device --exclude snakeway-wasm-test-device"

start-docker:
    docker compose -f dev/docker-compose-dev.yml up -d --remove-orphans

stop-docker:
    docker compose -f dev/docker-compose-dev.yml down

fetch-pebble-ca: start-docker
    @echo "Fetch Pebble's CA cert..."
    docker cp snakeway-pebble:/test/certs/pebble.minica.pem crates/snakeway-tests/certs/pebble-ca.pem
    @echo "Fetch Pebble's Issuing cert (used to sign HTTP requests)..."
    @curl -k https://localhost:15000/roots/0 > crates/snakeway-tests/certs/pebble-issuing-ca.pem
    @echo "All good."

# Install Snakeway dev CA into macOS System keychain
install-dev-ca:
    @echo "Installing Snakeway dev CA (macOS system trust)..."
    sudo security add-trusted-cert \
      -d -r trustRoot \
      -k /Library/Keychains/System.keychain \
      crates/snakeway-tests/certs/origin-ca.pem
    @echo "✓ Snakeway dev CA installed"

# Remove Snakeway dev CA from macOS System keychain
uninstall-dev-ca:
    @echo "Removing Snakeway dev CA from macOS system trust..."
    sudo security delete-certificate \
      -c "Snakeway Dev Root CA (DO NOT TRUST IN PROD)" \
      /Library/Keychains/System.keychain
    @echo "✓ Snakeway dev CA removed"

generate-dev-certs:
    mkdir -p crates/snakeway-tests/certs/
    @just fetch-pebble-ca
    ./dev/gen-test-certs.sh

# Install mkcert and nss, then create dev certs.
setup-tls-dev-cert:
    just brew-install "mkcert nss"
    mkdir -p ./data/certs/
    mkcert -install
    mkcert \
      -cert-file ./data/certs/snakeway.pem \
      -key-file ./data/certs/snakeway.key \
      localhost 127.0.0.1 ::1

generate-all-templates:
    @mkdir -p data/templates data/acme/orders
    @rm -fr data/templates/*
    @echo "Creating minimal conf..."
    @cargo run -q --all-features -- config init data/templates/minimal --template=minimal
    @echo "\nValidating minimal conf..."
    @cargo run -q --all-features -- config check data/templates/minimal

    @echo "\nCreating dev conf..."
    @cargo run -q --all-features -- config init data/templates/dev --template=dev
    @echo "\nValidating dev conf..."
    @cargo run -q --all-features -- config check data/templates/dev

    @echo "\nCreating httpbin conf..."
    @cargo run -q --all-features -- config init data/templates/httpbin --template=httpbin
    @echo "\nValidating httpbin conf..."
    @cargo run -q --all-features -- config check data/templates/httpbin

# Run unit tests
test:
    cargo nextest run --workspace {{ _skip_non_unit_test_crates }} --all-features

# Run unit tests with coverage, and output HTML
test-with-coverage:
    cargo llvm-cov nextest --workspace {{ _skip_non_unit_test_crates }} --all-features --html --ignore-filename-regex 'pingora|tests/|examples/'
    open target/llvm-cov/html/index.html

# Run unit tests with coverage, and output a summary
test-with-coverage-summary:
    cargo llvm-cov nextest --workspace {{ _skip_non_unit_test_crates }} --all-features --summary-only --ignore-filename-regex 'pingora|tests/|examples/'

# Run integration tests
integration-test *ARGS: fetch-pebble-ca generate-dev-certs build-wasm-test-device build-wasm-jwt-device
    cargo nextest run -p snakeway-tests {{ ARGS }}
