import {defineConfig} from "vitepress";

export default defineConfig({
    title: "Snakeway",
    description: "Programmable proxy built on top of Pingora.",

    themeConfig: {
        nav: [
            {text: "Guide", link: "/guide/what-is-snakeway"},
            {text: "Getting Started", link: "/getting-started/installation"},
            {text: "Configuration", link: "/configuration/overview"},
            {text: "Devices", link: "/devices/overview"},
            {text: "Roadmap", link: "/roadmap"}
        ],

        sidebar: {
            "/guide/": [
                {
                    text: "Guide",
                    items: [
                        {text: "What is Snakeway?", link: "/guide/what-is-snakeway"},
                        {text: "Why Snakeway Exists", link: "/guide/why-snakeway-exists"},
                        {text: "Mental Model", link: "/guide/mental-model"},
                        {text: "Architecture", link: "/guide/architecture"},
                        {text: "Lifecycle", link: "/guide/lifecycle"}
                    ]
                }
            ],

            "/getting-started/": [
                {
                    text: "Getting Started",
                    items: [
                        {text: "Installation", link: "/getting-started/installation"},
                        {text: "First Proxy", link: "/getting-started/first-proxy"},
                        {text: "Static Files", link: "/getting-started/static-files"},
                        {text: "Reloads", link: "/getting-started/reloads"}
                    ]
                }
            ],

            "/configuration/": [
                {
                    text: "Configuration",
                    items: [
                        {text: "Overview", link: "/configuration/overview"},
                        {text: "Server", link: "/configuration/server"},
                        {text: "Routes", link: "/configuration/routes"},
                        {text: "Devices", link: "/configuration/devices"},
                        {text: "Formats", link: "/configuration/formats"}
                    ]
                }
            ],

            "/devices/": [
                {
                    text: "Devices",
                    items: [
                        {text: "Overview", link: "/devices/overview"},
                        {text: "Execution Order", link: "/devices/execution-order"},
                        {text: "Built-in Devices", link: "/devices/builtin"},
                        {text: "Identity Device", link: "/devices/identity"},
                        {text: "Structured Logging", link: "/devices/structured-logging"},
                        {text: "WASM Devices", link: "/devices/wasm"}
                    ]
                }
            ],

            "/observability/": [
                {
                    text: "Observability",
                    items: [
                        {text: "Logging", link: "/observability/logging"},
                        {text: "Metrics", link: "/observability/metrics"},
                        {text: "Admin API", link: "/observability/admin-api"}
                    ]
                }
            ],

            "/internals/": [
                {
                    text: "Internals",
                    items: [
                        {text: "Request Pipeline", link: "/internals/request-pipeline"},
                        {text: "Threading Model", link: "/internals/threading-model"},
                        {text: "Safety and Sandboxing", link: "/internals/safety-and-sandboxing"},
                        {text: "Design Decisions", link: "/internals/design-decisions"}
                    ]
                }
            ]
        }
    }
});
