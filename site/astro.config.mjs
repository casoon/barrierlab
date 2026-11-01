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
    // Mermaid blocks become inline SVG at build time, so Shiki must not touch them first.
    // No runtime JavaScript, no external resource — GitHub renders the same source directly.
    syntaxHighlight: { type: 'shiki', excludeLangs: ['mermaid'] },
    rehypePlugins: [[rehypeMermaid, { strategy: 'inline-svg', dark: true }], rehypeMdLinks],
  },
  integrations: [
    casoonPages({
      name: 'BarrierLab',
      description:
        'One rule set for web audits: WCAG 2.2 accessibility rules, HTML conformance, perception and web checks as pure-Rust libraries, with WebAssembly for the browser.',
      tagline: 'Rust libraries for web audits',
      repo: 'casoon/barrierlab',
      lang: 'en',
      license: 'MIT',
      // A monorepo has no shared version and no shared changelog: both live per package.
      changelog: false,
      showcase: false,
      mdxComponents: './src/mdx-components.ts',
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
        '': 'Start',
        packages: 'Packages',
        a11y: 'Accessibility in detail',
        'html-conform': 'Conformance in detail',
      },
    }),
  ],
});
