// Serves the schema at its `$id`; the docs collection only publishes Markdown.
import type { APIRoute } from 'astro';
import schema from '../../../../../../docs/a11y/reference/annotations.schema.json?raw';

export const GET: APIRoute = () =>
  new Response(schema, { headers: { 'Content-Type': 'application/schema+json' } });
