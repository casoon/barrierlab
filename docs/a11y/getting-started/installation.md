---
title: "Installation"
description: "Welche Crates ein Werkzeug braucht und mit welcher Rust-Version sie bauen."
order: 1
---

Die Crates liegen auf crates.io und brauchen Rust 1.85 oder neuer. Welche davon ein Projekt
einbindet, hängt davon ab, was es tun will.

## Regeln ausführen

```sh
cargo add a11y-rules@0.10.0 a11y-dom@0.10.0 a11y-report@0.10.0
```

`a11y-rules` bringt die Regeln und die Einstiegsfunktionen `run`, `run_with_semantics`,
`run_with_rendering` und `run_full`. `a11y-dom` brauchen Sie für das Dokumentmodell, `a11y-report`
für den Bericht, den die Regeln zurückgeben.

## Accessible Name berechnen

```sh
cargo add accname@0.10.0 a11y-dom@0.10.0
```

`accname` ist generisch über das Dokumentmodell aus `a11y-dom` und lässt sich ohne die Regeln
verwenden. Ein Host, der die Rolle und den Namen nicht selbst liefert, hängt es in seine
`Semantics`-Implementierung ein, siehe [Schnellstart](../schnellstart/).

## Nur das Berichtsmodell

```sh
cargo add a11y-report@0.10.0
```

Für Werkzeuge, die eigene Prüfungen haben, aber denselben JSON-Vertrag sprechen wollen.
`a11y-report` hängt nur von `serde` ab.

## Versionen

Die vier Crates werden im Gleichschritt versioniert. Ein Bruch an einem von ihnen hebt alle vier
auf dieselbe Minor-Version. Neue Regelkennungen gelten ebenfalls als Minor-Schritt, weil sie
Befunde erscheinen lassen, die ein Konsument bisher nicht erwartet hat.
