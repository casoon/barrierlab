# Changelog

Alle nennenswerten Änderungen an `style-cascade` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

## [0.1.2] - 2026-11-08

### Added

- **Geteilter Regelsatz:** `RuleSet` (gelesene, indizierte Regeln; unveränderlich, `Send + Sync`) und `Cascade::with_rules(Arc<RuleSet>)`: Seiten mit denselben Stylesheets bauen den Satz einmal und legen je Seite nur die Zwischenspeicher an. `Cascade::new` bleibt, wie es war (casoon/barrierlab#145).
- **Import-Auflösung:** `expand_imports`, `Resolution`, `ExpandedImports`, `imports_in` und `wrap_media` (aus `a11y-static` hierher verschoben). `Resolution::Inert` lässt der Host einen Import fallen, von dem er weiß, dass er nur `@font-face` enthält, statt dass er alles unbestimmt macht (casoon/barrierlab#145).
- **Farbschema:** `Cascade::is_dark` und die Auswertung von `color-scheme` (CSS Color Adjustment 1 §2) samt `<meta name=color-scheme>`; `parse_color` liest die Systemfarben `Canvas`, `CanvasText`, `LinkText`, `VisitedText`, `ActiveText`, `ButtonFace`, `ButtonText`, `Field`, `FieldText` (casoon/barrierlab#145).

### Changed

- **UA-Farben (Verhalten):** `Cascade::specified` liefert für die UA-Farbe von Links, Formularfeldern und Schaltflächen die Systemfarbe (`linktext`, `fieldtext`, `buttontext`, `field`, `buttonface`) statt eines Hex-Werts; `Cascade::color` und `computed_style` lösen sie auf.

### Fixed

- **Farbschema:** `color-scheme: dark` kippt jetzt die UA-Standardfarben (Text, Links, Formularfelder, Leinwand); vorher blieb ein Link `#0000ee` auf dunklem Grund, was im Dunkeltest einen Kontrast-Fehlalarm ergab. `light-dark()` folgt dem verwendeten Farbschema des Elements statt der Vorliebe der Umgebung: ohne `color-scheme` gilt der helle Zweig auch bei dunkler Vorliebe (casoon/barrierlab#145).

## [0.1.1] - 2026-11-07

### Added

- **`@import`:** `Import::parse` und `Import::wrap` legen die Regeln eines gelieferten Imports in die Ebene (`layer`, `layer(x)`, anonym), unter `supports()` und die Medienliste (CSS Cascade 5 §6.4.1); ohne gelieferte Datei bleibt es bei `Unknown`. Siehe `docs/packages/style-cascade.md`.

### Fixed

- **Ebenen:** Ebenennamen werden vor dem Vergleich unescaped (CSS Syntax 3 §4.3.7): `@layer \61` und `@layer a` sind dieselbe Ebene, ein maskierter Punkt (`a\.b`) trennt keine Unterebene.

## [0.1.0] - 2026-11-06

### Added

- **Kaskade:** `Cascade::unresolved_imports` nennt die unaufgelösten `@import`-Regeln; der Host ersetzt sie durch die Regeln des importierten Blatts, um Antworten zu bekommen.
- **Kaskade:** `Cascade` ordnet Angaben nach Wichtigkeit, Ebene, `style`-Attribut, Spezifität und Reihenfolge; Vererbung, `var()`, `@media` und die Kurzschreibweisen `background`, `font`, `border*`, `text-decoration` und `list-style`. Unbestimmbares bleibt `Unknown`.
- **Berechnete Stile:** `Cascade::computed_style` liefert das `ComputedStyle` des Darstellungs-Tiers; `effective_background` und `backdrop_luminances` bestimmen den Hintergrund hinter dem Text, auch über Verläufe.
- **Selektoren:** `selector::Selector` mit `Specificity` und zwei Abgleichsmodi (`Resting` für die Kaskade, `Possible` für „kann diese Regel greifen?"); liest CSS-Escapes im Bezeichner (`.bg-error\/20`).
- **Darstellungsattribute:** `bgcolor`, `background`, `<font color>`, `<font size>`, `<body text>` und `<body link>` zählen wie eine Angabe mit Spezifität 0.
- **Farben:** `parse_color` liest Hex, `rgb()`, `hsl()`, `hwb()`, `oklab()`, `oklch()`, `lab()`, `lch()`, `color(srgb …)`, `light-dark()` und `color-mix()` in srgb und oklab.

### Changed

- **Selektoren:** Im Ruhezustand sind `:disabled`, `:enabled`, `:checked`, `:required`, `:optional`, `:read-only` und `:read-write` (HTML §4.16.3, `contenteditable` inklusive) aus den Attributen entschieden (inkl. `fieldset`-Vererbung und `legend`-Ausnahme) statt `Unknown`; `:nth-child()`, `:nth-last-child()` (mit `of S`), `:nth-of-type()`, `:nth-last-of-type()`, `:first-of-type`, `:last-of-type` und `:only-of-type` werden ausgewertet.
- **Medien:** `hover`, `pointer`, `prefers-contrast` und `forced-colors` sind `Unknown`, solange `Environment` sie nicht festlegt. `Environment` hat dafür die Felder `hover`, `fine_pointer`, `contrast_more` und `forced_colors` (je `Option<bool>`); wer `Environment` ohne `..Default::default()` als Literal baut, muss sie ergänzen.

### Fixed

- **Kaskade:** Verschachtelte Ebenen (`@layer a { @layer b {} }`, `@layer a.b`, anonyme Blöcke) werden nach CSS Cascade 5 §6.4 geordnet statt eingeebnet: Unterebenen stehen in ihrer Ebene, deren eigene Regeln schlagen sie (bei `!important` umgekehrt), und die Reihenfolge folgt dem ersten Auftreten in Dokumentreihenfolge, nicht mehr den vorab gelesenen `@layer`-Anweisungen. Eine Ebene, die nur unter einer Bedingung mit unbekanntem Ausgang oder mit ungültigem Namen vorkommt, macht die betroffenen Eigenschaften `Unknown`.
- **Kaskade:** Das `style`-Attribut steht über allen Ebenen, auch bei `!important`; Darstellungsattribute (`bgcolor` usw.) verlieren gegen jede Autorenregel, auch in einer `@layer` (CSS Cascade 5 §6.1, §6.5).
- **Kaskade:** Ein unaufgelöstes `@import` wurde stillschweigend ignoriert und ergab sichere Antworten; jetzt ist die Antwort `Unknown`, außer sie kommt aus einem `!important` im `style`-Attribut.
