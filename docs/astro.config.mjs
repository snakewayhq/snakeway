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
                    {label: 'Using Devices', link: '/guide/using-devices/'},
                    {label: 'Serving Static Files', link: '/guide/static-files/'},
                    {label: 'Logging', link: '/guide/logging/'},
                    {label: 'Admin API', link: '/guide/admin-api/'},
                ],
            },
            {
                label: 'Configuration Reference', items: [
                    {label: 'Overview', link: '/configuration/overview/'},
                    {label: 'Server', link: '/configuration/server/'},
                    {label: 'Ingress', link: '/configuration/ingress/'},
                    {label: 'Devices', link: '/configuration/devices/'},
                ],
            },
            {
                label: 'Devices', items: [
                    {label: 'Overview', link: '/devices/overview/'},
                    {label: 'Built-in Devices', link: '/devices/builtin/'},
                    {label: 'Request Filter', link: '/devices/request-filter/'},
                    {label: 'Identity', link: '/devices/identity/'},
                    {label: 'Network Policy', link: '/devices/network-policy/'},
                    {label: 'Structured Logging', link: '/devices/structured-logging/'},
                    {label: 'WASM Devices', link: '/devices/wasm/'},
                ],
            },
            {
                label: 'Internals', items: [
                    {label: 'Overview', link: '/internals/overview/'},
                    {label: 'Architecture', link: '/internals/architecture/'},
                    {label: 'Configuration', link: '/internals/configuration/'},
                    {label: 'Lifecycle', link: '/internals/lifecycle/'},
                    {label: 'Mental Model', link: '/internals/mental-model/'},
                ],
            },
        ],
    }),],
})
