---
title: "Darstellungsmodi"
description: "Die Konvention für Visualisierungen und Darstellungsmodi (Entwurf v0): drei Schichten je Grafik, ein seitenweiter Modus, und die Regeln, die sie prüfen."
order: 4
---

Eine Seite mit Diagrammen, Karten und 3D-Szenen soll niemanden ausschließen, ohne auf die
Grafiken zu verzichten. Die Konvention löst das mit **einer Einstellung für die ganze Seite** —
so selbstverständlich wie Hell und Dunkel — und mit **drei Schichten je Visualisierung**, aus
denen der Modus auswählt. Sie stammt aus der Referenzumsetzung
[Geographia](https://geographia.eu) und gilt seit dem 28.09.2026 BarrierLab-weit. Der Stand hier
ist **Entwurf v0**: Namen und Pflichtangaben können sich noch ändern.

Die Konvention ist keine WCAG-Anforderung. Sie ist ein Weg, mehrere Kriterien auf einmal
verlässlich zu erfüllen — 1.1.1 (Textalternative), 1.3.1 (Werte als Tabelle), 2.2.2 (Bewegung
abschalten) —, und macht diesen Weg maschinell prüfbar.

## Drei Modi

| Modus | zeigt | gedacht für |
|---|---|---|
| `visual` | alles, auch 3D, Animation, Interaktion | die Vorgabe |
| `calm` | Standbilder statt Bewegung | Bewegungsempfindliche, kein WebGL, `Save-Data` |
| `text` | keine Grafik; Kernaussage, Werte, Quelle | Screenreader-Nutzer, die die Seite linear lesen; Vorlesen; Druck |

Der Modus steht als `<html data-display="visual|calm|text">` am Dokument. Ein kleines,
blockierendes Skript in `<head>` setzt ihn **vor dem ersten Zeichnen**: aus `localStorage`
(Schlüssel `display`), ersatzweise `calm` bei `prefers-reduced-motion`. Ein Umschalter, markiert
mit `[data-display-toggle]`, ändert ihn. Es gibt **eine URL je Seite** — kein getrenntes
Text-Portal, das doppelt gepflegt wird und veraltet.

## Drei Schichten je Visualisierung

Jede Visualisierung ist eine `<figure data-viz="…">`. Der Wert sagt, was sie ist: `chart`,
`diagram`, `image`, `3d` oder `interactive`.

| Schicht | Inhalt | Pflicht |
|---|---|---|
| `[data-viz-text]` | die Kernaussage in einem Satz, die Werte (bei `chart` als `<table>`), Quelle | immer, und immer im HTML |
| `[data-viz-static]` | Standbild derselben Aussage: SVG oder Bild | bei `3d` und `interactive` |
| `[data-viz-live]` | 3D, Animation, Interaktion; wird nur in `visual` geladen | nein |
| `<figcaption>` | benennt die Visualisierung | immer |

Die Textschicht ist **nie** per `hidden`, `aria-hidden` oder `inert` verborgen. Außerhalb des
Textmodus wird sie nur visuell ausgeblendet (eine `visually-hidden`-Klasse); Screenreader lesen
sie dann im Seitenfluss, mit ihrer Struktur. Ein Verweis per `aria-describedby` ist nicht nötig
und liest die Aussage sonst doppelt vor.

```html
<html lang="de" data-display="visual">
<head>
  <script src="/display-init.js"></script>  <!-- klassisch, ohne defer -->
</head>
<body>
  <header>
    <div data-display-toggle>
      <button type="button" aria-pressed="true">Visuell</button>
      <button type="button" aria-pressed="false">Ruhig</button>
      <button type="button" aria-pressed="false">Text</button>
    </div>
  </header>
  <main>
    <figure data-viz="interactive">
      <div data-viz-live><!-- three.js, nur in "visual" --></div>
      <svg data-viz-static role="img" aria-label="Erwärmung je Region" viewBox="0 0 640 360">…</svg>
      <div class="viz-text" data-viz-text>
        <p>Die Kernaussage in einem Satz.</p>
        <table>…</table>
        <p>Quelle: …</p>
      </div>
      <figcaption>Erwärmung je Region</figcaption>
    </figure>
  </main>
</body>
</html>
```

Eine Skizze der Stile dazu:

```css
html:not([data-display="text"]) [data-viz-text] { /* visually hidden */ }
html[data-display="text"] [data-viz-live],
html[data-display="text"] [data-viz-static] { display: none; }
html[data-display="calm"] [data-viz-live] { display: none; }
```

## Warum so

- **Seitenweit statt je Grafik.** Gängig ist ein Umschalter „Tabelle zeigen" an jedem Diagramm.
  Wer keine Grafiken sehen kann oder will, muss ihn dann überall finden und betätigen. Ein Modus
  für die ganze Seite trifft die Entscheidung einmal.
- **Im Inhalt, nicht im Overlay.** Die Schichten schreibt der Autor, kein nachgeladenes Werkzeug
  rät sie. Was fehlt, fehlt im HTML — und ist damit prüfbar.
- **Text ist gestaltet, nicht abgerüstet.** Der Textmodus ist eine eigene, gute Leseansicht mit
  Kernaussage und Tabellen, keine Restseite.
- **Vor dem ersten Zeichnen.** Setzt erst ein spätes Skript den Modus, läuft die Animation an,
  bevor sie verschwindet — genau das, was `calm` verhindern soll.

## Prüfung

`a11y-rules` prüft die statische Hälfte; jede Regel meldet nur auf Seiten, die die Konvention
benutzen, trägt das Schlagwort `best-practice` und hängt am nächsten WCAG-Kriterium. Was erst
die laufende Seite zeigt — berechnete Sichtbarkeit, der Zeitpunkt von `data-display`, ob im
Textmodus noch Grafik sichtbar ist —, misst [auditmysite](https://github.com/casoon/auditmysite)
(`display/text-not-visible`, `display/text-media-visible` und die gerenderten Teile der übrigen).

| Kennung | prüft | Urteil, Schwere | WCAG |
|---|---|---|---|
| `viz/text-missing` | `figure[data-viz]` ohne nicht leeres `[data-viz-text]` | `FAIL`, hoch | 1.1.1 |
| `display/text-hidden` | `[data-viz-text]` mit `hidden`, `aria-hidden="true"` oder `inert` (an ihm oder einem Vorfahren) | `FAIL`, hoch; `REVIEW`, niedrig, wenn ein Element der Grafik per `aria-describedby`/`aria-details` darauf verweist | 1.1.1 |
| `viz/caption-missing` | `figure[data-viz]` ohne `<figcaption>` als Kind | `FAIL`, niedrig | 1.1.1 |
| `viz/static-missing` | `3d` oder `interactive` ohne `[data-viz-static]` | `FAIL`, mittel | 2.2.2 |
| `viz/table-missing` | `chart` ohne `<table>` | `REVIEW`, niedrig | 1.3.1 |
| `display/toggle-missing` | Visualisierungen, aber kein `[data-display-toggle]` | `FAIL`, mittel | 2.2.2 |
| `display/init-missing` | weder `html[data-display]` im Markup noch ein blockierendes Skript in `<head>` | `REVIEW`, niedrig | 2.2.2 |

`display/text-hidden` heißt im ersten Entwurf `viz/text-hidden`; die Kennung folgt auditmysite,
damit statische und gemessene Befunde zusammenfallen.

### Grenzen

- **Die Kernaussage wird nicht bewertet.** `viz/text-missing` prüft, dass Text da ist, nicht ob
  er die Aussage der Grafik trifft. Das kann keine Maschine entscheiden.
- **Medien außerhalb der Konvention** (`canvas`, `video`, `svg[role=img]` ohne `figure[data-viz]`)
  prüft keine Regel. Der Entwurf sah `viz/orphan-media` vor; dafür fehlt bisher ein echter
  Fall.
- **Ob ein Skript den Modus setzt,** zeigt erst die laufende Seite. Statisch steht nur fest, dass
  ohne blockierendes Skript in `<head>` niemand ihn rechtzeitig setzen kann.
- **Nur `figure` trägt die Prüfungen.** Ein `data-viz` an einem anderen Element wird nicht auf
  Text, Beschriftung und Standbild geprüft.
