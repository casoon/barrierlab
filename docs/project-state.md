# Project State

**Stand: 2026-09-23**

## Was ist das

Ein **Bibliotheks-Monorepo** für Web-Audits. Hier entsteht die gemeinsame
Berechnung, die heute verstreut in mehreren Repositories liegt; von hier wird
nach crates.io und npm veröffentlicht. Die Werkzeuge bleiben außerhalb und
binden die Pakete aus den Registries ein.

Vorgeschichte: Dieses Verzeichnis war bis 2026-09-22 reines Konzeptmaterial für
einen browserseitigen Vorabtest der Zugänglichkeit von Interaktionen (Snapshot →
handeln → Snapshot → Differenz). Dieser Motor kommt als Paket `a11y-perception`
hierher; ein eigenständiger Host dafür bekommt ein eigenes Repository. Der
Kenntnisstand dazu steht in `plan/reader/`.

## Stand

Phase 3 ist durch: die vier a11y-Crates liegen hier, mit ihrer Historie, und
0.10.2 ist **aus diesem Repository** auf crates.io veröffentlicht — inklusive
korrigierter `repository`-Metadaten. Die Veröffentlichung lief lokal mit
`cargo publish`; der Workflow erkennt die Pakete und die Tags, sein Upload-Weg
ist aber noch ohne Anmeldung (Trusted Publishing je Crate steht aus).

## Was im Repository liegt

```
.
├── Cargo.toml              # Workspace, noch ohne Member
├── rust-toolchain.toml     # stable, mit rustfmt und clippy
├── pnpm-workspace.yaml     # packages/*, site
├── release-plz.toml        # ein Tag je Paket: <paket>-vX.Y.Z
├── .github/workflows/      # ci.yml, release-plz.yml (bis Phase 3 nur manuell)
├── crates/                 # a11y-report, a11y-dom, accname, a11y-rules (0.10.2)
├── packages/               # leer — npm kommt mit a11y-wasm
├── examples/a11y/          # Beispielgenerator der a11y-Crates (eigener Workspace)
├── docs/                   # diese Doku, docs/a11y/ die ausführliche a11y-Doku
└── plan/                   # lokal, gitignored
```

Die Node-Schritte in CI prüfen weiter, ob es überhaupt etwas zu bauen gibt —
`packages/` ist bis Phase 5 leer. Die Rust-Schritte laufen seit dem Import
vollständig: fmt, clippy und Tests über den Workspace.

## Wo die Arbeit liegt

`plan/status.md` ist der einzige Index: Migrationsstand und der Backlog des
Reader-Themas. Der Referenzrahmen des Reader-Themas steht in
`plan/reader/spezifikation/` und wird fortgeschrieben, nicht abgearbeitet.

## Was hier absichtlich fehlt

`constraints.md` und `decisions.md` beschreiben laut Konvention den Ist-Zustand.
Beides entsteht mit dem ersten importierten Paket — Entscheidungen, die sich an
einer Zeile Code belegen lassen, gibt es noch nicht. Die Entscheidungen über
Zuschnitt, Historie, Werkzeuggrenze und Doku stehen bis dahin in
`plan/00-monorepo-konzept.md`.
