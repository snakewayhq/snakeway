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

const features = [
  {
    title: 'Device Pipeline',
    description:
      'Requests pass through a composable pipeline of devices. ' +
      'Each device inspects or transforms the request at a well-defined point in the lifecycle. ' +
      'Builtin devices handle identity resolution, network policy, rate limiting, and request filtering.',
  },
  {
    title: 'Protocol Safety',
    description:
      'Request smuggling detection (CL.TE, TE.CL, duplicate Content-Length), ' +
      'header normalization, body size enforcement, and Content-Length validation ' +
      'run automatically on every request.',
  },
  {
    title: 'TLS Automation',
    description:
      'ACME certificate issuance and renewal via HTTP-01 challenges. ' +
      'Supports Let\'s Encrypt and compatible CAs. ' +
      'Manual TLS configuration is also available for environments that manage certificates externally.',
  },
  {
    title: 'WebAssembly Extensibility',
    description:
      'Write custom devices in any language that compiles to WASM. ' +
      'Devices run in a sandboxed environment with access to request context. ' +
      'The WIT interface defines the contract between the proxy and user code.',
  },
  {
    title: 'Observability',
    description:
      'Structured logging with field-selectable identity signals, ' +
      'OpenTelemetry tracing, and an admin API for health checks, ' +
      'upstream status, traffic statistics, and configuration reload.',
  },
  {
    title: 'HCL Configuration',
    description:
      'Configuration is split across focused files: server settings, ' +
      'ingress definitions, and device pipelines. ' +
      'Validation runs at load time with clear error messages and source locations.',
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
      <HomepageHeader />
      <main>
        <Features />
      </main>
    </Layout>
  );
}
