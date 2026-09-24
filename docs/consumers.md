---
title: "Konsumenten"
description: "Die Werkzeuge außerhalb dieses Repositorys und was sie von hier benutzen."
order: 3
---

Die Werkzeuge leben außerhalb dieses Repositorys und binden die Pakete aus den
Registries ein. Diese Liste sagt, wer was benutzt — sie ist der Grund, warum ein
Paket hier liegt.

| Werkzeug | Repository | benutzt heute | wird benutzen |
|---|---|---|---|
| auditmysite | `casoon/auditmysite` | `a11y-report`, `a11y-dom`, `a11y-rules`, `accname`, `html-conform` | `a11y-perception`, `web-checks` |
| astro-post-audit | `casoon/astro-post-audit` | `a11y-report`, `a11y-dom`, `a11y-rules`, `accname`, `html-conform` | `web-checks` |
| liveaudit | `casoon/liveaudit` | die vier a11y-Crates direkt (WASM-Adapter im Repo) | `a11y-wasm` als npm-Paket |
| Reader-Host | noch kein Repository | — | `a11y-perception` |
| auditmysite_studio | `casoon/auditmysite_studio` | `auditmysite` als Bibliothek | — |

Regeln, die sich daraus ergeben:

- **Zwei Konsumenten, dann Bibliothek.** Etwas wandert hierher, wenn zwei Hosts
  es nachweislich gleich brauchen — nicht, weil es sich teilen ließe.
- Ein Host darf jederzeit hinter der neuesten Version hängen. Breaking Changes
  brauchen deshalb einen Grund und einen Eintrag im Changelog des Pakets.
- Die Website `casoon/web-barrierlab` verlinkt Werkzeuge und Pakete; sie hat eine
  Neutralitätsauflage (keine Agenturverweise, offene Quellen erlaubt). Links von
  dort zeigen auf GitHub, crates.io und die Doku-Seite dieses Repositorys.
