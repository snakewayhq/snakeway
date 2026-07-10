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

> I decided against using LLMs for this project after some initial success using them to flesh out tests, refactoring,
> bug fixes, and updating the docs.
>
> It worked well... at first...
> The more I used LLMs, the more problematic it became.
> I rigorously audit LLM code in this project before tagging a release.
> The amount of review time became wildly disproportionate to design and implementation.
> My mind began to flirt with cognitive surrender, which is counter to why this project exists: because I like writing
> Rust code and am interested in systems development.
>
> All the agent memory files and skills have been removed.
> Any important knowledge they captured has been added to the docs site under a "Contributing" section.
>
> Maybe I'll write a full blog post on this down the road ¯\(ツ)/¯
>
> ~Ethan, July 10th 2026

The software was largely written and architected [by a human](https://ethanhann.com/).

See [LLM_DISCLOSURE.md](LLM_DISCLOSURE.md) for a historical transparency report.

