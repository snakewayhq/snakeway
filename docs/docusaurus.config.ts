import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
    title: 'Snakeway',
    tagline: 'Programmable proxy built with Rust.',
    favicon: 'img/favicon.svg',

    future: {
        v4: true,
    },

    url: 'https://snakeway.dev',
    baseUrl: '/',

    onBrokenLinks: 'warn',

    i18n: {
        defaultLocale: 'en',
        locales: ['en'],
    },

    themes: [
        [
            '@easyops-cn/docusaurus-search-local',
            {
                hashed: true,
                indexBlog: false,
                docsRouteBasePath: '/docs',
            },
        ],
    ],

    plugins: [
        [
            '@docusaurus/plugin-content-blog',
            {
                id: 'releases',
                routeBasePath: 'releases',
                path: './releases',
                blogTitle: 'Release Notes',
                blogDescription: 'Snakeway release notes and changelogs.',
                showReadingTime: false,
                onUntruncatedBlogPosts: 'ignore',
                onInlineTags: 'warn',
                onInlineAuthors: 'warn',
                feedOptions: {
                    type: ['rss', 'atom'],
                    xslt: true,
                },
            },
        ],
    ],

    presets: [
        [
            'classic',
            {
                docs: {
                    sidebarPath: './sidebars.ts',
                    editUrl: 'https://github.com/ethanhann/snakeway/tree/main/docs/',
                    lastVersion: '0.10.0',
                    versions: {
                        current: {
                            label: '0.11.0-dev',
                            banner: 'unreleased',
                        },
                    },
                },
                blog: false,
                theme: {
                    customCss: './src/css/custom.css',
                },
            } satisfies Preset.Options,
        ],
    ],

    themeConfig: {
        image: 'img/logo.svg',
        colorMode: {
            respectPrefersColorScheme: true,
        },
        navbar: {
            title: 'Snakeway',
            logo: {
                alt: 'Snakeway Logo',
                src: 'img/logo.svg',
            },
            items: [
                {
                    type: 'docSidebar',
                    sidebarId: 'docs',
                    position: 'left',
                    label: 'Docs',
                },
                {to: '/releases', label: 'Release Notes', position: 'left'},
                {
                    type: 'docsVersionDropdown',
                    position: 'right',
                },
                {
                    href: 'https://github.com/ethanhann/snakeway',
                    position: 'right',
                    className: 'header-github-link',
                    'aria-label': 'GitHub repository',
                },
            ],
        },
        footer: {
            style: 'dark',
            links: [
                {
                    title: 'Documentation',
                    items: [
                        {
                            label: 'Getting Started',
                            to: '/docs/introduction/getting-started',
                        },
                        {
                            label: 'Configuration',
                            to: '/docs/configuration/overview',
                        },
                    ],
                },
                {
                    title: 'More',
                    items: [
                        {
                            label: 'Release Notes',
                            to: '/releases',
                        },
                        {
                            label: 'GitHub',
                            href: 'https://github.com/ethanhann/snakeway',
                        },
                    ],
                },
            ],
            copyright: `Copyright © ${new Date().getFullYear()} Snakeway. Built with Docusaurus.`,
        },
        prism: {
            theme: prismThemes.github,
            darkTheme: prismThemes.dracula,
            additionalLanguages: ['hcl', 'toml', 'bash', 'rust'],
        },
    } satisfies Preset.ThemeConfig,
};

export default config;
