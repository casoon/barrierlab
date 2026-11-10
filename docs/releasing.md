---
title: "Releases"
description: "Tags, release-plz, crates.io, npm und der WASM-Bauweg."
order: 4
---

## Tags

Ein Tag je Paket: `<paket>-vX.Y.Z`, zum Beispiel `accname-v0.11.0`. Es gibt
keinen gemeinsamen Repository-Tag. Konfiguriert in `release-plz.toml`.

Die vier a11y-core-Crates bleiben im Gleichschritt versioniert,
weil sie einen gemeinsamen Vertrag bilden. Alles andere versioniert eigenständig.

## crates.io

**Kein Release-PR.** Actions dürfen in diesem Repo keine Pull Requests anlegen,
und bei einem Betreuer braucht es den Umweg nicht. Der Ablauf:

1. Version im Crate-Manifest anheben und den Changelog-Eintrag schreiben —
   normaler Commit auf `main`.
2. `.github/workflows/release-plz.yml` läuft bei jedem Push auf `main` und
   veröffentlicht in Abhängigkeitsreihenfolge, was noch nicht in der Registry
   steht. Unveränderte Versionen übergeht es.
3. Tag (`<paket>-vX.Y.Z`) und GitHub-Release legt release-plz dabei selbst an.

Mit `workflow_dispatch` und `dry_run` (Standard) zeigt der Workflow nur die
Versionen im Repo, ohne etwas zu veröffentlichen.
### Trusted Publishing

Veröffentlicht wird über **crates.io Trusted Publishing** (GitHub OIDC), nicht
über ein Token. `release-plz` tauscht das OIDC-Token selbst gegen ein
kurzlebiges crates.io-Token — deshalb steht in diesem Repository **kein**
`CARGO_REGISTRY_TOKEN` und auch nicht `rust-lang/crates-io-auth-action`.

Auf der Repo-Seite ist alles eingerichtet:

| | Wert |
|---|---|
| Workflow | `.github/workflows/release-plz.yml` |
| Berechtigung | `id-token: write` |
| Umgebung | keine |

Auf crates.io trägt **jedes Crate einmalig** einen Trusted Publisher ein, unter
*Settings → Trusted Publishing → Add*:

| Feld | Wert |
|---|---|
| Repository owner | `casoon` |
| Repository name | `barrierlab` |
| Workflow filename | `release-plz.yml` |
| Environment | leer |

Eine GitHub-Umgebung wird **nicht** benutzt. Ihr Name ginge in den OIDC-Anspruch
ein und müsste auf crates.io genauso eingetragen sein; das ist eine zusätzliche
Stelle, an der beides auseinanderlaufen kann. Wer den Schutz später will, trägt
die Umgebung an beiden Stellen gleichzeitig ein — im Job **und** in jeder
Crate-Konfiguration.

**Ein neues Crate lässt sich damit nicht anlegen** — crates.io verlangt für die
erste Version eine Veröffentlichung von Hand. Erst danach greift der Weg oben.

`xml-adapters` ist so ein Fall (noch nicht veröffentlicht). Es hängt von `xpath-eval` und `relax-ng` ab, die es als Dev-Abhängigkeit zurückholt: `xpath-eval` führt `xml-adapters` nur per `path` ohne `version` (Cargo lässt den Eintrag beim Verpacken weg), damit die Veröffentlichungsreihenfolge nicht zirkulär wird. Reihenfolge beim ersten Mal: `xml-adapters` von Hand, danach läuft alles über `release-plz`.
- `repository`-Metadaten zeigen nach dem Import auf dieses Repository. Der erste
  Release eines umgezogenen Crates korrigiert damit auch den Link auf crates.io.

## npm

Nur für Pakete unter `packages/`. Veröffentlicht werden `@casoon/a11y-wasm` und
`@casoon/html-conform-wasm` (Tag-Präfix `html-conform-wasm-npm-v`, sonst derselbe Ablauf) über
**npm Trusted Publishing** (GitHub OIDC, Provenance, kein Token) aus
`.github/workflows/release-npm.yml`:

1. Version in `packages/a11y-wasm/package.json` auf die des Crates bringen, Commit
   auf `main`.
2. Tag `a11y-wasm-npm-vX.Y.Z` auf diesen Commit setzen und pushen. Der Tag muss
   zur Version in `package.json` passen. Ein eigenes Präfix, weil release-plz
   `a11y-wasm-vX.Y.Z` für das Crate mit dem Workflow-Token setzt und ein solcher
   Tag keinen Workflow startet.
3. Der Job `build` baut das WASM-Artefakt, packt das Tarball und prüft, dass
   LICENSE, README und `pkg/` darin liegen. Der Job `publish-npm` wartet auf die
   Freigabe in der GitHub-Umgebung `npm` und veröffentlicht genau dieses Tarball.

Auf npmjs.com trägt **jedes Paket** einmalig einen Trusted Publisher ein
(*Settings → Trusted Publisher*): Organisation `casoon`, Repository `barrierlab`,
Workflow `release-npm.yml`, Umgebung `npm`, „Allow npm publish“ angehakt. Ein
neues Paket muss dafür einmal von Hand veröffentlicht werden, weil npm den Eintrag nur für
vorhandene Pakete anbietet; npm zeigt danach eine Frist (zwei Tage), in der ein Publish aus CI den
Eintrag bestätigen muss.

## WASM

`wasm-pack` kennt kein `--profile` und baut deshalb immer mit
`[profile.release]`. Für Größenoptimierung gibt es `[profile.wasm-release]`
(`opt-level = "z"`, `panic = "abort"`) im Workspace-Manifest; gebaut wird
entsprechend über

```
cargo build --profile wasm-release --target wasm32-unknown-unknown -p a11y-wasm
wasm-bindgen --target web --out-dir packages/a11y-wasm/pkg <artefakt>
```

statt über `wasm-pack build --release`.

## Binaries: `html-conform`

Das Crate `html-conform` baut mit dem Feature `cli` ein Kommandozeilenwerkzeug. Seine
Binaries entstehen in `.github/workflows/release-html-conform-binaries.yml`: je Ziel
(Linux x86_64 und aarch64 als musl, macOS x86_64 und aarch64, Windows x86_64) ein Archiv mit
`.sha256`, dazu eine kombinierte `SHA256SUMS` und eine Build-Provenance-Attestation
(`actions/attest-build-provenance`). Alles landet am GitHub-Release des Tags
`html-conform-vX.Y.Z`; wo release-plz noch keines angelegt hat, legt der Workflow es an.

Der Tag kommt weiterhin von release-plz (unverändert). Ein mit dem Workflow-Token gesetzter Tag
startet allerdings keinen Workflow (wie bei `release-npm.yml`). Deshalb nach dem
release-plz-Lauf von Hand anstoßen:

```
gh workflow run release-html-conform-binaries.yml -f tag=html-conform-vX.Y.Z
```

Ein von Hand gepushter Tag `html-conform-v*` startet den Workflow selbst. Der Tag muss zur
Version in `crates/html-conform/Cargo.toml` passen. Im README des Crates steht, wie ein
Download zu prüfen ist (`shasum -a 256 -c`, `gh attestation verify`).

## Was hier sonst nicht veröffentlicht wird

Keine weitere Binärauslieferung. Plattform-Binaries und die npm-Pakete, die sie
nachladen, bleiben bei den Werkzeugen (astro-post-audit, auditmysite).
