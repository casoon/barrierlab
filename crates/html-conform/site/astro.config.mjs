// @ts-check
import casoonPages from '@casoon/pages-theme';
import { defineConfig } from 'astro/config';

// Project page: https://casoon.github.io/html-conform/ — `base` is the GitHub Pages path.
export default defineConfig({
  site: 'https://casoon.github.io/html-conform',
  base: '/html-conform/',
  integrations: [
    casoonPages({
      name: 'html-conform',
      description:
        'HTML5 conformance checking as a Rust dependency, validated against the Nu Html Checker (vnu) test corpus, without a JVM.',
      repo: 'casoon/html-conform',
      version: '0.2.1',
      license: 'MIT',
      branch: 'master',
      packages: [
        { label: 'crates.io', href: 'https://crates.io/crates/html-conform' },
        { label: 'docs.rs', href: 'https://docs.rs/html-conform' },
      ],
      docsGroups: {
        'getting-started': 'Getting started',
        guides: 'Guides',
        reference: 'Reference',
      },
    }),
  ],
});
