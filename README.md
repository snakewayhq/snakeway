```
   ███████╗███╗   ██╗ █████╗ ██╗  ██╗███████╗██╗    ██╗ █████╗ ██╗   ██╗
   ██╔════╝████╗  ██║██╔══██╗██║ ██╔╝██╔════╝██║    ██║██╔══██╗╚██╗ ██╔╝
   ███████╗██╔██╗ ██║███████║█████╔╝ █████╗  ██║ █╗ ██║███████║ ╚████╔╝ 
   ╚════██║██║╚██╗██║██╔══██║██╔═██╗ ██╔══╝  ██║███╗██║██╔══██║  ╚██╔╝  
   ███████║██║ ╚████║██║  ██║██║  ██╗███████╗╚███╔███╔╝██║  ██║   ██║   
   ╚══════╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝ ╚══╝╚══╝ ╚═╝  ╚═╝   ╚═╝   

            A programmable edge proxy built on Pingora
```

# Snakeway

**Snakeway** is a modern, extensible **L7 reverse proxy** built on **Pingora**.

It is designed for engineers who want **control**, **performance**, and **composability** without dragging in a bloated
control plane.

## Example Configuration

```toml
[server]
listen = "0.0.0.0:8080"

[[route]]
path = "/"
upstream = "http://localhost:3000"

[[route]]
path = "/assets"
file_dir = "./public"
```

## Documentation

- Conceptual overview: `docs/index.md`
- Architecture and lifecycle docs: `docs/guide/`
- Configuration reference: `docs/configuration/`

## Status

Pre-1.0.

Actively developed. APIs may shift while foundations are finalized.

## License

Apache 2.0
