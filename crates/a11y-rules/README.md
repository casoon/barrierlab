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

// Nicht beurteilt: die Tier-2- und Tier-3-Regeln, weil dieser Host weder
// Semantik noch Darstellung liefert.
assert_eq!(report.summary.rules_not_run, 7);
```

Mit einem Host, der `Semantics` erfüllt, laufen die über `run_with_semantics`
mit; ein Host mit `Semantics` **und** `Rendering` nimmt `run_full`. Die Trennung
ist keine Formalie: Regeln, die einen echten Accessible Name brauchen — Links,
Buttons, SVG —, dürfen ohne ihn nicht raten, und Kontrast lässt sich aus
statischem Markup überhaupt nicht bestimmen.

| Host liefert | Einstieg |
|---|---|
| nur Struktur | `run` |
| + Semantik | `run_with_semantics` |
| + Darstellung | `run_with_rendering` |
| beides | `run_full` |

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

**Tier 1** (Struktur), 34 Kennungen: `document/lang-missing`,
`document/lang-invalid`, `document/title-missing`, `document/title-empty`,
`zoom/viewport-locked`, `zoom/viewport-scale-limited`, `zoom/viewport-missing`,
`headings/empty`, `headings/skip-level`, `headings/h1-missing`,
`headings/h1-multiple`, `images/alt-missing`,
`images/alt-suspicious`, `forms/label-missing`, `forms/placeholder-as-label`,
`aria/role-invalid`, `aria/role-abstract`, `aria/reference-missing`,
`aria/required-attribute-missing`,
`ids/duplicate`, `keyboard/positive-tabindex`, `keyboard/hidden-focusable`,
`keyboard/skip-link-missing`,
`landmarks/main-missing`, `landmarks/main-duplicate`,
`landmarks/navigation-missing`, `landmarks/banner-missing`,
`landmarks/contentinfo-missing`,
`lists/invalid-structure`, `lists/empty`, `lists/term-without-definition`,
`lists/item-outside-list`,
`tables/header-missing`, `tables/name-missing`,
`tables/presentational-with-headers`.

Nicht jede fehlende Landmark ist ein Verstoß: `main` muss da sein, aber eine
Seite darf ohne Navigation auskommen. `landmarks/navigation-missing`,
`landmarks/banner-missing`, `landmarks/contentinfo-missing`,
`headings/h1-multiple` und `keyboard/skip-link-missing` liefern deshalb
`REVIEW`, nicht `FAIL` — der Sprunglink lässt sich ohnehin nur heuristisch
über Linktext und Klassennamen erkennen.


**Tier 2** (Semantik), 5 Kennungen: `links/name-missing`,
`buttons/name-missing`, `svg/name-missing`, `links/ambiguous-name`,
`links/generic-name`.

`links/ambiguous-name` und `links/generic-name` sind zwei verschiedene Regeln:
Die eine sagt „zwei Links heißen gleich, führen aber woandershin", die andere
„dieser Text sagt für sich genommen nichts über das Ziel". Beide liefern
`REVIEW`.

**Tier 3** (Darstellung), 2 Kennungen: `contrast/text-insufficient`,
`contrast/text-undetermined`.

Zu Tier 3 gehört eine Auflage an den Host: `ComputedStyle::background_color` ist
die **effektive** Hintergrundfarbe, über die Vorfahren aufgelöst. Kann der Host
sie nicht bestimmen — Hintergrundbild, Verlauf, `background-blend-mode` —,
liefert er `None`, und die Regel meldet `contrast/text-undetermined` mit
`UNTESTED`. Eine Prüfung gegen geratenes Weiß wäre schlimmer als keine Aussage:
Sie erzeugt ein `PASS`, auf das sich jemand verlässt.

## Lizenz

MIT.
