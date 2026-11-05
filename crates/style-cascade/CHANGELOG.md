# Changelog

Alle nennenswerten Änderungen an `style-cascade` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

### Added

- **Kaskade:** `Cascade::unresolved_imports` nennt die unaufgelösten `@import`-Regeln; der Host ersetzt sie durch die Regeln des importierten Blatts, um Antworten zu bekommen.
- **Kaskade:** `Cascade` ordnet Angaben nach Wichtigkeit, Ebene, `style`-Attribut, Spezifität und Reihenfolge; Vererbung, `var()`, `@media` und die Kurzschreibweisen `background`, `font`, `border*`, `text-decoration` und `list-style`. Unbestimmbares bleibt `Unknown`.
- **Berechnete Stile:** `Cascade::computed_style` liefert das `ComputedStyle` des Darstellungs-Tiers; `effective_background` und `backdrop_luminances` bestimmen den Hintergrund hinter dem Text, auch über Verläufe.
- **Selektoren:** `selector::Selector` mit `Specificity` und zwei Abgleichsmodi (`Resting` für die Kaskade, `Possible` für „kann diese Regel greifen?"); liest CSS-Escapes im Bezeichner (`.bg-error\/20`).
- **Darstellungsattribute:** `bgcolor`, `background`, `<font color>`, `<font size>`, `<body text>` und `<body link>` zählen wie eine Angabe mit Spezifität 0.
- **Farben:** `parse_color` liest Hex, `rgb()`, `hsl()`, `hwb()`, `oklab()`, `oklch()`, `lab()`, `lch()`, `color(srgb …)`, `light-dark()` und `color-mix()` in srgb und oklab.

### Fixed

- **Kaskade:** Das `style`-Attribut steht über allen Ebenen, auch bei `!important`; Darstellungsattribute (`bgcolor` usw.) verlieren gegen jede Autorenregel, auch in einer `@layer` (CSS Cascade 5 §6.1, §6.5).
- **Kaskade:** Ein unaufgelöstes `@import` wurde stillschweigend ignoriert und ergab sichere Antworten; jetzt ist die Antwort `Unknown`, außer sie kommt aus einem `!important` im `style`-Attribut.
