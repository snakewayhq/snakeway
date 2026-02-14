import {defineConfig} from 'astro/config'
import starlight from '@astrojs/starlight'

export default defineConfig({
    site: 'https://snakeway.dev', base: '/',

    integrations: [starlight({
        title: 'Snakeway',
        description: 'Programmable proxy built with rust.',

        // TOC
        tableOfContents: {minHeadingLevel: 2, maxHeadingLevel: 2},

        // Logo
        logo: {
            dark: "./src/assets/logo-dark.svg",
            light: "./src/assets/logo.svg",
            alt: "Snakeway Logo",
        },

        // Styles
        customCss: [
            './src/styles/custom.css',
        ],

        // Sidebar
        sidebar: [
            {
                label: 'Introduction', items: [
                    {label: 'Getting Started', link: '/introduction/getting-started/'},
                    {label: 'Philosophy', link: '/introduction/philosophy/'},
                    {label: 'Why Snakeway Exists', link: '/introduction/why-snakeway-exists/'},
                    {label: 'Roadmap', link: '/introduction/roadmap/'},
                ],
            },
            {
                label: 'Guide', items: [
                    {label: 'CLI', link: '/guide/cli/'},
                    {label: 'Understanding Devices', link: '/guide/understanding-devices/'},
                    {label: 'Authoring WASM Devices', link: '/guide/authoring-wasm-devices/'},
                    {label: 'Admin API', link: '/guide/admin-api/'},
                    {label: 'Logging', link: '/guide/logging/'},
                    {label: 'Serving Static Files', link: '/guide/static-files/'},
                ],
            },
            {
                label: 'Configuration Reference', items: [
                    {label: 'Overview', link: '/configuration/overview/'},
                    {label: 'Server', link: '/configuration/server/'},
                    {label: 'Ingress', link: '/configuration/ingress/'},
                    {
                        label: 'Devices', items: [
                            {label: 'Request Filter', link: '/configuration/devices/request-filter/'},
                            {label: 'Identity', link: '/configuration/devices/identity/'},
                            {label: 'Network Policy', link: '/configuration/devices/network-policy/'},
                            {
                                label: 'Request Rate Limiting',
                                link: '/configuration/devices/request-rate-limiting/'
                            },
                            {label: 'Structured Logging', link: '/configuration/devices/structured-logging/'},
                        ],
                    },
                ],
            },
            {
                label: 'Release Notes', items: [
                    {label: 'v0.7.0 (latest)', link: '/releases/v0_7_0/'},
                    {label: 'v0.6.0', link: '/releases/v0_6_0/'},
                ],
            },
            {
                label: 'Internals', items: [
                    {label: 'Architecture', link: '/internals/architecture/'},
                    {label: 'Mental Model', link: '/internals/mental-model/'},
                    {label: 'HTTP Lifecycle', link: '/internals/lifecycle/'},
                    {label: 'Configuration Subsystem', link: '/internals/configuration/'},
                ],
            },
        ],
    }),],
})
