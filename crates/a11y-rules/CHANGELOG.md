# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [0.14.1] - 2026-10-01

### Fixed

- `lists/invalid-structure`: Ein `<li>` mit einer anderen expliziten Rolle
  als `listitem` (etwa `role="group"` bei Slidern) zählt nicht als
  Listeneintrag. Bis 0.14.0 fing auditmysites eigene `aria-roles`-Regel diesen
  Fall ab; mit B1 ist sie entfallen. Beleg: berlin.de,
  `ul.swiper-wrapper > li[role=group]` (wie axe `list`).

## [0.14.0] - 2026-09-30

Rollen aus WAI-ARIA Graphics (`graphics-document`, `-object`, `-symbol`) und
DPUB-ARIA 1.1 (`doc-*`) gelten für `aria/role-invalid` als gültig; die
Attributregeln urteilen über sie nicht (auditmysite-Korpus
`svg_graphics_role_no_name`).

Die ARIA-Regeln aus auditmysite (casoon/barrierlab#14, B1). Norm ist WAI-ARIA
1.2 und ARIA in HTML; die Rollen- und Attributtabellen sind aus der
Spezifikation erzeugt, als Bitmasken je Rolle.

### Added

| Neue Kennung | Tier | ersetzt in auditmysite |
|---|---|---|
| `aria/attribute-unknown` | 1 | `aria-attr-name-invalid` (`aria_roles`) |
| `aria/attribute-value-invalid` | 1 | `aria-valid-attr-value` (Wertebereiche) |
| `aria/owns-conflict` | 1 | `duplicate-id-aria` (`parsing`) |
| `aria/tab-selected-missing` | 1 | `aria-tab-selected-state`, `tab-no-aria-selected` (`widget_rules`) |
| `aria/tabpanel-missing` | 1 | `aria-tablist-tabpanel` (`widget_rules`) |
| `aria/combobox-popup-missing` | 1 | `aria-combobox-options` (`widget_rules`) |
| `popover/target-missing`, `popover/target-invalid` | 1 | `modern-attribute-misuse` (`modern_attributes`) |
| `inert/dialog-inert` | 1 | `modern-attribute-misuse` (`active_surface_inert`) |
| `aria/attribute-not-allowed` | 2 | `aria-allowed-attr` |
| `aria/attribute-prohibited` | 2 | `aria-prohibited-attr` |
| `aria/required-parent-missing` | 2 | `aria-required-parent` |
| `aria/required-children-missing` | 2 | `aria-roles` (erforderliche Bestandteile) |

Die IDREF-Prüfung von `aria-valid-attr-value` übernimmt `aria/reference-missing`
(siehe Changed). `aria/role-invalid` und `aria/role-abstract` gab es schon; sie
ersetzen die Rollenprüfung von `aria-roles`.

Kontext und Bestandteile werden am DOM geprüft, nicht am Accessibility-Tree:
Ein `<li>` in `<ul role="tablist">` ist nach ARIA in HTML ein `listitem`, auch
wenn Chrome es zu `generic` glättet. Damit meldet der Fall aus
auditmysite#715 (`ul[role=tablist] > li > a[role=tab]`) wie axe
`aria/required-children-missing` an der Tabliste und
`aria/required-parent-missing` an Tabs und Einträgen. `generic`,
`none`/`presentation` und vom Host Ausgeblendetes sind durchlässig
(`<tbody>` in Chrome, auditmysite#659); `aria-owns` zählt als Besitz.

### Changed

- `aria/reference-missing` prüft auch `aria-details`, `aria-flowto` und
  `aria-errormessage` — Letzteres nur, solange `aria-invalid` gesetzt und nicht
  `false` ist (ARIA 1.2: die Fehlermeldung ist erst dann maßgeblich).

### Abweichungen von auditmysite

- `aria/attribute-not-allowed` und `aria/attribute-prohibited` urteilen über
  die Rolle des Hosts, explizit oder implizit — auditmysite nur über explizite
  `role`-Angaben (bei `prohibited` zusätzlich `div`/`span`). `<div
  aria-expanded>` ist damit ein Befund. Dazu die „MUST NOT"-Fälle aus ARIA in
  HTML (`aria-checked` an nativer Checkbox/Radio, `aria-valuemin`/`-max`
  neben `min`/`max`, `aria-placeholder` neben `placeholder`,
  `aria-disabled`/`-readonly`/`-required="false"` neben dem nativen Attribut,
  abweichendes `aria-colspan`/`-rowspan`). Leere Werte zählen wie fehlende.
  Rollen außerhalb von ARIA 1.2 (browserinterne, `mark`) bleiben ohne Urteil.
- `aria/attribute-value-invalid` meldet `aria-current` und `aria-invalid` nicht:
  ARIA 1.2 legt fest, dass ein unbekannter Wert dort als `true` gilt.
  Vergleich ohne Groß-/Kleinschreibung, leere Werte ohne Befund.
- `aria/required-children-missing` prüft nur Behälter mit expliziter Rolle —
  native Tabellen und Listen geben ihre Bestandteile per HTML vor
  (auditmysite#659, #674). Die leere native Tabelle aus dem Korpus
  `table_required_rows_tbody` (nur `<caption>`) ist deshalb kein Befund mehr.
  Kein Befund ohne jeden besessenen Knoten, unter `aria-busy="true"` und bei
  `aria-expanded="false"`.
- `aria/required-parent-missing` lässt über ARIA 1.2 hinaus `group` für
  `listitem` (ARIA 1.1), `combobox` für `option` und `radiogroup` für
  `menuitemradio` zu. Stößt der Weg nach oben auf eine Rolle, die ARIA 1.2 nicht
  kennt, entsteht kein Befund.
- `aria/tab-selected-missing` ist `REVIEW` (ARIA sagt SHOULD) und steht an der
  Tabliste, wenn *kein* Tab `aria-selected="true"` trägt — nicht an jedem Tab
  ohne das Attribut; die übrigen haben mit der Vorgabe `false` den richtigen
  Zustand.
- `aria/tabpanel-missing` ist `REVIEW`: ARIA beschreibt das Panel nur als
  üblich.
- `aria/combobox-popup-missing` stützt sich auf das MUST für `aria-controls`
  (ARIA 1.2); ein Popup im Teilbaum oder per `aria-owns` (ARIA 1.1) genügt
  weiterhin.
- `inert/dialog-inert` ist `FAIL` nur für `<dialog open>`; `role="dialog"` wird
  `REVIEW`, sichtbare `role="menu"` gar nicht geprüft (ein aus dem Bild
  geschobenes Menü mit `inert` ist richtig gebaut).
- Nicht übernommen aus `modern_attributes`: der fehlende Name offener Dialoge
  und Popover (gehört zur Namensprüfung von Dialogen) und „Fokus in einem
  inerten Teilbaum" (braucht das aktive Element, das kein Tier liefert).

## [0.13.5] - 2026-09-30

### Fixed

- `landmarks/main-missing` hinter einem offenen Dialog: Auch ein
  `role="dialog"`/`"alertdialog"` ohne `aria-modal` zählt — der
  Consent-Dialog von administracion.gob.es blendet `<main>` ohne es aus
  (auditmysite#709).

## [0.13.4] - 2026-09-30

### Fixed

- `landmarks/main-missing`: Ist ein modaler Dialog offen (`role="dialog"`/
  `"alertdialog"` mit `aria-modal="true"`, oder `<dialog open>`) und keine
  main-Landmark erreichbar, ist das `REVIEW` (Low) statt `FAIL` — die Seite
  blendet ihren Inhalt korrekt aus, solange der Dialog offen ist. Beleg: fünf
  von 21 EU-Portalen mit offenem Consent-Dialog (auditmysite#709).

## [0.13.3] - 2026-09-30

### Fixed

- `keyboard/skip-link-missing`: Ein Fragmentlink auf den Anfang des
  Hauptinhalts (die main-Landmark oder ein Element an ihrem Anfang) ist ein
  Sprunglink, wo immer er steht — auch hinter den Links eines Cookie-Banners
  (bund.de, #26). `href=""` beendet die Folge der Fragmentlinks nicht mehr.
  Ein Anker mitten im Inhalt zählt weiter nicht.

## [0.13.2] - 2026-09-30

Aus dem Vergleichslauf von auditmysite gegen echte Seiten (auditmysite#690).

### Fixed

- Der Inhalt eines geschlossenen `<details>` — alles außer der ersten
  `<summary>` — gilt als nicht dargestellt (HTML, „The details element"). Chrome
  blendet ihn über `content-visibility` aus, berechnetes `display` und
  `visibility` halten ihn für sichtbar. Beleg: geographia.eu, 22 Karten-Links
  in `details.more` als `links/name-missing` gemeldet.
- `aria/reference-missing`: Ein `aria-controls` an einem Element mit
  `aria-expanded="false"` darf auf ein Ziel zeigen, das erst beim Öffnen
  entsteht (wie axe-core). Beleg: die Menüs der Web-Komponenten auf bund.de
  und sachsen-anhalt.de.

## [0.13.1] - 2026-09-30

Abgleich mit auditmysite vor dem Umzug seiner Regeln (casoon/barrierlab#13,
auditmysite#690): Wo die geteilte Fassung einen Fall übersah oder zu Unrecht
meldete, den auditmysite richtig behandelt, ist sie nachgezogen.

### Fixed

- `zoom/viewport-locked`: `content` wird nach CSS Viewport zerlegt — Semikolon
  und Leerraum trennen wie das Komma. `width=device-width; maximum-scale=1`
  blieb bisher unerkannt.
- `aria/required-attribute-missing` folgt ARIA 1.2: `slider` braucht nur noch
  `aria-valuenow` (min/max haben Vorgaben), `option` kein `aria-selected`.
  Native Felder mit expliziter Rolle (`<input type="range" role="slider">`,
  `<input type="checkbox" role="switch">`, `<meter>`, `<progress>`) übermitteln
  ihren Zustand selbst und werden nicht gemeldet (auditmysite#656). Neu
  geprüft: `meter` und der fokussierbare `separator` ohne `aria-valuenow`.
- `aria/reference-missing` prüft auch `aria-activedescendant` und meldet
  leere `aria-controls`, `aria-owns` und `aria-activedescendant`.
- `keyboard/hidden-focusable`: `disabled` nimmt nur Formularfelder aus der
  Tab-Folge; ein `<a href disabled>` bleibt erreichbar und wird gemeldet.
- `landmarks/banner-missing`, `contentinfo-missing`: Ein `<header>`/`<footer>`
  unter einem Vorfahren mit Rolle `article`, `complementary`, `main`,
  `navigation` oder `region` ist keine Seiten-Landmark (HTML-AAM,
  auditmysite#639) — bisher zählte nur der Tag des Vorfahren.
- `keyboard/skip-link-missing` erkennt den Sprunglink am Ziel und an der
  Stellung wie axe `isSkipLink`: Fragmentlinks vor dem ersten Link, der die
  Seite verlässt. „Aller au contenu" wurde bisher nicht erkannt
  (auditmysite#642). Die Wortliste bleibt als zweiter Weg.
- `images/alt-missing`: Ein `<img>` mit nicht leerem `aria-label`,
  `aria-labelledby` oder `title` hat eine Textalternative (ARIA6, ARIA10, H67).
- `svg/name-missing`: Ein SVG in einem benannten Link oder Button braucht
  keinen eigenen Namen; `role="none presentation"` gilt als dekorativ.

## [0.13.0] - 2026-09-30

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
