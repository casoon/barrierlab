# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [Unreleased]

### Changed

- **Geltungsbereich je Regel.** `Meta` trägt ein neues Feld `scope`
  (`Scope::AccessibilityTree`, `Rendered`, `Markup`), und jede Regel läuft über
  eine Sicht auf das Dokument, in der versteckte Teilbäume fehlen: unter
  `aria-hidden="true"` (nur `AccessibilityTree`) und nicht dargestellt —
  `display: none`/`visibility: hidden`, wo der Host Stile liefert, sonst das
  `hidden`-Attribut. Bisher liefen die Regeln auch über versteckte Elemente;
  die Namensberechnung überspringt versteckten Inhalt zu Recht, der Button im
  versteckten Banner bekam also einen leeren Namen und einen `FAIL`. Gemessen
  mit LiveAudit auf barrierlab.eu (237 Seiten, 29.09.2026): 992 solche FAILs
  (`buttons/`, `links/`, `svg/name-missing`, `lists/empty`), danach keiner
  (liveaudit#1). Hosts mit eigenem `is_ignored` ändert das nichts an den
  Tier-2-Regeln, wohl aber an den strukturellen.
- `forms/label-missing` sieht ein nicht dargestelltes `<label>` nicht mehr als
  Label an, wenn der Host Stile liefert (liveaudit#3).
- `keyboard/hidden-focusable` prüft jedes fokussierbare Element unter einem
  Vorfahren mit `aria-hidden="true"`, nicht nur das Element mit dem Attribut
  (axe: `aria-hidden-focus`). Ausgenommen: `inert`, `tabindex` < 0, `<a>` ohne
  `href`, `<input type="hidden">` (liveaudit#4). Der Befundtext nennt den
  Vorfahren.
- Verweise (`aria/reference-missing`, `ids/duplicate`) und die dokumentweiten
  Regeln sehen weiterhin das ganze Markup: `aria-labelledby` darf auf
  Verstecktes zeigen.

### Added

- **Checkliste (`manual/*`, Tier 1).** Kriterien, die keine Maschine
  entscheiden kann, erscheinen je Seite einmal als `UNTESTED`, sobald die Seite
  etwas enthält, für das sie gelten: `media-alternatives` (1.2.1–1.2.5, bei
  Audio/Video), `image-alternatives` (1.1.1, bei Bildern — auch `alt=""`),
  `visual-structure` (1.3.1, bei Text), `use-of-color` (1.4.1, bei Links und
  Feldern), `timing` (2.2.1, bei Feldern oder `meta refresh`),
  `error-handling` (3.3.1, 3.3.3, bei Feldern), `authentication` (3.3.8, bei
  Passwortfeldern oder Captcha-Spuren). Verortet am Wurzelknoten
  (liveaudit#7).
- **Heuristiken (Tier 3, alle `REVIEW`)** über `Rendering::layout` und
  `bounds` (liveaudit#6):
  `order/visual-mismatch` (1.3.2, 2.4.3), `motion/infinite-animation` (2.2.2;
  je Gruppe, entfällt bei einem Bedienelement zum Anhalten),
  `reflow/min-width` (1.4.10), `keyboard/pointer-only` (2.1.1, 4.1.2;
  `cursor: pointer` oder `onclick` ohne Rolle und tabindex — per JavaScript
  angehängte Handler sieht keine Seite von innen), `focus/obscured` (2.4.11),
  `targets/size` (2.5.8, mit der Abstandsausnahme nach Normtext; Checkbox und
  Radio im Label zählen mit dem Label), `focus/indicator-missing` und
  `focus/indicator-unmeasured` (2.4.7; ohne Messung `UNTESTED`).
  Belegt an barrierlab.eu: auf 237 intakten Seiten 28 Zielgrößen-Kandidaten,
  sonst keine Meldung; die absichtlich eingebauten Barrieren Reihenfolge,
  Bewegung, Reflow, Verdeckung und Fokus werden gemeldet.

**Breaking:** `Meta` hat ein Pflichtfeld mehr; die Regeln der
`*_rules()`-Listen laufen im Runner über die interne Sicht. Hosts, die nur
`run_*` und `*_metas()` benutzen, sind nicht betroffen.

## [0.12.2] - 2026-09-29

### Fixed

- `lists/invalid-structure` und `lists/empty` sehen durch `<slot>` hindurch:
  Was unter einem `<slot>` hängt, zählt als Kind der Liste. Web-Komponenten
  bauen ihre Liste im Shadow Root (`<ul><slot></slot></ul>`) und bekommen die
  Einträge aus dem Light DOM (Beleg: sachsen-anhalt.de, `muse-link-list`).
  Voraussetzung ist ein Host, der den flachen Baum liefert — siehe `a11y-dom`.
- `<ul>`/`<ol>` mit einer anderen gültigen Rolle als `list` wird nicht mehr als
  Liste geprüft — bisher galt das nur für `presentation`/`none`. Eine leere
  `<ul role="listbox">` (APG-Autocomplete, Beleg: gov.uk) ist keine leere Liste.

## [0.12.1] - 2026-09-28

### Added

- Feature `de` (Vorgabe: an) für die deutschen Befundtexte und Hinweise.
  Mit `default-features = false` kommen die deutschen Vorlagen gar nicht erst
  in den Binärcode; `Locale::De` und `Meta::help_de` gibt es dann nicht —
  wer Deutsch verlangt, bekommt einen Compile-Fehler statt stillem Englisch.
  Rein additiv: Mit den Vorgaben ändert sich nichts.

## [0.12.0] - 2026-09-28

### Added

- **Befundtexte auf Deutsch.** `Locale` (`En`, `De`; Vorgabe `En`) und die
  Einstiege `run_in`, `run_with_semantics_in`, `run_full_in` und
  `run_with_rendering_in`. Übersetzt ist, was ein Werkzeug einem Menschen zeigt:
  `message`, der Grund eines nicht gelaufenen Vermerks und der Hinweis der
  Deklaration (`Meta::help_de`, `Meta::help_in`). Kennungen, Outcomes,
  Schweregrade und WCAG-Bezüge hängen nicht von der Sprache ab. Die bisherigen
  Einstiege liefern unverändert Englisch.

### Changed

- **`ids/duplicate` folgt WCAG 2.2.** 4.1.1 (Parsing) ist gestrichen; gemeldet
  wird nur noch eine doppelte ID, auf die ein IDREF zeigt (`for`, `form`,
  `list`, `headers` und die ARIA-Verweise) — dann ist die Beziehung nicht
  eindeutig auflösbar. WCAG-Bezug jetzt 4.1.2, wie axe-core `duplicate-id-aria`.
  Eine Dopplung, auf die niemand zeigt, erzeugt keinen Befund mehr. Die Regel
  lag bisher als Filter in auditmysite; hier gilt sie für alle Hosts gleich.
- **Bruch:** Regelfunktionen nehmen die Sprache mit
  (`fn(&D, Locale, &mut Vec<Finding>)`), und `Meta` hat ein weiteres Feld. Wer
  nur die `run*`-Einstiege benutzt, merkt davon nichts.

## [0.11.3] - 2026-09-28

### Changed

- Gleichschritt mit `accname` 0.11.3; keine eigene Änderung.

## [0.11.2] - 2026-09-28

### Changed

- Gleichschritt mit `accname` 0.11.2; keine eigene Änderung.

## [0.11.1] - 2026-09-28

### Changed

- Edition 2024 statt 2021, Edition und MSRV werden jetzt vom Workspace geerbt.
  Rein intern; die öffentliche Fläche ist unverändert.

## [0.11.0] - 2026-09-23

### Changed

- Befundtexte und Regel-Hinweise auf Englisch. Entstanden im Repository
  `casoon/a11y-core` und hier nachgezogen; ab jetzt ist dieses Monorepo die
  einzige Quelle.

## [0.10.2] - 2026-09-23

### Changed

- Das Crate liegt jetzt im Monorepo `casoon/barrierlab`; `repository` und
  `homepage` zeigen dorthin. Keine Änderung an Code oder öffentlicher Fläche.
