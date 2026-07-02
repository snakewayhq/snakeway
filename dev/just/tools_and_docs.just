install-tools: install-dev-tools install-build-tools
    @echo "Finished installing tools"

# If on a Mac, install packages with homebrew.
brew-install packages:
    {{ if os() == "macos" { "brew install " + packages } else { "" } }}

install-build-tools:
    @brew install zig
    @cargo install cargo-component cargo-zigbuild wit-bindgen-cli

install-dev-tools:
    go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
    go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
    just brew-install "grpcurl go"
    bun i -g wscat
    cargo install tokio-console samply cargo-nextest cargo-llvm-cov machete

docs:
    cd docs && npm start

docs-build:
    cd docs && npm run build
