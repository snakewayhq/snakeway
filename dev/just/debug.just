# Show max vs used file descriptors
debug-file-descriptors:
    @echo "Max file descriptors:"
    @ulimit -n
    @echo "\nCurrent descriptors in use:"
    @lsof -p $(pgrep snakeway) | wc -l

dump-config:
    cargo run -q --all-features -- config dump|jq
