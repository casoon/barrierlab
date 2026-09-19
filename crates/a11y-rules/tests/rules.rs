//! Regeltests gegen die Referenz-Arena.

use a11y_dom::{elements, Arena, ArenaNode, Document, Node, Semantics};
use a11y_report::{Outcome, Report};
use a11y_rules::{run, run_with_semantics};
use accname::IdIndex;

/// Ein Tier-2-Host für die Tests: die Arena plus echte Namensberechnung.
///
/// Zeigt zugleich, wie ein Host `accname` in `Semantics` einhängt — der Index
/// wird einmal gebaut und gehalten, nicht je Aufruf.
struct MitSemantik<'a> {
    doc: &'a Arena,
    ids: IdIndex<'a, ArenaNode<'a>>,
}

impl<'a> MitSemantik<'a> {
    fn new(doc: &'a Arena) -> Self {
        MitSemantik {
            ids: IdIndex::build(doc.root()),
            doc,
        }
    }
}

impl Document for MitSemantik<'_> {
    type N<'n>
        = ArenaNode<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.doc.root()
    }
}

impl Semantics for MitSemantik<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        node.attr("aria-hidden") == Some("true")
    }
}

/// Ein Dokument, das möglichst viele Regeln auslöst — Grundlage der
/// Zusicherungen über die Namensmenge.
fn fehlerhaft() -> Arena {
    Arena::builder()
        .open("html")
        .open("head")
        .open("title")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "user-scalable=no")
        .close()
        .close()
        .open("body")
        .open("h2")
        .text("Springt")
        .close()
        .open("h4")
        .close()
        .open("img")
        .attr("src", "a.png")
        .close()
        .open("img")
        .attr("src", "b.png")
        .attr("alt", "b.png")
        .close()
        .open("a")
        .attr("href", "/x")
        .close()
        .open("button")
        .close()
        .open("svg")
        .close()
        .open("input")
        .attr("id", "d")
        .close()
        .open("div")
        .attr("id", "d")
        .attr("role", "buton")
        .close()
        .open("div")
        .attr("role", "widget")
        .attr("aria-labelledby", "fehlt")
        .attr("tabindex", "4")
        .close()
        .open("ul")
        .open("div")
        .close()
        .close()
        .open("table")
        .open("tr")
        .open("td")
        .text("x")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build()
}

fn ids(r: &Report) -> Vec<&str> {
    r.findings.iter().map(|f| f.rule_id.as_str()).collect()
}

fn hat(r: &Report, id: &str) -> bool {
    r.findings.iter().any(|f| f.rule_id == id)
}

/// Ein minimal korrektes Dokument, von dem die Einzeltests abweichen.
fn sauber() -> a11y_dom::ArenaBuilder {
    Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("Seite")
        .close()
        .close()
}

#[test]
fn sauberes_dokument_erzeugt_nur_h1_hinweis_nicht() {
    let doc = sauber()
        .open("body")
        .open("h1")
        .text("Titel")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(r.findings.is_empty(), "unerwartet: {:?}", ids(&r));
}

#[test]
fn fehlendes_lang_und_title() {
    let doc = Arena::builder()
        .open("html")
        .open("body")
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "document/lang-missing"));
    assert!(hat(&r, "document/title-missing"));
}

#[test]
fn unplausibler_sprachcode() {
    for code in ["", "d", "deutsch-", "1de"] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", code)
            .open("head")
            .open("title")
            .text("x")
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        assert!(
            hat(&r, "document/lang-invalid"),
            "Code {code:?} haette auffallen muessen"
        );
    }
    // Gueltige Formen duerfen nicht anschlagen.
    for code in ["de", "de-DE", "en", "zh-Hant-TW"] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", code)
            .open("head")
            .open("title")
            .text("x")
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        assert!(
            !hat(&r, "document/lang-invalid"),
            "Code {code:?} ist gueltig"
        );
    }
}

#[test]
fn bild_ohne_alt_ist_fail_verdaechtiges_alt_ist_review() {
    let doc = sauber()
        .open("body")
        .open("img")
        .attr("src", "a.png")
        .close()
        .open("img")
        .attr("src", "b.png")
        .attr("alt", "b.png")
        .close()
        .open("img")
        .attr("src", "c.png")
        .attr("alt", "Ein Hund im Schnee")
        .close()
        .open("img")
        .attr("src", "d.png")
        .attr("alt", "")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);

    let fehlend = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-missing")
        .unwrap();
    assert_eq!(fehlend.outcome, Outcome::Fail);

    let verdaechtig = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-suspicious")
        .unwrap();
    assert_eq!(
        verdaechtig.outcome,
        Outcome::Review,
        "heuristische Regeln liefern Review, nicht Fail"
    );

    // Genau einmal je Regel: der gute Alt-Text und das leere alt schlagen nicht an.
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("images/"))
            .count(),
        2
    );
}

#[test]
fn ueberschriften_luecke_und_leere_ueberschrift() {
    let doc = sauber()
        .open("body")
        .open("h1")
        .text("A")
        .close()
        .open("h3")
        .text("B")
        .close()
        .open("h4")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "headings/skip-level"));
    assert!(hat(&r, "headings/empty"));
    assert!(!hat(&r, "headings/h1-missing"));
}

#[test]
fn label_zuordnung_ueber_alle_vier_wege() {
    let doc = sauber()
        .open("body")
        // 1. label[for]
        .open("label")
        .attr("for", "a")
        .text("A")
        .close()
        .open("input")
        .attr("id", "a")
        .close()
        // 2. verschachtelt
        .open("label")
        .text("B")
        .open("input")
        .attr("id", "b")
        .close()
        .close()
        // 3. aria-label
        .open("input")
        .attr("id", "c")
        .attr("aria-label", "C")
        .close()
        // 4. gar nichts
        .open("input")
        .attr("id", "d")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "forms/label-missing")
            .count(),
        1,
        "nur das vierte Feld ist unbeschriftet: {:?}",
        ids(&r)
    );
}

#[test]
fn placeholder_ersetzt_kein_label() {
    let doc = sauber()
        .open("body")
        .open("input")
        .attr("id", "a")
        .attr("title", "Suche")
        .attr("placeholder", "Suchen…")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    // title zaehlt als Label, also kein label-missing - aber der Platzhalter
    // bleibt ein eigener Befund.
    assert!(!hat(&r, "forms/label-missing"));
    assert!(hat(&r, "forms/placeholder-as-label"));
}

#[test]
fn versteckte_felder_werden_uebergangen() {
    let doc = sauber()
        .open("body")
        .open("input")
        .attr("type", "hidden")
        .attr("name", "csrf")
        .close()
        .open("input")
        .attr("type", "submit")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!hat(&r, "forms/label-missing"));
}

#[test]
fn ungueltige_und_abstrakte_rollen() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("role", "buton")
        .close()
        .open("div")
        .attr("role", "widget")
        .close()
        .open("div")
        .attr("role", "button")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "aria/role-invalid"));
    assert!(hat(&r, "aria/role-abstract"));
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id.starts_with("aria/role"))
            .count(),
        2
    );
}

#[test]
fn aria_verweise_auf_fehlende_ids() {
    let doc = sauber()
        .open("body")
        .open("h2")
        .attr("id", "da")
        .text("Da")
        .close()
        .open("div")
        .attr("aria-labelledby", "da fehlt")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "aria/reference-missing")
        .unwrap();
    assert!(f.message.contains("fehlt"), "{}", f.message);
    assert!(
        !f.message.contains(" da"),
        "vorhandene ID darf nicht gemeldet werden: {}",
        f.message
    );
}

#[test]
fn doppelte_ids_melden_nur_den_zweiten_treffer() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("id", "x")
        .close()
        .open("div")
        .attr("id", "x")
        .close()
        .open("div")
        .attr("id", "x")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "ids/duplicate")
            .count(),
        1
    );
}

#[test]
fn positiver_tabindex_und_verstecktes_fokussierbares() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("tabindex", "4")
        .close()
        .open("div")
        .attr("tabindex", "0")
        .close()
        .open("a")
        .attr("href", "/x")
        .attr("aria-hidden", "true")
        .text("X")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "keyboard/positive-tabindex")
            .count(),
        1
    );
    assert!(hat(&r, "keyboard/hidden-focusable"));
}

#[test]
fn viewport_sperre() {
    for content in [
        "width=device-width, user-scalable=no",
        "width=device-width,maximum-scale=1.0",
    ] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        assert!(hat(&run(&doc), "zoom/viewport-locked"), "{content}");
    }
    // Erlaubter Viewport
    let doc = Arena::builder()
        .open("html")
        .attr("lang", "de")
        .open("head")
        .open("title")
        .text("x")
        .close()
        .open("meta")
        .attr("name", "viewport")
        .attr("content", "width=device-width, initial-scale=1")
        .close()
        .close()
        .close()
        .build();
    assert!(!hat(&run(&doc), "zoom/viewport-locked"));
}

#[test]
fn listen_und_tabellen() {
    let doc = sauber()
        .open("body")
        .open("ul")
        .open("div")
        .text("falsch")
        .close()
        .close()
        .open("table")
        .open("tr")
        .open("td")
        .text("x")
        .close()
        .close()
        .close()
        .open("table")
        .attr("role", "presentation")
        .open("tr")
        .open("td")
        .text("Layout")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(hat(&r, "lists/invalid-structure"));
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "tables/header-missing")
            .count(),
        1,
        "role=presentation ist eine Layouttabelle und gemeint"
    );
}

// --- Tier-Mechanik --------------------------------------------------------

#[test]
fn ohne_semantik_werden_tier2_regeln_als_nicht_gelaufen_vermerkt() {
    let doc = sauber()
        .open("body")
        .open("a")
        .attr("href", "/x")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);

    assert_eq!(r.summary.rules_not_run, 4);
    assert!(
        !hat(&r, "links/name-missing"),
        "ohne Accessible Name darf die Regel nicht raten"
    );

    // Der Vermerk laeuft ueber die Befund-Kennung, nicht ueber eine
    // uebergeordnete Regelkennung -- nur so passen rule_runs und findings
    // zusammen.
    let vermerk = r
        .rule_runs
        .iter()
        .find(|x| x.rule_id == "links/name-missing")
        .unwrap();
    assert!(!vermerk.did_run());
    assert_eq!(
        vermerk.not_run,
        Some(a11y_report::NotRun::CapabilityMissing)
    );
}

#[test]
fn mit_semantik_laufen_tier2_regeln_mit() {
    let arena = sauber()
        .open("body")
        .open("a")
        .attr("href", "/x")
        .close()
        .open("a")
        .attr("href", "/y")
        .text("Mehr")
        .close()
        .open("button")
        .close()
        .close()
        .close()
        .build();
    let doc = MitSemantik::new(&arena);
    let r = run_with_semantics(&doc);

    assert_eq!(r.summary.rules_not_run, 0);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "links/name-missing")
            .count(),
        1
    );
    assert!(hat(&r, "buttons/name-missing"));
}

#[test]
fn gleicher_linktext_verschiedene_ziele() {
    let arena = sauber()
        .open("body")
        .open("a")
        .attr("href", "/a")
        .text("Mehr")
        .close()
        .open("a")
        .attr("href", "/b")
        .text("Mehr")
        .close()
        .open("a")
        .attr("href", "/c")
        .text("Zum Bericht")
        .close()
        .close()
        .close()
        .build();
    let doc = MitSemantik::new(&arena);
    let r = run_with_semantics(&doc);
    let treffer: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "links/ambiguous-name")
        .collect();
    assert_eq!(treffer.len(), 2, "beide mehrdeutigen Links werden markiert");
    assert_eq!(treffer[0].outcome, Outcome::Review);
}

#[test]
fn gleicher_linktext_gleiches_ziel_ist_in_ordnung() {
    let arena = sauber()
        .open("body")
        .open("a")
        .attr("href", "/a")
        .text("Mehr")
        .close()
        .open("a")
        .attr("href", "/a")
        .text("Mehr")
        .close()
        .close()
        .close()
        .build();
    let doc = MitSemantik::new(&arena);
    assert!(!hat(&run_with_semantics(&doc), "links/ambiguous-name"));
}

#[test]
fn aria_hidden_elemente_bleiben_bei_tier2_aussen_vor() {
    let arena = sauber()
        .open("body")
        .open("a")
        .attr("href", "/x")
        .attr("aria-hidden", "true")
        .close()
        .close()
        .close()
        .build();
    let doc = MitSemantik::new(&arena);
    assert!(!hat(&run_with_semantics(&doc), "links/name-missing"));
}

#[test]
fn jede_kennung_hinterlaesst_genau_einen_ausfuehrungsvermerk() {
    let doc = sauber().open("body").close().close().build();
    let r = run(&doc);
    let mut kennungen: Vec<&str> = r.rule_runs.iter().map(|x| x.rule_id.as_str()).collect();
    let anzahl = kennungen.len();
    kennungen.sort_unstable();
    kennungen.dedup();
    assert_eq!(kennungen.len(), anzahl, "doppelte Vermerke");
    let deklariert: usize = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .map(|m| m.ids.len())
        .sum();
    assert_eq!(anzahl, deklariert, "ein Vermerk je deklarierter Kennung");
}

#[test]
fn befunde_tragen_wcag_kriterien_und_eine_verortung() {
    let doc = sauber()
        .open("body")
        .open("img")
        .attr("src", "a.png")
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let f = r
        .findings
        .iter()
        .find(|f| f.rule_id == "images/alt-missing")
        .unwrap();
    assert_eq!(f.wcag, vec!["1.1.1"]);
    assert!(f.location.node.is_some());

    // Die Verortung laesst sich zum Knoten zurueckaufloesen.
    let id: u32 = f.location.node.as_ref().unwrap().parse().unwrap();
    let knoten = doc.get(a11y_dom::NodeId(id)).unwrap();
    assert_eq!(knoten.local_name(), "img");
    let _ = elements(&doc).count();
}

// --- Die Zusicherung, die den Bericht auswertbar macht --------------------

/// `rule_runs` und `findings` müssen dieselbe Namensmenge benutzen. Sonst
/// liefert ein Join über `rule_id` stillschweigend nichts — genau der Fehler,
/// der hier einmal drinsteckte: Vermerke trugen `images/alt`, Befunde
/// `images/alt-missing`.
#[test]
fn jeder_befund_hat_einen_passenden_ausfuehrungsvermerk() {
    let arena = fehlerhaft();
    let doc = MitSemantik::new(&arena);
    let r = run_with_semantics(&doc);

    let vermerkt: std::collections::HashSet<&str> =
        r.rule_runs.iter().map(|x| x.rule_id.as_str()).collect();

    for f in &r.findings {
        assert!(
            vermerkt.contains(f.rule_id.as_str()),
            "Befund {:?} hat keinen Ausfuehrungsvermerk — rule_runs und findings \
             benutzen verschiedene Namensmengen",
            f.rule_id
        );
    }
    assert!(!r.findings.is_empty(), "Testdokument muss Befunde erzeugen");
}

/// Jede erzeugte Kennung muss in `Meta::ids` deklariert sein. Fehlt eine, ist
/// sie im Bericht als „nie gelaufen" unsichtbar.
#[test]
fn jede_erzeugte_kennung_ist_deklariert() {
    let arena = fehlerhaft();
    let doc = MitSemantik::new(&arena);
    let r = run_with_semantics(&doc);

    let deklariert: std::collections::HashSet<&str> = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .flat_map(|m| m.ids.iter().copied())
        .collect();

    for f in &r.findings {
        assert!(
            deklariert.contains(f.rule_id.as_str()),
            "Kennung {:?} wird erzeugt, aber in keinem Meta::ids deklariert",
            f.rule_id
        );
    }
}

/// Keine Kennung darf doppelt deklariert sein — sonst gäbe es zwei Vermerke
/// für denselben Befundtyp.
#[test]
fn keine_kennung_ist_doppelt_deklariert() {
    let mut alle: Vec<&str> = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .flat_map(|m| m.ids.iter().copied())
        .collect();
    let anzahl = alle.len();
    alle.sort_unstable();
    alle.dedup();
    assert_eq!(alle.len(), anzahl, "doppelt deklarierte Kennung");
}

// --- Verschaerfungen gegenueber 0.3.0 ------------------------------------

/// HTML-Attributwerte sind nicht normiert. `user-scalable=NO` sperrt den Zoom
/// genauso wie die Kleinschreibung -- bis 0.3.0 verglich die Regel den
/// `content`-Wert unveraendert und sah darueber hinweg.
#[test]
fn viewport_sperre_ist_schreibweisenunabhaengig() {
    for content in [
        "width=device-width, user-scalable=NO",
        "width=device-width, User-Scalable=No",
        "width=device-width, MAXIMUM-SCALE=1.0",
    ] {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        assert!(hat(&run(&doc), "zoom/viewport-locked"), "{content}");
    }
}

/// `role="list"` steht im Markup und ist damit Tier-1-entscheidbar. Bis 0.3.0
/// sah die Regel nur `<ul>`/`<ol>` und war fuer ARIA-Listen blind.
#[test]
fn liste_per_rolle_wird_geprueft() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("role", "list")
        .open("span")
        .text("kein Eintrag")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(hat(&run(&doc), "lists/invalid-structure"));
}

/// ... und eine korrekt ausgezeichnete ARIA-Liste darf nicht auffallen.
#[test]
fn korrekte_rollenliste_erzeugt_keinen_befund() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("role", "list")
        .open("div")
        .attr("role", "listitem")
        .text("Eintrag")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!hat(&r, "lists/invalid-structure"), "{:?}", r.findings);
    assert!(!hat(&r, "lists/empty"), "{:?}", r.findings);
}

/// Eine Liste ohne Eintraege kuendigt der Assistenztechnik eine Struktur an,
/// die es nicht gibt. Neue Kennung `lists/empty`.
#[test]
fn leere_liste_wird_gemeldet() {
    let doc = sauber()
        .open("body")
        .open("ul")
        .close()
        .close()
        .close()
        .build();
    assert!(hat(&run(&doc), "lists/empty"));
}

/// `role="presentation"` sagt ausdruecklich, dass hier keine Liste gemeint
/// ist -- ein Strukturbefund darauf waere falsch.
#[test]
fn praesentationsliste_erzeugt_keinen_strukturbefund() {
    let doc = sauber()
        .open("body")
        .open("ul")
        .attr("role", "presentation")
        .open("div")
        .text("Layout")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!hat(&r, "lists/invalid-structure"), "{:?}", r.findings);
    assert!(!hat(&r, "lists/empty"), "{:?}", r.findings);
}

/// Eine Kopfzelle kann per Rolle ausgezeichnet sein. Bis 0.3.0 suchte die
/// Regel nur `<th>` und meldete solche Tabellen faelschlich als kopflos.
#[test]
fn kopfzelle_per_rolle_zaehlt_als_kopf() {
    let doc = sauber()
        .open("body")
        .open("table")
        .open("tr")
        .open("td")
        .attr("role", "columnheader")
        .text("Spalte")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert!(!hat(&r, "tables/header-missing"), "{:?}", r.findings);
}

/// ... und eine Tabelle, die nur per Rolle eine ist, wird ueberhaupt geprueft.
#[test]
fn rollentabelle_ohne_kopf_faellt_auf() {
    let doc = sauber()
        .open("body")
        .open("div")
        .attr("role", "table")
        .open("div")
        .attr("role", "row")
        .open("div")
        .attr("role", "cell")
        .text("x")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(hat(&run(&doc), "tables/header-missing"));
}

/// Die Einstufungen, bei denen eine stille Absenkung fachlich etwas kaputt
/// machen würde. Nicht der ganze Katalog — nur die Fälle, über die schon
/// einmal entschieden wurde.
#[test]
fn einstufungen_bleiben_wo_sie_begruendet_wurden() {
    use a11y_report::Severity;

    let metas: Vec<_> = a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::semantics_metas())
        .collect();
    let sev = |id: &str| {
        metas
            .iter()
            .find(|m| m.ids.contains(&id))
            .unwrap_or_else(|| panic!("Kennung {id} nicht deklariert"))
            .severity
    };

    // Bricht die Tabreihenfolge reproduzierbar und fuer jeden, der mit der
    // Tastatur navigiert.
    assert_eq!(sev("keyboard/positive-tabindex"), Severity::High);

    // WCAG 4.1.1 wurde in WCAG 2.2 entfernt; der echte Schaden entsteht erst
    // bei einer Referenz, und dafuer gibt es aria/reference-missing.
    assert_eq!(sev("ids/duplicate"), Severity::Medium);

    // Ein Feld ohne Label ist fuer Screenreader-Nutzer unbenutzbar.
    assert_eq!(sev("forms/label-missing"), Severity::Critical);
}

// --- Die Luecken, die beim Abloesen der auditmysite-Regeln auffielen -----

#[test]
fn begriff_ohne_definition() {
    let doc = sauber()
        .open("body")
        .open("dl")
        // Vollstaendig: Begriff mit Definition.
        .open("dt")
        .attr("id", "gut")
        .text("HTML")
        .close()
        .open("dd")
        .text("Auszeichnungssprache")
        .close()
        .close()
        .open("dl")
        // Unvollstaendig: Begriff allein.
        .open("dt")
        .attr("id", "schlecht")
        .text("CSS")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let treffer: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "lists/term-without-definition")
        .collect();
    assert_eq!(
        treffer.len(),
        1,
        "nur der zweite Begriff ist unvollstaendig: {:?}",
        ids(&r)
    );
}

#[test]
fn begriff_in_einer_div_gruppe_braucht_seine_definition_dort() {
    // HTML erlaubt <div>-Gruppen in einer <dl>. Ein <dt> in einer solchen
    // Gruppe braucht sein <dd> dort, nicht irgendwo in der Liste.
    let doc = sauber()
        .open("body")
        .open("dl")
        .open("div")
        .open("dt")
        .text("A")
        .close()
        .close()
        .open("div")
        .open("dt")
        .text("B")
        .close()
        .open("dd")
        .text("b")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "lists/term-without-definition")
            .count(),
        1,
        "nur die erste Gruppe ist unvollstaendig"
    );
}

#[test]
fn rollen_zaehlen_auch_bei_begriff_und_definition() {
    let doc = sauber()
        .open("body")
        .open("div")
        .open("div")
        .attr("role", "term")
        .text("A")
        .close()
        .open("div")
        .attr("role", "definition")
        .text("a")
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!hat(&run(&doc), "lists/term-without-definition"));
}

#[test]
fn praesentationale_tabelle_mit_kopfzellen() {
    let doc = sauber()
        .open("body")
        // Sauber: Layouttabelle ohne Koepfe.
        .open("table")
        .attr("role", "presentation")
        .open("tr")
        .open("td")
        .text("x")
        .close()
        .close()
        .close()
        // Widerspruechlich: als praesentational ausgezeichnet, aber mit Kopf.
        .open("table")
        .attr("role", "none")
        .open("tr")
        .open("th")
        .text("Kopf")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "tables/presentational-with-headers")
            .count(),
        1
    );
    // Und keine der beiden wird als kopflose Datentabelle gemeldet.
    assert!(!hat(&r, "tables/header-missing"));
}

#[test]
fn tabelle_ohne_namen_ist_review_nicht_fail() {
    let doc = sauber()
        .open("body")
        .open("table")
        .open("caption")
        .text("Umsätze")
        .close()
        .open("tr")
        .open("th")
        .text("Jahr")
        .close()
        .close()
        .close()
        .open("table")
        .attr("aria-label", "Kosten")
        .open("tr")
        .open("th")
        .text("Jahr")
        .close()
        .close()
        .close()
        .open("table")
        .open("tr")
        .open("th")
        .text("Jahr")
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    let treffer: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.rule_id == "tables/name-missing")
        .collect();
    assert_eq!(treffer.len(), 1, "caption und aria-label zaehlen als Name");
    // Ob eine Tabelle einen Namen braucht, ist nicht zwingend entscheidbar.
    assert_eq!(treffer[0].outcome, Outcome::Review);
}

#[test]
fn viewport_unterscheidet_verstoss_von_begrenzung() {
    let fall = |content: &str| {
        let doc = Arena::builder()
            .open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("x")
            .close()
            .open("meta")
            .attr("name", "viewport")
            .attr("content", content)
            .close()
            .close()
            .close()
            .build();
        let r = run(&doc);
        ids(&r)
            .iter()
            .filter(|i| i.starts_with("zoom/"))
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
    };

    // Unter 200 %: Verstoss gegen 1.4.4.
    assert_eq!(fall("maximum-scale=1.5"), vec!["zoom/viewport-locked"]);
    assert_eq!(fall("user-scalable=NO"), vec!["zoom/viewport-locked"]);

    // Zwischen 200 und 500 %: erfuellt 1.4.4, begrenzt aber. Eigene Kennung,
    // und nicht beide zugleich.
    assert_eq!(fall("maximum-scale=3"), vec!["zoom/viewport-scale-limited"]);

    // Ab 500 % oder ohne Begrenzung: nichts.
    assert!(fall("maximum-scale=5").is_empty());
    assert!(fall("width=device-width, initial-scale=1").is_empty());
}

#[test]
fn listeneintrag_ausserhalb_einer_liste() {
    // Die Listenpruefung laeuft ueber Listen und sieht nur, was darin steht.
    // Ein verwaistes <li> wird dabei nie besucht.
    let doc = sauber()
        .open("body")
        .open("ul")
        .open("li")
        .text("drin")
        .close()
        .close()
        .open("div")
        .open("li")
        .text("verwaist")
        .close()
        .close()
        .close()
        .close()
        .build();
    let r = run(&doc);
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.rule_id == "lists/item-outside-list")
            .count(),
        1,
        "nur der verwaiste Eintrag: {:?}",
        ids(&r)
    );
}

#[test]
fn ein_verschachtelter_eintrag_gilt_nicht_als_verwaist() {
    // <li> in einer Unterliste hat die aeussere Liste als Vorfahren.
    let doc = sauber()
        .open("body")
        .open("ul")
        .open("li")
        .text("a")
        .open("ul")
        .open("li")
        .text("a1")
        .close()
        .close()
        .close()
        .close()
        .close()
        .close()
        .build();
    assert!(!hat(&run(&doc), "lists/item-outside-list"));
}
