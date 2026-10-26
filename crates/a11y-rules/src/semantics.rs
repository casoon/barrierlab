//! Tier-2-Regeln: brauchen einen echten Accessible Name.
//!
//! Warum das nicht strukturell geht: Der Accessible Name eines Links oder
//! Buttons entsteht nach einem mehrstufigen Verfahren — `aria-labelledby` löst
//! Verweisketten auf, versteckte Teilbäume zählen unter bestimmten Bedingungen
//! doch mit, `alt` eines eingebetteten Bildes fließt ein. Eine Näherung aus
//! Teilbaumtext plus `aria-label` liegt in genau den Fällen falsch, in denen
//! es darauf ankommt.
//!
//! Hosts ohne [`Semantics`] melden diese Regeln als `UNTESTED` — siehe
//! [`crate::run`].
//!
//! [`Semantics`]: a11y_dom::Semantics

use a11y_dom::{Node, NodeId, Semantics, Tier, elements};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, pick, tr};
use crate::registry::{Meta, SemanticsRule};
use crate::sicht::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

pub(crate) fn named<'n, D: Semantics>(doc: &'n D, n: D::N<'n>) -> bool {
    doc.accessible_name(n).is_some_and(|s| !s.trim().is_empty())
}

fn link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("a") || !n.has_attr("href") || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "links/name-missing",
                    pick!(
                        locale,
                        "The link has no accessible name.",
                        "Der Link hat keinen zugänglichen Namen.",
                    ),
                )
                .with_severity(Severity::Critical)
                .with_wcag(["2.4.4", "4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

fn button_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        let ist_button = n.is_element("button") || doc.role(n).as_deref() == Some("button");
        // Eine `<summary>` meldet `summary/name-missing` — `accname` gibt ihr
        // die Rolle `button`, Chrome nicht; so meldet jeder Host sie einmal.
        if !ist_button || n.is_element("summary") || doc.is_ignored(n) {
            continue;
        }
        if !named(doc, n) {
            out.push(
                Finding::fail(
                    "buttons/name-missing",
                    pick!(
                        locale,
                        "The button has no accessible name.",
                        "Der Button hat keinen zugänglichen Namen.",
                    ),
                )
                .with_severity(Severity::Critical)
                .with_wcag(["4.1.2"])
                .at(at(n.id())),
            );
        }
    }
}

fn svg_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for n in elements(doc) {
        if !n.is_element("svg") || doc.is_ignored(n) {
            continue;
        }
        // Rein dekoratives SVG ist korrekt ausgezeichnet und gemeint.
        if n.attr("role").is_some_and(|r| {
            r.split_whitespace().next().is_some_and(|x| {
                x.eq_ignore_ascii_case("presentation") || x.eq_ignore_ascii_case("none")
            })
        }) || n.attr("aria-hidden") == Some("true")
        {
            continue;
        }
        // Das Icon in einem benannten Link oder Button: Der Name des
        // Bedienelements trägt die Aussage, die Grafik braucht keinen eigenen.
        // Beleg: auditmysite `is_graphic_in_named_control` (mit.edu).
        if a11y_dom::ancestors(n).any(|a| {
            (a.is_element("a")
                || a.is_element("button")
                || a.attr("role").is_some_and(|r| {
                    r.split_whitespace().next().is_some_and(|x| {
                        x.eq_ignore_ascii_case("link") || x.eq_ignore_ascii_case("button")
                    })
                }))
                && named(doc, a)
        }) {
            continue;
        }
        // Verstoß nur mit ausdrücklicher Bildrolle (ACT 7d6734): Sie sagt,
        // dass die Grafik etwas mitteilt. Ein SVG ohne Rolle kann ebenso gut
        // Schmuck neben Text sein — `REVIEW` (auditmysite#779).
        if !named(doc, n) {
            let bild = n.attr("role").is_some_and(|r| {
                r.split_whitespace().next().is_some_and(|x| {
                    ["img", "graphics-document", "graphics-symbol"]
                        .iter()
                        .any(|b| x.eq_ignore_ascii_case(b))
                })
            });
            let text = pick!(
                locale,
                "The SVG has no accessible name and is not marked decorative.",
                "Das SVG hat keinen zugänglichen Namen und ist nicht als dekorativ ausgezeichnet."
            );
            let f = if bild {
                Finding::fail("svg/name-missing", text)
            } else {
                Finding::review("svg/name-missing", text)
            };
            out.push(
                f.with_severity(Severity::High)
                    .with_wcag(["1.1.1"])
                    .at(at(n.id())),
            );
        }
    }
}

/// Gleicher Linktext, unterschiedliches Ziel — für Screenreader-Nutzer, die
/// sich eine Linkliste ausgeben lassen, nicht unterscheidbar.
/// Namen, die für sich genommen nichts über das Ziel sagen.
///
/// Wer eine Liste aller Links abruft — Screenreader können das —, hört eine
/// Folge von „mehr", „hier", „weiterlesen". Welcher wohin führt, steht nur im
/// umgebenden Text, den diese Liste nicht mitliefert.
///
/// Übernommen aus auditmysites `link_purpose` (casoon/barrierlab#13, Plan 98):
/// 15 Sprachen und, anders als die frühere enge Liste, auch Beschriftungen wie
/// „Start", „Weiter", „Download" oder „Info" — sie benennen eine Handlung,
/// aber kein Ziel. Ob der Zusammenhang das Ziel klärt, entscheidet
/// [`hat_kontext`] über die Schwere, nicht über die Meldung.
const NICHTSSAGEND: &[&str] = &[
    // Englisch
    "click here",
    "click",
    "here",
    "read more",
    "more",
    "learn more",
    "info",
    "information",
    "details",
    "link",
    "this link",
    "link here",
    "go",
    "continue",
    "download",
    "view",
    "see more",
    "see all",
    "read",
    "start",
    "begin",
    "submit",
    "next",
    "previous",
    // Deutsch
    "weiterlesen",
    "mehr erfahren",
    "mehr",
    "mehr dazu",
    "mehr lesen",
    "hier klicken",
    "klick hier",
    "klicken sie hier",
    "siehe hier",
    "hier",
    "weiter",
    "dieser link",
    "alle anzeigen",
    "ansehen",
    "jetzt lesen",
    "öffnen",
    // Französisch
    "cliquez ici",
    "ici",
    "en savoir plus",
    "lire la suite",
    "lire plus",
    "plus",
    "voir plus",
    "voir tout",
    "continuer",
    "télécharger",
    "suivant",
    "précédent",
    "commencer",
    // Spanisch
    "haz clic aquí",
    "aquí",
    "leer más",
    "más",
    "saber más",
    "ver más",
    "ver todo",
    "continuar",
    "descargar",
    "siguiente",
    "anterior",
    "empezar",
    // Italienisch
    "clicca qui",
    "qui",
    "leggi di più",
    "scopri di più",
    "più",
    "vedi di più",
    "vedi tutto",
    "continua",
    "scarica",
    "successivo",
    "precedente",
    "inizia",
    // Portugiesisch
    "clique aqui",
    "aqui",
    "leia mais",
    "saiba mais",
    "mais",
    "ver mais",
    "ver tudo",
    "baixar",
    "próximo",
    "começar",
    // Niederländisch
    "klik hier",
    "meer lezen",
    "lees meer",
    "meer",
    "bekijk meer",
    "volgende",
    "vorige",
    "downloaden",
    "beginnen",
    // Schwedisch
    "klicka här",
    "här",
    "läs mer",
    "mer",
    "nästa",
    "föregående",
    "ladda ner",
    "börja",
    // Norwegisch
    "klikk her",
    "her",
    "les mer",
    "neste",
    "forrige",
    "last ned",
    // Dänisch
    "klik her",
    "læs mere",
    "mere",
    "næste",
    // Finnisch
    "klikkaa tästä",
    "lue lisää",
    "lisää",
    "seuraava",
    "edellinen",
    "lataa",
    // Polnisch
    "kliknij tutaj",
    "tutaj",
    "czytaj więcej",
    "więcej",
    "następny",
    "poprzedni",
    "pobierz",
    // Türkisch
    "buraya tıklayın",
    "devamını oku",
    "daha fazla",
    "sonraki",
    "önceki",
    "indir",
    // Tschechisch / Slowakisch
    "klikněte zde",
    "zde",
    "číst více",
    "více",
    "další",
    "předchozí",
    "stáhnout",
    // Rumänisch
    "citește mai mult",
    "mai mult",
    "următor",
    "descărcați",
    // Ungarisch
    "tovább olvas",
    "több",
    "következő",
    "előző",
    "letöltés",
    // Zeichen
    "...",
    ">",
    ">>",
    "→",
];

/// Für 2.4.9 (AAA) reicht der Linktext allein; Zusammenhang zählt nicht.
/// Deshalb eine weitere Liste: Wörter, die eine Handlung oder Gattung nennen,
/// aber kein Ziel („Seite", „Artikel", „Öffnen", „Speichern").
const NUR_LINKTEXT_VAGE: &[&str] = &[
    "click here",
    "click",
    "here",
    "read more",
    "more",
    "learn more",
    "info",
    "information",
    "details",
    "link",
    "this link",
    "go",
    "continue",
    "download",
    "view",
    "see more",
    "see all",
    "read",
    "start",
    "begin",
    "submit",
    "next",
    "previous",
    "page",
    "article",
    "post",
    "weiterlesen",
    "mehr erfahren",
    "mehr",
    "hier klicken",
    "klicken sie hier",
    "hier",
    "weiter",
    "alle anzeigen",
    "ansehen",
    "jetzt lesen",
    "öffnen",
    "seite",
    "yes",
    "no",
    "ok",
    "okay",
    "open",
    "close",
    "show",
    "hide",
    "toggle",
    "buy",
    "get",
    "add",
    "remove",
    "edit",
    "update",
    "save",
    "send",
    "try",
    "see",
    "...",
    ">",
    ">>",
    "→",
    "»",
];

/// Der Vergleichsschlüssel eines Linknamens: getrimmt, kleingeschrieben,
/// ohne nachgestellte Pfeile und Satzzeichen („Mehr ›", „Weiter →").
fn schluessel(name: &str) -> String {
    let t = name.trim().to_lowercase();
    let ohne = t
        .trim_end_matches(['.', '!', '…', '>', '›', '→', '»'])
        .trim();
    // Reine Zeichenfolgen wie „..." oder „>>" behalten ihre Zeichen.
    if ohne.is_empty() { t } else { ohne.to_string() }
}

/// Links mit `href`, die der Baum nicht auslässt, samt Namen.
fn links_mit_namen<'d, D: Semantics>(doc: &'d D) -> Vec<(D::N<'d>, String)> {
    elements(doc)
        .filter(|n| n.is_element("a") && n.has_attr("href") && !doc.is_ignored(*n))
        .filter_map(|n| {
            let name = doc.accessible_name(n)?;
            let name = name.trim().to_string();
            (!name.is_empty()).then_some((n, name))
        })
        .collect()
}

/// Ob der Zusammenhang den Zweck des Links programmatisch bestimmbar macht
/// (2.4.4): eine Beschreibung, Text direkt daneben, Text im selben Absatz,
/// Listenpunkt oder in derselben Tabellenzelle, oder eine Überschrift davor im
/// selben Baustein (Technik H80) — eine Teaserkarte „Building Information
/// Modeling … Mehr erfahren" nennt ihren Zweck so. Höchstens vier Ebenen
/// hinauf: eine ferne Überschrift ist kein Kontext für diesen Link.
fn hat_kontext<'a, D: Semantics>(doc: &'a D, link: D::N<'a>) -> bool {
    if link
        .attr("aria-describedby")
        .is_some_and(|v| !v.trim().is_empty())
    {
        return true;
    }
    let mut aktuell = link;
    for tiefe in 0..4 {
        let Some(eltern) = aktuell.parent() else {
            break;
        };
        let behaelter = ["p", "li", "td", "th", "dd"]
            .iter()
            .any(|t| eltern.is_element(t));
        if (tiefe == 0 && text_daneben(doc, eltern, link))
            || (behaelter && text_ausser(doc, eltern, link))
            || ueberschrift_davor(doc, eltern, link)
        {
            return true;
        }
        aktuell = eltern;
    }
    false
}

fn sichtbarer_text<'a, D: Semantics>(doc: &'a D, n: D::N<'a>) -> bool {
    n.kind() == a11y_dom::NodeKind::Text
        && !n.text().trim().is_empty()
        && !a11y_dom::ancestors(n).any(|a| doc.is_ignored(a))
}

/// Text als Geschwister des Links, unmittelbar oder in einem `<p>`.
fn text_daneben<'a, D: Semantics>(doc: &'a D, eltern: D::N<'a>, link: D::N<'a>) -> bool {
    eltern.children().filter(|k| *k != link).any(|k| {
        sichtbarer_text(doc, k)
            || (k.is_element("p") && a11y_dom::descendants(k).any(|t| sichtbarer_text(doc, t)))
    })
}

/// Text irgendwo im Teilbaum von `behaelter`, außerhalb des Links.
fn text_ausser<'a, D: Semantics>(doc: &'a D, behaelter: D::N<'a>, link: D::N<'a>) -> bool {
    a11y_dom::descendants(behaelter)
        .filter(|n| *n != link && !a11y_dom::ancestors(*n).any(|a| a == link))
        .any(|n| sichtbarer_text(doc, n))
}

/// Eine benannte Überschrift in `behaelter`, die in Dokumentreihenfolge vor
/// dem Link steht.
fn ueberschrift_davor<'a, D: Semantics>(doc: &'a D, behaelter: D::N<'a>, link: D::N<'a>) -> bool {
    for n in a11y_dom::descendants(behaelter) {
        if n == link {
            return false;
        }
        let ist_ueberschrift = ["h1", "h2", "h3", "h4", "h5", "h6"]
            .iter()
            .any(|t| n.is_element(t))
            || doc.role(n).as_deref() == Some("heading");
        if ist_ueberschrift && !doc.is_ignored(n) && named(doc, n) {
            return true;
        }
    }
    false
}

/// Heuristisch: Die Liste kann einen Namen treffen, der im Zusammenhang doch
/// eindeutig ist. Deshalb `REVIEW`, nicht `FAIL` — mit erkennbarem Kontext
/// als Low, ohne als Medium.
fn generic_link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for (n, name) in links_mit_namen(doc) {
        if !NICHTSSAGEND.contains(&schluessel(&name).as_str()) {
            continue;
        }
        let kontext = hat_kontext(doc, n);
        let meldung = if kontext {
            tr!(
                locale,
                "The link text \"{}\" says nothing about its target; the surrounding text may explain it.",
                "Der Linktext \"{}\" sagt nichts über das Ziel; der umgebende Text erklärt es womöglich.",
                name
            )
        } else {
            tr!(
                locale,
                "The link text \"{}\" says nothing about its target.",
                "Der Linktext \"{}\" sagt nichts über das Ziel.",
                name
            )
        };
        out.push(
            Finding::review("links/generic-name", meldung)
                .with_severity(if kontext {
                    Severity::Low
                } else {
                    Severity::Medium
                })
                .with_wcag(["2.4.4"])
                .at(at(n.id())),
        );
    }
}

/// Eine nackte Adresse als Linktext („https://…", „www.…") — vorgelesen ein
/// Zeichensalat, und sie sagt nicht, was dort steht. Mit erkennbarem Kontext
/// keine Meldung.
fn url_link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for (n, name) in links_mit_namen(doc) {
        if !sieht_aus_wie_url(&name) || hat_kontext(doc, n) {
            continue;
        }
        out.push(
            Finding::review(
                "links/url-name",
                tr!(
                    locale,
                    "The link text is a bare address: \"{}\".",
                    "Der Linktext ist eine nackte Adresse: \"{}\".",
                    name
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["2.4.4"])
            .at(at(n.id())),
        );
    }
}

fn sieht_aus_wie_url(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    t.starts_with("http://")
        || t.starts_with("https://")
        || t.starts_with("www.")
        || (!t.contains(' ') && [".com", ".org", ".net"].iter().any(|e| t.contains(e)))
}

/// Ein einzelnes Zeichen als Linkname („x", „i"). Ziffern bleiben außen vor:
/// Seitenzahlen einer Blätternavigation sind im Zusammenhang verständlich.
/// Zeichen aus [`NICHTSSAGEND`] meldet `links/generic-name`.
fn single_character_link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for (n, name) in links_mit_namen(doc) {
        let mut zeichen = name.chars();
        let (Some(c), None) = (zeichen.next(), zeichen.next()) else {
            continue;
        };
        if c.is_ascii_digit() || NICHTSSAGEND.contains(&schluessel(&name).as_str()) {
            continue;
        }
        out.push(
            Finding::review(
                "links/single-character-name",
                tr!(
                    locale,
                    "The link is named by a single character: \"{}\".",
                    "Der Link heißt nur \"{}\" — ein einzelnes Zeichen.",
                    name
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["2.4.4"])
            .at(at(n.id())),
        );
    }
}

/// Hinweise im Linknamen, dass ein neues Fenster oder Tab aufgeht.
const NEUES_FENSTER: &[&str] = &[
    "new window",
    "new tab",
    "opens in",
    "(external)",
    "external link",
    "[external]",
    "neues fenster",
    "neuem fenster",
    "neuen tab",
    "neuer tab",
    "neuem tab",
    "öffnet in",
    "externer link",
    "(extern)",
];

/// `target="_blank"` ohne Ankündigung im Namen (Technik G201, 3.2.5 AAA).
/// auditmysite prüfte hier `aria-haspopup` statt `target` — das traf nie den
/// gemeinten Fall.
fn new_window_links<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for (n, name) in links_mit_namen(doc) {
        if !n
            .attr("target")
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("_blank"))
        {
            continue;
        }
        let klein = name.to_lowercase();
        if NEUES_FENSTER.iter().any(|h| klein.contains(h)) {
            continue;
        }
        out.push(
            Finding::review(
                "links/new-window-unannounced",
                tr!(
                    locale,
                    "The link \"{}\" opens a new window without saying so.",
                    "Der Link \"{}\" öffnet ein neues Fenster, ohne es anzukündigen.",
                    name
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["3.2.5"])
            .at(at(n.id())),
        );
    }
}

/// 2.4.9 (AAA): Der Linktext allein muss den Zweck nennen; Zusammenhang zählt
/// hier nicht.
fn purpose_link_only<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    for (n, name) in links_mit_namen(doc) {
        if !NUR_LINKTEXT_VAGE.contains(&schluessel(&name).as_str()) {
            continue;
        }
        out.push(
            Finding::review(
                "links/purpose-link-only",
                tr!(
                    locale,
                    "The link text \"{}\" does not name its purpose on its own.",
                    "Der Linktext \"{}\" nennt seinen Zweck nicht für sich allein.",
                    name
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["2.4.9"])
            .at(at(n.id())),
        );
    }
}

fn ambiguous_link_names<D: Semantics>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    use std::collections::HashMap;
    let mut nach_name: HashMap<String, Vec<(NodeId, String)>> = HashMap::new();

    for n in elements(doc) {
        if !n.is_element("a") || doc.is_ignored(n) {
            continue;
        }
        let (Some(name), Some(href)) = (doc.accessible_name(n), n.attr("href")) else {
            continue;
        };
        let key = name.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        nach_name
            .entry(key)
            .or_default()
            .push((n.id(), href.to_string()));
    }

    for (name, treffer) in nach_name {
        if treffer.len() < 2 {
            continue;
        }
        let ziele: std::collections::HashSet<&str> =
            treffer.iter().map(|(_, h)| h.as_str()).collect();
        if ziele.len() < 2 {
            continue; // gleicher Text, gleiches Ziel: unproblematisch
        }
        for (id, _) in &treffer {
            out.push(
                Finding::review(
                    "links/ambiguous-name",
                    tr!(
                        locale,
                        "Several links are named \"{name}\" but point to different targets.",
                        "Mehrere Links heißen \"{name}\", zeigen aber auf verschiedene Ziele."
                    ),
                )
                .with_severity(Severity::Medium)
                .with_wcag(["2.4.4"])
                .at(at(*id)),
            );
        }
    }
}

/// Die Deklarationen. Siehe [`crate::structure::METAS`] zur Begründung der
/// Trennung von den Funktionen.
pub const METAS: &[Meta] = &[
    Meta {
        ids: &["links/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4", "4.1.2"],
        severity: Severity::Critical,
        help: "Every link needs a name that describes its target.",
        #[cfg(feature = "de")]
        help_de: "Jeder Link braucht einen Namen, der sein Ziel beschreibt.",
    },
    Meta {
        ids: &["buttons/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Critical,
        help: "Every button needs a name that describes what it does.",
        #[cfg(feature = "de")]
        help_de: "Jeder Button braucht einen Namen, der seine Wirkung beschreibt.",
    },
    Meta {
        ids: &["svg/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1"],
        severity: Severity::High,
        help: "Informative SVGs need a name, decorative ones role=\"presentation\".",
        #[cfg(feature = "de")]
        help_de: "Informative SVGs brauchen einen Namen, dekorative role=\"presentation\".",
    },
    Meta {
        ids: &["links/ambiguous-name"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4"],
        severity: Severity::Medium,
        help: "Links with the same name should point to the same target.",
        #[cfg(feature = "de")]
        help_de: "Gleich benannte Links sollten auf dasselbe Ziel zeigen.",
    },
    Meta {
        ids: &["links/generic-name"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4"],
        severity: Severity::Medium,
        help: "Link text should say where it leads without the surrounding sentence.",
        #[cfg(feature = "de")]
        help_de: "Der Linktext soll auch ohne den umgebenden Satz sagen, wohin er führt.",
    },
    Meta {
        ids: &["links/url-name"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4"],
        severity: Severity::Low,
        help: "A bare address as link text does not say what the target contains.",
        #[cfg(feature = "de")]
        help_de: "Eine nackte Adresse als Linktext sagt nicht, was das Ziel enthält.",
    },
    Meta {
        ids: &["links/single-character-name"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.4"],
        severity: Severity::Medium,
        help: "A single character does not name a link's target.",
        #[cfg(feature = "de")]
        help_de: "Ein einzelnes Zeichen benennt kein Linkziel.",
    },
    Meta {
        ids: &["links/new-window-unannounced"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["3.2.5"],
        severity: Severity::Low,
        help: "Say in the link text when a link opens a new window or tab.",
        #[cfg(feature = "de")]
        help_de: "Im Linktext ankündigen, wenn ein Link ein neues Fenster oder Tab öffnet.",
    },
    Meta {
        ids: &["links/purpose-link-only"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.9"],
        severity: Severity::Low,
        help: "The link text alone should name the link's purpose.",
        #[cfg(feature = "de")]
        help_de: "Der Linktext allein soll den Zweck des Links nennen.",
    },
    Meta {
        ids: &["aria/attribute-not-allowed", "aria/attribute-prohibited"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Use only ARIA attributes the element's role supports.",
        #[cfg(feature = "de")]
        help_de: "Nur ARIA-Attribute verwenden, die die Rolle des Elements unterstützt.",
    },
    Meta {
        ids: &[
            "aria/required-parent-missing",
            "aria/required-children-missing",
        ],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1", "4.1.2"],
        severity: Severity::High,
        help: "Roles that belong together must be nested as WAI-ARIA requires.",
        #[cfg(feature = "de")]
        help_de: "Zusammengehörige Rollen müssen verschachtelt sein, wie WAI-ARIA es verlangt.",
    },
    Meta {
        ids: &["names/required-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.1.1", "4.1.2"],
        severity: Severity::High,
        help: "Elements whose role requires a name (fields, toggles, menu items, meters, …) need one.",
        #[cfg(feature = "de")]
        help_de: "Elemente, deren Rolle einen Namen verlangt (Felder, Schalter, Menüpunkte, Messanzeigen, …), brauchen einen.",
    },
    Meta {
        ids: &["names/symbol-only"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Medium,
        help: "An accessible name should be words, not a single symbol.",
        #[cfg(feature = "de")]
        help_de: "Ein zugänglicher Name sollte aus Wörtern bestehen, nicht aus einem einzelnen Symbol.",
    },
    Meta {
        ids: &["dialog/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Every dialog needs a name, usually via aria-labelledby on its heading.",
        #[cfg(feature = "de")]
        help_de: "Jeder Dialog braucht einen Namen, meist per aria-labelledby auf seine Überschrift.",
    },
    Meta {
        ids: &["dialog/modal-unmarked"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Medium,
        help: "A modal dialog should carry aria-modal=\"true\".",
        #[cfg(feature = "de")]
        help_de: "Ein modaler Dialog sollte aria-modal=\"true\" tragen.",
    },
    Meta {
        ids: &["summary/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "The <summary> of a <details> element needs text.",
        #[cfg(feature = "de")]
        help_de: "Die <summary> eines <details>-Elements braucht Text.",
    },
    Meta {
        ids: &["status/live-overridden"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.3"],
        severity: Severity::High,
        help: "Live regions should keep the aria-live their role implies.",
        #[cfg(feature = "de")]
        help_de: "Live-Regionen sollten das aria-live behalten, das ihre Rolle vorgibt.",
    },
    Meta {
        ids: &["label-in-name/mismatch"],
        tier: Tier::Semantics,
        scope: Scope::Rendered,
        wcag: &["2.5.3"],
        severity: Severity::Medium,
        help: "The accessible name must contain the visible label text.",
        #[cfg(feature = "de")]
        help_de: "Der zugängliche Name muss den sichtbaren Beschriftungstext enthalten.",
    },
    Meta {
        ids: &["forms/purpose-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.5"],
        severity: Severity::Medium,
        help: "Fields asking for information about the user carry the matching autocomplete token.",
        #[cfg(feature = "de")]
        help_de: "Felder, die nach Angaben zur Person fragen, tragen die passende autocomplete-Angabe.",
    },
    Meta {
        ids: &["forms/required-unmarked", "forms/instructions-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["3.3.2"],
        severity: Severity::Medium,
        help: "Required fields and expected formats are stated in the label or description.",
        #[cfg(feature = "de")]
        help_de: "Pflichtfelder und erwartete Formate stehen in Beschriftung oder Beschreibung.",
    },
    Meta {
        ids: &["forms/title-only-label"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1"],
        severity: Severity::Medium,
        help: "A field should have a visible label, not only a title attribute.",
        #[cfg(feature = "de")]
        help_de: "Ein Feld sollte eine sichtbare Beschriftung haben, nicht nur ein title-Attribut.",
    },
    Meta {
        ids: &["forms/group-name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1", "3.3.2"],
        severity: Severity::Medium,
        help: "A fieldset needs a legend, a form group a name.",
        #[cfg(feature = "de")]
        help_de: "Ein fieldset braucht eine legend, eine Formulargruppe einen Namen.",
    },
    Meta {
        ids: &["landmarks/not-unique"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["1.3.1"],
        severity: Severity::Medium,
        help: "Landmarks of the same role should have distinct names.",
        #[cfg(feature = "de")]
        help_de: "Landmarks derselben Rolle sollten unterscheidbare Namen haben.",
    },
    Meta {
        ids: &[
            "landmarks/not-top-level",
            "landmarks/banner-duplicate",
            "landmarks/contentinfo-duplicate",
        ],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &[],
        severity: Severity::Medium,
        help: "banner, contentinfo and main belong at the top level; banner and contentinfo at most once.",
        #[cfg(feature = "de")]
        help_de: "banner, contentinfo und main gehören auf die oberste Ebene; banner und contentinfo höchstens einmal.",
    },
    Meta {
        ids: &["landmarks/content-outside"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &[],
        severity: Severity::Medium,
        help: "All content belongs inside a landmark.",
        #[cfg(feature = "de")]
        help_de: "Aller Inhalt gehört in eine Landmark.",
    },
    Meta {
        ids: &[
            "keyboard/focusable-no-role",
            "keyboard/interactive-not-focusable",
        ],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.1.1"],
        severity: Severity::High,
        help: "Interactive roles must be focusable; focusable elements need an interactive role.",
        #[cfg(feature = "de")]
        help_de: "Interaktive Rollen müssen fokussierbar sein; fokussierbare Elemente brauchen eine interaktive Rolle.",
    },
    Meta {
        ids: &["dialog/focusable-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["2.4.3"],
        severity: Severity::Medium,
        help: "A dialog needs at least one focusable element.",
        #[cfg(feature = "de")]
        help_de: "Ein Dialog braucht mindestens ein fokussierbares Element.",
    },
    Meta {
        ids: &["patterns/accordion-controls-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::Low,
        help: "An expanded button should reference the region it controls with aria-controls.",
        #[cfg(feature = "de")]
        help_de: "Ein aufgeklappter Button sollte per aria-controls auf den gesteuerten Bereich verweisen.",
    },
    Meta {
        ids: &["frames/name-missing"],
        tier: Tier::Semantics,
        scope: Scope::AccessibilityTree,
        wcag: &["4.1.2"],
        severity: Severity::High,
        help: "Every iframe needs a name, usually a title saying what it contains.",
        #[cfg(feature = "de")]
        help_de: "Jeder Rahmen braucht einen Namen, meist einen title, der sagt, was er enthält.",
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
fn funktionen<D: Semantics>() -> [fn(&D, Locale, &mut Vec<Finding>); 29] {
    [
        link_names,
        button_names,
        svg_names,
        ambiguous_link_names,
        generic_link_names,
        url_link_names,
        single_character_link_names,
        new_window_links,
        purpose_link_only,
        crate::aria::attributes_allowed,
        crate::aria::required_context,
        crate::names::required_names,
        crate::names::symbol_names,
        crate::names::dialog_names,
        crate::names::dialog_modal,
        crate::names::summary_names,
        crate::names::live_regions,
        crate::names::label_in_name,
        crate::forms::purpose_missing,
        crate::forms::instructions,
        crate::forms::title_only,
        crate::forms::group_name_missing,
        crate::landmarks::not_unique,
        crate::landmarks::structure,
        crate::landmarks::content_outside,
        crate::landmarks::keyboard,
        crate::landmarks::dialog_focusable,
        crate::landmarks::accordion_controls,
        crate::media::frame_names,
    ]
}

/// Alle Tier-2-Regeln.
pub fn rules<D: Semantics>() -> Vec<SemanticsRule<D>> {
    METAS
        .iter()
        .zip(funktionen::<D>())
        .map(|(meta, run)| SemanticsRule { meta: *meta, run })
        .collect()
}
