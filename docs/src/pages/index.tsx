import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header style={{
      padding: '4rem 0',
      textAlign: 'center',
    }}>
      <div className="container">
        <h1 className="hero__title">{siteConfig.title}</h1>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div style={{display: 'flex', gap: '1rem', justifyContent: 'center', marginTop: '2rem'}}>
          <Link
            className="button button--primary button--lg"
            to="/docs/introduction/getting-started">
            Get Started
          </Link>
          <Link
            className="button button--secondary button--lg"
            to="/docs/configuration/overview">
            Configuration Reference
          </Link>
        </div>
      </div>
    </header>
  );
}

function Features() {
  const features = [
    {
      title: 'Programmable',
      description: 'Compose builtin devices or write your own in WebAssembly. The device pipeline gives you control over every request without sacrificing safety.',
    },
    {
      title: 'Secure by Default',
      description: 'Request smuggling detection, header normalization, network policies, rate limiting, and TLS automation are built in — not bolted on.',
    },
    {
      title: 'Built on Pingora',
      description: 'Powered by Cloudflare\'s battle-tested Rust proxy framework. Async I/O, connection pooling, and HTTP/2 come for free.',
    },
  ];

  return (
    <section style={{padding: '2rem 0'}}>
      <div className="container">
        <div className="row">
          {features.map((feature, idx) => (
            <div key={idx} className="col col--4" style={{marginBottom: '2rem'}}>
              <h3>{feature.title}</h3>
              <p>{feature.description}</p>
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
      description={siteConfig.tagline}>
      <HomepageHeader />
      <main>
        <Features />
      </main>
    </Layout>
  );
}
