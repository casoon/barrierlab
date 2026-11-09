---
title: "Stand"
description: "Was im Repository liegt und wo die offenen Aufgaben stehen."
order: 5
---

**Stand: 2026-11-05**

## Was ist das

Ein **Bibliotheks-Monorepo** für Web-Audits. Die gemeinsame Berechnung der
Werkzeuge (auditmysite, astro-post-audit, liveaudit, Relief) liegt hier und
wird nach crates.io und npm veröffentlicht. Die Werkzeuge bleiben in eigenen
Repositories und binden die Pakete aus den Registries ein
([Konsumenten](consumers.md)).

## Was im Repository liegt

16 Crates unter `crates/`, dazu die npm-Pakete `packages/a11y-wasm` und `packages/html-conform-wasm`:

| Schicht | Pakete |
|---|---|
| L0 | `html5-parser`, `css-tokenize`, `csp-parse`, `media-query-parse`, `xpath-eval`, `relax-ng`, `schematron-engine` |
| L1 | `a11y-dom`, `a11y-report`, `html-conform`, `stylesheet-parse` |
| L2 | `accname`, `style-cascade`, `a11y-rules`, `a11y-perception`, `web-checks` |
| L3 | `a11y-wasm` (Crate und npm-Paket `@casoon/a11y-wasm`, gebaut über `packages/a11y-wasm/build.mjs`), `html-conform-wasm` (Crate und npm-Paket `@casoon/html-conform-wasm`, noch nicht veröffentlicht) |

Schichten und Regeln: [Architektur](architecture.md). Je Paket eine Seite unter
`docs/packages/`.

```
.
├── crates/          # die 16 Crates
├── packages/        # a11y-wasm, html-conform-wasm (npm)
├── examples/a11y/   # Beispielgenerator der a11y-Crates (eigener Workspace)
├── docs/            # diese Doku
├── site/            # Projektseite (Pages)
├── xtask/           # Hilfsskripte
└── release-plz.toml # ein Tag je Paket: <paket>-vX.Y.Z
```

## Wo die Aufgaben stehen

Ein lokales `plan/` wird nicht mehr genutzt. **GitHub-Issues sind der einzige
Tracker.** Tracking-Issue für `html-conform` ist #89; der Rest steht in den
offenen Issues dieses Repositorys.

## Prüfungen

Lokal: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`. Releases laufen über `release-plz` (siehe
[Releases](releasing.md)).
