# html-conform

Ergänzt die Regeln im Wurzelverzeichnis; hier steht nur, was für dieses Paket
zusätzlich gilt.

## Schichten

```
HTML-String → parse (HTML5-Baum) → schema (RELAX NG) → assertions (Schematron) → Vec<Finding>
```

Jede Schicht ist einzeln testbar und liefert in dasselbe `Finding`.

## Feste Regeln

- **`rules/*.sch` ist der einzige Ort für Fachlogik der Assertion-Schicht** —
  deklarative XPath-Regeln, kein Rust-Code für Co-Constraints.
- **`assertions.rs` spricht die Engine nur über den `SchematronEngine`-Trait an**,
  nie direkt gegen eine konkrete Implementierung.
- **Kein `reqwest`, kein Docker, keine JVM zur Laufzeit.** vnu, Trang und Java
  sind Werkzeuge der Vendor- und Bauzeit, nichts davon geht ins Paket.
- `schema/` und `tests/corpus/` sind vendorter Fremdcode (vnu,
  `validator/validator`) — nur über `xtask/vendor-*.sh` aktualisieren.
- Die Prüfung ist **differentiell gegen vnu abgeglichen, nicht deckungsgleich**.
  Welche Klassen fehlen, steht in `docs/html-conform/`; eine Behauptung über
  vollständige Abdeckung gehört dort nicht hin.

## Eigene Workspaces

`fuzz/` und `xtask/check-file/` gehören nicht zum Workspace des Monorepos
(`workspace.exclude`) und haben deshalb eine eigene `[workspace]`-Zeile. CI baut
sie einzeln mit.
