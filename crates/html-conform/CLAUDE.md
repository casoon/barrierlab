# html-conform

Rust-Crate zur HTML5-Konformitätsprüfung (vgl. Nu Html Checker / vnu) — reine
Rust-Abhängigkeit, keine JVM-Laufzeit. Konzept: `konzept.txt`. Umsetzungsplan:
`plan/`.

## Architektur

```
HTML-String → parse (HTML5-Baum) → schema (RelaxNG) → assertions (Schematron) → Vec<Finding>
```

Jede Schicht unabhängig testbar, alle liefern in eine gemeinsame
`Finding`-Struct (`rule_id`, `severity`, `message`, `line`, `column`).

## Arbeitsweise

- Aktueller Stand & nächster Schritt: `plan/00-STATUS.md`.
- Phasenpläne mit Schritten/Exit-Kriterien: `plan/0N-*.md`. Vor größeren
  Änderungen die passende Phase lesen, nicht am Plan vorbei arbeiten.
- Getroffene Entscheidungen (xmloxide-Eignung, RNC/RNG, Lizenz):
  `plan/DECISIONS.md` — dort nachschlagen, bevor diese Fragen neu aufgerollt
  werden.
- Domänen-Workflows (Schema/Korpus vendoren, Schematron-Regeln pflegen) sind
  als Skills unter `.claude/skills/` hinterlegt und laden sich bei Bedarf.

## Feste Regeln

- Lizenz: **MIT** (bewusst kein "MIT OR Apache-2.0" — Abweichung vom
  Rust-Ökosystem-Standard, siehe `plan/DECISIONS.md`). `Cargo.toml`:
  `license = "MIT"`.
- `schema/` und `tests/corpus/` sind vendorter Fremdcode (vnu,
  `validator/validator`, MIT-lizenziert) — nicht inhaltlich per Hand ändern,
  nur über die `xtask/vendor-*.sh`-Skripte aktualisieren.
- `rules/*.sch` ist der einzige Ort für Fachlogik der Assertion-Schicht —
  deklarative XPath-Regeln, kein Rust-Code für Co-Constraints.
- `assertions.rs` spricht die Schematron-Engine ausschließlich über den
  `SchematronEngine`-Trait an, nie direkt gegen eine konkrete Engine
  programmieren.
- Kein `reqwest`, kein Docker, keine JVM-Laufzeitabhängigkeit im
  ausgelieferten Binary — vnu/Trang/Java sind ausschließlich
  Build-/Vendor-Zeit-Werkzeuge.

## Definition of Done

Siehe "Exit-Kriterien" in der jeweiligen `plan/0N-*.md`-Datei — nicht global
definiert, sondern pro Phase.
