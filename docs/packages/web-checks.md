---
title: "web-checks"
description: "Die gemeinsame Auswertung zweier Auditoren: robots.txt, Meta-Längen, OpenGraph und Twitter Cards."
order: 8
---

Die gemeinsame Auswertung zweier Auditoren, die dasselbe an derselben Seite
prüfen: auditmysite über eine laufende Seite in Chrome, astro-post-audit über
ein gebautes `dist/`. Verschieden ist die Erhebung, gleich ist die Auswertung.

## Stand

0.3.0, Familien `robots`, `meta` und `social`. Weitere folgen einzeln; strukturierte
Daten sind der nächste Kandidat, Render-blocking wurde geprüft und bleibt draußen
(gemeinsam wären rund fünf Zeilen, der Rest sind verschiedene Prüfungen).

## Was das Paket nicht tut

- **Es holt nichts.** Kein HTTP, kein Dateisystem, kein Browser.
- **Es formuliert nichts.** Befundkennungen, Schweregrade und Texte bleiben beim
  Host; sie lauten dort verschieden und sind mehrsprachig. Hier stehen Daten und
  Prädikate.

Das zweite ist eine Entscheidung gegen den ersten Entwurf: gemeinsame
`Finding`-Rückgaben hätten erzwungen, die Kennungen beider Werkzeuge zu
vereinheitlichen. Das ist eine eigene, sichtbare Entscheidung — nicht der
Nebeneffekt eines Umzugs.

## Aufbau

```mermaid
flowchart LR
  a["auditmysite\nrobots.txt über HTTP"] --> txt["robots.txt als Text"]
  b["astro-post-audit\ndist/robots.txt"] --> txt
  txt --> parse["robots::parse"]
  parse --> groups["Group\nuser_agent · allows · disallows · crawl_delay"]
  parse --> sm["sitemaps"]
  groups --> cls["classify_bot → BotClass"]
  groups --> pd["path_is_disallowed\nlängste Regel gewinnt"]
  cls & pd & sm --> host["Befunde des Hosts\n(eigene Kennungen, eigene Sprache)"]
```

## Was die Zusammenlegung aufgedeckt hat

Die beiden Werkzeuge widersprachen sich bei denselben Bots — an derselben Seite
hätten sie Gegenteiliges gemeldet:

| Bot | auditmysite | astro-post-audit | jetzt |
|---|---|---|---|
| GPTBot | Training | **Citation** | Training |
| ClaudeBot | Mixed | **Citation** | Mixed |
| anthropic-ai | Mixed | **Citation** | Mixed |

Maßgeblich ist, was die Anbieter selbst schreiben: OpenAI trennt `GPTBot`
(Training) von `OAI-SearchBot` (Suche), Anthropic `ClaudeBot` von
`Claude-SearchBot` und `Claude-User`. Die Suchvarianten fehlten in beiden
Registern und sind jetzt drin.

Das ist der Ertrag dieser Scheibe: nicht weniger Zeilen, sondern **eine**
Antwort statt zweier widersprüchlicher. Für astro-post-audit ändert sich damit
die Befundlage — „KI-Citation-Bot gesperrt" wird für GPTBot zu
„KI-Training-Bot gesperrt", was die übliche und unauffällige Lage ist.

### `meta` und `social` (0.3.0)

Titel- und Beschreibungslänge prüften beide Werkzeuge mit denselben Grenzen
(60 / 160) — und beide in **Bytes**: Ein Umlaut zählte doppelt, „Überschrift"
war früher zu lang als „Ueberschrift". `meta::length` zählt Zeichen nach
browserüblicher Leerraum-Zusammenfassung. Die Mindestlängen (30 / 120) gab es
nur in auditmysite; das Paket führt beide Grenzen, der Host entscheidet, welche
er meldet.

Bei OpenGraph galt ein Tag in auditmysite schon als vorhanden, wenn das Element
existierte, in astro-post-audit erst mit Inhalt. Jetzt einheitlich: leerer
Inhalt fehlt. Die Wertprüfungen aus astro-post-audit (`twitter:card` nur
`summary`, `summary_large_image`, `app`, `player`; `og:image` absolut) stehen
jetzt beiden zur Verfügung.

## Öffentliche Fläche

| Eintrag | Zweck |
|---|---|
| `robots::parse` | Text → `RobotsTxt` mit Gruppen und Sitemaps |
| `RobotsTxt::wildcard_disallows_all`, `blocks`, `crawl_delays` | die Fragen, die beide Hosts stellen |
| `Group::disallows_path` | gilt eine Sperre für diesen Pfad |
| `classify_bot`, `BotClass` | Einordnung als Daten, ohne Wertung |
| `path_is_disallowed` | Längste-Regel-Auswertung mit `*` und `$` |
| `meta::length`, `LengthRange::classify`, `meta::TITLE`, `meta::DESCRIPTION` | Länge in Zeichen, Einordnung zu kurz / ok / zu lang |
| `social::is_present`, `is_complete`, `completeness` | Anwesenheit (leer = fehlt) und Vollständigkeit über `*_REQUIRED` / `*_FIELDS` |
| `social::is_valid_twitter_card`, `is_absolute_url` | Wertprüfungen |

## Grenzen

Die Bot-Register sind eine Momentaufnahme: Anbieter benennen Crawler um und
fügen neue hinzu. Ein unbekannter Name ist `Unknown`, nicht „harmlos".

Die Pfadauswertung folgt Googles Auslegung (längere Regel gewinnt, bei
Gleichstand `Allow`). Andere Suchmaschinen legen die Spezifikation teils anders
aus; wer das genau braucht, prüft gegen das Ziel, nicht gegen dieses Paket.
