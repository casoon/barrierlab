// @ts-check
import casoonPages from '@casoon/pages-theme';
import { defineConfig } from 'astro/config';

// Project page: https://casoon.github.io/a11y-core/ — `base` is the GitHub Pages path.
export default defineConfig({
  site: 'https://casoon.github.io/a11y-core',
  base: '/a11y-core/',
  integrations: [
    casoonPages({
      name: 'a11y-core',
      description:
        'Gemeinsame Rust-Bausteine für Accessibility-Werkzeuge: ein Regelbestand für Build-Zeit, CI und die laufende Seite, mit identischen Regelkennungen und identischem JSON.',
      repo: 'casoon/a11y-core',
      // Die auf crates.io veröffentlichte Fassung.
      version: '0.10.0',
      license: 'MIT',
      packages: [
        { label: 'crates.io', href: 'https://crates.io/crates/a11y-rules' },
        { label: 'docs.rs', href: 'https://docs.rs/a11y-rules' },
      ],
      docsGroups: {
        'getting-started': 'Einstieg',
        concepts: 'Konzepte',
        reference: 'Referenz',
      },
    }),
  ],
});
