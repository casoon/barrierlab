# barrierlab

Bibliotheks-Monorepo für Web-Audits. **Hier liegen nur Bibliotheken** — die
Werkzeuge (auditmysite, astro-post-audit, liveaudit) haben eigene Repositories
und binden die Pakete über crates.io und npm ein.

Architektur, Konventionen und je Paket eine Seite: [`docs/`](docs/).
Lokaler Stand und offene Punkte: `plan/status.md` (gitignored).

## Schichten

```
L0  html5-parser · csp-parse · media-query-parse · xpath-eval · relax-ng · schematron-engine
L1  a11y-dom · a11y-report · html-conform
L2  accname · a11y-rules · a11y-perception · web-checks
L3  a11y-wasm (Crate + npm)
──  Repo-Grenze: Hosts binden nur über die Registries ein
```

## Feste Regeln

- **Abhängigkeiten nur nach unten.** Kein Paket importiert eines derselben oder
  einer höheren Schicht. Kein Host in diesem Repository.
- **Browserfrei bis L3.** Kein `chromiumoxide`, keine CDP-Typen in einer
  öffentlichen API. Ein Paket nimmt Daten entgegen, es beschafft sie nicht.
- **Kein Host-Format im Kern.** Instanzdokumente kommen über ein generisches
  Trait herein, nicht über einen eingebauten Parser für HTML oder XML.
- **Intern `path` + `version`** über `workspace.dependencies` — dasselbe
  Manifest baut lokal und lässt sich veröffentlichen.
- **Ein Befundmodell**: `a11y-report`. Kein Paket definiert ein eigenes.
- **„Lief nicht" ist nicht „bestanden".** Fehlt einer Prüfung ihre
  Voraussetzung, entsteht `NotRun` mit Grund, niemals ein stiller Erfolg.
- **Kein `unsafe`** ohne Grund und Kommentar; wo möglich `#![forbid(unsafe_code)]`.
- **Lizenz MIT** überall. `reuse lint` und `cargo deny check licenses` müssen
  durchlaufen (Konfiguration im Wurzelverzeichnis).
- **Vendorter Fremdcode wird nicht von Hand geändert**, nur über die
  `xtask/vendor-*.sh`-Skripte: `crates/html-conform/{schema,tests/corpus}`,
  `crates/relax-ng/tests/corpus/relaxng`. Herkunft steht in
  `THIRD-PARTY-NOTICES.md` bzw. `UPSTREAM.md`.
- **Kein Code aus `dholroyd/relaxng-rust`** — unlizenziert. Die öffentlich
  beschriebene Architektur darf als Anregung dienen, der Quelltext nicht.

## Normative Grundlage je Paket

Bei Unklarheit in der Spezifikation nachschlagen, **nicht** aus anderen
Implementierungen raten. Fremdverhalten (vnu, Chrome) ist Vergleichspunkt, nicht
Norm — Abweichungen werden als solche ausgewiesen.

| Paket | Norm |
|---|---|
| `html5-parser` | WHATWG HTML, Tokenizer und Tree Construction |
| `csp-parse` | Content Security Policy Level 3 |
| `media-query-parse` | CSS Syntax Level 3 (Tokenisierung), Media Queries Level 4 (Grammatik, inkl. Range-Syntax) |
| `xpath-eval` | XPath 1.0 |
| `relax-ng` | RELAX NG Specification (relaxng.org) |
| `schematron-engine` | ISO/IEC 19757-3; XPath ausschließlich über `xpath-eval` |
| `accname` | accname 1.2 und HTML-AAM |
| `a11y-rules` | WCAG 2.2, Kennungen in `Meta::ids` vorab deklariert |
| `html-conform` | HTML-Konformanz, differentiell gegen vnu — siehe `crates/html-conform/CLAUDE.md` |

## Releases

Ein Tag je Paket (`<paket>-vX.Y.Z`), Changelog je Crate, `release-plz` auf
`main`. Die vier a11y-Crates bleiben im Gleichschritt versioniert. Einzelheiten:
[`docs/releasing.md`](docs/releasing.md).

`a11y-wasm` wird über `packages/a11y-wasm/build.mjs` gebaut, **nicht** über
`wasm-pack`: das kennt kein `--profile` und würde das Größenbudget sprengen.

## Definition of Done

1. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace` — alle drei grün.
2. Neue oder geänderte öffentliche Fläche: Eintrag im `CHANGELOG.md` des Pakets
   und, wenn sich die Aussage des Pakets ändert, in `docs/packages/<name>.md`.
3. Eine neue Regel zeigt auf einen Fall aus einem echten Korpus. Ohne Beleg
   kommt sie nicht hinein — das hält Sonderfallsammlungen klein.
4. Ein Feld gilt erst als vorhanden, wenn ein Lauf es gefüllt gesehen hat.
   Teuer gelernt: zwei Felder eines Snapshots waren monatelang tot, ohne dass
   Tests oder Lesen es zeigten.
