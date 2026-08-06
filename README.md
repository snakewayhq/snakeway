# Snakeway

[![CI](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/badge.json)](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml)
[![Tests](https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/tests-badge.json)](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml)
[![Integration Tests](https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/integration-tests-badge.json)](https://github.com/snakewayhq/snakeway/actions/workflows/build.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

**Snakeway** is a modern, extensible **L7 reverse proxy** built with **Rust**.

It is designed for engineers who want **control**, **performance**, and **composability** without dragging in a bloated
control plane.

## Documentation

See https://snakeway.dev/

## Status

Pre-1.0.

Actively developed. APIs may shift while foundations are finalized.

## Contributing

See the [contributing guide](https://snakeway.dev/docs/contributing/overview) on the documentation site.

## License

Apache 2.0

## Request Enrichment

### Geo IP Database

Any MMDB database is supported, but the default for integration tests is [IP Geolocation by DB-IP](https://db-ip.com).

### User Agent Parsing

Uses Woothee by default, but also as a secondary options [ua-parser](https://github.com/ua-parser/uap-core).

## LLM Notice

The software was largely written and architected [by a human](https://ethanhann.com/).

See [LLM_DISCLOSURE.md](LLM_DISCLOSURE.md) for a historical transparency report.

> I wrote a blog post detailing my thoughts on how to be productive with LLMs without falling into the trap of cognitive surrender:
> https://ethanhann.com/blog/what-llms-cannot-speed-up/
>
> ~Ethan, August 5th 2026

