# Changelog

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).
Einträge bis 0.10.1 stehen gesammelt in
[a11y-core bis 0.11.0](https://casoon.github.io/barrierlab/docs/packages/a11y-core-history/) —
die vier Crates lagen bis dahin im Repository `casoon/a11y-core`. Was die aus
auditmysite übernommenen Regeln ersetzen, worauf sie sich stützen und wo sie
abweichen, steht je Schritt in
[Übernahme aus auditmysite](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/).

## [Unreleased]

### Added

- **Regelkorpus:** `tests/korpus.rs` prüft jede Kennung aus `Meta::ids` direkt mit Treffer, ähnlichem konformem Gegenbeispiel und nicht anwendbarem Dokument, dazu Grenzwerte und Varianten; ein Gate scheitert, sobald einer Kennung eine der drei Fallarten fehlt (casoon/auditmysite#783).
- **Mutationskatalog:** `tests/mutations/*.toml` nennt je Kennung gezielte Mutationen, die ein Korpusfall töten muss; `xtask/mutate.py` führt sie nacheinander aus, ein Gate prüft Abdeckung und Eindeutigkeit.

### Changed

- **Geltungsbereich:** `media/audio-autoplay` sieht das ganze Markup (versteckter Ton spielt trotzdem), `tables/headers-attr-invalid` nur noch den Accessibility-Tree (ACT a25f45), die Checkliste `manual/*` das Dargestellte auch unter `aria-hidden`.
- **Landmarks:** Eine Seite ohne jeden Inhalt meldet keine `landmarks/*-missing` mehr; `landmarks/content-outside` übergeht, was der Baum auslässt (inert hinter einem modalen Dialog).
- **Sprunglink:** `keyboard/skip-link-missing` meldet nur, wenn es etwas zu überspringen gibt (ein Link, Navigation oder Kopfbereich).
- **Links:** `color/link-indistinct` ist `REVIEW` statt `FAIL`, wenn die Linkfarbe 3:1 zum Text erreicht (G183); die Linkregeln (`links/*-name`, `purpose-link-only`, `color/link-indistinct`) prüfen auch Elemente mit der Rolle `link`.
- **Kontextwechsel:** `context/on-input` ist `REVIEW` statt `FAIL`, wenn das Feld eine Beschreibung hat (G13), und prüft auch Kontrollkästchen (F37).

### Fixed

- **Kontrast:** Ohne Toleranz gegen die Schwelle (4,499:1 erreicht 4,5:1 nicht, Understanding 1.4.3); inaktive Bedienelemente sind ausgenommen.
- **Dokument:** Ein leeres `lang` ist `document/lang-missing`, nicht `lang-invalid`; Untertags über 8 Zeichen sind ungültig; nur der HTML-`<title>` zählt, nicht der eines SVG.
- **Überschriften:** `role="heading"` mit `aria-level` zählt für `headings/h1-missing`, `empty` und `skip-level`; ein `hN` mit fremder Rolle nicht; ein leeres `aria-label` benennt keine Überschrift.
- **Rollen:** `aria/role-invalid` meldet nicht, wenn ein späteres Token eine gültige Rolle ist (ACT 674b10).
- **Namen:** `names/required-missing` umfasst `tree`, `grid` und `treegrid`; `label-in-name/mismatch` meldet auch einen Namen, der nur ein Teil des sichtbaren Textes ist.
- **Formulare:** `forms/placeholder-as-label` gilt auch neben einem `aria-label`; `forms/autocomplete-invalid` nicht an Schaltflächen und deaktivierten Feldern; ein `<button>` mit ungültigem `type` ist ein Absende-Button.
- **Listen:** `lists/empty` nicht zusätzlich zu `lists/invalid-structure`; ein `<dt>` ohne folgendes `<dd>` ist `lists/term-without-definition`; `<menu>` ist eine Liste.
- **Tabellen:** `title` benennt eine Tabelle (`tables/name-missing`).
- **Sprache:** Zitate sind für `language/part-unmarked` nicht mehr ausgenommen.
- **Stylesheets:** `text/justified` und `text/line-height-tight` lesen das `style`-Attribut; `focus/outline-removed` zählt ein `:focus-visible`, das den Rahmen entfernt, als Entfernen und `:focus:not(:focus-visible)` nicht.
- **Zeit:** `timing/meta-refresh` meldet auch genau 20 Stunden (ausgenommen ist erst „länger als 20 Stunden").
- **Bilder:** `images/server-side-map` nur an einem `img[ismap]` in einem Link.
- **Darstellungskonvention:** `viz/*` übergeht Figuren unter `hidden`.
- **Zielgröße:** `targets/size` und `targets/size-enhanced` messen einen Inline-Link samt der ersetzten Elemente darin (Bild im Link) und behandeln `iframe`, `object` und `embed` nicht als Ziel.
- **Scrollbereiche:** `keyboard/scrollable-region-not-focusable` meldet auf einer leeren Seite kein „ungemessen" mehr, wenn der Host `html` oder `body` misst.
- **Landmarks:** `landmarks/content-outside` übergeht einen ausgelassenen Knoten nur, wenn unter ihm nichts Erreichbares liegt oder er eine Landmark ist; Inhalt in einem generischen `div`, das Chrome auslässt, bleibt gemeldet.
- **Links:** `color/link-indistinct` meldet einen Link ohne eigenen Stil neben einem gestylten Elternelement nicht mehr als ungemessen; er wird nicht dargestellt.
- **Objekte:** Der Ersatzinhalt eines `<object>` zählt als Textalternative, auch wenn der Browser ihn nach dem Laden nicht darstellt (H53).
- **Listen:** `lists/item-outside-list` steht jetzt in `Meta::ids` der Listenregel. Bisher fehlte für die Kennung der `RuleRun`, und sie fehlte in der [Regelreferenz](https://casoon.github.io/barrierlab/docs/a11y/reference/regeln/).

## [0.26.0] - 2026-10-26

### Changed

- **Bedingt:** `keyboard/click-handler-not-focusable` ist `REVIEW`, wenn ein per Tab erreichbares Element darin steckt; `svg/name-missing` ist `FAIL` nur mit Rolle `img` oder `graphics-*`, sonst `REVIEW`.
- **Formulare:** Ein Feld, das nur einen Platzhalter hat, meldet nicht mehr `forms/label-missing`, sondern `forms/placeholder-as-label` (`REVIEW`): Der Platzhalter stiftet den Namen.
- **Landmarks:** `landmarks/main-missing`, `main-duplicate`, `not-top-level`, `banner-duplicate`, `contentinfo-duplicate` und `content-outside` sind Best Practice ohne 1.3.1 bzw. 2.4.1.
- **Normative Zuordnung:** Alle Regeln geprüft (auditmysite#779): Ein `FAIL`
  mit WCAG-Kriterium steht nur noch dort, wo das Markup den Verstoß belegt.
  Entscheidung und Begründung je Regel:
  [Normative Zuordnung](https://casoon.github.io/barrierlab/docs/packages/a11y-rules/#normative-zuordnung).
- **Rahmen:** `frames/name-missing` trägt nur noch 4.1.2, nicht mehr 2.4.1.
- **Rollen und Listen:** `aria/role-redundant`, `lists/role-redundant`, `patterns/tooltip-unreferenced` und `lists/empty` sind Best Practice ohne Kriterium.
- **Überschriften:** `headings/h1-missing` und `headings/skip-level` tragen das Schlagwort `best-practice` statt 1.3.1. Eine korrekt ausgezeichnete h2 ohne h1 ist kein Verstoß.
- **Überschriften:** `headings/none` (2.4.1) meldet nur noch, wenn neben Überschriften auch main-Landmark und Sprunglink fehlen — `FAIL`, wenn Navigation oder Kopfbereich als Landmark ausgezeichnet sind, sonst `REVIEW`. Eine einfache Seite mit `<main>` erzeugt keinen Befund mehr.
- **Vermutungen:** `REVIEW` statt `FAIL` für `tables/header-missing`, `keyboard/positive-tabindex`, `ids/duplicate`, `links/used-as-button`, `images/server-side-map` (jetzt 2.1.1 statt 1.1.1), `dialog/focusable-missing` und `forms/error-unidentified`, `group-missing`, `group-name-missing`, `no-submit`, `placeholder-as-label`.
- **Viewport:** `zoom/viewport-missing` ist Best Practice ohne 1.4.4 und 1.4.10.

## [0.25.0] - 2026-10-25

### Changed

- **Gleichschritt:** mit `a11y-report` 0.25.0; keine eigene Änderung.

## [0.24.1] - 2026-10-24

### Fixed

- **Viewport:** `zoom/viewport-scale-limited` (Begrenzung zwischen 200 % und
  500 %) trägt nicht mehr 1.4.4, sondern das Schlagwort `best-practice`: 1.4.4
  verlangt 200 % und ist in diesem Fall erfüllt. Hosts zählten den Befund
  bisher als WCAG-Verstoß (auditmysite, inros-lackner.de mit
  `maximum-scale=2`). Der Verstoß unter 200 % (`zoom/viewport-locked`) trägt
  1.4.4 weiter.

## [0.24.0] - 2026-10-24

### Added

- **Links:** Linkzweck aus auditmysite (`link_purpose`, `link_purpose_link_only`;
  casoon/barrierlab#13, #18), alle Befunde Tier 2 und `REVIEW`:
  `links/url-name` (2.4.4, niedrig, nur ohne Kontext),
  `links/single-character-name` (2.4.4, mittel, Ziffern ausgenommen),
  `links/new-window-unannounced` (3.2.5 AAA, niedrig) und
  `links/purpose-link-only` (2.4.9 AAA, niedrig). Details:
  [Linkzweck](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0240-linkzweck).
- **Links:** `links/new-window-unannounced` prüft `target="_blank"`; die
  Hinweisliste kennt auch deutsche Ankündigungen („öffnet in neuem Fenster").

### Changed

- **Links:** `links/generic-name` nutzt auditmysites Liste: rund 150 Phrasen
  in 15 Sprachen. Bewusste Kehrtwende (Entscheidung des Nutzers vom
  23.10.2026): Navigationsbeschriftungen wie „Start", „Weiter", „Download"
  oder „Info", bisher bewusst ausgenommen, zählen jetzt. Erwartbar mehr
  Hinweise in Navigationen; die Testvorlage meldet jetzt „Start".
- **Links:** `links/generic-name` wertet den Zusammenhang aus: Mit
  `aria-describedby`, Text daneben, Text im selben `p`/`li`/`td`/`th`/`dd` oder
  einer Überschrift davor im selben Baustein (bis vier Ebenen hoch) ist der
  Befund Low statt Medium. Nachgestellte Pfeile und Satzzeichen („Mehr ›")
  werden vor dem Vergleich entfernt.
- **WASM:** 154,4 KB gzip (Budget 170 KB).

## [0.23.0] - 2026-10-23

### Added

- **Checkliste:** sieben neue `manual/*`-Punkte für die Prüfhinweise, die
  auditmysite bisher selbst erzeugte (#39, casoon/auditmysite#722), wie die
  übrigen `UNTESTED` einmal je Seite und nur mit Anlass:
  `manual/pointer-gestures` (2.5.1), `manual/motion-actuation` (2.5.4, eigene
  Regel mit `Scope::Markup`), `manual/no-timing` (2.2.3),
  `manual/interruptions` (2.2.4), `manual/re-authenticating` (2.2.5),
  `manual/unusual-words` (3.1.3), `manual/help` (3.3.5). Anlässe:
  [Prüfhinweise](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#checkliste).
- **Kontrast:** `contrast/control-insufficient` (`FAIL`, mittel) und
  `contrast/control-undetermined` (`UNTESTED`), WCAG 1.4.11: Die Grenze eines
  selbst gezeichneten Kontrollkästchens, Optionsfelds, Schalters oder
  Schiebereglers (`appearance: none`) braucht 3:1 gegenüber dem Hintergrund
  des Elternelements. Grenze ist der obere Rahmen, sonst die eigene Fläche;
  native Elemente werden nicht gemessen. Fehlt der Hintergrund: `UNTESTED`.
  [Herkunft](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#kontrast-von-bedienelementen).

### Changed

- **Baumbereiche:** Alle ID-Verweise lösen je Baumbereich auf (#23):
  `ids/duplicate`, `aria/reference-missing`, `aria/owns-conflict`,
  `popover/target-missing`, `label[for]`, `headers`, `form`,
  `aria-describedby` in den Formularregeln, Tooltips, Visualisierungen.
  Befunde ändern sich für Dokumente mit Shadow DOM, wenn der Host die Bereiche
  liefert: gleiche IDs in getrennten Roots sind kein Duplikat mehr; ein Verweis
  aus dem Light DOM in eine Shadow Root ist `aria/reference-missing`; ein
  `<label for>` über die Grenze lässt `forms/label-missing` stehen. Ohne
  Shadow DOM unverändert.

## [0.22.0] - 2026-10-05

### Added

- **Zielgrößen:** `targets/size-enhanced` (`REVIEW`, niedrig; WCAG 2.5.5 AAA):
  44 × 44 CSS-px, Ausnahmen „inline" und „Equivalent", keine
  Abstandsausnahme. Gemeldet wird nur, was `targets/size` nicht schon meldet.
  Belege der Heuristik-Änderungen dieser Version (auditmysite#698):
  [Zielgrößen und Heuristiken](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0220-zielgrößen-und-heuristiken-698).

### Changed

- **Bewegung:** `motion/infinite-animation` meldet `<marquee>` — ohne eigene
  Pause, auch ohne gemessenes Layout (Korpus `no_focus_targets`). Lade-Kreisel
  mit kurzem Durchlauf bleiben gemeldet: 2.2.2 gilt für Bewegung, die
  insgesamt länger als fünf Sekunden läuft (bewusste Abweichung von
  auditmysite).
- **Fokus:** `focus/obscured` übergeht optisch verborgene Elemente (spiegel.de:
  1 × 1 px großer Sprunglink).
- **Reihenfolge:** `order/visual-mismatch` zählt auch umgeordneten Text ohne
  Bedienelemente — 1.3.2 betrifft die Lesereihenfolge (Korpus
  `text_and_layout`). Nicht mehr gemeldet: Behälter, deren Bedienelemente alle
  zum selben Ziel führen (spiegel.de: 20 Teaserkarten mit Bild- und
  Titellink, `flex-row-reverse`).
- **Tastatur:** `keyboard/pointer-only` übergeht das Kartenmuster:
  `cursor: pointer` auf einem Kasten mit Link oder Knopf darin
  (bundesregierung.de: 27 Teaserkarten).
- **Zielgrößen:** `targets/size` (2.5.8), Ausnahme „Equivalent": Ein sichtbarer
  Link zum selben Dokument von mindestens 24 px genügt; Verweise innerhalb der
  Seite (`#setup`, `#`) haben kein Gegenstück, ein Gegenstück unter
  `aria-hidden`, `inert`, optisch verborgen oder außerhalb des Bilds zählt
  nicht (Korpus `target_size_equivalent`).
- **Zielgrößen:** `targets/size`: Nachbarn unter `inert` oder mit
  `pointer-events: none` engen nicht ein (`target_size_hidden_neighbours`,
  #705); solche Ziele, optisch verborgene und aus dem Bild geschobene
  (Sprunglinks bei −99999 px, bundesregierung.de) werden auch selbst nicht
  gemessen.
- **Zielgrößen:** `targets/size`: Ein Ziel, das beim Messen noch wächst
  (`Layout::animating`), bleibt `UNTESTED` statt zu klein
  (`target_size_animation`, #706).
- **Zielgrößen:** `targets/size`: Zeigerziele sind alle Bedienelemente, nicht
  nur die per Tab erreichbaren. Bei „roving tabindex" haben die übrigen
  Elemente einer Gruppe `tabindex="-1"` und werden trotzdem angeklickt
  (bundesregierung.de, Karussellpunkte 10 × 10 px, `#slick-slide-control00`).

## [0.21.0] - 2026-10-05

### Added

- **Kontrast:** `contrast/text-enhanced` (`FAIL`, mittel; WCAG 1.4.6 AAA):
  7:1, bei großem Text 4,5:1. Gemeldet wird nur, was 1.4.3 besteht; was schon
  dort verfehlt, steht nicht doppelt. Belege (casoon/barrierlab#47):
  [Kontrast](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0210-kontrast-47).
- **Kontrast:** Über Bildern und Verläufen: Fehlt die Hintergrundfarbe,
  liefert der Host aber eine Abtastung (`Rendering::sampled_backdrop`),
  urteilt die Regel wie auditmysite vor der Umstellung — Median verfehlt:
  `FAIL`; Median und 40. Perzentil bestehen: bestanden; dazwischen `REVIEW`
  unter `contrast/text-undetermined` (bzw. `contrast/text-enhanced` für AAA).
- **Kontrast:** Befunde tragen das gemessene und das geforderte Verhältnis als
  `Evidence::computed` (`contrast_ratio`, `required_ratio`, bei Abtastung
  zusätzlich `contrast_ratio_p40`) — für Berichte, die die Zahlen zeigen.

### Changed

- **Heuristiken:** Ist das `Layout`-Feld, das eine Heuristik braucht, an keinem
  Element gemessen, meldet sie einmal `UNTESTED` für die Seite. Vorher lief
  sie still ohne Befund und zählte als gelaufen. Betrifft
  `order/visual-mismatch`, `motion/infinite-animation`, `reflow/min-width`,
  `focus/obscured` und den `cursor`-Teil von `keyboard/pointer-only`.
- **Kontrast:** Optisch verborgener Text (`Rendering::visually_hidden`) wird
  nicht gemessen — er hat keinen sichtbaren Kontrast.
- **Kontrast:** Text, dessen Mitte ein fixiertes oder klebendes Element
  überdeckt (`Layout::obscured`, etwa ein Cookie-Banner), ist
  `contrast/text-undetermined` statt eines Urteils über Farben, die man so
  nicht sieht (auditmysite #716).
- **Kontrast:** Text unter `aria-hidden` wird weiter gemessen; WCAG 1.4.3 gilt
  für sichtbaren Text, unabhängig vom Accessibility-Tree (bewusste Abweichung
  von auditmysite, #395).

## [0.20.0] - 2026-10-05

### Added

- **Darstellung:** Der zweite Teil von casoon/barrierlab#21: drei Regeln aus
  auditmysite, die gemessene Darstellungswerte brauchen. Alle drei sind
  Tier 3 und urteilen mit `FAIL`; liefert der Host ein Feld nicht, steht je
  Regel und Seite ein `UNTESTED` im Bericht, kein stiller Nicht-Befund.
  Belege und Abweichungen:
  [Darstellungswerte](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0200-darstellungswerte-21-zweiter-teil).
- **Farbe:** `color/link-indistinct` (`FAIL`, mittel; 1.4.1): ein Link im
  Fließtext, der sich nur über die Farbe abhebt; braucht
  `text_decoration_line`, `font_style`, `font_family`, `border_bottom_style`.
- **Listen:** `lists/role-redundant` (`FAIL`, niedrig; 4.1.2): `role="list"`
  an `<ul>`/`<ol>`, das sie nicht braucht; braucht
  `ComputedStyle::list_style_type`.
- **Tastatur:** `keyboard/scrollable-region-not-focusable` (`FAIL`, hoch;
  2.1.1): ein scrollbarer Bereich ohne Tastaturzugang; braucht
  `Rendering::scroll_overflow_px`, Puffer 13 px wie axe.

## [0.19.1] - 2026-10-03

### Fixed

- **Bewegung:** `motion/reduced-motion-ignored` fand keine Animation, die der
  Browser geliefert hat: Chrome schreibt die Kurzschreibweise in `cssText` voll
  aus (`2s linear 0s infinite normal none running spin`), und das `none` des
  Füllmodus galt als Name. Jetzt nach CSS Animations: Ein Schlüsselwort geht
  zuerst an die Eigenschaft, die es noch nicht hat, erst der Rest ist der Name.
  Gefunden bei der Umstellung von auditmysite (casoon/auditmysite#743, Korpus
  `media_and_motion`).
- **Formulare:** Quiz radio buttons and checkboxes no longer trigger format-instruction hints (casoon/astro-post-audit#75).
- **Formulare:** Search inputs, search regions and comboboxes no longer request personal autocomplete tokens (casoon/astro-post-audit#76).
- **Sprache:** Language detection excludes descendants with their own non-empty `lang` (casoon/astro-post-audit#77).

## [0.19.0] - 2026-10-03

### Added

- **API:** `run_stylesheets` / `run_stylesheets_in` ergänzen einen Bericht um
  diese Regeln. Jedes `run_*` vermerkt sie vorher als
  `NotRun::CapabilityMissing` („Host liefert keine Stylesheets").
- **API:** `stylesheet_metas`, `stylesheet_rules`, `StylesheetRule`; neue
  Schicht `Tier::Stylesheets` in `a11y-dom`.
- **Stylesheets:** Die Regeln über Stylesheets aus auditmysite
  (casoon/barrierlab#21, erster Teil). Dort las JavaScript
  `document.styleSheets`; hier kommen die Sheets als Text herein, geparst vom
  neuen Paket `stylesheet-parse`: `focus/outline-removed` (`FAIL`, hoch;
  2.4.7), `motion/reduced-motion-ignored` (`REVIEW`, mittel; 2.3.3 AAA),
  `orientation/content-hidden` (`REVIEW`, mittel, hoch für
  `html`/`body`/`main`; 1.3.4), `text/justified` und `text/line-height-tight`
  (`REVIEW`, niedrig; 1.4.8 AAA). Belege und Abweichungen:
  [Stylesheets](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0190-stylesheets-21-erster-teil).
- **Stylesheets:** Ein Selektor-Abgleich gegen das Dokument (Teilmenge von
  Selectors Level 4, dreiwertig: Treffer, kein Treffer, unbekannt), damit nur
  zählt, was auf der Seite vorkommt — wie `querySelectorAll` in auditmysite
  (#712).

### Fixed

- **Bilder und Medien:** `images/area-alt-missing` und `media/audio-autoplay`
  (0.18.0) meldeten mit berechneten Stilen nichts: Das UA-Stylesheet gibt
  `<area>` und `<audio>` ohne `controls` `display: none`, die Sicht ließ sie
  deshalb weg. Für diese beiden zählt jetzt nur das `hidden`-Attribut. Gefunden
  bei der Umstellung von auditmysite (casoon/auditmysite#743, Korpus
  `misc_content_checks`, `media_and_visual`).

## [0.18.0] - 2026-10-03

### Added

- **Bilder und Medien:** B6 (casoon/barrierlab#19), Tier 1:
  `images/area-alt-missing`, `images/input-alt-missing`, `objects/alt-missing`
  (je `FAIL`, hoch; 1.1.1), `images/server-side-map` (`FAIL`, mittel; 1.1.1),
  `media/audio-autoplay` (`REVIEW`, mittel; 1.4.2). Tier 2, weil es die
  Namensberechnung braucht: `frames/name-missing` (`FAIL`, hoch; 2.4.1,
  4.1.2).
  [B6](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0180-bilder-und-medien-b6).
- **Darstellungskonvention:** Die statischen Regeln der
  [Darstellungsmodi](https://casoon.github.io/barrierlab/docs/a11y/concepts/darstellungsmodi/)
  (#22, Entwurf v0; `figure[data-viz]`, `html[data-display]`), Tier 1, Modul
  `viz`, nur auf Seiten mit der Konvention und mit dem Schlagwort
  `best-practice`: `viz/text-missing` (`FAIL`, hoch; 1.1.1),
  `display/text-hidden` (`FAIL`, hoch; `REVIEW`, niedrig bei Verweis per
  `aria-describedby`/`aria-details`; 1.1.1), `viz/caption-missing` (`FAIL`,
  niedrig; 1.1.1), `viz/static-missing` (`FAIL`, mittel; 2.2.2),
  `viz/table-missing` (`REVIEW`, niedrig; 1.3.1), `display/toggle-missing`
  (`FAIL`, mittel; 2.2.2), `display/init-missing` (`REVIEW`, niedrig; 2.2.2).
  [#22](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0180-darstellungskonvention-22).
- **Links und Zeiger:** B5 (casoon/barrierlab#18), Tier 1:
  `keyboard/click-handler-not-focusable` (`FAIL`, hoch; 2.1.1),
  `links/used-as-button` (`FAIL`, niedrig; 4.1.2),
  `navigation/location-missing` (`REVIEW`, niedrig; 2.4.8 AAA).
- **Regelmigration:** Die letzten drei Pakete der Regelmigration aus
  auditmysite (B5–B7, casoon/barrierlab#18–#20) und die statischen Regeln der
  Darstellungskonvention (casoon/barrierlab#22): 28 neue Kennungen. Belege,
  nicht übernommene Prüfungen und Abweichungen stehen je Paket in
  [Übernahme aus auditmysite](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0180-links-und-zeiger-b5).
- **Tabellen, Dokument, Sprache, Rollen:** B7 (casoon/barrierlab#20), Tier 1:
  `tables/header-without-data` (`FAIL`, hoch), `tables/data-undetermined`
  (`UNTESTED`, hoch), `tables/headers-attr-invalid` (`FAIL`, hoch; alle 1.3.1),
  `document/lang-mismatch` (`FAIL`, mittel; 3.1.1), `language/part-unmarked`
  (`REVIEW`, mittel) und `language/part-undetermined` (`UNTESTED`, mittel; beide
  3.1.2), `language/abbreviation-unexpanded` (`REVIEW`, niedrig; 3.1.4 AAA),
  `timing/meta-refresh` (`FAIL`, hoch; 2.2.1),
  `headings/section-without-heading` (`REVIEW`, niedrig; 2.4.10 AAA),
  `aria/role-redundant` (`FAIL`, niedrig; 4.1.2), `names/title-only`
  (`REVIEW`, mittel; 4.1.2), `patterns/tooltip-unreferenced` (`FAIL`, niedrig;
  1.4.13).
  [B7](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0180-tabellen-dokument-sprache-rollen-b7).

### Changed

- **Checkliste:** `manual/media-alternatives` erscheint auch, wenn die Seite
  ein Video von YouTube, Vimeo, Dailymotion oder Wistia per `<iframe>`
  einbettet.
- **Tastatur:** `keyboard/pointer-only` (Tier 3) meldet ein Inline-`onclick`
  nicht mehr, das `keyboard/click-handler-not-focusable` schon meldet. Auf
  einem Host mit Darstellung entstand sonst für dasselbe Element ein zweiter
  Befund.

## [0.17.0] - 2026-10-01

### Added

- **Dialoge:** `dialog/focusable-missing` (`FAIL`, mittel; 2.4.3).
- **Landmarks:** Die Landmark-, Tastatur- und Strukturregeln aus auditmysite
  (casoon/barrierlab#17, B4), Tier 2 außer `headings/none`. Die Landmark-Rolle
  bestimmt die Regel selbst aus dem Markup, nicht aus der Rolle des Hosts.
  Neu: `landmarks/not-unique` (`REVIEW`, mittel), `landmarks/not-top-level`,
  `landmarks/banner-duplicate`, `landmarks/contentinfo-duplicate`,
  `landmarks/content-outside` (je `FAIL`, mittel). Belege und Abweichungen:
  [B4](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0170-landmarks-tastatur-struktur-b4).
- **Muster:** `patterns/accordion-controls-missing` (`REVIEW`, niedrig).
- **Tastatur:** `keyboard/focusable-no-role` (`REVIEW`, niedrig) und
  `keyboard/interactive-not-focusable` (`REVIEW`, hoch).
- **Überschriften:** `headings/none` (Tier 1; `FAIL`, mittel; `REVIEW`,
  niedrig hinter offenem Dialog).

## [0.16.0] - 2026-10-01

### Added

- **Anmeldung:** `auth/captcha` (`REVIEW`, mittel).
- **Formulare:** Die Formularregeln aus auditmysite (casoon/barrierlab#16,
  B3): `forms/autocomplete-invalid` (`FAIL`, niedrig),
  `forms/purpose-missing` (`REVIEW`, mittel), `forms/error-unidentified`,
  `forms/group-missing`, `forms/group-name-missing` (je `FAIL`, mittel),
  `forms/required-unmarked` (`REVIEW`, mittel), `forms/instructions-missing`
  (`REVIEW`, niedrig), `forms/title-only-label` (`REVIEW`, mittel),
  `forms/no-submit` (`FAIL`, mittel; `REVIEW`, niedrig ohne `action` und
  Textfeld), `forms/redundant-entry` (`REVIEW`, mittel). Tier, Belege und
  Abweichungen:
  [B3](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0160-formulare-b3).
- **Kontext:** Kontextwechsel bei Fokus und Eingabe: `context/on-input`
  (`FAIL`, mittel; `REVIEW`, niedrig), `context/on-focus` (`REVIEW`, hoch),
  `context/autofocus` (`REVIEW`, mittel).

### Changed

- **Namen:** `names/required-missing` meldet Formularfeld-Rollen (`textbox`,
  `searchbox`, `combobox`, `listbox`, `spinbutton`, `slider`, `checkbox`,
  `radio`, `radiogroup`, `switch`) ohne Namen als Critical — dieselbe Schwere
  wie das native Feld ohne Label (`forms/label-missing`). Bisher High; die
  übrigen Rollen bleiben High. Beleg: auditmysite-Kalibrierung
  `combobox_missing_expanded`, `widget_patterns` (dort meldete die abgelöste
  Regel `label` Critical).

## [0.15.0] - 2026-10-01

### Added

- **Dialoge:** `dialog/name-missing` (hoch), `dialog/modal-unmarked` (mittel).
- **Namen:** Die Namensregeln aus auditmysite (casoon/barrierlab#15, B2), alle
  Tier 2: `names/required-missing` (hoch), `names/symbol-only` (mittel),
  `label-in-name/mismatch` (mittel). Unbenannte Buttons, Links und SVGs melden
  weiter ihre eigenen Regeln, nicht noch einmal `names/required-missing`.
  Abweichungen:
  [B2](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0150-namen-b2).
- **Status:** `status/live-overridden` (hoch für `alert`, mittel für `status`,
  `log`).
- **Summary:** `summary/name-missing` (hoch).

### Changed

- **Namen:** Ein leerer `role="tab"` braucht einen Namen (WCAG 4.1.2, auch ohne
  „Name Required" in ARIA 1.2) — nachgezogen nach dem Vergleichslauf mit
  auditmysite.
- **Namen:** `label-in-name/mismatch` nimmt Symbol-Labels aus (`¹⁾`,
  Understanding 2.5.3) und meldet gleiche Wörter in anderer Reihenfolge als
  `REVIEW` (dm.de).
- **Namen:** `buttons/name-missing` meldet keine `<summary>` mehr. `accname`
  gibt ihr die Rolle `button`, Chrome `DisclosureTriangle`;
  `summary/name-missing` meldet sie jetzt auf jedem Host genau einmal.

## [0.14.2] - 2026-10-01

### Fixed

- **Listen:** Ein verwaistes `listitem` meldet nur noch
  `lists/item-outside-list`, nicht zusätzlich `aria/required-parent-missing`
  (auditmysite-Kalibrierung `invalid_aria.html`). Dafür erkennt
  `lists/item-outside-list` auch ein `<li>` unter einer Liste, die per `role`
  etwas anderes geworden ist (`ul[role=tablist] > li`, auditmysite#715); ein
  `<li>` mit eigener Rolle (`none` im Menü-Muster, `tab`) gilt dort nicht als
  Listeneintrag.

## [0.14.1] - 2026-10-01

### Fixed

- **Listen:** `lists/invalid-structure`: Ein `<li>` mit einer anderen
  expliziten Rolle als `listitem` (etwa `role="group"` bei Slidern) zählt nicht
  als Listeneintrag. Bis 0.14.0 fing auditmysites eigene `aria-roles`-Regel
  diesen Fall ab; mit B1 ist sie entfallen. Beleg: berlin.de,
  `ul.swiper-wrapper > li[role=group]` (wie axe `list`).

## [0.14.0] - 2026-10-01

### Added

- **ARIA:** Die ARIA-Regeln aus auditmysite (casoon/barrierlab#14, B1); die
  Rollen- und Attributtabellen sind aus WAI-ARIA 1.2 erzeugt, als Bitmasken je
  Rolle. Tier 1: `aria/attribute-unknown`, `aria/attribute-value-invalid`,
  `aria/owns-conflict`, `aria/tab-selected-missing`, `aria/tabpanel-missing`,
  `aria/combobox-popup-missing`. Tier 2: `aria/attribute-not-allowed`,
  `aria/attribute-prohibited`, `aria/required-parent-missing`,
  `aria/required-children-missing`. Abweichungen:
  [B1](https://casoon.github.io/barrierlab/docs/packages/a11y-rules-auditmysite/#0140-aria-b1).
- **Inert:** `inert/dialog-inert` (Tier 1).
- **Popover:** `popover/target-missing`, `popover/target-invalid` (Tier 1).

### Changed

- **ARIA:** Rollen aus WAI-ARIA Graphics (`graphics-document`, `-object`,
  `-symbol`) und DPUB-ARIA 1.1 (`doc-*`) gelten für `aria/role-invalid` als
  gültig; die Attributregeln urteilen über sie nicht (auditmysite-Korpus
  `svg_graphics_role_no_name`).
- **ARIA:** `aria/reference-missing` prüft auch `aria-details`, `aria-flowto`
  und `aria-errormessage` — Letzteres nur, solange `aria-invalid` gesetzt und
  nicht `false` ist (ARIA 1.2: die Fehlermeldung ist erst dann maßgeblich).

## [0.13.5] - 2026-09-30

### Fixed

- **Landmarks:** `landmarks/main-missing` hinter einem offenen Dialog: Auch ein
  `role="dialog"`/`"alertdialog"` ohne `aria-modal` zählt — der
  Consent-Dialog von administracion.gob.es blendet `<main>` ohne es aus
  (auditmysite#709).

## [0.13.4] - 2026-09-30

### Fixed

- **Landmarks:** `landmarks/main-missing`: Ist ein modaler Dialog offen
  (`role="dialog"`/`"alertdialog"` mit `aria-modal="true"`, oder
  `<dialog open>`) und keine main-Landmark erreichbar, ist das `REVIEW` (Low)
  statt `FAIL` — die Seite blendet ihren Inhalt korrekt aus, solange der Dialog
  offen ist. Beleg: fünf von 21 EU-Portalen mit offenem Consent-Dialog
  (auditmysite#709).

## [0.13.3] - 2026-09-30

### Fixed

- **Tastatur:** `keyboard/skip-link-missing`: Ein Fragmentlink auf den Anfang
  des Hauptinhalts (die main-Landmark oder ein Element an ihrem Anfang) ist ein
  Sprunglink, wo immer er steht — auch hinter den Links eines Cookie-Banners
  (bund.de, #26). `href=""` beendet die Folge der Fragmentlinks nicht mehr. Ein
  Anker mitten im Inhalt zählt weiter nicht.

## [0.13.2] - 2026-09-30

### Fixed

- **ARIA:** `aria/reference-missing`: Ein `aria-controls` an einem Element mit
  `aria-expanded="false"` darf auf ein Ziel zeigen, das erst beim Öffnen
  entsteht (wie axe-core). Beleg: die Menüs der Web-Komponenten auf bund.de und
  sachsen-anhalt.de (Vergleichslauf auditmysite#690).
- **Sichtbarkeit:** Der Inhalt eines geschlossenen `<details>` — alles außer
  der ersten `<summary>` — gilt als nicht dargestellt (HTML, „The details
  element"). Chrome blendet ihn über `content-visibility` aus, berechnetes
  `display` und `visibility` halten ihn für sichtbar. Beleg: geographia.eu, 22
  Karten-Links in `details.more` als `links/name-missing` gemeldet
  (Vergleichslauf auditmysite#690).

## [0.13.1] - 2026-09-30

### Fixed

- **ARIA:** `aria/required-attribute-missing` folgt ARIA 1.2: `slider` braucht
  nur noch `aria-valuenow` (min/max haben Vorgaben), `option` kein
  `aria-selected`. Native Felder mit expliziter Rolle
  (`<input type="range" role="slider">`, `<input type="checkbox" role="switch">`,
  `<meter>`, `<progress>`) übermitteln ihren Zustand selbst und werden nicht
  gemeldet (auditmysite#656). Neu geprüft: `meter` und der fokussierbare
  `separator` ohne `aria-valuenow`.
- **ARIA:** `aria/reference-missing` prüft auch `aria-activedescendant` und
  meldet leere `aria-controls`, `aria-owns` und `aria-activedescendant`.
- **Bilder:** `images/alt-missing`: Ein `<img>` mit nicht leerem `aria-label`,
  `aria-labelledby` oder `title` hat eine Textalternative (ARIA6, ARIA10, H67).
- **Landmarks:** `landmarks/banner-missing`, `contentinfo-missing`: Ein
  `<header>`/`<footer>` unter einem Vorfahren mit Rolle `article`,
  `complementary`, `main`, `navigation` oder `region` ist keine
  Seiten-Landmark (HTML-AAM, auditmysite#639) — bisher zählte nur der Tag des
  Vorfahren.
- **SVG:** `svg/name-missing`: Ein SVG in einem benannten Link oder Button
  braucht keinen eigenen Namen; `role="none presentation"` gilt als dekorativ.
- **Tastatur:** `keyboard/hidden-focusable`: `disabled` nimmt nur
  Formularfelder aus der Tab-Folge; ein `<a href disabled>` bleibt erreichbar
  und wird gemeldet.
- **Tastatur:** `keyboard/skip-link-missing` erkennt den Sprunglink am Ziel und
  an der Stellung wie axe `isSkipLink`: Fragmentlinks vor dem ersten Link, der
  die Seite verlässt. „Aller au contenu" wurde bisher nicht erkannt
  (auditmysite#642). Die Wortliste bleibt als zweiter Weg.
- **Viewport:** `zoom/viewport-locked`: `content` wird nach CSS Viewport
  zerlegt — Semikolon und Leerraum trennen wie das Komma.
  `width=device-width; maximum-scale=1` blieb bisher unerkannt. Wie alle
  Korrekturen dieser Version aus dem Abgleich mit auditmysite vor dem Umzug
  seiner Regeln (casoon/barrierlab#13, auditmysite#690): Wo die geteilte
  Fassung einen Fall übersah oder zu Unrecht meldete, ist sie nachgezogen.

## [0.13.0] - 2026-09-30

### Breaking

- **API:** `Meta` hat ein Pflichtfeld mehr (`scope`); die Regeln der
  `*_rules()`-Listen laufen im Runner über die interne Sicht. Hosts, die nur
  `run_*` und `*_metas()` benutzen, sind nicht betroffen.

### Added

- **Checkliste:** `manual/*`, Tier 1. Kriterien, die keine Maschine
  entscheiden kann, erscheinen je Seite einmal als `UNTESTED`, sobald die Seite
  etwas enthält, für das sie gelten: `media-alternatives` (1.2.1–1.2.5, bei
  Audio/Video), `image-alternatives` (1.1.1, bei Bildern — auch `alt=""`),
  `visual-structure` (1.3.1, bei Text), `use-of-color` (1.4.1, bei Links und
  Feldern), `timing` (2.2.1, bei Feldern oder `meta refresh`),
  `error-handling` (3.3.1, 3.3.3, bei Feldern), `authentication` (3.3.8, bei
  Passwortfeldern oder Captcha-Spuren). Verortet am Wurzelknoten
  (liveaudit#7).
- **Heuristiken:** Tier 3, alle `REVIEW`, über `Rendering::layout` und
  `bounds` (liveaudit#6): `order/visual-mismatch` (1.3.2, 2.4.3),
  `motion/infinite-animation` (2.2.2; je Gruppe, entfällt bei einem
  Bedienelement zum Anhalten), `reflow/min-width` (1.4.10),
  `keyboard/pointer-only` (2.1.1, 4.1.2; `cursor: pointer` oder `onclick` ohne
  Rolle und tabindex — per JavaScript angehängte Handler sieht keine Seite von
  innen), `focus/obscured` (2.4.11), `targets/size` (2.5.8, mit der
  Abstandsausnahme nach Normtext; Checkbox und Radio im Label zählen mit dem
  Label), `focus/indicator-missing` und `focus/indicator-unmeasured` (2.4.7;
  ohne Messung `UNTESTED`).
- **Heuristiken:** Belegt an barrierlab.eu: auf 237 intakten Seiten 28
  Zielgrößen-Kandidaten, sonst keine Meldung; die absichtlich eingebauten
  Barrieren Reihenfolge, Bewegung, Reflow, Verdeckung und Fokus werden
  gemeldet.

### Changed

- **Formulare:** `forms/label-missing` sieht ein nicht dargestelltes `<label>`
  nicht mehr als Label an, wenn der Host Stile liefert (liveaudit#3).
- **Geltungsbereich:** `Meta` trägt ein neues Feld `scope`
  (`Scope::AccessibilityTree`, `Rendered`, `Markup`), und jede Regel läuft über
  eine Sicht auf das Dokument, in der versteckte Teilbäume fehlen: unter
  `aria-hidden="true"` (nur `AccessibilityTree`) und nicht dargestellt —
  `display: none`/`visibility: hidden`, wo der Host Stile liefert, sonst das
  `hidden`-Attribut. Bisher liefen die Regeln auch über versteckte Elemente;
  die Namensberechnung überspringt versteckten Inhalt zu Recht, der Button im
  versteckten Banner bekam also einen leeren Namen und einen `FAIL`.
- **Geltungsbereich:** Gemessen mit LiveAudit auf barrierlab.eu (237 Seiten,
  29.09.2026): 992 FAILs durch versteckte Elemente (`buttons/`, `links/`,
  `svg/name-missing`, `lists/empty`), danach keiner (liveaudit#1). Hosts mit
  eigenem `is_ignored` ändert das nichts an den Tier-2-Regeln, wohl aber an den
  strukturellen.
- **Geltungsbereich:** Verweise (`aria/reference-missing`, `ids/duplicate`)
  und die dokumentweiten Regeln sehen weiterhin das ganze Markup:
  `aria-labelledby` darf auf Verstecktes zeigen.
- **Tastatur:** `keyboard/hidden-focusable` prüft jedes fokussierbare Element
  unter einem Vorfahren mit `aria-hidden="true"`, nicht nur das Element mit dem
  Attribut (axe: `aria-hidden-focus`). Ausgenommen: `inert`, `tabindex` < 0,
  `<a>` ohne `href`, `<input type="hidden">` (liveaudit#4). Der Befundtext
  nennt den Vorfahren.

## [0.12.2] - 2026-09-29

### Fixed

- **Listen:** `lists/invalid-structure` und `lists/empty` sehen durch `<slot>`
  hindurch: Was unter einem `<slot>` hängt, zählt als Kind der Liste.
  Web-Komponenten bauen ihre Liste im Shadow Root (`<ul><slot></slot></ul>`)
  und bekommen die Einträge aus dem Light DOM (Beleg: sachsen-anhalt.de,
  `muse-link-list`). Voraussetzung ist ein Host, der den flachen Baum liefert —
  siehe `a11y-dom`.
- **Listen:** `<ul>`/`<ol>` mit einer anderen gültigen Rolle als `list` wird
  nicht mehr als Liste geprüft — bisher galt das nur für
  `presentation`/`none`. Eine leere `<ul role="listbox">` (APG-Autocomplete,
  Beleg: gov.uk) ist keine leere Liste.

## [0.12.1] - 2026-09-28

### Added

- **Texte:** Feature `de` (Vorgabe: an) für die deutschen Befundtexte und
  Hinweise. Mit `default-features = false` kommen die deutschen Vorlagen gar
  nicht erst in den Binärcode; `Locale::De` und `Meta::help_de` gibt es dann
  nicht — wer Deutsch verlangt, bekommt einen Compile-Fehler statt stillem
  Englisch. Rein additiv: Mit den Vorgaben ändert sich nichts.

## [0.12.0] - 2026-09-28

### Breaking

- **API:** Regelfunktionen nehmen die Sprache mit
  (`fn(&D, Locale, &mut Vec<Finding>)`), und `Meta` hat ein weiteres Feld. Wer
  nur die `run*`-Einstiege benutzt, merkt davon nichts.

### Added

- **Texte:** Befundtexte auf Deutsch. `Locale` (`En`, `De`; Vorgabe `En`) und
  die Einstiege `run_in`, `run_with_semantics_in`, `run_full_in` und
  `run_with_rendering_in`. Übersetzt ist, was ein Werkzeug einem Menschen
  zeigt: `message`, der Grund eines nicht gelaufenen Vermerks und der Hinweis
  der Deklaration (`Meta::help_de`, `Meta::help_in`). Kennungen, Outcomes,
  Schweregrade und WCAG-Bezüge hängen nicht von der Sprache ab. Die bisherigen
  Einstiege liefern unverändert Englisch.

### Changed

- **IDs:** `ids/duplicate` folgt WCAG 2.2. 4.1.1 (Parsing) ist gestrichen;
  gemeldet wird nur noch eine doppelte ID, auf die ein IDREF zeigt (`for`,
  `form`, `list`, `headers` und die ARIA-Verweise) — dann ist die Beziehung
  nicht eindeutig auflösbar. WCAG-Bezug jetzt 4.1.2, wie axe-core
  `duplicate-id-aria`. Eine Dopplung, auf die niemand zeigt, erzeugt keinen
  Befund mehr. Die Regel lag bisher als Filter in auditmysite; hier gilt sie
  für alle Hosts gleich.

## [0.11.3] - 2026-09-28

### Changed

- **Gleichschritt:** mit `accname` 0.11.3; keine eigene Änderung.

## [0.11.2] - 2026-09-28

### Changed

- **Gleichschritt:** mit `accname` 0.11.2; keine eigene Änderung.

## [0.11.1] - 2026-09-28

### Changed

- **Build:** Edition 2024 statt 2021, Edition und MSRV werden jetzt vom
  Workspace geerbt. Rein intern; die öffentliche Fläche ist unverändert.

## [0.11.0] - 2026-09-23

### Changed

- **Texte:** Befundtexte und Regel-Hinweise auf Englisch. Entstanden im
  Repository `casoon/a11y-core` und hier nachgezogen; ab jetzt ist dieses
  Monorepo die einzige Quelle.

## [0.10.2] - 2026-09-23

### Changed

- **Repository:** Das Crate liegt jetzt im Monorepo `casoon/barrierlab`;
  `repository` und `homepage` zeigen dorthin. Keine Änderung an Code oder
  öffentlicher Fläche.
