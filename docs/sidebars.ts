import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'category',
      label: 'Introduction',
      items: [
        'introduction/getting-started',
        'introduction/philosophy',
        'introduction/why-snakeway-exists',
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
            'configuration/devices/request-filter',
            'configuration/devices/identity',
            'configuration/devices/network-policy',
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
        'internals/control-plane-and-data-plane',
        'internals/observability',
        'internals/mental-model',
        'internals/lifecycle',
        'internals/configuration',
        'internals/tls-cert-renewal',
      ],
    },
  ],
};

export default sidebars;
