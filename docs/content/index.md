---
layout: home

title: Snakeway
titleTemplate: false

hero:
  name: Snakeway
  text: Programmable Edge Proxy
  tagline: A modern, extensible reverse proxy built on Pingora
  image:
    src: /logo.svg
    alt: Snakeway logo
  actions:
    - theme: brand
      text: Get Started
      link: /guide/what-is-snakeway
    - theme: alt
      text: GitHub
      link: https://github.com/snakewayhq/snakeway

features:
  - icon: 🧩
    title: Device-Based Architecture
    details: Intercept, inspect, and modify requests at well-defined lifecycle phases using built-in Rust or WASM devices.

  - icon: 🔄
    title: Hot Reload, Zero Downtime
    details: Reload configuration and devices via signal or admin API without restarting the server.

  - icon: 📁
    title: High-Performance Static Files
    details: Native file serving with ETag, compression, range requests, and cache control. No upstream required.

  - icon: 📊
    title: Observability First
    details: Structured logs and admin endpoints designed for real production debugging.

  - icon: 🦀
    title: Built on Pingora
    details: Leverages Cloudflare’s Pingora 0.6.0 for serious throughput and modern HTTP support.

  - icon: 🛠️
    title: Designed to Evolve
    details: A forward-looking architecture that grows from edge proxy to full traffic intelligence platform.

footer:
  message: Released under the Apache 2.0 License
---
