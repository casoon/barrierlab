# Releases

## Tags

Ein Tag je Paket: `<paket>-vX.Y.Z`, zum Beispiel `accname-v0.11.0`. Es gibt
keinen gemeinsamen Repository-Tag. Konfiguriert in `release-plz.toml`.

Die vier a11y-core-Crates bleiben im Gleichschritt versioniert (heute 0.10.1),
weil sie einen gemeinsamen Vertrag bilden. Alles andere versioniert eigenständig.

## crates.io

`release-plz` führt Changelog und Version je Crate, öffnet einen Release-PR und
veröffentlicht nach dem Merge in Abhängigkeitsreihenfolge.

- Workflow: `.github/workflows/release-plz.yml`. Bis zum ersten Import nur von
  Hand auslösbar, mit `dry_run` als Standard.
- Veröffentlichung über **crates.io Trusted Publishing** (GitHub OIDC), nicht
  über `CARGO_REGISTRY_TOKEN`. Dafür muss jedes Crate einmalig auf crates.io
  einen Trusted Publisher für dieses Repository und diesen Workflow eintragen.
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
