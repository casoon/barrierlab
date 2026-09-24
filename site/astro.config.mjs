// @ts-check
import casoonPages from '@casoon/pages-theme';
import { defineConfig } from 'astro/config';
import rehypeMermaid from 'rehype-mermaid';
import rehypeMdLinks from './src/lib/rehype-md-links.mjs';

// Project page: https://casoon.github.io/barrierlab/
export default defineConfig({
  site: 'https://casoon.github.io/barrierlab',
  base: '/barrierlab/',
  markdown: {
    // Mermaid-Blöcke werden zur Bauzeit zu Inline-SVG. Shiki darf sie deshalb
    // nicht vorher anfassen, sonst steht dort nur eingefärbter Quelltext.
    // Kein Laufzeit-JavaScript, keine externe Ressource — dieselbe Quelle
    // rendert GitHub direkt.
    syntaxHighlight: { type: 'shiki', excludeLangs: ['mermaid'] },
    rehypePlugins: [[rehypeMermaid, { strategy: 'inline-svg', dark: true }], rehypeMdLinks],
  },
  integrations: [
    casoonPages({
      name: 'BarrierLab',
      description:
        'Gemeinsame Bibliotheken für Web-Audits: Zugänglichkeit, HTML-Konformanz, Wahrnehmung.',
      repo: 'casoon/barrierlab',
      license: 'MIT',
      // Ein Monorepo hat keine gemeinsame Version und keinen gemeinsamen
      // Changelog: beides steht je Paket.
      changelog: false,
      showcase: false,
      packages: [
        { label: 'a11y-report', href: 'https://crates.io/crates/a11y-report' },
        { label: 'a11y-dom', href: 'https://crates.io/crates/a11y-dom' },
        { label: 'accname', href: 'https://crates.io/crates/accname' },
        { label: 'a11y-rules', href: 'https://crates.io/crates/a11y-rules' },
        { label: 'a11y-perception', href: 'https://crates.io/crates/a11y-perception' },
        { label: 'a11y-wasm', href: 'https://crates.io/crates/a11y-wasm' },
        { label: 'web-checks', href: 'https://crates.io/crates/web-checks' },
        { label: 'html-conform', href: 'https://crates.io/crates/html-conform' },
        { label: '@casoon/a11y-wasm', href: 'https://www.npmjs.com/package/@casoon/a11y-wasm' },
      ],
      docsGroups: {
        '': 'Einstieg',
        packages: 'Pakete',
        a11y: 'Zugänglichkeit im Detail',
        'html-conform': 'Konformanz im Detail',
      },
    }),
  ],
});
