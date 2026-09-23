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

## Was im Repository liegt

```
.
├── Cargo.toml              # Workspace, noch ohne Member
├── rust-toolchain.toml     # stable, mit rustfmt und clippy
├── pnpm-workspace.yaml     # packages/*, site
├── release-plz.toml        # ein Tag je Paket: <paket>-vX.Y.Z
├── .github/workflows/      # ci.yml, release-plz.yml (bis Phase 3 nur manuell)
├── crates/                 # leer — die Pakete ziehen einzeln mit Historie ein
├── packages/               # leer — npm kommt mit a11y-wasm
├── docs/                   # diese Doku
└── plan/                   # lokal, gitignored
```

**Es ist noch kein Paket importiert.** Ein virtuelles Manifest ohne Member lässt
sich nicht bauen; die Rust- und Node-Schritte in CI prüfen deshalb erst, ob es
etwas zu bauen gibt. Diese Bedingung fällt mit dem ersten Crate weg.

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
