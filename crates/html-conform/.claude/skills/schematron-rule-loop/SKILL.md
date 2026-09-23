---
name: schematron-rule-loop
description: Use when writing, fixing, or reviewing Schematron assertion rules (rules/*.sch) or running the differential test loop (tests/differential.rs) against the vnu corpus to close conformance diffs. Covers XPath-based co-constraint rules (ARIA, forms, tables, obsolete elements). Not for RELAX NG schema or corpus vendoring (see vnu-vendoring), and not for the parse/schema Rust layers themselves.
---

# Schematron-Regel-Loop

Der iterative Kern des Projekts (Loop B, `plan/08-assertion-refinement-loop.md`):
Diffs zwischen `html-conform` und vnu schließen, indem Regeln in `rules/`
ergänzt/korrigiert werden — als kleine, deklarative XPath-Assertions, nicht
als Rust-Code.

## Gotchas

- Eine Regel = eine kleine, isoliert testbare XPath-Assertion. Regeldateien
  bleiben nach Thema getrennt: `aria.sch`, `tables.sch`,
  `obsolete-elements.sch`, `attributes.sch` — keine Themenmischung in
  einer Datei.
- Jede Regel muss vor dem Schreiben an mindestens einer konkreten
  `messages.json`-Meldung verifiziert werden — nicht aus der
  ARIA-/HTML5-Spezifikation extrapolieren, so plausibel es klingt.
  `forms.sch`s `forms.input-needs-accessible-name` (Phase 06, ein
  Phase-02-Canary-Case) hatte am Ende keine einzige Entsprechung im
  echten vnu-Korpus und wurde komplett entfernt — siehe
  `plan/DECISIONS.md`s 2026-08-24-Eintrag.
- Ziel ist **nicht zwingend 0 Diffs** gegen den vnu-Korpus. Das vorher in
  `plan/DECISIONS.md`/`plan/08-assertion-refinement-loop.md` fixierte
  Akzeptanzziel gilt. Regeln nicht künstlich überanpassen, um die letzten
  Prozentpunkte zu erzwingen — das produziert brüchige Regeln.
- Semantisch heikle Regeln (ARIA-Kombinationen, ID-Eindeutigkeit) trotzdem
  sorgfältig prüfen, auch wenn der Differential-Test grün ist — ein
  grüner Test heißt nur "nicht schlechter als die Baseline", nicht
  "fachlich korrekt".
- `assertions.rs` spricht die Engine nur über den `SchematronEngine`-Trait
  an — beim Debuggen ggf. die Mock-Engine aus den Tests nutzen, statt gegen
  die echte Engine zu kämpfen.
- Ein Fehlschlag im Differential-Test muss genau zeigen, welche Assertion
  falsch oder unvollständig ist. Wenn das aus dem Diff nicht hervorgeht, ist
  der Testfall zu grob geschnitten — kleiner schneiden, nicht raten.

## Ablauf (ein Iterationszyklus)

1. Diff-Liste aus `tests/differential.rs` ziehen: Testdatei, erwartete
   vnu-Meldung, aktuelles `html-conform`-Ergebnis.
2. Diffs nach Themenblock gruppieren (aria/forms/tables/obsolete-elements/
   sonstige).
3. Pro Gruppe: neue oder korrigierte Regel in `rules/<thema>.sch`
   formulieren.
4. `tests/differential.rs` erneut laufen lassen, Delta der Diff-Zahl
   notieren.
5. Ergebnis (Diff-Zahl vorher/nachher, geänderte Regeln) im Iterations-Log
   in `plan/08-assertion-refinement-loop.md` eintragen und committen.

## Fertig, wenn

- Diff-Zahl-Delta für die bearbeitete Gruppe dokumentiert.
- Iterations-Log in `plan/08-assertion-refinement-loop.md` aktualisiert.
