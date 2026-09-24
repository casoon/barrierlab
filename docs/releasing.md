---
title: "Releases"
description: "Tags, release-plz, crates.io, npm und der WASM-Bauweg."
order: 4
---

## Tags

Ein Tag je Paket: `<paket>-vX.Y.Z`, zum Beispiel `accname-v0.11.0`. Es gibt
keinen gemeinsamen Repository-Tag. Konfiguriert in `release-plz.toml`.

Die vier a11y-core-Crates bleiben im Gleichschritt versioniert (heute 0.10.1),
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
| Umgebung | `crates-io` |

Auf crates.io trägt **jedes Crate einmalig** einen Trusted Publisher ein, unter
*Settings → Trusted Publishing → Add*:

| Feld | Wert |
|---|---|
| Repository owner | `casoon` |
| Repository name | `barrierlab` |
| Workflow filename | `release-plz.yml` |
| Environment | `crates-io` |

Der Name der Umgebung geht in den OIDC-Anspruch ein und wird mitgeprüft. Ohne
ihn könnte jeder, der einen Branch pushen darf, einen geänderten Workflow
starten und von dort veröffentlichen.

**Ein neues Crate lässt sich damit nicht anlegen** — crates.io verlangt für die
erste Version eine Veröffentlichung von Hand. Erst danach greift der Weg oben.
- `repository`-Metadaten zeigen nach dem Import auf dieses Repository. Der erste
  Release eines umgezogenen Crates korrigiert damit auch den Link auf crates.io.

## npm

Nur für Pakete unter `packages/`. Tag-getrieben, mit `--provenance`. Scope ist
noch nicht entschieden (`@casoon` wie die übrigen Pakete oder `@barrierlab`).

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

## Was hier nicht veröffentlicht wird

Keine Binärauslieferung. Plattform-Binaries und die npm-Pakete, die sie
nachladen, bleiben bei den Werkzeugen (astro-post-audit, auditmysite).
