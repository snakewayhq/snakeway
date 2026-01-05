---
title: Snakeway
description: A modern, extensible reverse proxy built with Rust
template: splash
hero:
  title: Snakeway
  subtitle: Programmable Edge Proxy
  tagline: A modern, extensible reverse proxy built with Rust
  image:
    dark: ../../assets/logo.svg
    light: ../../assets/logo_black.svg
    alt: Snakeway logo
  actions:
    - text: Get Started
      link: /getting-started/installation/
    - text: GitHub
      link: https://github.com/snakewayhq/snakeway
      icon: github
      variant: secondary

features:
  - title: Device-Based Architecture
    icon: tabler:puzzle
    description: Intercept, inspect, and modify requests at well-defined lifecycle phases using built-in Rust or WASM devices.

  - title: Hot Reload, Zero Downtime
    icon: tabler:refresh
    description: Reload configuration and devices via signal or admin API without restarting the server.

  - title: High-Performance Static Files
    icon: tabler:folder
    description: Native file serving with ETag, compression, range requests, and cache control. No upstream required.

  - title: Observability First
    icon: tabler:chart-bar
    description: Structured logs and admin endpoints designed for real production debugging.

  - title: Built on Pingora
    icon: tabler:brand-rust
    description: Leverages Cloudflare’s Pingora 0.6.0 for serious throughput and modern HTTP support.

  - title: Designed to Evolve
    icon: tabler:tools
    description: A forward-looking architecture that grows from edge proxy to full traffic intelligence platform.

footer:
  text: Released under the Apache 2.0 License
---
