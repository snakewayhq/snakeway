```
   ███████╗███╗   ██╗ █████╗ ██╗  ██╗███████╗██╗    ██╗ █████╗ ██╗   ██╗
   ██╔════╝████╗  ██║██╔══██╗██║ ██╔╝██╔════╝██║    ██║██╔══██╗╚██╗ ██╔╝
   ███████╗██╔██╗ ██║███████║█████╔╝ █████╗  ██║ █╗ ██║███████║ ╚████╔╝ 
   ╚════██║██║╚██╗██║██╔══██║██╔═██╗ ██╔══╝  ██║███╗██║██╔══██║  ╚██╔╝  
   ███████║██║ ╚████║██║  ██║██║  ██╗███████╗╚███╔███╔╝██║  ██║   ██║   
   ╚══════╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝ ╚══╝╚══╝ ╚═╝  ╚═╝   ╚═╝   

            A programmable edge proxy built with Rust.
```

# Snakeway

**Snakeway** is a modern, extensible **L7 reverse proxy** built with **Rust**.

It is designed for engineers who want **control**, **performance**, and **composability** without dragging in a bloated
control plane.

## Documentation

- Conceptual overview: `docs/index.md`
- Architecture and lifecycle docs: `docs/guide/`
- Configuration reference: `docs/configuration/`
    - [Services & Circuit Breaking](docs/configuration/services.md)
    - [Admin API & Observability](docs/configuration/admin.md)

## Status

Pre-1.0.

Actively developed. APIs may shift while foundations are finalized.

## License

Apache 2.0

## Geo IP Database

Any MMDB database is supported, but the default for integration tests is [IP Geolocation by DB-IP](https://db-ip.com).
