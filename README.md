# Snakeway

**Snakeway** is a modern, extensible **L7 reverse proxy** built with **Rust**.

It is designed for engineers who want **control**, **performance**, and **composability** without dragging in a bloated
control plane.

## Documentation

See https://snakeway.dev/

## Status

Pre-1.0.

Actively developed. APIs may shift while foundations are finalized.

## License

Apache 2.0

## Geo IP Database

Any MMDB database is supported, but the default for integration tests is [IP Geolocation by DB-IP](https://db-ip.com).

## User Agent Parsing

Uses Woothee by default, but also as a secondary options [ua-parser](https://github.com/ua-parser/uap-core).
