# BarrierLab

Gemeinsame Bibliotheken für Web-Audits: Zugänglichkeit, HTML-Konformanz und
Wahrnehmung. **Hier liegen nur Bibliotheken** — die Werkzeuge, die sie benutzen,
haben eigene Repositories und binden die Pakete über crates.io und npm ein.

| Was | Wo |
|---|---|
| Doku, Architektur, Konventionen | [`docs/`](docs/) |
| Rust-Crates | `crates/` |
| npm-Pakete | `packages/` |

## Stand

Gerüst. Noch kein Paket importiert — die Crates ziehen einzeln ein, jedes mit
seiner Historie. Reihenfolge und Begründung: [`docs/architecture.md`](docs/architecture.md).

## Werkzeuge, die diese Bibliotheken benutzen

[auditmysite](https://github.com/casoon/auditmysite) ·
[astro-post-audit](https://github.com/casoon/astro-post-audit) ·
[liveaudit](https://github.com/casoon/liveaudit)

Einzelheiten: [`docs/consumers.md`](docs/consumers.md).

## Lizenz

MIT
