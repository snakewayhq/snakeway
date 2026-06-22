import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';

function HomepageHeader() {
    const {siteConfig} = useDocusaurusContext();
    return (
        <header className="hero-banner">
            <div className="container">
                <h1>{siteConfig.title}</h1>
                <div style={{display: 'flex', gap: '0.5rem', justifyContent: 'center', margin: '0.75rem 0'}}>
                    <a href="https://github.com/snakewayhq/snakeway/actions/workflows/build.yml">
                        <img
                            src="https://github.com/snakewayhq/snakeway/actions/workflows/build.yml/badge.svg?branch=main"
                            alt="CI"/>
                    </a>
                    <a href="https://github.com/snakewayhq/snakeway/actions/workflows/build.yml">
                        <img src="https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/badge.json"
                             alt="Coverage"/>
                    </a>
                    <a href="https://github.com/snakewayhq/snakeway/actions/workflows/build.yml">
                        <img src="https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/tests-badge.json"
                             alt="Tests"/>
                    </a>
                    <a href="https://github.com/snakewayhq/snakeway/actions/workflows/build.yml">
                        <img
                            src="https://img.shields.io/endpoint?url=https://snakeway.dev/coverage/integration-tests-badge.json"
                            alt="Integration Tests"/>
                    </a>
                </div>
                <p>
                    A programmable reverse proxy built on{' '}
                    <a href="https://github.com/cloudflare/pingora">Pingora</a>.
                    Configure routing, middleware, TLS, and traffic policy with HCL.
                    Extend behavior with WebAssembly devices.
                </p>
                <div style={{display: 'flex', gap: '0.75rem', justifyContent: 'center'}}>
                    <Link
                        className="button button--primary button--lg"
                        to="/docs/introduction/getting-started">
                        Get Started
                    </Link>
                    <Link
                        className="button button--outline button--lg"
                        to="/docs/configuration/overview">
                        Configuration
                    </Link>
                </div>
            </div>
        </header>
    );
}

// Simple monochrome SVG icons (24x24 viewbox, stroke-based)
const icons = {
    pipeline: (
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
             strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="2" x2="12" y2="6"/>
            <rect x="8" y="6" width="8" height="4" rx="1"/>
            <line x1="12" y1="10" x2="12" y2="14"/>
            <rect x="8" y="14" width="8" height="4" rx="1"/>
            <line x1="12" y1="18" x2="12" y2="22"/>
        </svg>
    ),
    shield: (
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
             strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
            <polyline points="9 12 11 14 15 10"/>
        </svg>
    ),
    lock: (
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
             strokeLinecap="round" strokeLinejoin="round">
            <rect x="3" y="11" width="18" height="11" rx="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
            <line x1="12" y1="15" x2="12" y2="18"/>
        </svg>
    ),
    cube: (
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
             strokeLinecap="round" strokeLinejoin="round">
            <path
                d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
            <polyline points="3.27 6.96 12 12.01 20.73 6.96"/>
            <line x1="12" y1="22.08" x2="12" y2="12"/>
        </svg>
    ),
    activity: (
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
             strokeLinecap="round" strokeLinejoin="round">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
        </svg>
    ),
    fileText: (
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5"
             strokeLinecap="round" strokeLinejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="16" y1="13" x2="8" y2="13"/>
            <line x1="16" y1="17" x2="8" y2="17"/>
        </svg>
    ),
};

const features = [
    {
        title: 'Proxying and Routing',
        icon: icons.pipeline,
        description:
            'Route HTTP, HTTPS, WebSocket, and gRPC traffic to upstream services. ' +
            'Longest-path matching maps requests to backends. ' +
            'Serve static files directly without an upstream.',
    },
    {
        title: 'Load Balancing and Health',
        icon: icons.activity,
        description:
            'Distribute traffic across backends with round-robin, weighted, or failover strategies. ' +
            'Circuit breakers and health checks remove unhealthy upstreams automatically.',
    },
    {
        title: 'Automatic TLS',
        icon: icons.lock,
        description:
            'Certificates are issued and renewed automatically through Let\'s Encrypt or any compatible ACME CA. ' +
            'Manual certificate configuration is also supported.',
    },
    {
        title: 'Traffic Policy',
        icon: icons.shield,
        description:
            'Rate limiting, network policy enforcement, identity resolution, and request filtering ' +
            'run as composable middleware in the request pipeline. ' +
            'Request smuggling protection is applied automatically.',
    },
    {
        title: 'Extensibility',
        icon: icons.cube,
        description:
            'Write custom middleware in any language that compiles to WebAssembly. ' +
            'Extensions run in a sandboxed environment with access to request and response context.',
    },
    {
        title: 'Observability',
        icon: icons.fileText,
        description:
            'Structured logging, OpenTelemetry tracing, and an admin API ' +
            'for health checks, upstream status, traffic statistics, and live configuration reload.',
    },
];

function Features() {
    return (
        <section className="features-section">
            <div className="container">
                <div className="row">
                    {features.map((feature, idx) => (
                        <div key={idx} className="col col--4">
                            <div className="feature-card">
                                <div className="feature-card__icon">{feature.icon}</div>
                                <h3>{feature.title}</h3>
                                <p>{feature.description}</p>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </section>
    );
}

export default function Home(): React.JSX.Element {
    const {siteConfig} = useDocusaurusContext();
    return (
        <Layout
            title={siteConfig.title}
            description="Programmable reverse proxy built on Pingora.">
            <HomepageHeader/>
            <main>
                <Features/>
            </main>
        </Layout>
    );
}
