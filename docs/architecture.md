---
title: "Architektur"
description: "Schichten, Abhängigkeitsgraph und die Regeln, unter denen ein Paket hier liegt."
order: 1
---

## Grundsatz

Dieses Repository enthält **Bibliotheken**, keine Werkzeuge. Jeder Host —
auditmysite, astro-post-audit, liveaudit, Relief —
lebt in seinem eigenen Repository und bindet die Pakete über crates.io bzw. npm
ein. Eine Bibliothek hier darf nie einen Host voraussetzen.

## Schichten

```mermaid
flowchart BT
  subgraph L0["L0 · Parser und Engines"]
    h5[html5-parser]
    csp[csp-parse]
    tok[css-tokenize]
    mq[media-query-parse]
    xp[xpath-eval]
    rng[relax-ng]
    sch[schematron-engine]
  end
  subgraph L1["L1 · Modell"]
    dom[a11y-dom]
    rep[a11y-report]
    hc[html-conform]
    cssp[stylesheet-parse]
  end
  subgraph L2["L2 · Berechnung"]
    acc[accname]
    cas[style-cascade]
    rules[a11y-rules]
    perc[a11y-perception]
    wc[web-checks]
  end
  subgraph L3["L3 · Auslieferung"]
    wasm[a11y-wasm]
  end
  subgraph L4["L4 · Werkzeuge, eigene Repositories"]
    ams[auditmysite]
    apa[astro-post-audit]
    la[liveaudit]
    rl[Relief]
  end

  sch --> xp
  hc --> h5 & csp & mq & rng & sch
  cssp --> tok
  mq --> tok
  acc --> dom
  cas --> dom & cssp & mq
  rules --> dom & rep & acc & cas
  perc --> rep
  wc --> rep
  wasm --> rules
  ams --> rules & hc & perc & wc
  apa --> rules & hc & wc
  la --> wasm
  rl --> perc & acc & dom & rep
```

| Schicht | Aufgabe | Regel |
|---|---|---|
| L0 | Syntax lesen: HTML, CSP, Media Queries, XPath, RELAX NG, Schematron | keine Bewertung, keine Zugänglichkeitsbegriffe |
| L1 | Datenmodell: Baum, Befund, Konformanz, Stylesheet | ein Befundmodell für alle Hosts (`a11y-report`) |
| L2 | Berechnung: Name, Kaskade, Regeln, Wahrnehmung, Prüfungen | reine Funktionen über L1, browserfrei; `a11y-rules` darf auf `accname` und `style-cascade` aufbauen |
| L3 | Auslieferung in andere Laufzeiten (WASM) | reicht durch, bewertet nicht |
| L4 | Erhebung, Orchestrierung, Ausgabe | außerhalb dieses Repositorys |

## Feste Regeln

1. **Abhängigkeiten nur nach unten.** Kein Paket importiert ein Paket einer
   höheren Schicht. Innerhalb einer Schicht sind Abhängigkeiten erlaubt, solange
   der Graph azyklisch ist; in L2 steht `a11y-rules` oben und hängt von
   `accname` und `style-cascade` ab (`schematron-engine` → `xpath-eval` in L0
   ist derselbe Fall). Kein Host im Repository.
2. **Browserfrei bis L3.** Kein `chromiumoxide`, keine CDP-Typen in einer
   öffentlichen API. Die Aufnahme im Browser bleibt Sache der Hosts; Pakete
   nehmen Daten entgegen, sie holen sie nicht. Zwei benannte Ausnahmen vom
   Zuschnitt: Die Datentypen von `a11y-perception` spiegeln den
   CDP-Zugänglichkeitsbaum 1:1 — bewusst, damit Hosts ihn ohne Umwege füllen
   können; CDP-Typen selbst stehen nicht in der API. `web-checks` ist
   SEO-Code, den zwei Hosts gemeinsam brauchen, keine Zugänglichkeit.
3. **Intern `path` + `version`.** Interne Abhängigkeiten stehen mit beidem in
   `workspace.dependencies`, damit dasselbe Manifest lokal baut und
   veröffentlichbar bleibt. Hosts binden ausschließlich aus der Registry ein.
4. **Adapter bewerten nicht.** Was nur übersetzt (`a11y-wasm`), trifft keine
   Entscheidung über Befunde.
5. **Ein Feld gilt erst als vorhanden, wenn ein Lauf es gefüllt gesehen hat.**
   Teuer gelernt: zwei Felder eines Snapshots waren monatelang tot, ohne dass
   Tests oder Lesen es zeigten.

## Reihenfolge des Aufbaus

| Schritt | Paket | Woher | Zustand |
|---|---|---|---|
| 1 | `a11y-report`, `a11y-dom`, `accname`, `a11y-rules` | `casoon/a11y-core` | **importiert** (0.10.1), Historie mitgekommen |
| 2 | `html-conform` + `html5-parser`, `csp-parse`, `media-query-parse`, `xpath-eval`, `relax-ng`, `schematron-engine` | eigene Repositories | **importiert**, `=`-Pins aufgelöst |
| 3 | `a11y-wasm` | `casoon/liveaudit`, `packages/core` | **importiert**, Crate + npm-Paket `@casoon/a11y-wasm` |
| 4 | `a11y-perception` | `casoon/auditmysite` | **importiert** (0.1.0), browserfrei, 27 Tests |
| 5 | `web-checks` | Doppelungen aus auditmysite und astro-post-audit | **erste Familie** (`robots`) drin; die übrigen einzeln, jede vorher geprüft |

Jeder Schritt zieht die Historie des Ursprungs mit; die alten Repositories
werden danach archiviert, nicht gelöscht — veröffentlichte Pakete verweisen auf
sie.
