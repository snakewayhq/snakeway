import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'category',
      label: 'Introduction',
      items: [
        'introduction/why-snakeway-exists',
        'introduction/philosophy',
        'introduction/getting-started',
        'introduction/roadmap',
      ],
    },
    {
      type: 'category',
      label: 'Guide',
      items: [
        'guide/cli',
        'guide/tls-cert-management',
        'guide/understanding-devices',
        'guide/authoring-wasm-devices',
        'guide/admin-api',
        'guide/logging',
        'guide/static-files',
      ],
    },
    {
      type: 'category',
      label: 'Configuration Reference',
      items: [
        'configuration/overview',
        'configuration/entry-point',
        'configuration/ingress',
        {
          type: 'category',
          label: 'Devices',
          items: [
            'configuration/devices/identity',
            'configuration/devices/network-policy',
            'configuration/devices/request-filter',
            'configuration/devices/request-rate-limiting',
            'configuration/devices/structured-logging',
          ],
        },
      ],
    },
    {
      type: 'category',
      label: 'Internals',
      items: [
        'internals/architecture',
        'internals/mental-model',
        'internals/lifecycle',
        'internals/control-plane-and-data-plane',
        'internals/configuration',
        'internals/tls-cert-renewal',
        'internals/observability',
      ],
    },
  ],
};

export default sidebars;
