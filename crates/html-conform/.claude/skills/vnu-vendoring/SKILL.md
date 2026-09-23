---
name: vnu-vendoring
description: Use when vendoring or updating the vnu RELAX NG schema (schema/html5/*.rnc|*.rng) or the vnu test corpus (tests/corpus/) from validator/validator into this repo, including RNC-to-RNG conversion via Trang and SPDX/REUSE license headers. Not for writing or fixing Schematron assertion rules (see schematron-rule-loop), and not for general RELAX NG/Schematron concepts unrelated to this repo's vendoring process.
---

# vnu-Vendoring

Betrifft `schema/` und `tests/corpus/` in diesem Repo — beides vendorter,
MIT-lizenzierter Fremdcode aus `validator/validator` (vnu). Hintergrund:
`plan/03-schema-vendoring.md`, `plan/07-corpus-differential.md`.

## Gotchas

- Nie `.rnc`/`.rng`/Korpus-Dateien von Hand editieren — jede Änderung läuft
  über `xtask/vendor-schema.sh` bzw. `xtask/vendor-corpus.sh`, sonst geht die
  Nachvollziehbarkeit zum Quell-Commit verloren.
- Ob `xmloxide` `.rnc` (Compact Syntax) nativ liest, ist projektspezifisch
  geklärt in `plan/DECISIONS.md` — dort nachsehen statt neu zu recherchieren.
- Trang-Konvertierung `.rnc` → `.rng` erzeugt bei komplexen `choice`-Patterns
  nachweislich abweichendes Verhalten. Nach jeder Konvertierung: Stichprobe
  aus bekannten validen/invaliden HTML-Beispielen gegen das neue Schema
  laufen lassen, bevor es als "vendored" gilt.
- `schema/` und `tests/corpus/` müssen vom **selben** `validator/validator`-
  Commit/Tag stammen — sonst driften Schema und Erwartungswerte auseinander,
  ohne dass das beim Testen auffällt.
- Konvertierte `.rng`-Dateien werden committed, nicht bei jedem Build neu
  erzeugt — sonst entsteht eine faktische Java-Laufzeitabhängigkeit, genau
  das, was das Crate vermeiden soll.
- Jede vendorte Datei braucht einen SPDX-Lizenz-Header (Tag
  `SPDX` + `-License-Identifier`, Wert `MIT`); bei Formaten ohne
  Kommentarsyntax (z.B. manche Korpus-XML-Dateien) stattdessen eine
  `.license`-Sidecar-Datei daneben.

## Ablauf

1. Quell-Commit/Tag von `validator/validator` festlegen, in
   `plan/DECISIONS.md` eintragen.
2. `xtask/vendor-schema.sh` bzw. `xtask/vendor-corpus.sh` ausführen —
   Skript-Header dokumentiert Quelle und ggf. Trang-Version.
3. Falls Konvertierung nötig: Trang (`relaxng/jing-trang`) benutzen, nicht
   selbst parsen oder zur Laufzeit konvertieren.
4. SPDX-Header bzw. `.license`-Sidecar für jede neue/geänderte Datei
   ergänzen.
5. Stichprobentest gegen bekannte valide/invalide Beispiele fahren, Ergebnis
   im Commit erwähnen.
6. `THIRD-PARTY-NOTICES.md` aktualisieren (Quelle, Commit, betroffene Pfade).

## Fertig, wenn

- Alle vendorten Dateien haben durchgängig SPDX-Header oder
  `.license`-Sidecar.
- Stichprobentest bestanden und dokumentiert.
- `plan/DECISIONS.md` und `THIRD-PARTY-NOTICES.md` sind aktuell.
