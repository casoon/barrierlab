# a11y-rules

Accessibility-Regeln, generisch über das Dokumentmodell aus
[`a11y-dom`](https://crates.io/crates/a11y-dom). Teil von
[a11y-core](https://github.com/casoon/a11y-core).

Ein Regelbestand, mehrere Oberflächen: Build-Zeit, CI/Crawl und die laufende
Seite — mit identischen Regelkennungen und identischem JSON.

## Nicht gelaufen ist nicht bestanden

`run` läuft mit dem, was der Host liefert, und hält für jede Regel fest, ob sie
laufen konnte. Eine Tier-2-Regel auf einem Host ohne `Semantics` erzeugt keinen
stillen Nicht-Befund, sondern einen Vermerk mit `NotRun::CapabilityMissing`.

```rust
use a11y_dom::Arena;
use a11y_rules::run;

let doc = Arena::builder()
    .open("html").open("body")
        .open("img").attr("src", "logo.png").close()
    .close().close()
    .build();

let report = run(&doc);
assert!(report.findings.iter().any(|f| f.rule_id == "images/alt-missing"));

// Nicht beurteilt: die Tier-2-Regeln, weil dieser Host keine Semantik liefert.
assert_eq!(report.summary.rules_not_run, 4);
```

Mit einem Host, der `Semantics` erfüllt, laufen die über `run_with_semantics`
mit. Die Trennung ist keine Formalie: Regeln, die einen echten Accessible Name
brauchen — Links, Buttons, SVG —, dürfen ohne ihn nicht raten.

## Eine Namensmenge für Befunde und Vermerke

`Meta::ids` deklariert **alle** Befund-Kennungen, die eine Regel erzeugen kann,
und `RuleRun` wird je Kennung geführt. Damit benutzen `rule_runs` und `findings`
dieselbe Namensmenge und lassen sich über `rule_id` verbinden.

In 0.1.0 war das getrennt: Vermerke trugen eine übergeordnete Regelkennung
(`images/alt`), Befunde die spezifische (`images/alt-missing`). Ein Join lieferte
stillschweigend nichts. Zwei Tests sichern die Zusicherung jetzt ab — jede
erzeugte Kennung muss deklariert sein, und jeder Befund muss einen passenden
Vermerk haben.

## Regeln

**Tier 1** (Struktur), 25 Kennungen: `document/lang-missing`,
`document/lang-invalid`, `document/title-missing`, `document/title-empty`,
`zoom/viewport-locked`, `zoom/viewport-scale-limited`, `headings/empty`,
`headings/skip-level`, `headings/h1-missing`, `images/alt-missing`,
`images/alt-suspicious`, `forms/label-missing`, `forms/placeholder-as-label`,
`aria/role-invalid`, `aria/role-abstract`, `aria/reference-missing`,
`ids/duplicate`, `keyboard/positive-tabindex`, `keyboard/hidden-focusable`,
`lists/invalid-structure`, `lists/empty`, `lists/term-without-definition`,
`lists/item-outside-list`,
`tables/header-missing`, `tables/name-missing`,
`tables/presentational-with-headers`.


**Tier 2** (Semantik), 4 Kennungen: `links/name-missing`,
`buttons/name-missing`, `svg/name-missing`, `links/ambiguous-name`.

## Lizenz

MIT.
