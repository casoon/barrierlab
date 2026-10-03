//! Regeln über die Stylesheets der Seite (casoon/barrierlab#21): entfernter
//! Fokusrahmen (2.4.7), Bewegung ohne `prefers-reduced-motion` (2.3.3),
//! Inhalt, der je nach Ausrichtung verschwindet (1.3.4), und Blocksatz oder
//! enger Zeilenabstand in Absätzen (1.4.8).
//!
//! Portiert aus auditmysite (`focus_visible_css`, `reduced_motion`,
//! `orientation`, `visual_presentation`). Dort las JavaScript
//! `document.styleSheets` in der laufenden Seite. Hier kommen die Stylesheets
//! als Text herein, geparst von `stylesheet-parse`; der Host liefert, was er hat —
//! auditmysite die Sheets der Seite, astro-post-audit die Dateien aus `dist/`.
//! Fremde Sheets, die der Browser nicht herausgibt, fehlen in beiden Fällen.
//!
//! Der Browser hat bisher die Kurzschreibweisen zerlegt (`animation`,
//! `transition`, `outline`); das tun jetzt die Hilfen unten. Was sie nicht
//! sicher lesen, zählt nicht als Beleg.

use stylesheet_parse::{Declaration, Scoped, Stylesheet};

use a11y_dom::{Document, Node};
use a11y_report::{Finding, Location, Severity};

use crate::locale::{Locale, tr};
use crate::selektor::{Selektor, Treffer, trifft};

fn seite<D: Document>(doc: &D) -> Location {
    Location::node(doc.root().id().to_string())
}

fn wert<'a>(decls: &'a [Declaration], name: &str) -> Option<&'a str> {
    // Die letzte Angabe gewinnt, `!important` vor allen anderen
    // (`max_by_key` liefert bei Gleichstand das letzte Element).
    decls
        .iter()
        .filter(|d| d.name == name)
        .max_by_key(|d| d.important)
        .map(|d| d.value.as_str())
}

/// Eine kommagetrennte Liste auf oberster Ebene (Kommas in Klammern zählen
/// nicht).
fn liste(v: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut tiefe, mut start) = (0i32, 0usize);
    for (i, c) in v.char_indices() {
        match c {
            '(' => tiefe += 1,
            ')' => tiefe -= 1,
            ',' if tiefe == 0 => {
                out.push(v[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(v[start..].trim());
    out.retain(|s| !s.is_empty());
    out
}

/// Die Teile einer Angabe, getrennt an Leerraum außerhalb von Klammern.
fn teile(v: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut tiefe, mut start) = (0i32, None::<usize>);
    for (i, c) in v.char_indices() {
        match c {
            '(' => {
                tiefe += 1;
                start.get_or_insert(i);
            }
            ')' => tiefe -= 1,
            c if c.is_whitespace() && tiefe == 0 => {
                if let Some(s) = start.take() {
                    out.push(&v[s..i]);
                }
            }
            _ => {
                start.get_or_insert(i);
            }
        }
    }
    if let Some(s) = start {
        out.push(&v[s..]);
    }
    out
}

/// Eine Zeitangabe in Millisekunden: `0.3s`, `300ms`, `.01ms`.
fn zeit_ms(t: &str) -> Option<f64> {
    let t = t.trim().to_ascii_lowercase();
    if let Some(n) = t.strip_suffix("ms") {
        n.parse().ok()
    } else if let Some(n) = t.strip_suffix('s') {
        n.parse::<f64>().ok().map(|s| s * 1000.0)
    } else {
        None
    }
}

const ZEITFUNKTIONEN: &[&str] = &[
    "ease",
    "linear",
    "ease-in",
    "ease-out",
    "ease-in-out",
    "step-start",
    "step-end",
];

fn ist_funktion(t: &str) -> bool {
    t.contains('(')
}

// --- Kurzschreibweisen --------------------------------------------------------

/// Je Übergang: Eigenschaft und Dauer.
fn uebergaenge(decls: &[Declaration]) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    if let Some(v) = wert(decls, "transition") {
        for einzel in liste(v) {
            let mut eigenschaft = None;
            let mut dauer = None;
            for t in teile(einzel) {
                let tl = t.to_ascii_lowercase();
                if let Some(ms) = zeit_ms(&tl) {
                    dauer.get_or_insert(ms);
                } else if !ist_funktion(&tl)
                    && !ZEITFUNKTIONEN.contains(&tl.as_str())
                    && !matches!(tl.as_str(), "normal" | "allow-discrete")
                {
                    eigenschaft.get_or_insert(tl);
                }
            }
            out.push((
                eigenschaft.unwrap_or_else(|| "all".into()),
                dauer.unwrap_or(0.0),
            ));
        }
    }
    let eigenschaften = wert(decls, "transition-property").map(liste);
    let dauern = wert(decls, "transition-duration").map(|d| {
        liste(d)
            .into_iter()
            .map(|t| zeit_ms(t).unwrap_or(0.0))
            .collect::<Vec<_>>()
    });
    match (eigenschaften, dauern) {
        (Some(p), d) => {
            let d = d.unwrap_or_else(|| out.iter().map(|(_, ms)| *ms).collect());
            out = p
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let ms = if d.is_empty() { 0.0 } else { d[i % d.len()] };
                    (p.to_ascii_lowercase(), ms)
                })
                .collect();
        }
        // Nur `transition-duration`, ohne Eigenschaft: Der Browser setzt
        // damit keinen Übergang (Tailwind `.duration-200`).
        (None, Some(d)) if !d.is_empty() => {
            for (i, e) in out.iter_mut().enumerate() {
                e.1 = d[i % d.len()];
            }
        }
        _ => {}
    }
    out
}

const ANIMATIONS_SCHLUESSEL: &[&str] = &[
    "infinite",
    "normal",
    "reverse",
    "alternate",
    "alternate-reverse",
    "forwards",
    "backwards",
    "both",
    "running",
    "paused",
    "initial",
    "inherit",
    "unset",
];

/// Die Animationsnamen einer Regel, ohne `none`.
fn animationsnamen(decls: &[Declaration]) -> Vec<String> {
    let mut namen = Vec::new();
    if let Some(v) = wert(decls, "animation-name") {
        namen.extend(liste(v).into_iter().map(String::from));
    } else if let Some(v) = wert(decls, "animation") {
        for einzel in liste(v) {
            let name = teile(einzel).into_iter().find(|t| {
                let tl = t.to_ascii_lowercase();
                zeit_ms(&tl).is_none()
                    && tl.parse::<f64>().is_err()
                    && !ist_funktion(&tl)
                    && !ZEITFUNKTIONEN.contains(&tl.as_str())
                    && !ANIMATIONS_SCHLUESSEL.contains(&tl.as_str())
            });
            if let Some(n) = name {
                namen.push(n.trim_matches(['"', '\'']).to_string());
            }
        }
    }
    namen.retain(|n| !n.eq_ignore_ascii_case("none"));
    namen
}

/// Größte Animationsdauer einer Regel in ms, wenn angegeben.
fn animationsdauer(decls: &[Declaration]) -> Option<f64> {
    if let Some(v) = wert(decls, "animation-duration") {
        return liste(v).into_iter().filter_map(zeit_ms).reduce(f64::max);
    }
    let v = wert(decls, "animation")?;
    liste(v)
        .into_iter()
        .filter_map(|e| teile(e).into_iter().find_map(zeit_ms))
        .reduce(f64::max)
}

// --- focus/outline-removed ----------------------------------------------------

fn ist_null(v: &str) -> bool {
    matches!(v.trim(), "0" | "0px" | "0em" | "0rem")
}

fn entfernt_rahmen(decls: &[Declaration]) -> bool {
    let entfernt = |v: &str| {
        let v = v.trim().to_ascii_lowercase();
        v == "none" || ist_null(&v) || v.split_whitespace().any(|t| t == "none")
    };
    wert(decls, "outline").is_some_and(entfernt)
        || wert(decls, "outline-style").is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
        || wert(decls, "outline-width").is_some_and(ist_null)
}

/// Ein sichtbarer Hinweis in derselben Regel: ein wieder gesetzter
/// Fokusrahmen, Rahmen, Schatten, Hintergrund oder Unterstreichung.
///
/// Der wieder gesetzte Rahmen fehlte in auditmysite. sueddeutsche.de
/// (2026-10-03) entfernt ihn mit `[data-whatinput] *:focus` für alle und
/// setzt ihn mit `[data-whatintent='keyboard'] *:focus { outline: 2px solid }`
/// für die Tastatur wieder.
fn ersetzt_rahmen(decls: &[Declaration]) -> bool {
    let rahmen_gesetzt = ["outline", "outline-style", "outline-width"]
        .iter()
        .filter_map(|n| wert(decls, n))
        .any(|v| !matches!(v.trim(), "initial" | "unset" | "revert" | "inherit"))
        && !entfernt_rahmen(decls);
    let gesetzt = |v: &str| {
        let v = v.trim().to_ascii_lowercase();
        !v.is_empty() && v != "none" && !ist_null(&v) && v != "transparent" && v != "inherit"
    };
    decls.iter().any(|d| {
        let n = d.name.as_str();
        (n == "box-shadow"
            || n == "background-color"
            || n == "background"
            || n == "text-decoration"
            || n == "text-decoration-line"
            || n == "border"
            || n == "border-bottom"
            || n == "border-color"
            || n == "border-bottom-color")
            && gesetzt(&d.value)
    }) || rahmen_gesetzt
}

fn fokus_selektor(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    s.contains(":focus") && !s.contains(":focus-within") && !s.contains(":focus-visible")
}

/// 2.4.7: Ein Stylesheet entfernt den Fokusrahmen unter `:focus`, ohne dass
/// es irgendwo einen Ersatz gibt.
///
/// Wie auditmysite seitenweit: Ein `:focus-visible` irgendwo, oder ein
/// Ersatz in irgendeiner `:focus`-Regel, nimmt den Befund zurück — ein
/// globaler Reset mit `outline: none` und ein Hinweis per `box-shadow` in
/// einer anderen Regel sind das verbreitete Muster.
pub(crate) fn outline_removed<D: Document>(
    doc: &D,
    sheets: &[Stylesheet],
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    let regeln: Vec<Scoped<'_>> = sheets.iter().flat_map(|s| s.style_rules()).collect();
    let mit_focus_visible = regeln.iter().any(|r| {
        r.selectors
            .iter()
            .any(|s| s.to_ascii_lowercase().contains(":focus-visible"))
    });
    let mit_ersatz = regeln
        .iter()
        .any(|r| r.selectors.iter().any(|s| fokus_selektor(s)) && ersetzt_rahmen(r.declarations));
    if mit_focus_visible || mit_ersatz {
        return;
    }
    let entfernt: Vec<&str> = regeln
        .iter()
        .filter(|r| entfernt_rahmen(r.declarations))
        .flat_map(|r| r.selectors.iter().filter(|s| fokus_selektor(s)))
        .map(String::as_str)
        .collect();
    if entfernt.is_empty() {
        return;
    }
    let beispiele = entfernt
        .iter()
        .take(3)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    out.push(
        Finding::fail(
            "focus/outline-removed",
            tr!(
                locale,
                "A stylesheet removes the focus outline ({beispiele}) and no rule replaces it; \
                 keyboard users cannot see where focus is.",
                "Ein Stylesheet entfernt den Fokusrahmen ({beispiele}), und keine Regel ersetzt \
                 ihn; wer mit der Tastatur bedient, sieht nicht, wo der Fokus steht."
            ),
        )
        .with_severity(Severity::High)
        .with_wcag(["2.4.7"])
        .at(seite(doc)),
    );
}

// --- motion/reduced-motion-ignored --------------------------------------------

/// Bewegung im Sinne von 2.3.3: Lage, Verschiebung, Drehung, Skalierung.
/// Farbe und Deckkraft gehören nicht dazu.
fn ist_bewegung(p: &str) -> bool {
    let p = p.trim().to_ascii_lowercase();
    matches!(
        p.as_str(),
        "transform"
            | "translate"
            | "rotate"
            | "scale"
            | "top"
            | "left"
            | "right"
            | "bottom"
            | "position"
            | "offset-path"
            | "offset-distance"
    ) || p.starts_with("inset")
        || p.starts_with("margin")
}

const ZUSTAND: &[&str] = &[
    ":hover",
    ":focus",
    ":active",
    ":checked",
    ":target",
    ":focus-within",
    ":focus-visible",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lage {
    Normal,
    Reduziert,
    Abgeschirmt,
}

fn lage(media: &[&str]) -> Lage {
    let mut l = Lage::Normal;
    for m in media {
        let m = m.to_ascii_lowercase();
        if m.contains("prefers-reduced-motion") {
            l = if m.contains("no-preference") || m.split_whitespace().any(|w| w == "not") {
                Lage::Abgeschirmt
            } else {
                Lage::Reduziert
            };
        }
    }
    l
}

/// Neutralisiert ab dieser Dauer (ms): die üblichen Resets `0.01ms`/`1ms`.
const NEUTRALISIERT_BIS_MS: f64 = 50.0;

fn selektor_teile(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 2.3.3 (AAA): Bewegung durch Animation oder Übergang, die unter
/// `prefers-reduced-motion: reduce` nicht abgeschaltet wird.
///
/// Wie auditmysite (#712): Beleg ist nur Bewegung, die auf dieser Seite
/// laufen kann — der Selektor muss ein Element im Dokument treffen
/// ([`crate::selektor`]). `transition: all` zählt nur, wenn eine
/// Zustandsregel eine Bewegungs-Eigenschaft setzt.
pub(crate) fn reduced_motion_ignored<D: Document>(
    doc: &D,
    sheets: &[Stylesheet],
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    let regeln: Vec<Scoped<'_>> = sheets.iter().flat_map(|s| s.style_rules()).collect();
    let keyframes: Vec<_> = sheets.iter().flat_map(|s| s.keyframes()).collect();
    let bewegte_keyframes = |name: &str| -> Option<String> {
        keyframes
            .iter()
            .filter(|(k, ctx)| k.name == name && lage(&ctx.media) != Lage::Abgeschirmt)
            .flat_map(|(k, _)| k.frames.iter())
            .flat_map(|f| f.declarations.iter())
            .find(|d| ist_bewegung(&d.name))
            .map(|d| d.name.clone())
    };
    let zustand_bewegt = regeln.iter().any(|r| {
        lage(&r.context.media) == Lage::Normal
            && r.selectors.iter().any(|s| {
                let s = s.to_ascii_lowercase();
                ZUSTAND.iter().any(|z| s.contains(z))
            })
            && r.declarations.iter().any(|d| ist_bewegung(&d.name))
    });

    // Abschaltungen im Block `prefers-reduced-motion: reduce`.
    let abschaltungen: Vec<(Vec<String>, bool, bool)> = regeln
        .iter()
        .filter(|r| lage(&r.context.media) == Lage::Reduziert)
        .map(|r| {
            let d = r.declarations;
            let animation_aus = animationsnamen(d).is_empty()
                && (wert(d, "animation-name")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
                    || wert(d, "animation").is_some_and(|v| v.trim().eq_ignore_ascii_case("none")))
                || animationsdauer(d).is_some_and(|ms| ms <= NEUTRALISIERT_BIS_MS)
                || wert(d, "animation-play-state")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("paused"));
            let uebergang_aus = wert(d, "transition-property")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
                || wert(d, "transition").is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
                || wert(d, "transition-duration").is_some_and(|v| {
                    liste(v)
                        .into_iter()
                        .filter_map(zeit_ms)
                        .reduce(f64::max)
                        .is_some_and(|ms| ms <= NEUTRALISIERT_BIS_MS)
                })
                || {
                    let u = uebergaenge(d);
                    !u.is_empty() && u.iter().all(|(_, ms)| *ms <= NEUTRALISIERT_BIS_MS)
                };
            (
                r.selectors.iter().map(|s| selektor_teile(s)).collect(),
                animation_aus,
                uebergang_aus,
            )
        })
        .collect();
    let abgeschaltet = |selektor: &str, animation: bool| {
        let s = selektor_teile(selektor);
        abschaltungen.iter().any(|(sel, a, u)| {
            (if animation { *a } else { *u }) && sel.iter().any(|x| x.contains('*') || *x == s)
        })
    };

    let mut belege: Vec<String> = Vec::new();
    for r in regeln
        .iter()
        .filter(|r| lage(&r.context.media) == Lage::Normal)
    {
        for selektor in &r.selectors {
            let mut beleg = None;
            for name in animationsnamen(r.declarations) {
                if let Some(p) = bewegte_keyframes(&name) {
                    if !abgeschaltet(selektor, true) {
                        beleg = Some(format!("{selektor}: @keyframes {name} ({p})"));
                    }
                    break;
                }
            }
            if beleg.is_none() {
                let bewegt = uebergaenge(r.declarations)
                    .into_iter()
                    .filter(|(_, ms)| *ms > 0.0)
                    .find_map(|(p, _)| {
                        if ist_bewegung(&p) {
                            Some(p)
                        } else if p == "all" && zustand_bewegt {
                            Some("all".to_string())
                        } else {
                            None
                        }
                    });
                if let Some(p) = bewegt
                    && !abgeschaltet(selektor, false)
                {
                    beleg = Some(format!("{selektor}: transition {p}"));
                }
            }
            if let Some(b) = beleg
                && trifft(doc, selektor) != Treffer::Nein
            {
                belege.push(b);
            }
        }
    }
    if belege.is_empty() {
        return;
    }
    let anzahl = belege.len();
    let beispiele = belege
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ");
    out.push(
        Finding::review(
            "motion/reduced-motion-ignored",
            tr!(
                locale,
                "{anzahl} style rules move content and are not switched off under \
                 prefers-reduced-motion: reduce ({beispiele}).",
                "{anzahl} Stilregeln bewegen Inhalt und werden unter \
                 prefers-reduced-motion: reduce nicht abgeschaltet ({beispiele})."
            ),
        )
        .with_severity(Severity::Medium)
        .with_wcag(["2.3.3"])
        .at(seite(doc)),
    );
}

// --- orientation/content-hidden ---------------------------------------------

fn ganzer_inhalt(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "html" | "body" | "main" | ":root" | "*" | "body > *"
    )
}

/// 1.3.4: Unter einer Media Query auf die Ausrichtung wird Inhalt
/// ausgeblendet oder gedreht.
///
/// `REVIEW` statt auditmysites Verstoß: Meist ist das gewöhnliches Umbauen
/// eines Layouts. Echte Sperren sehen so aus wie auf welt.de (2026-10-03):
/// Ein Dialog blendet im Querformat auf dem Telefon seinen Inhalt aus und
/// zeigt stattdessen „bitte drehen". Ob das Ausgeblendete nur in einer
/// Ausrichtung zu haben ist, entscheidet ein Mensch. Schwere hoch, wenn
/// `html`, `body` oder `main` betroffen sind, sonst mittel.
///
/// Gemeldet wird nur, was ein Element der Seite trifft, und keine
/// Pseudo-Elemente: Breakpoint-Marker wie `body:before { display: none }`
/// unter `(orientation: landscape)` (bundesregierung.de) tragen keinen
/// Inhalt.
pub(crate) fn orientation_content_hidden<D: Document>(
    doc: &D,
    sheets: &[Stylesheet],
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    let mut belege: Vec<(String, String)> = Vec::new();
    let mut ganz = false;
    for r in sheets.iter().flat_map(|s| s.style_rules()) {
        let Some(media) = r
            .context
            .media
            .iter()
            .find(|m| m.to_ascii_lowercase().contains("orientation"))
        else {
            continue;
        };
        let d = r.declarations;
        let versteckt = wert(d, "display").is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
            || wert(d, "visibility").is_some_and(|v| v.trim().eq_ignore_ascii_case("hidden"));
        let gedreht = wert(d, "transform")
            .is_some_and(|v| v.to_ascii_lowercase().contains("rotate"))
            || wert(d, "rotate").is_some_and(|v| !v.trim().eq_ignore_ascii_case("none"));
        if !versteckt && !gedreht {
            continue;
        }
        for sel in &r.selectors {
            if sel.contains("::")
                || sel.to_ascii_lowercase().contains(":before")
                || sel.to_ascii_lowercase().contains(":after")
                || trifft(doc, sel) == Treffer::Nein
            {
                continue;
            }
            ganz |= ganzer_inhalt(sel);
            belege.push((sel.clone(), media.to_string()));
        }
    }
    if belege.is_empty() {
        return;
    }
    let anzahl = belege.len();
    let beispiele = belege
        .iter()
        .take(3)
        .map(|(s, m)| format!("{s} @media {m}"))
        .collect::<Vec<_>>()
        .join("; ");
    out.push(
        Finding::review(
            "orientation/content-hidden",
            tr!(
                locale,
                "{anzahl} style rules hide or rotate content depending on the orientation \
                 ({beispiele}). Check that the content is available in portrait and landscape.",
                "{anzahl} Stilregeln blenden je nach Ausrichtung Inhalt aus oder drehen ihn \
                 ({beispiele}). Prüfen, ob der Inhalt hoch- und querformatig erreichbar ist."
            ),
        )
        .with_severity(if ganz {
            Severity::High
        } else {
            Severity::Medium
        })
        .with_wcag(["1.3.4"])
        .at(seite(doc)),
    );
}

// --- text/justified, text/line-height-tight -----------------------------------

/// Der wirksame Wert einer vererbten Eigenschaft an einem Element: die
/// letzte treffende Angabe am Element selbst, sonst am nächsten Vorfahren.
/// Spezifität bleibt außen vor; `!important` gewinnt.
fn wirksam<'a, N: Node<'a>>(n: N, regeln: &[(Selektor, String, bool)]) -> Option<String> {
    for e in std::iter::once(n).chain(a11y_dom::ancestors(n)) {
        let treffend = regeln
            .iter()
            .filter(|(sel, _, _)| sel.trifft(e) == Treffer::Ja)
            .max_by_key(|(_, _, wichtig)| *wichtig);
        if let Some((_, v, _)) = treffend {
            return Some(v.clone());
        }
    }
    None
}

fn regeln_fuer(sheets: &[Stylesheet], eigenschaft: &str) -> Vec<(Selektor, String, bool)> {
    sheets
        .iter()
        .flat_map(|s| s.style_rules())
        .filter(|r| r.context.media.is_empty())
        .flat_map(|r| {
            r.declarations
                .iter()
                .filter(|d| d.name == eigenschaft)
                .map(|d| {
                    (
                        Selektor::lies(&r.selectors.join(", ")),
                        d.value.clone(),
                        d.important,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Zeilenhöhe als Faktor der Schriftgröße, wenn ohne Schriftgröße bestimmbar.
fn zeilenfaktor(v: &str) -> Option<f64> {
    let v = v.trim().to_ascii_lowercase();
    if let Some(p) = v.strip_suffix('%') {
        return p.parse::<f64>().ok().map(|p| p / 100.0);
    }
    v.strip_suffix("em").unwrap_or(&v).parse().ok()
}

/// 1.4.8 (AAA): Absätze im Blocksatz oder mit Zeilenhöhe unter 1,5.
///
/// Anders als auditmysite, das jede Regel mit einem Selektor ab `body`, `p`,
/// `div`, `section` … las, zählt hier der Wert, der an den `<p>` der Seite
/// tatsächlich ankommt — sonst meldete der Standardwert
/// `html { line-height: 1.15 }` aus normalize.css auf jeder Seite, auch wenn
/// die Absätze 1,6 haben. Ohne Media Query, ohne Spezifität; Zeilenhöhe nur
/// ohne Einheit, in `em` oder `%`.
pub(crate) fn visual_presentation<D: Document>(
    doc: &D,
    sheets: &[Stylesheet],
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    let absaetze: Vec<_> = a11y_dom::elements(doc)
        .filter(|n| n.is_element("p") && a11y_dom::has_text(*n))
        .collect();
    if absaetze.is_empty() {
        return;
    }
    let ausrichtung = regeln_fuer(sheets, "text-align");
    let zeilen = regeln_fuer(sheets, "line-height");
    let blocksatz = absaetze
        .iter()
        .filter(|p| {
            wirksam(**p, &ausrichtung).is_some_and(|v| v.trim().eq_ignore_ascii_case("justify"))
        })
        .count();
    let eng: Vec<String> = absaetze
        .iter()
        .filter_map(|p| wirksam(*p, &zeilen))
        .filter(|v| zeilenfaktor(v).is_some_and(|z| z > 0.0 && z < 1.5))
        .collect();
    if blocksatz > 0 {
        out.push(
            Finding::review(
                "text/justified",
                tr!(
                    locale,
                    "{blocksatz} paragraphs are set justified; uneven word spacing makes them \
                     harder to read.",
                    "{blocksatz} Absätze stehen im Blocksatz; ungleiche Wortabstände erschweren \
                     das Lesen."
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.4.8"])
            .at(seite(doc)),
        );
    }
    if let Some(lh) = eng.first() {
        let anzahl = eng.len();
        out.push(
            Finding::review(
                "text/line-height-tight",
                tr!(
                    locale,
                    "{anzahl} paragraphs have a line height below 1.5 (for example {lh}).",
                    "{anzahl} Absätze haben eine Zeilenhöhe unter 1,5 (etwa {lh})."
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.4.8"])
            .at(seite(doc)),
        );
    }
}

// --- Registrierung ------------------------------------------------------------

use a11y_dom::Tier;

use crate::registry::{Meta, StylesheetRule};
use crate::sicht::Scope;

/// Die Deklarationen, in derselben Reihenfolge wie [`rules`].
pub(crate) const METAS: &[Meta] = &[
    Meta {
        ids: &["focus/outline-removed"],
        tier: Tier::Stylesheets,
        scope: Scope::Markup,
        wcag: &["2.4.7"],
        severity: Severity::High,
        help: "Keep a visible focus indicator: do not remove the outline on :focus without a \
               replacement, or limit the removal to :focus:not(:focus-visible).",
        #[cfg(feature = "de")]
        help_de: "Einen sichtbaren Fokushinweis behalten: den Rahmen unter :focus nicht ohne \
                  Ersatz entfernen, oder das Entfernen auf :focus:not(:focus-visible) \
                  beschränken.",
    },
    Meta {
        ids: &["motion/reduced-motion-ignored"],
        tier: Tier::Stylesheets,
        scope: Scope::Rendered,
        wcag: &["2.3.3"],
        severity: Severity::Medium,
        help: "Switch off movement under @media (prefers-reduced-motion: reduce).",
        #[cfg(feature = "de")]
        help_de: "Bewegung unter @media (prefers-reduced-motion: reduce) abschalten.",
    },
    Meta {
        ids: &["orientation/content-hidden"],
        tier: Tier::Stylesheets,
        scope: Scope::Rendered,
        wcag: &["1.3.4"],
        severity: Severity::Medium,
        help: "Content must work in portrait and landscape; do not hide or rotate it per \
               orientation.",
        #[cfg(feature = "de")]
        help_de: "Inhalt muss hoch- und querformatig funktionieren; ihn nicht je nach \
                  Ausrichtung ausblenden oder drehen.",
    },
    Meta {
        ids: &["text/justified", "text/line-height-tight"],
        tier: Tier::Stylesheets,
        scope: Scope::AccessibilityTree,
        wcag: &["1.4.8"],
        severity: Severity::Low,
        help: "Running text: no justified alignment, line height at least 1.5.",
        #[cfg(feature = "de")]
        help_de: "Fließtext: kein Blocksatz, Zeilenhöhe mindestens 1,5.",
    },
];

pub(crate) fn rules<D: Document>() -> Vec<StylesheetRule<D>> {
    type Lauf<D> = fn(&D, &[Stylesheet], Locale, &mut Vec<Finding>);
    let funktionen: [Lauf<D>; 4] = [
        outline_removed,
        reduced_motion_ignored,
        orientation_content_hidden,
        visual_presentation,
    ];
    METAS
        .iter()
        .zip(funktionen)
        .map(|(meta, run)| StylesheetRule { meta: *meta, run })
        .collect()
}
