# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[docs/packages/a11y-core-history.md](../../docs/packages/a11y-core-history.md) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`.

## [0.21.0] - 2026-10-05

Vor dem Release hat auditmysite (casoon/auditmysite#698) die neuen Eingaben
in echten Läufen gefüllt: `visually_hidden` und `obscured` auf gov.uk,
bundesregierung.de, wetter.com, n-tv.de und spiegel.de (der Suchknopf von
gov.uk als verborgen erkannt), `sampled_backdrop` auf berlin.de und in den
Bild-Fixtures, die damit ihre bisherigen Urteile behalten.

Kontrast und `Layout` nach dem Vergleich mit auditmysites eigener
Kontrastregel (casoon/barrierlab#47, casoon/auditmysite#698).

### Added

| Neue Kennung | Urteil, Schwere | Norm |
|---|---|---|
| `contrast/text-enhanced` | `FAIL`, mittel | WCAG 1.4.6 (AAA): 7:1, bei großem Text 4,5:1 |

Gemeldet wird nur, was 1.4.3 besteht; was schon dort verfehlt, steht nicht
doppelt. Beleg: die Linkfarbe von gov.uk, #1d70b8 auf Weiß, 5,17:1.

- Kontrast über Bildern und Verläufen: Fehlt die Hintergrundfarbe, liefert
  der Host aber eine Abtastung (`Rendering::sampled_backdrop`), urteilt die
  Regel wie auditmysite vor der Umstellung — Median verfehlt: `FAIL`; Median
  und 40. Perzentil bestehen: bestanden; dazwischen `REVIEW` unter
  `contrast/text-undetermined` (bzw. `contrast/text-enhanced` für AAA). Belege:
  auditmysites `tests/fixtures/image_contrast.html` (dunkler Verlauf besteht,
  heller verfehlt, geteilter ist Hinweis).
- Kontrastbefunde tragen das gemessene und das geforderte Verhältnis als
  `Evidence::computed` (`contrast_ratio`, `required_ratio`, bei Abtastung
  zusätzlich `contrast_ratio_p40`) — für Berichte, die die Zahlen zeigen.

### Changed

- Kontrast: Optisch verborgener Text (`Rendering::visually_hidden`) wird
  nicht gemessen — er hat keinen sichtbaren Kontrast. Beleg: gov.uk,
  `button.gem-c-search__submit`, „Search GOV.UK" per `text-indent: -5000px`
  versteckt, bisher `FAIL` 3,91:1.
- Kontrast: Text, dessen Mitte ein fixiertes oder klebendes Element überdeckt
  (`Layout::obscured`, etwa ein Cookie-Banner), ist
  `contrast/text-undetermined` statt eines Urteils über Farben, die man so
  nicht sieht (auditmysite #716).
- Kontrast: Text unter `aria-hidden` wird weiter gemessen. WCAG 1.4.3 gilt
  für sichtbaren Text, unabhängig vom Accessibility-Tree. auditmysite nahm ihn
  aus (#395) — eine bewusste Abweichung.
- Heuristiken: Ist das `Layout`-Feld, das eine Heuristik braucht, an keinem
  Element gemessen, meldet sie einmal `UNTESTED` für die Seite. Vorher lief
  sie still ohne Befund und zählte als gelaufen. Betrifft
  `order/visual-mismatch`, `motion/infinite-animation`, `reflow/min-width`,
  `focus/obscured` und den `cursor`-Teil von `keyboard/pointer-only`.

## [0.20.0] - 2026-10-05

### Fixed

- Quiz radio buttons and checkboxes no longer trigger format-instruction hints (casoon/astro-post-audit#75).
- Search inputs, search regions and comboboxes no longer request personal autocomplete tokens (casoon/astro-post-audit#76).
- Language detection excludes descendants with their own non-empty `lang` (casoon/astro-post-audit#77).

## [Unreleased]

Der zweite Teil von casoon/barrierlab#21: drei Regeln aus auditmysite, die
gemessene Darstellungswerte brauchen. Norm sind WCAG 2.2 (1.4.1, 2.1.1,
4.1.2) und ARIA in HTML. Alle drei sind Tier 3 und urteilen mit `FAIL` —
was sie lesen, ist gemessen, nicht vermutet. Liefert der Host ein Feld nicht,
steht je Regel und Seite ein `UNTESTED` im Bericht, kein stiller Nicht-Befund.

### Added

| Neue Kennung | Urteil, Schwere | braucht | ersetzt in auditmysite |
|---|---|---|---|
| `lists/role-redundant` | `FAIL`, niedrig (4.1.2) | `ComputedStyle::list_style_type` | `redundant-role` für `<ul>`/`<ol>` (`redundant_role`) |
| `color/link-indistinct` | `FAIL`, mittel (1.4.1) | `text_decoration_line`, `font_style`, `font_family`, `border_bottom_style` | `link-in-text-block` (`use_of_color`) |
| `keyboard/scrollable-region-not-focusable` | `FAIL`, hoch (2.1.1) | `Rendering::scroll_overflow_px` | `scrollable-region-focusable` (`scrollable_region`) |

Belege und mitgebrachte Korrekturen aus dem auditmysite-Korpus
(`tests/darstellung.rs`): `redundant_role_list_style` (#644: `role="list"`
bei `list-style: none` ist nicht überflüssig), `link_in_text_block_context`
(#710: nur Links im Fließtext; Navigation, Logo, Listen aus nur einem Link
und Fußzeilen mit Trennern zählen nicht), `scrollable_region_focusable`
(#717: gov.si `div.menus`; Bereich mit `tabindex`, mit Link darin, mit
`overflow: hidden` oder ohne Überhang bestehen; Puffer 13 px wie axe).

Die neuen Felder hat auditmysite (casoon/auditmysite#698) vor diesem Release
gegen den unveröffentlichten Stand gefüllt gesehen: auf gov.uk,
bundesregierung.de, wetter.com, n-tv.de und spiegel.de jeweils an allen
dargestellten Elementen, Farben und Hintergrund dort etwas seltener
(Verlauf, Hintergrundbild, transparente Textfüllung). Die drei
Korpusfälle melden wie erwartet.

### Abweichungen von auditmysite

- `color/link-indistinct` vergleicht wie auditmysite Unterstreichung,
  Gewicht, Schnitt, Schrift, Unterkante und Hintergrund mit dem
  Elternelement. Der Hintergrund ist hier der effektive, den der Host über
  die Vorfahren auflöst (`ComputedStyle::background_color`), nicht der
  eigene des Elements.
- `keyboard/scrollable-region-not-focusable` prüft nicht selbst, ob Inhalt
  sichtbar ist — das übernimmt die Sicht der Regel (versteckte Teilbäume
  fehlen darin).
- Keine Obergrenze je Seite (auditmysite: 10 bzw. 20 Befunde).

## [0.19.1] - 2026-10-04

### Fixed

- `motion/reduced-motion-ignored` fand keine Animation, die der Browser
  geliefert hat: Chrome schreibt die Kurzschreibweise in `cssText` voll aus
  (`2s linear 0s infinite normal none running spin`), und das `none` des
  Füllmodus galt als Name. Jetzt nach CSS Animations: Ein Schlüsselwort geht
  zuerst an die Eigenschaft, die es noch nicht hat, erst der Rest ist der Name.
  Gefunden bei der Umstellung von auditmysite (casoon/auditmysite#743, Korpus
  `media_and_motion`).

## [0.19.0] - 2026-10-03

Die Regeln über Stylesheets aus auditmysite (casoon/barrierlab#21, erster
Teil). Dort las JavaScript `document.styleSheets`; hier kommen die Sheets als
Text herein, geparst vom neuen Paket `stylesheet-parse`. Norm sind WCAG 2.2 (1.3.4,
1.4.8, 2.3.3, 2.4.7), CSS Syntax Level 3 und Selectors Level 4.

### Added

| Neue Kennung | Urteil, Schwere | ersetzt in auditmysite |
|---|---|---|
| `focus/outline-removed` | `FAIL`, hoch (2.4.7) | `focus-visible-outline-none` (`focus_visible_css`) |
| `motion/reduced-motion-ignored` | `REVIEW`, mittel (2.3.3, AAA) | `prefers-reduced-motion` (`reduced_motion`) |
| `orientation/content-hidden` | `REVIEW`, mittel; hoch für `html`/`body`/`main` (1.3.4) | `css-orientation-lock` (`orientation`), Stylesheet-Teil |
| `text/justified` | `REVIEW`, niedrig (1.4.8, AAA) | `visual-presentation` (`visual_presentation`), Blocksatz |
| `text/line-height-tight` | `REVIEW`, niedrig (1.4.8, AAA) | `visual-presentation`, Zeilenabstand |

- `run_stylesheets` / `run_stylesheets_in`: ergänzt einen Bericht um diese
  Regeln. Jedes `run_*` vermerkt sie vorher als `NotRun::CapabilityMissing`
  („Host liefert keine Stylesheets").
- `stylesheet_metas`, `stylesheet_rules`, `StylesheetRule`; neue Schicht
  `Tier::Stylesheets` in `a11y-dom`.
- Ein Selektor-Abgleich gegen das Dokument (Teilmenge von Selectors Level 4,
  dreiwertig: Treffer, kein Treffer, unbekannt), damit nur zählt, was auf der
  Seite vorkommt — wie `querySelectorAll` in auditmysite (#712).

Belege: der auditmysite-Korpus (`media_and_motion`, `reduced_motion_transform`,
`reduced_motion_override`, `text_and_layout`) und die Stylesheets der 48
echten Seiten (337 Sheets, 24 MB, Abruf 2026-10-03). Auf ihnen meldet
`motion/reduced-motion-ignored` 30 Seiten, `text/line-height-tight` 21 (etwa
gov.uk mit 1,32 im Fließtext), `focus/outline-removed`,
`orientation/content-hidden` und `text/justified` keine.

### Fixed

- `images/area-alt-missing` und `media/audio-autoplay` (0.18.0) meldeten mit
  berechneten Stilen nichts: Das UA-Stylesheet gibt `<area>` und `<audio>`
  ohne `controls` `display: none`, die Sicht ließ sie deshalb weg. Für diese
  beiden zählt jetzt nur das `hidden`-Attribut. Gefunden bei der Umstellung von
  auditmysite (casoon/auditmysite#743, Korpus `misc_content_checks`,
  `media_and_visual`).

### Abweichungen von auditmysite

- `focus/outline-removed`: Ein in einer `:focus`-Regel wieder gesetzter
  Rahmen zählt als Ersatz. sueddeutsche.de entfernt ihn für alle und setzt ihn
  für die Tastatur wieder (`[data-whatintent='keyboard'] *:focus`);
  auditmysite meldete das als Verstoß.
- `orientation/content-hidden` ist `REVIEW` statt Verstoß und meldet nur
  Selektoren, die ein Element der Seite treffen, ohne Pseudo-Elemente. In
  auditmysites Form schlug die Regel auf vier der 48 Seiten an: zweimal
  Breakpoint-Marker (`body:before`, zwei Seiten von bundesregierung.de), ein
  Schließknopf (n-tv.de) und welt.de. Der echte Fall, ein Dialog, der im Querformat
  „bitte drehen" zeigt (welt.de), ist erst mit geöffnetem Dialog im Dokument.
  Die Prüfung von `transform: rotate` am berechneten Stil von `body`/`html`
  bleibt beim Host.
- `text/*`: Gemessen wird an den `<p>` der Seite, nicht an Selektoren, die
  mit `body`, `p`, `div` … beginnen. Sonst meldete `html { line-height: 1.15 }`
  aus normalize.css jede Seite. Zeilenhöhe nur ohne Einheit, in `em` oder `%`.
  `REVIEW` statt Verstoß; der `UNTESTED`-Vermerk für Farbwahl und Spaltenbreite
  gehört zur Checkliste.
- `motion/reduced-motion-ignored`: eine Meldung je Seite wie in auditmysite,
  aber `REVIEW` statt Verstoß. Eine allein stehende `transition-duration` ist
  kein Übergang (Tailwind `.duration-200`).

### Noch offen in #21

`list-style-type` (für `redundant_role` an Listen), Linkfarbe gegen Textfarbe
(`use_of_color`), Scroll-Ausmaße (`scrollable_region`) und
`non_text_contrast_css`. Sie brauchen neue Felder in `ComputedStyle`/`Layout`,
die erst ein Host-Lauf gefüllt sehen muss (auditmysite#698).

## [0.18.0] - 2026-10-03

Die letzten drei Pakete der Regelmigration aus auditmysite (B5–B7,
casoon/barrierlab#18–#20) und die statischen Regeln der
Darstellungskonvention (casoon/barrierlab#22). 28 neue Kennungen.

### Links- und Zeigerregeln (B5)

Die Links- und Zeigerregeln aus auditmysite (casoon/barrierlab#18, B5). Norm
sind WCAG 2.2 (2.1.1, 2.4.8, 4.1.2), der HTML-Standard (`<a>` ohne `href` ist
ein Platzhalter, kein Link) und WAI-ARIA 1.2 (`aria-current`). Alle drei
Regeln sind Tier 1: Sie lesen nur Attribute.

#### Added

| Neue Kennung | Tier | Urteil, Schwere | ersetzt in auditmysite |
|---|---|---|---|
| `keyboard/click-handler-not-focusable` | 1 | `FAIL`, hoch (2.1.1) | `click-events-have-key-events` (`click_handlers`) |
| `links/used-as-button` | 1 | `FAIL`, niedrig (4.1.2) | `link-as-button` (`fake_navigation_link`) |
| `navigation/location-missing` | 1 | `REVIEW`, niedrig (2.4.8, AAA) | `location` (`location`) |

Belege: der auditmysite-Korpus (`keyboard_and_targets`, `audit_exclude_cap`)
und 48 echte Seiten, abgerufen am 2026-10-03 (auditmysites Referenzseiten
und 40 verbreitete deutsche und internationale Seiten). Dort fand
`links/used-as-button` den Aufruf der Consent-Einstellungen
(`<a href="#" onclick="UC_UI_recall();">`, wetter.com) und einen
Neu-laden-Link (craigslist.org), `keyboard/click-handler-not-focusable`
nichts — wetter.com hängt Handler über `data-onclick` an, das kein Handler ist.
`navigation/location-missing` meldet auf 30 der 48 Seiten, fast alle davon
Startseiten; daher `REVIEW`.

#### Changed

- `keyboard/pointer-only` (Tier 3) meldet ein Inline-`onclick` nicht mehr,
  das `keyboard/click-handler-not-focusable` schon meldet. Auf einem Host mit
  Darstellung entstand sonst für dasselbe Element ein zweiter Befund.

#### Nicht übernommen

- `pointer_cancellation` (2.5.2): Der statische Teil sucht `onmousedown` und
  `ontouchstart` an Bedienelementen. Auf keiner der 48 Seiten und in keinem
  Korpusfall kam das vor — ohne Beleg kommt die Regel nicht hinein. Der
  seitenweite `UNTESTED`-Vermerk gehört zur manuellen Checkliste
  (casoon/barrierlab#39).

#### Abweichungen von auditmysite

- `<a onclick>` **ohne** `href` meldet `keyboard/click-handler-not-focusable`,
  nicht `links/used-as-button`: Ohne `href` ist das Element kein Link, wird
  nicht als Link angesagt und ist nicht fokussierbar. auditmysite meldet es
  als Scheinlink.
- `navigation/location-missing` ist `REVIEW` statt Verstoß: 2.4.8 lässt sich
  auch mit Titel, Überschriften oder einer Sitemap erfüllen.
  `aria-current="true"` zählt wie in auditmysite nicht als Ortsangabe — auf
  t-online.de markiert es Karussellpunkte.
- Keine Obergrenze je Seite (auditmysite: 10 bzw. 20 Befunde).

### Bild- und Medienregeln (B6)

Die Bild- und Medienregeln aus auditmysite (casoon/barrierlab#19, B6). Norm
sind WCAG 2.2 (1.1.1, 1.4.2, 2.4.1, 4.1.2) und der HTML-Standard (`<area>`,
`<input type="image">`, `<object>`, `ismap`, `<iframe>`). Fünf Regeln lesen
nur Attribute und Text und sind Tier 1; `frames/name-missing` braucht die
Namensberechnung (`aria-labelledby`) und ist Tier 2.

#### Added

| Neue Kennung | Tier | Urteil, Schwere | ersetzt in auditmysite |
|---|---|---|---|
| `images/area-alt-missing` | 1 | `FAIL`, hoch (1.1.1) | `area-alt` (`image_input_rules`) |
| `images/input-alt-missing` | 1 | `FAIL`, hoch (1.1.1) | `input-image-alt` (`image_input_rules`) |
| `objects/alt-missing` | 1 | `FAIL`, hoch (1.1.1) | `object-alt` (`image_input_rules`) |
| `images/server-side-map` | 1 | `FAIL`, mittel (1.1.1) | `server-side-image-map` (`server_side_image_map`) |
| `media/audio-autoplay` | 1 | `REVIEW`, mittel (1.4.2) | `background-audio` (`background_audio`) |
| `frames/name-missing` | 2 | `FAIL`, hoch (2.4.1, 4.1.2) | `frame-title` (`media_rules`) |

Belege: der auditmysite-Korpus (`misc_content_checks`, `forms_and_misc`,
`object_no_alt`, `media_and_visual`, `frame_missing_title`) und die 48 echten
Seiten vom 2026-10-03. Dort meldet nur `frames/name-missing`: die drei
Sportdaten-Rahmen auf n-tv.de mit `title=""`. Den Sportdaten-Rahmen auf
spiegel.de (ohne `title`) setzt Alpine erst im Browser aus einem `<template>`
ein; er steht als Fall in den Tests. `<area>`, `<input type="image">`,
`<object>`, `ismap` und `<audio autoplay>` kommen auf keiner der 48 Seiten vor
— die Belege dafür sind die Korpusfälle.

#### Changed

- `manual/media-alternatives` erscheint auch, wenn die Seite ein Video von
  YouTube, Vimeo, Dailymotion oder Wistia per `<iframe>` einbettet — die
  Plattformliste aus auditmysites `video-caption`. Ob es Untertitel hat,
  steuert der Player der Plattform. Beleg: das YouTube-Video auf w3.org/WAI,
  das bisher keinen Checklistenpunkt auslöste.

#### Nicht übernommen

- `media-alt` (`media_alternative`, 1.2.8): meldet auf jeder Seite einen
  `UNTESTED`-Hinweis, auch ohne Medien. Wo Medien sind, sagt
  `manual/media-alternatives` dasselbe (Transkript, Audiodeskription); ohne
  Medien gibt es nichts zu prüfen. Hosts bilden `media-alt` auf
  `manual/media-alternatives` ab.
- `video-caption` (`media_rules`, 1.2.2): ohne geprüfte Untertiteldatei bleibt
  es beim `UNTESTED`-Hinweis, und den gibt `manual/media-alternatives`. Ob
  eine `<track>`-Datei tatsächlich lädt, prüft nur der Host über das Netz;
  dieser Teil bleibt in auditmysite, ebenso `frame-tested` (ob ein Rahmen
  fremd ist, weiß nur der Browser).
- `role="application"` ohne Namen (`media_rules`): kein Beleg — weder im
  Korpus noch auf den 48 Seiten ein unbenanntes.
- `role="img"` ohne Namen (`media_rules`): SVGs deckt `svg/name-missing`,
  `<img>` deckt `images/alt-missing`; ein unbenanntes anderes Element mit
  `role="img"` kommt weder im Korpus noch auf den 48 Seiten vor.
- Benanntes dekoratives Element (`media_rules`): `aria-label` an
  `role="presentation"`/`"none"` meldet schon `aria/attribute-prohibited`. Die
  einzigen Fälle auf den 48 Seiten (wikipedia.org, `title` an
  `role="presentation"`) tragen `aria-hidden="true"` und stehen nicht im
  Accessibility-Tree.
- `<embed>` aus `object-alt`: kein Beleg.

#### Abweichungen von auditmysite

- `aria-labelledby` zählt an `<area>`, `<input type="image">` und `<object>`
  als Alternative, `aria-label` auch an `<area>`. auditmysite kennt an
  `<area>` nur `alt` und sonst nur `aria-label`.
- `media/audio-autoplay` ist `REVIEW` und führt 1.4.2 statt 1.4.7: Ob der Ton
  länger als drei Sekunden läuft und sich anhalten lässt, steht nicht im
  Markup; selbststartender Ton ist Gegenstand von 1.4.2 (Audio Control),
  1.4.7 betrifft Hintergrundgeräusche unter Sprache.
- `frames/name-missing` führt 2.4.1 und 4.1.2 (Technik H64), auditmysite nur
  2.4.1. Ohne berechnete Stile gilt ein Rahmen mit Inline-`display: none`
  als unsichtbar (craigslist.org), dazu wie in auditmysite einer mit Breite
  und Höhe 0 oder 1 (duckduckgo.com). Mit Stilen (Tier 3) nimmt die Sicht
  versteckte Rahmen ohnehin heraus; die Prüfung auf ≤ 1 px gerenderte Größe
  bleibt beim Host.

### Darstellungskonvention (#22)

Die statischen Regeln der Darstellungskonvention (casoon/barrierlab#22,
Entwurf v0): `figure[data-viz]` mit Text-, Standbild- und Live-Schicht,
`html[data-display]` mit Umschalter. Die Konvention steht auf der Seite
[Darstellungsmodi](../../docs/a11y/concepts/darstellungsmodi.md).

**Entscheidung zum Paket:** `a11y-rules`, nicht `web-checks`. Die Regeln
brauchen den DOM — Vorfahren, Nachfahren, Verweise per `aria-describedby` —
und lesen dabei nur Attribute und Struktur; alle sieben sind Tier 1 (Modul
`viz`). Sie melden nur auf Seiten, die die Konvention benutzen, tragen das
Schlagwort `best-practice` und hängen wie in auditmysite am nächsten
WCAG-Kriterium. Die Kennungen `display/*` sind dieselben wie in auditmysite
(`src/wcag/rules/display_modes.rs`); der dort gemessene Teil (berechnete
Sichtbarkeit, Zeitpunkt von `data-display`, Textmodus) bleibt dort.

#### Added

| Neue Kennung | Tier | Urteil, Schwere | in auditmysite |
|---|---|---|---|
| `viz/text-missing` | 1 | `FAIL`, hoch (1.1.1) | — (`display/text-not-visible` misst den Textmodus) |
| `display/text-hidden` | 1 | `FAIL`, hoch; `REVIEW`, niedrig bei Verweis per `aria-describedby`/`aria-details` (1.1.1) | `display/text-hidden`, Attribut-Teil |
| `viz/caption-missing` | 1 | `FAIL`, niedrig (1.1.1) | — |
| `viz/static-missing` | 1 | `FAIL`, mittel (2.2.2) | — |
| `viz/table-missing` | 1 | `REVIEW`, niedrig (1.3.1) | — |
| `display/toggle-missing` | 1 | `FAIL`, mittel (2.2.2) | `display/toggle-missing`, statischer Teil |
| `display/init-missing` | 1 | `REVIEW`, niedrig (2.2.2) | `display/init-missing`, statischer Teil |

Belege: die gebauten Seiten der Referenzumsetzung Geographia (371 Seiten,
`web-geographia/apps/*/dist`, Stand 2026-10-03). `viz/text-missing` meldet
die Startseite, die ihre Diagramme in `.viz-desc` ohne `[data-viz-text]`
beschreibt, und „Build Earth 2.0" (`space/solar-system/settlement/`), das
auch `viz/static-missing` auslöst; `viz/caption-missing` die Klima-Monitor-
und Deutschlandkarten ohne `<figcaption>`; `display/toggle-missing` die
Laborseiten ohne Kopfzeile; `viz/table-missing` 196 Diagramme, vor allem
Sparklines. `display/text-hidden` folgt dem Fall aus auditmysite#704
(`div#layers-home-desc` auf geographia.eu/atmosphere/) samt den dort
festgehaltenen Testfällen; im heutigen Build blendet Geographia nur visuell
aus. `display/init-missing` meldet dort nichts. Auf den 48 echten Seiten ohne
Konvention meldet keine der sieben Regeln.

- Die Kernaussage wird nur auf „nicht leer" geprüft, nicht auf Qualität.
- `viz/orphan-media` aus dem Entwurf (`canvas`, `video`, `svg[role=img]`
  außerhalb der Konvention) ist nicht aufgenommen: In Geographia steht jedes
  solche Medium in `[data-viz]` oder unter `aria-hidden`, ein echter
  Positivfall fehlt.
- `display/init-missing` kann nur die Abwesenheit feststellen; ob ein
  vorhandenes Skript den Modus setzt, zeigt erst die laufende Seite.

#### Abweichungen von auditmysite

- `viz/*` gibt es in auditmysite nicht; dort prüft `display/text-not-visible`
  den Text erst im gerenderten Textmodus.
- Ist die `figure` selbst per `hidden` ausgeblendet, meldet
  `display/text-hidden` nichts (wie auditmysite#725); `aria-hidden` oder
  `inert` an der `figure` dagegen bleiben ein Befund, weil die Grafik dann
  sichtbar ist.
- Keine Obergrenze je Seite (auditmysite: 20 Befunde).

### Tabellen-, Dokument-, Sprach- und Rollenregeln (B7)

Die Tabellen-, Dokument-, Sprach- und Rollenregeln aus auditmysite
(casoon/barrierlab#20, B7). Norm sind WCAG 2.2 (1.3.1, 1.4.13, 2.2.1,
2.4.10, 3.1.1, 3.1.2, 3.1.4, 4.1.2), der HTML-Standard (`headers`,
`lang`/`xml:lang`, „shared declarative refresh steps"), ARIA in HTML
(überflüssige Rollen) und WAI-ARIA 1.2. Alle Regeln sind Tier 1: Sie lesen
Tags, Attribute und Text.

#### Added

| Neue Kennung | Tier | Urteil, Schwere | ersetzt in auditmysite |
|---|---|---|---|
| `tables/header-without-data` | 1 | `FAIL`, hoch (1.3.1) | `th-has-data-cells` (`table_extended`) |
| `tables/data-undetermined` | 1 | `UNTESTED`, hoch (1.3.1) | `th-has-data-cells`, `incomplete` (`table_extended`) |
| `tables/headers-attr-invalid` | 1 | `FAIL`, hoch (1.3.1) | `td-headers-attr` (`table_extended`) |
| `document/lang-mismatch` | 1 | `FAIL`, mittel (3.1.1) | `html-xml-lang-mismatch` (`language_extended`) |
| `language/part-unmarked` | 1 | `REVIEW`, mittel (3.1.2) | `language-of-parts` (`language_of_parts`) |
| `language/part-undetermined` | 1 | `UNTESTED`, mittel (3.1.2) | — (auditmysite schweigt auf anderen Sprachen) |
| `language/abbreviation-unexpanded` | 1 | `REVIEW`, niedrig (3.1.4, AAA) | `abbreviations` (`abbreviations`) |
| `timing/meta-refresh` | 1 | `FAIL`, hoch (2.2.1) | `meta-refresh` (`timing_adjustable`) |
| `headings/section-without-heading` | 1 | `REVIEW`, niedrig (2.4.10, AAA) | `heading-order` (`section_headings`, Abschnittszählung) |
| `aria/role-redundant` | 1 | `FAIL`, niedrig (4.1.2) | `redundant-role` (`redundant_role`) |
| `names/title-only` | 1 | `REVIEW`, mittel (4.1.2) | `title-only-description` (`content_on_hover`) |
| `patterns/tooltip-unreferenced` | 1 | `FAIL`, niedrig (1.4.13) | `content-on-hover-focus` (`content_on_hover`) |

Belege: der auditmysite-Korpus und dieselben 48 echten Seiten wie B5 (Abruf
2026-10-03). Treffer dort:

- `aria/role-redundant`: 251 Befunde auf 5 Seiten — wetter.com 188
  (`li[role=menuitem] > a[href][role=link]`), lidl.de 39 und sparkasse.de 16
  (`a[href][role=link]`), bahn.de und spiegel.de je 4 (`button[role=button]`
  als Akkordeon-Auslöser). Dazu Korpus `redundant_role_list_style`.
- `names/title-only`: 200 Befunde auf 10 Seiten — spiegel.de 163
  (Teaser-Links um ein Bild mit leerem `alt`), focus.de 16 (Symbol-Links),
  bahn.de 7 (Suchknöpfe), dazu heise.de, tagesschau.de, faz.net,
  sueddeutsche.de, check24.de, zeit.de und basf.com. Dazu Korpus
  `name_description_best_practice`.
- `headings/section-without-heading`: ein Hinweis auf 7 Seiten, fast nur
  Teaser-Karten als `<article>` (t-online.de 61, faz.net 39, bild.de 25,
  check24.de 20, basf.com 15, web.de 4, spiegel.de 1).
- Auf keiner der 48 Seiten, belegt im Korpus: `tables/header-without-data`
  (`table_headers_no_data`, `table_grid_rows_unrendered`),
  `tables/headers-attr-invalid` (`forms_extended`), `document/lang-mismatch`
  und `language/abbreviation-unexpanded` (`misc_content_checks`),
  `language/part-unmarked` (`text_and_layout`), `timing/meta-refresh`
  (`meta_refresh_present`), `patterns/tooltip-unreferenced`
  (`forms_and_misc`). Der einzige Treffer von `language/part-unmarked` auf
  echten Seiten war ein Fehlalarm und ist behoben (siehe unten).

Mitgebrachte Korrekturen aus auditmysite, je als Test:

- auditmysite#638: leere Datenzellen, Zellen mit `aria-hidden`-Kind und ein
  Spaltenkopf über Zeilenköpfen sind kein „Kopf ohne Daten"
  (`korpus_table_headers_tbody_ignored_issue_638`,
  `spaltenkopf_ueber_zeilenkoepfen_barrierlab_eu_issue_638`).
- auditmysite#639: die Tabelle im Kapitel mit `<header>` im Artikel
  (`tabelle_im_kapitel_geographia_issue_639`; der Landmark-Teil steht seit B4
  in `tests/landmarks.rs`).
- auditmysite#654: Ein noch nicht dargestellter Zeilenvorrat ist
  `tables/data-undetermined`, nicht `FAIL`
  (`korpus_table_grid_rows_unrendered_issue_654`,
  `native_tabelle_mit_koerper_issue_654`).
- auditmysite#659: `tbody > tr` mit Zeilenkopf und Datenzelle
  (`korpus_table_required_rows_tbody_issue_659`; die Rollenregel steht seit
  B1 in `tests/aria.rs`).
- auditmysite#644: `role="list"` auf `<ul>`/`<ol>` wird nicht gemeldet
  (`korpus_redundant_role_list_style_issue_644`).

#### Nicht übernommen

- `presentation-semantic-children` (`info_relationships`): Nach WAI-ARIA 1.2
  nimmt `role="presentation"`/`"none"` nur dem Element selbst die Semantik;
  Nachfahren behalten ihre, außer den erforderlichen Bestandteilen von
  Tabellen und Listen. Den Tabellenfall meldet schon
  `tables/presentational-with-headers`, F92 betrifft das Element selbst. Auf
  den 48 Seiten hätte die Regel 187 `<li role="none">` um Menülinks gemeldet
  (t-online.de: das APG-Muster Menüleiste) — durchweg Fehlalarme.
- `section_headings`, Lücken in der Gliederung: meldet schon
  `headings/skip-level`.
- `section_headings`, „mehr als 10 Absätze, weniger als 3 Überschriften":
  kein Beleg — auf keiner der 48 Seiten, in keinem Korpusfall.
- `timing_adjustable`: Der seitenweite `UNTESTED`-Vermerk für Skript-Fristen
  und `timeouts` (2.2.6) gehören zur Checkliste (`manual/timing`).
- `redundant_role` für `<ul>`/`<ol>` mit `role="list"`: Ob die Rolle
  überflüssig ist, hängt am berechneten `list-style-type` (auditmysite#644),
  und `ComputedStyle` in `a11y-dom` hat dieses Feld nicht. Bis es das gibt,
  wird das Paar nicht geprüft statt geraten; `aria/role-redundant` sagt über
  Listen nichts aus. auditmysite meldet `ol#numbered-list` im Korpus mit Stil
  weiterhin selbst, bis `a11y-dom` das Feld liefert.

#### Abweichungen von auditmysite

- `tables/data-undetermined` erkennt ausstehende Zeilen an einer
  Zeilengruppe ohne Zeile in der Sicht — auch am leeren `<tbody>` im Markup,
  das auf seine Daten wartet. Ist die ganze Zeilengruppe ausgeblendet, fehlt
  sie in der Sicht, und die Kopfzellen melden `FAIL`.
- `tables/headers-attr-invalid` prüft nur Zellen (`td`, `th` und ihre
  Rollen); nur dort definiert HTML das Attribut.
- `language/part-unmarked` meldet den innersten Textblock (`<li><p>` einmal),
  lässt `<script>`, `<style>` und `<template>` aus — auf sparkasse.de las
  sich das eingebettete CSS sonst als Englisch — und meldet auf Seiten, die
  weder Deutsch noch Englisch sind, `language/part-undetermined` statt nichts.
- `language/abbreviation-unexpanded` ist `REVIEW` statt Verstoß: 3.1.4 lässt
  sich auch mit der Ausschreibung im Text oder einem Glossar erfüllen. Ein
  leeres `title` zählt als fehlend.
- `timing/meta-refresh` meldet `0` s nicht: Eine sofortige Weiterleitung ist
  keine Frist (H76, axe `meta-refresh`); web.de und gmx.net leiten so ohne
  JavaScript weiter. Eine Anweisung ohne Ziffern führt der Browser nicht aus.
- `headings/section-without-heading` zählt Artikel und benannte Abschnitte
  ohne eigene Überschrift, ohne Navigationen, statt alle Abschnitte gegen alle
  Überschriften der Seite; `REVIEW` statt Verstoß.
- `aria/role-redundant` meldet `<li role="listitem">` nur in einer Liste ohne
  eigene Rolle: In `ul[role=list]` gehört es zur selben WebKit-Abhilfe
  (lidl.de).
- `names/title-only` ist `REVIEW` statt Verstoß, wie `forms/title-only-label`:
  `title` ist eine gültige Namensquelle (H65). Textfelder meldet
  `forms/title-only-label`; `submit` und `reset` haben einen Vorgabenamen.
- `patterns/tooltip-unreferenced` lässt neben `aria-describedby` auch
  `aria-labelledby` als Verweis gelten.
- Keine Obergrenze je Seite (auditmysite: 5, 10 bzw. 20 Befunde).

## [0.17.0] - 2026-10-01

Die Landmark-, Tastatur- und Strukturregeln aus auditmysite
(casoon/barrierlab#17, B4). Norm sind WAI-ARIA 1.2 (Landmark-Rollen,
`aria-activedescendant`), HTML-AAM (`header`, `footer`, `aside`, `section`,
`form`) und WCAG 2.2 (1.3.1, 2.1.1, 2.4.1, 2.4.3, 4.1.2). Alle neuen Regeln
außer `headings/none` sind Tier 2: Ob `<form>` und `<section>` Landmarks sind,
hängt an ihrem Accessible Name, ob ein Element interaktiv ist, an seiner Rolle.
Die Landmark-Rolle bestimmt die Regel selbst aus dem Markup, nicht aus der
Rolle des Hosts — `accname` und ältere Chrome-Fassungen geben einem `<header>`
in `<main>` noch `banner`.

### Added

| Neue Kennung | Tier | Urteil, Schwere | ersetzt in auditmysite |
|---|---|---|---|
| `landmarks/not-unique` | 2 | `REVIEW`, mittel | `landmark-unique` (`landmark_granular`) |
| `landmarks/not-top-level` | 2 | `FAIL`, mittel | `landmark-banner-is-top-level`, `landmark-contentinfo-is-top-level`, `landmark-main-is-top-level` (`landmark_granular`) |
| `landmarks/banner-duplicate` | 2 | `FAIL`, mittel | `landmark-no-duplicate-banner` (`landmark_granular`) |
| `landmarks/contentinfo-duplicate` | 2 | `FAIL`, mittel | `landmark-no-duplicate-contentinfo` (`landmark_granular`) |
| `landmarks/content-outside` | 2 | `FAIL`, mittel | `region` (`region`) |
| `headings/none` | 1 | `FAIL`, mittel; `REVIEW`, niedrig hinter offenem Dialog | `bypass` „No headings found" (`bypass_blocks`) |
| `keyboard/focusable-no-role` | 2 | `REVIEW`, niedrig | `focusable-no-role` (`keyboard`) |
| `keyboard/interactive-not-focusable` | 2 | `REVIEW`, hoch | `keyboard` „appears not keyboard-focusable" (`keyboard`) |
| `dialog/focusable-missing` | 2 | `FAIL`, mittel (2.4.3) | `dialog-no-focusable` (`patterns/modal_dialog`) |
| `patterns/accordion-controls-missing` | 2 | `REVIEW`, niedrig | `accordion-no-controls` (`patterns/accordion`) |

Mitgebrachte Korrekturen als Tests (`tests/landmarks.rs`): auditmysite#639
(`<header>`/`<footer>` in `main`, `article` oder unter `role="main"` sind
keine banner/contentinfo), #642 (Sprunglink am Ziel erkannt, nicht am Text),
#709 (keine Überschriften hinter offenem Dialog: Hinweis), #727 (unbenanntes
`<form>`/`<section>` ist keine Landmark — weder für die Eindeutigkeit noch als
umgebende Landmark). #638 und #644 betreffen Tabellen und `redundant-role`,
keine Regel dieses Pakets.

Bleibt im Host: `keyboard-trap` (2.1.2) — der Hinweis je modalem Dialog und
der seitenweite `UNTESTED`-Vermerk brauchen echte Tastaturbedienung. Aus
`patterns/` die Mustererkennung und die Journeys, dazu
`accordion-trigger-not-button` und `aria-expanded-required` (siehe unten).

Nicht übernommen: `accordion-trigger-not-button` — an Rollen, die
`aria-expanded` nicht unterstützen, meldet das schon
`aria/attribute-not-allowed`; an Rollen, die es unterstützen (`link`,
`menuitem`, `tab`, `treeitem`, …), erlaubt WAI-ARIA 1.2 den Zustand
ausdrücklich, und fehlender Fokus fällt unter
`keyboard/interactive-not-focusable`. `aria-expanded-required` — rät eine
Aufklappnavigation aus dem Wort „menu"/„Menü" im Namen; sprachabhängig wie der
Fehler aus #642, ohne Norm dahinter. `tab-no-aria-selected` und
`aria-dialog-name` liefen schon als `aria/tab-selected-missing` und
`dialog/name-missing` (B1, B2), fehlende main- und banner-Landmark sowie
doppelte main schon als `landmarks/main-missing`, `landmarks/banner-missing`
und `landmarks/main-duplicate` (B0).

### Abweichungen von auditmysite

- `landmarks/not-unique` ist `REVIEW` statt Verstoß: WAI-ARIA 1.2 und die APG
  verlangen unterscheidbare Namen nur als SHOULD, WCAG 1.3.1 setzt keine an
  Landmarks voraus. Wie in auditmysite zählen gleich benannte und gleich
  unbenannte Landmarks; mehrere `main`, `banner` oder `contentinfo` melden
  also zusätzlich zu ihrem Duplikatbefund.
- `<aside>` in `article`, `aside`, `nav` oder `section` ist nur mit Namen
  `complementary` (HTML-AAM).
- `landmarks/banner-duplicate` und `landmarks/contentinfo-duplicate` zeigen
  wie `landmarks/main-duplicate` auf die zweite Landmark, auditmysite auf die
  erste.
- `landmarks/content-outside` meldet das äußerste Element ohne Landmark darin,
  einmal je Block (wie axe `region`), nicht jeden Textknoten und jedes
  benannte Element einzeln. Steht Text unmittelbar neben einer Landmark, trägt
  der umgebende Container den Befund. Die Befundzahl je Seite sinkt damit.
- `keyboard/focusable-no-role` zählt nur die Tabfolge (`tabindex` ≥ 0):
  `tabindex="-1"`, etwa am Ziel eines Sprunglinks, erreicht niemand per Tab.
  Ein benannter Bereich (`region`) mit `tabindex="0"` ist das empfohlene Muster
  für scrollbare Bereiche und zählt nicht.
- `keyboard/interactive-not-focusable` nimmt deaktivierte Felder, native
  `<option>` (Chrome führt sie nicht als `option`), Elemente unter
  `aria-activedescendant` und Inertes aus. Fokussierbar heißt wie Chrome
  `focusable`: nativ oder mit irgendeinem `tabindex`, auch `-1` (Tabs mit
  rovingem `tabindex`).
- `dialog/focusable-missing` sucht in allen Nachfahren, auditmysite nur in
  den direkten Kindern. Ein geschlossenes `<dialog>` zählt nicht.
- `patterns/accordion-controls-missing` ist `REVIEW` statt Verstoß: WAI-ARIA
  1.2 verlangt `aria-controls` am Button nicht, die APG nennt es beim
  Disclosure-Muster optional. Ausgenommen wie in auditmysite: zugeklappte
  Buttons, `<summary>` und Buttons in `navigation`/`banner`.
- `headings/none` zählt auch `role="heading"`; versteckte Überschriften
  zählen nicht.

## [0.16.0] - 2026-10-01

`names/required-missing` meldet Formularfeld-Rollen (`textbox`, `searchbox`,
`combobox`, `listbox`, `spinbutton`, `slider`, `checkbox`, `radio`,
`radiogroup`, `switch`) ohne Namen als Critical — dieselbe Schwere wie das
native Feld ohne Label (`forms/label-missing`). Bisher High; die übrigen Rollen
bleiben High. Beleg: auditmysite-Kalibrierung `combobox_missing_expanded`,
`widget_patterns` (dort meldete die abgelöste Regel `label` Critical).

Die Formularregeln aus auditmysite (casoon/barrierlab#16, B3). Norm sind
WCAG 2.2 (1.3.1, 1.3.5, 3.2.1, 3.2.2, 3.3.1, 3.3.2, 3.3.7, 3.3.8), der
HTML-Standard („Autofill", Formulareigentümer) und WAI-ARIA 1.2
(`aria-invalid`, `aria-errormessage`, `aria-required`). Kennungen: `forms/*`
für Felder und Formulare, `context/*` für den Kontextwechsel bei Fokus und
Eingabe, `auth/*` für die Anmeldung.

### Added

| Neue Kennung | Tier | Urteil, Schwere | ersetzt in auditmysite |
|---|---|---|---|
| `forms/autocomplete-invalid` | 1 | `FAIL`, niedrig | `autocomplete-valid` „Invalid autocomplete value" (`input_purpose`) |
| `forms/purpose-missing` | 2 | `REVIEW`, mittel | `autocomplete-valid` „lacks autocomplete" (`input_purpose`), `identify-purpose` (`identify_purpose`) |
| `forms/error-unidentified` | 1 | `FAIL`, mittel | `input-error-message` (`form_rules`), `aria-invalid-without-describedby` (`error_identification`) |
| `forms/group-missing` | 1 | `FAIL`, mittel | `form-field-group` (`form_rules`, AX- und DOM-Teil) |
| `forms/group-name-missing` | 2 | `FAIL`, mittel | `label` „Form group has no legend or label" (`instructions`) |
| `forms/required-unmarked` | 2 | `REVIEW`, mittel | `label` „Required field not clearly indicated" (`instructions`), `label` „may not indicate required status" (`form_rules`) |
| `forms/instructions-missing` | 2 | `REVIEW`, niedrig | `label` „may require format instructions" (`instructions`) |
| `forms/title-only-label` | 2 | `REVIEW`, mittel | `label-title-only` (`label_title_only`) |
| `forms/no-submit` | 1 | `FAIL`, mittel; `REVIEW`, niedrig ohne `action` und Textfeld | `form-no-submit` (`form_rules`) |
| `forms/redundant-entry` | 1 | `REVIEW`, mittel | `redundant-entry` (`redundant_entry`) |
| `context/on-input` | 1 | `FAIL`, mittel; `REVIEW`, niedrig | `input-no-context-change`, Inline-`onchange` (`on_input`) |
| `context/on-focus` | 1 | `REVIEW`, hoch | `focus-no-context-change` „onfocus" (`on_focus`) |
| `context/autofocus` | 1 | `REVIEW`, mittel | `focus-no-context-change` „autofocus" (`on_focus`) |
| `auth/captcha` | 1 | `REVIEW`, mittel | `accessible-auth-captcha` (`accessible_authentication`) |

Mitgebrachte Korrekturen als Tests (`tests/forms.rs`): auditmysite#643
(Anleitung per `aria-describedby` zählt gleich welchen Wortlauts; kurze
Formatbegriffe nur als ganzes Wort; ein einzelnes Kontrollkästchen ist keine
Gruppe, gleichnamige im selben Formular schon), #656 (native Datumsfelder,
Zahlenfelder nur über die Beschriftung), #658 (der Name eines Dings ist kein
Personenname, auch über `id` und `name`), #728 (Formular nur aus Schaltern:
`REVIEW`, niedrig).

Bleibt im Host: der Einfüge-Test an Passwort- und Einmalcode-Feldern
(`accessible-auth-paste-blocked`), das Nachschlagen aufgerufener Funktionen
über `window` und die Namensvermutung („Language") aus `on_input`.

Nicht übernommen, weil schon abgedeckt: `label` „no accessible label" und
„Placeholder used as only label" (`instructions`) sowie `labels.rs`
`check_form_control` — `forms/label-missing`, `forms/placeholder-as-label` und
`names/required-missing` melden dieselben Fälle, auch ARIA-Widgets ohne Namen,
leeres oder ins Leere zeigendes `aria-labelledby` und leeres `<label for>`.
Ebenso der seitenweite `UNTESTED`-Vermerk aus `identify_purpose` (1.3.6 für
Symbole und Bereiche).

### Abweichungen von auditmysite

- `forms/autocomplete-invalid` prüft die ganze Grammatik des HTML-Standards
  (`[section-*] [shipping|billing] [Kontakt] Feldname [webauthn]`), nicht nur
  das letzte Token, mit der vollständigen Liste der Feldnamen. Geprüft werden
  `input` (außer `hidden`), `select` und `textarea`.
- `forms/purpose-missing` ist `REVIEW` statt Verstoß: Ob ein Feld die Person
  betrifft, ist aus Beschriftung, `id` und `name` geraten. Ein Fall statt zwei
  (`identify-purpose` meldete dieselben Felder nach `id`/`name`, niedrig). Die
  Beschriftung ist der Accessible Name des Hosts; `email`- und `tel`-Felder
  zählen immer. Technische Adressen (`ip_address`) zählen nicht.
- `forms/error-unidentified` meldet einen Fall, den auditmysite zweimal
  meldete. Als Beschreibung zählen `aria-describedby` und
  `aria-errormessage` mit Text; ein Ziel außerhalb der Sicht (versteckt oder
  fehlend — das meldet `aria/reference-missing`) gilt als Beschreibung.
- `forms/group-missing` prüft auf Struktur: Als Gruppe zählen `<fieldset>`,
  `<details>` und `role="group"`/`"radiogroup"` als Vorfahre.
- `forms/required-unmarked` ist `REVIEW` statt Verstoß und meldet den Fall
  einmal (auditmysite: zweimal, niedrig und mittel): Der Screenreader sagt das
  Pflichtfeld an; ob es sichtbar gekennzeichnet ist, sieht nur ein Mensch.
- `forms/title-only-label` ist `REVIEW` statt Verstoß: `title` ist eine
  zulässige Technik (H65). Felder mit `placeholder` meldet
  `forms/placeholder-as-label`.
- `forms/group-name-missing` meldet kein `radiogroup` (das tut
  `names/required-missing`).
- `forms/no-submit` zählt Felder und Buttons, die per `form`-Attribut
  außerhalb des Formulars stehen.
- `context/on-input` liest nur den Handler im Markup; ruft er eine Funktion
  auf, bleibt es `REVIEW`. Neu: auch Optionsfelder mit `onchange` (F37).
- `context/on-focus` und `context/autofocus` sind `REVIEW` statt Verstoß: Ob
  ein `onfocus` den Kontext wechselt, steht nicht im Markup, und `autofocus`
  wechselt ihn für sich genommen nicht. Schweregrade unverändert.

## [0.15.0] - 2026-10-01

Nachgezogen nach dem Vergleichslauf mit auditmysite: Ein leerer `role="tab"`
braucht einen Namen (WCAG 4.1.2, auch ohne „Name Required" in ARIA 1.2);
`label-in-name/mismatch` nimmt Symbol-Labels aus (`¹⁾`, Understanding 2.5.3)
und meldet gleiche Wörter in anderer Reihenfolge als `REVIEW` (dm.de).


Die Namensregeln aus auditmysite (casoon/barrierlab#15, B2). Norm sind
WAI-ARIA 1.2 („Accessible Name Required"), accname 1.2, HTML-AAM und WCAG
2.5.3; alle in Tier 2.

### Added

| Neue Kennung | Schwere | ersetzt in auditmysite |
|---|---|---|
| `names/required-missing` | hoch | `aria-command-name` (ohne `button` und `a[href]`), `aria-input-field-name`, `aria-meter-name`, `aria-progressbar-name`, `aria-toggle-field-name`, `aria-treeitem-name` (`aria_naming_rules`); `aria-label` „kein Name" (`accessible_name`) |
| `names/symbol-only` | mittel | `aria-label` „Icon Only" (`accessible_name`) |
| `dialog/name-missing` | hoch | `aria-dialog-name` (`aria_naming_rules`), `dialog-name` (`dialog_rules`) |
| `dialog/modal-unmarked` | mittel | `dialog-name` „Dialog Modal" (`dialog_rules`) |
| `summary/name-missing` | hoch | `summary-name` (`summary_name`) |
| `status/live-overridden` | hoch (`alert`), mittel (`status`, `log`) | `aria-live-region-role` (`status_messages`) |
| `label-in-name/mismatch` | mittel | `label-content-name-mismatch` (`label_in_name`) |

Unbenannte Buttons, Links (`a[href]`) und SVGs melden weiter
`buttons/name-missing`, `links/name-missing` und `svg/name-missing`;
`names/required-missing` meldet sie nicht noch einmal. Ein natives
Formularfeld ganz ohne Beschriftung meldet `forms/label-missing`;
`names/required-missing` nur, wenn eine Beschriftung behauptet wird, aber leer
ausgeht (`<label for>` ohne Text).

### Changed

- `buttons/name-missing` meldet keine `<summary>` mehr. `accname` gibt ihr
  die Rolle `button`, Chrome `DisclosureTriangle`; `summary/name-missing`
  meldet sie jetzt auf jedem Host genau einmal.

### Abweichungen von auditmysite

- `names/required-missing` prüft `menu` und `tab` nicht (ARIA 1.2 verlangt
  dort keinen Namen) und keine native `<option>` (die leere
  Platzhalteroption eines `<select>` ist üblich; Chrome stellt sie nicht als
  `option` aus). Unbenannte, nicht fokussierbare Elemente sind ein Verstoß wie
  in `aria_naming_rules`, nicht die Warnung aus `accessible_name`.
- `dialog/name-missing` meldet einen Fall, für den auditmysite zwei
  Kennungen hatte. Ein geschlossenes `<dialog>` (ohne `open`) wird nicht
  geprüft.
- `dialog/modal-unmarked` ist `REVIEW` statt Verstoß (ARIA verlangt
  `aria-modal` nicht; ein nicht modaler Dialog trägt es zu Recht nicht) und
  prüft nur `role="dialog"`: Ob ein natives `<dialog>` per `showModal()`
  geöffnet wurde, liefert kein Tier.
- `summary/name-missing` meldet nur die `<summary>`, die ihr `<details>`
  bedient, nicht zusätzlich das `<details>`.
- `status/live-overridden` ist ein Verstoß nur bei `aria-live="off"` (die
  Region wird dann nicht angesagt). Eine andere Dringlichkeit (`alert` mit
  `polite`, `status` mit `assertive`) ist `REVIEW`: ARIA erlaubt das
  Überschreiben, angesagt wird weiterhin. Implizite Rollen zählen mit
  (`<output>` ist `status`).
- `names/symbol-only` ist `REVIEW` statt Verstoß — 4.1.2 verlangt einen
  Namen, über seine Güte sagt es nichts. Buchstaben zählen nach Unicode.
- `label-in-name/mismatch` prüft alle Rollen mit Namen aus dem Inhalt
  (`button`, `link`, `menuitem*`, `tab`, `checkbox`, `radio`, `switch`,
  `option`, `treeitem`), deren Name aus `aria-label` oder `aria-labelledby`
  stammt — auditmysite nur `button` mit `aria-label`. Verglichen wird der
  berechnete Name, ohne Groß-/Kleinschreibung, Satzzeichen und Leerraum, mit
  Buchstaben nach Unicode. Wie in auditmysite: kein Befund, wenn der Name im
  sichtbaren Text steht; `REVIEW`, wenn der sichtbare Text mehr als doppelt so
  lang ist wie der Name (#513). Visuell versteckter Text (`.sr-only`) ist ohne
  Stile nicht erkennbar und zählt als sichtbar.
- Nicht übernommen aus `accessible_name`: leeres `aria-labelledby` und
  `aria-describedby` an einem Element, das einen Namen hat — die übrigen
  Namensquellen greifen, es schadet nicht (wie schon bei
  `aria/reference-missing`). Ebenfalls noch nicht übernommen:
  `description-duplicates-name` (auditmysite#713). Dafür bräuchte
  `a11y_dom::Semantics` die Accessible Description, die es bisher nicht
  liefert.

## [0.14.2] - 2026-10-01

### Fixed

- Ein verwaistes `listitem` meldet nur noch `lists/item-outside-list`, nicht
  zusätzlich `aria/required-parent-missing` (auditmysite-Kalibrierung
  `invalid_aria.html`). Dafür erkennt `lists/item-outside-list` auch ein `<li>`
  unter einer Liste, die per `role` etwas anderes geworden ist
  (`ul[role=tablist] > li`, auditmysite#715); ein `<li>` mit eigener Rolle
  (`none` im Menü-Muster, `tab`) gilt dort nicht als Listeneintrag.

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
