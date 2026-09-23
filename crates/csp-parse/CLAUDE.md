# csp-parse

Rust-Crate: eigenständiger Parser für die Content-Security-Policy-
Direktiven-Grammatik. Konzept & Herkunft: `README.md`. Umsetzungsplan:
`plan/`.

Projektname (Repo) und voraussichtlicher crates.io-Paketname sind hier
identisch, `csp-parse`. Vor Veröffentlichung erneut prüfen.

Schwesterprojekt von [`html-conform`](../html-conform) (künftiger
Baustein für dessen `w:content-security-policy`/`w:integrity-metadata`-
Datatype-Implementierung, Phase 05c dort). Steht aber für sich:
generisch, kein HTML-Bezug.

## Architektur (Arbeitstitel, siehe `plan/` für Details)

```
CSP-String (ein Policy oder Komma-getrennte Liste) → Tokenizer
                                                     → Parser
                                                     → strukturierte Direktiven
                                                       (Name + Werte je Direktive)
```

## Arbeitsweise

- Aktueller Stand & nächster Schritt: `plan/00-STATUS.md`.
- Phasenpläne mit Schritten/Exit-Kriterien: `plan/0N-*.md`. Vor größeren
  Änderungen die passende Phase lesen, nicht am Plan vorbei arbeiten.
- Getroffene Entscheidungen: `plan/DECISIONS.md` — dort nachschlagen,
  bevor offene Fragen neu aufgerollt werden.

## Feste Regeln

- Lizenz: **MIT**, von Anfang an.
- Normative Grundlage: [Content Security Policy Level 3](https://www.w3.org/TR/CSP3/).
  Bei Unklarheiten dort nachschlagen, nicht aus anderen Implementierungen
  raten (auch nicht aus vnu/htmlunit-csp — deren Verhalten darf als
  Referenz/Vergleichspunkt dienen, ist aber nicht normativ, siehe eigene
  Recherche-Notizen in `html-conform/plan/05c-research-group-b.md` zu
  vnus dortigen Quirks/Abweichungen).
- Kein HTML-Bezug im Kern.
- Kein `unsafe` ohne expliziten Grund und Kommentar.

## Definition of Done

Siehe "Exit-Kriterien" in der jeweiligen `plan/0N-*.md`-Datei — nicht
global definiert, sondern pro Phase.
