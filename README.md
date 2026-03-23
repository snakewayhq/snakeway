# Snakeway

[![CI](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/badge.json)](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml)
[![Tests](https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/tests-badge.json)](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

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

## LLM Disclosure

The software was largely written and architected by a human.

An LLM was used *mostly* for fleshing out tests, refactoring, bug fixes,
and attempting to keep the docs from drifting.

See [LLM_DISCLOSURE.md](LLM_DISCLOSURE.md) for a full transparency report.

