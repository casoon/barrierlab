import { docsSchema } from '@casoon/pages-theme/content';
import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';

// Sources: ../docs (docs/ in the project repository). The living project docs at the
// root of docs/ (architecture, conventions, consumers, releasing, project-state — for
// development) stay off the site; only index.md and the group folders are published.
// docs/packages/README.md is the template for a package page, not a page.
export const collections = {
  docs: defineCollection({
    loader: glob({ pattern: ['index.md', '*/**/*.{md,mdx}', '!packages/README.md'], base: '../docs' }),
    schema: docsSchema,
  }),
};
