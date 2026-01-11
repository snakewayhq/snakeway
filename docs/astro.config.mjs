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
            dark: "./src/assets/logo.svg",
            light: "./src/assets/logo_black.svg",
            alt: "Snakeway Logo",
        },

        // Sidebar
        sidebar: [
            {
                label: 'Guide', items: [{label: 'What is Snakeway?', link: '/guide/what-is-snakeway/'},
                    {
                        label: 'Why Snakeway Exists', link: '/guide/why-snakeway-exists/'
                    },
                    {label: 'Mental Model', link: '/guide/mental-model/'},
                    {
                        label: 'Architecture', link: '/guide/architecture/'
                    }, {
                        label: 'Lifecycle', link: '/guide/lifecycle/'
                    }, {label: 'Roadmap', link: '/guide/roadmap/'}],
            },
            {
                label: 'Getting Started', items: [{label: 'Installation', link: '/getting-started/installation/'}, {
                    label: 'Configuration', link: '/getting-started/configuration/'
                }, {label: 'Your First Proxy', link: '/getting-started/first-proxy/'}, {
                    label: 'Static Files', link: '/getting-started/static-files/'
                }, {label: 'Reloads', link: '/getting-started/reloads/'},],
            },

            {
                label: 'Configuration', items: [{label: 'Overview', link: '/configuration/overview/'}, {
                    label: 'Admin', link: '/configuration/admin/'
                }, {label: 'Server', link: '/configuration/server/'}, {
                    label: 'Listeners', link: '/configuration/listeners/'
                }, {label: 'Services', link: '/configuration/services/'}, {
                    label: 'Routes', link: '/configuration/routes/'
                }, {label: 'Devices', link: '/configuration/devices/'},],
            }, {
                label: 'Devices', items: [{label: 'Overview', link: '/devices/overview/'}, {
                    label: 'Built-in Devices', link: '/devices/builtin/'
                }, {label: 'Identity', link: '/devices/identity/'}, {
                    label: 'Structured Logging', link: '/devices/structured-logging/'
                }, {label: 'WASM Devices', link: '/devices/wasm/'},],
            },
            {
                label: 'Observability', items: [
                    {label: 'Logging', link: '/observability/logging/'},
                    {label: 'Metrics', link: '/observability/metrics/'},
                    {label: 'Admin API', link: '/observability/admin-api/'},
                ],
            },
            {
                label: 'Internals', items: [
                    {label: 'Configuration', link: '/internals/configuration/'},
                ],
            },
        ],
    }),],
})
