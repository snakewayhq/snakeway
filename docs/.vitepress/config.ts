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
                        // todo {text: "Why Snakeway Exists", link: "/guide/why-snakeway-exists"},
                        {text: "Mental Model", link: "/guide/mental-model"},
                        // todo {text: "Architecture", link: "/guide/architecture"},
                        {text: "Lifecycle", link: "/guide/lifecycle"}
                    ]
                }
            ],

            "/getting-started/": [
                {
                    text: "Getting Started",
                    items: [
                        // todo {text: "Installation", link: "/getting-started/installation"},
                        {text: "Configuration", link: "/getting-started/configuration"},
                        // todo {text: "First Proxy", link: "/getting-started/first-proxy"},
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
                        {text: "Admin", link: "/configuration/admin"},
                        {text: "Server", link: "/configuration/server"},
                        {text: "Listeners", link: "/configuration/listeners"},
                        {text: "Services", link: "/configuration/services"},
                        // todo {text: "Routes", link: "/configuration/routes"},
                        // todo {text: "Devices", link: "/configuration/devices"}
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
                        // todo {text: "WASM Devices", link: "/devices/wasm"}
                    ]
                }
            ],

            "/observability/": [
                {
                    text: "Observability",
                    items: [
                        // todo {text: "Logging", link: "/observability/logging"},
                        // todo {text: "Metrics", link: "/observability/metrics"},
                        // todo {text: "Admin API", link: "/observability/admin-api"}
                    ]
                }
            ],

            "/internals/": [
                {
                    text: "Internals",
                    items: [
                        // todo {text: "Request Pipeline", link: "/internals/request-pipeline"},
                        // todo {text: "Threading Model", link: "/internals/threading-model"},
                        // todo {text: "Safety and Sandboxing", link: "/internals/safety-and-sandboxing"},
                        // todo {text: "Design Decisions", link: "/internals/design-decisions"}
                    ]
                }
            ]
        }
    }
});
