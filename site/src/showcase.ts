import type { ShowcaseExample } from '@casoon/pages-theme/showcase';

// Alle Dateien in examples/ erzeugt `cargo run --manifest-path examples/Cargo.toml` mit den
// Crates dieses Repositorys; die Seite liest sie nur und rechnet nichts nach.
const files = import.meta.glob<string>('../../examples/*.{html,json}', {
  query: '?raw',
  import: 'default',
  eager: true,
});

export function raw(file: string): string {
  const found = files[`../../examples/${file}`];
  if (found === undefined) throw new Error(`Beispiel fehlt: examples/${file}`);
  return found;
}

export interface Finding {
  rule_id: string;
  outcome: 'fail' | 'review' | 'pass' | 'untested';
  severity: 'low' | 'medium' | 'high' | 'critical';
  message: string;
  wcag?: string[];
}

export interface RuleRun {
  rule_id: string;
  not_run?: string;
  findings: number;
  reason?: string;
}

export interface Report {
  findings: Finding[];
  rule_runs: RuleRun[];
  summary: Record<string, number>;
}

export interface Rule {
  id: string;
  tier: 'structure' | 'semantics' | 'rendering';
  wcag: string[];
  severity: string;
  help: string;
}

export const report = (file: string): Report => JSON.parse(raw(file));
export const rules: Rule[] = JSON.parse(raw('rules.json'));

const escape = (s: string) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** Der Bericht als Tabelle: Zählwerk, Befunde, dann die Kennungen, die nicht laufen konnten. */
export function reportHtml(r: Report): string {
  const s = r.summary;
  const rows = r.findings
    .map(
      (f) =>
        `<tr><td><code>${escape(f.rule_id)}</code></td><td>${f.outcome}</td><td>${f.severity}</td><td>${escape((f.wcag ?? []).join(', '))}</td><td>${escape(f.message)}</td></tr>`,
    )
    .join('');
  const findings = r.findings.length
    ? `<div class="scroll" role="region" aria-label="Befunde" tabindex="0" style="overflow-x:auto"><table><thead><tr><th scope="col">Kennung</th><th scope="col">Zustand</th><th scope="col">Schwere</th><th scope="col">WCAG</th><th scope="col">Meldung</th></tr></thead><tbody>${rows}</tbody></table></div>`
    : '<p>Keine Befunde.</p>';
  const notRun = r.rule_runs.filter((x) => x.not_run);
  const notRunHtml = notRun.length
    ? `<p>Nicht gelaufen (<code>${escape(notRun[0].not_run ?? '')}</code>): ${notRun.map((x) => `<code>${escape(x.rule_id)}</code>`).join(', ')}</p>`
    : '';
  return `<div class="cp-prose"><p><code>summary</code>: fail ${s.fail}, review ${s.review}, pass ${s.pass}, untested ${s.untested}, rules_not_run ${s.rules_not_run}</p>${findings}${notRunHtml}</div>`;
}

const catalogue = [
  {
    slug: 'teaser-struktur',
    title: 'Teaser-Seite, nur Struktur',
    page: 'teaser',
    run: 'struktur',
    tags: ['run', 'tier 1'],
    description:
      'Ein Host ohne Semantik. Die Tier-2- und Tier-3-Kennungen stehen als nicht gelaufen im Bericht.',
  },
  {
    slug: 'teaser-semantik',
    title: 'Teaser-Seite, mit Semantik',
    page: 'teaser',
    run: 'semantik',
    tags: ['run_with_semantics', 'tier 1', 'tier 2', 'accname'],
    description:
      'Dasselbe Dokument mit Rolle und Accessible Name aus accname. Jetzt fallen der namenlose Button, das SVG und die zwei „mehr“-Links auf.',
  },
  {
    slug: 'vollstaendig-struktur',
    title: 'Vollständige Seite, nur Struktur',
    page: 'vollstaendig',
    run: 'struktur',
    tags: ['run', 'tier 1'],
    description:
      'Keine Befunde und trotzdem kein „bestanden“: sieben Kennungen konnten ohne Semantik und Darstellung nicht laufen.',
  },
];

export const examples: ShowcaseExample[] = catalogue.map(({ page, run, ...meta }) => {
  const file = `${page}.${run}.json`;
  return {
    ...meta,
    file: `examples/${file}`,
    input: { code: raw(`${page}.html`), lang: 'html' },
    output: { html: reportHtml(report(file)), kind: 'panel' },
  };
});
