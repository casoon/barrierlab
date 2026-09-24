/**
 * Schreibt relative Links auf `.md`-Dateien in Routen um.
 *
 * Die Dokumentation soll **eine** Quelle für zwei Ansichten sein: GitHub
 * rendert `docs/` direkt und braucht Dateinamen, die Seite braucht Routen.
 * Statt eine der beiden zu opfern, wird zur Bauzeit umgeschrieben.
 *
 * Der Haken ist die Ebene. Eine Seite liegt unter einer Route mit eigenem
 * Segment (`architecture.md` → `/docs/architecture/`), deshalb muss ein
 * Nachbarlink eine Ebene hoch: `conventions.md` → `../conventions/`. Eine
 * Ordner-Startseite (`index.md` → `/docs/`) ist dagegen der Ordner selbst —
 * dort bleibt der Link ohne `../`, sonst zeigt er aus der Doku heraus.
 *
 * Absolute URLs und reine Anker bleiben unangetastet.
 */
export default function rehypeMdLinks() {
  return (tree, file) => {
    const quelle = file?.history?.[0] ?? '';
    const istOrdnerstart = /(^|\/)index\.mdx?$/i.test(quelle);
    walk(tree, istOrdnerstart ? '' : '../');
  };
}

function walk(node, prefix) {
  if (node.tagName === 'a' && typeof node.properties?.href === 'string') {
    node.properties.href = umschreiben(node.properties.href, prefix);
  }
  for (const kind of node.children ?? []) walk(kind, prefix);
}

function umschreiben(href, prefix) {
  if (/^[a-z]+:|^\/\//i.test(href) || href.startsWith('#')) return href;
  const [pfad, anker = ''] = href.split('#');
  if (!/\.mdx?$/.test(pfad)) return href;

  // Nur `index` ist die Seite des Ordners. `README.md` bleibt eine eigene
  // Route (`readme/`) — so zeigt GitHub die Datei weiter automatisch im Ordner
  // an und der Link stimmt trotzdem.
  const route = pfad
    .replace(/\.mdx?$/, '')
    .replace(/(^|\/)index$/i, '$1')
    .replace(/(^|\/)README$/, '$1readme');
  const ziel = route === '' || route.endsWith('/') ? route : `${route}/`;
  return `${prefix}${ziel}${anker ? `#${anker}` : ''}`;
}
