import { ansiToHtml } from '@casoon/pages-theme/ansi';
import type { ShowcaseExample } from '@casoon/pages-theme/showcase';

// html_conform::CheckReport as serialized by serde (src/finding.rs).
interface Finding {
  rule_id: string;
  severity: 'Error' | 'Warning' | 'Info';
  message: string;
  location: { line: number; column: number; byte_offset: number } | null;
}
interface CheckReport {
  findings: Finding[];
}

// Each sample document next to the report html-conform produced for it. The reports are
// written by `cargo run --example findings` (examples/findings.rs) and committed, so the
// site shows the crate's own output without building Rust.
const inputs = import.meta.glob<string>('../../examples/showcase/*.html', {
  query: '?raw',
  import: 'default',
  eager: true,
});
const reports = import.meta.glob<CheckReport>('../../examples/showcase/*.json', {
  import: 'default',
  eager: true,
});

function load<T>(files: Record<string, T>, path: string): T {
  const found = files[path];
  if (found === undefined) {
    throw new Error(`Missing ${path}: run \`cargo run --example findings\``);
  }
  return found;
}

export const input = (slug: string) => load(inputs, `../../examples/showcase/${slug}.html`);
export const report = (slug: string) => load(reports, `../../examples/showcase/${slug}.json`);

const colour = { Error: '1;31', Warning: '1;33', Info: '1;36' } as const;
const sgr = (code: string, text: string) => `\x1b[${code}m${text}\x1b[0m`;
const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;

function wrap(text: string, width: number): string[] {
  const lines: string[] = [];
  let line = '';
  for (const word of text.split(' ')) {
    if (line && line.length + 1 + word.length > width) {
      lines.push(line);
      line = word;
    } else {
      line = line ? `${line} ${word}` : word;
    }
  }
  if (line) lines.push(line);
  return lines;
}

/**
 * A report as terminal lines, one block per finding in the order html-conform returned them:
 * severity, line:column, rule ID, then the message.
 */
export function formatReport({ findings }: CheckReport, width = 72): string {
  if (findings.length === 0) return `${sgr('1;32', 'no findings')}  report.findings is empty`;
  const lines = findings.flatMap((finding) => {
    const at = finding.location ? `${finding.location.line}:${finding.location.column}` : '-';
    return [
      `${sgr(colour[finding.severity], finding.severity.toLowerCase().padEnd(8))}${sgr('2', at.padEnd(7))}${finding.rule_id}`,
      ...wrap(finding.message, width - 2).map((line) => `  ${line}`),
    ];
  });
  const count = (severity: Finding['severity']) =>
    findings.filter((finding) => finding.severity === severity).length;
  const summary = [plural(count('Error'), 'error'), plural(count('Warning'), 'warning')];
  if (count('Info') > 0) summary.push(plural(count('Info'), 'info'));
  lines.push('', summary.join(', '));
  return lines.join('\n');
}

const catalogue = [
  {
    slug: 'valid-document',
    title: 'A conforming page',
    description:
      'A small page with a time element and a srcset. Nothing to report: the findings list is empty.',
  },
  {
    slug: 'parse-errors',
    title: 'Parser errors',
    description:
      'An unknown named character reference and a duplicate attribute. The HTML5 parser recovers from both and reports them as parser.html5.',
  },
  {
    slug: 'content-model',
    title: 'Content model',
    description:
      'A span directly inside a list and the obsolete align attribute fail the RELAX NG schema; a footer inside a footer is caught by a Schematron rule.',
  },
  {
    slug: 'attribute-values',
    title: 'Attribute values',
    description:
      'A media query, a language tag, a date, an image candidate string, a URL and an input type, each rejected with the offending value.',
  },
  {
    slug: 'aria-and-ids',
    title: 'ARIA and IDs',
    description:
      'A focusable element hidden with aria-hidden, a checkbox role without its required state, and an id used twice.',
  },
  {
    slug: 'table-grid',
    title: 'Table cell grid',
    description:
      'A rowspan and a colspan that claim the same slot, found by laying the table out as a grid. The invalid scope value is reported by the schema and by a Schematron rule.',
  },
  {
    slug: 'csp-and-scripts',
    title: 'CSP and script contents',
    description:
      "A meta Content-Security-Policy without 'unsafe-inline' turns every inline script into a warning. The import map has an unknown key, and the speculation rule gives urls as a string.",
  },
];

export const examples: ShowcaseExample[] = catalogue.map(({ slug, title, description }) => {
  const result = report(slug);
  const rules = [...new Set(result.findings.map((finding) => finding.rule_id))];
  return {
    slug,
    title,
    description,
    file: `examples/showcase/${slug}.html`,
    tags: rules.length > 0 ? rules : ['no findings'],
    input: { code: input(slug), lang: 'html' },
    output: { html: ansiToHtml(formatReport(result)), kind: 'terminal' },
  };
});
