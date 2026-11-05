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

use crate::locale::{Locale, de_text, tr};
use style_cascade::selector::{Match, Mode, Selector, any_match};

fn page<D: Document>(doc: &D) -> Location {
    Location::node(doc.root().id().to_string())
}

fn value<'a>(decls: &'a [Declaration], name: &str) -> Option<&'a str> {
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
fn list(v: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, c) in v.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
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
fn parts(v: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, None::<usize>);
    for (i, c) in v.char_indices() {
        match c {
            '(' => {
                depth += 1;
                start.get_or_insert(i);
            }
            ')' => depth -= 1,
            c if c.is_whitespace() && depth == 0 => {
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
fn time_ms(t: &str) -> Option<f64> {
    let t = t.trim().to_ascii_lowercase();
    if let Some(n) = t.strip_suffix("ms") {
        n.parse().ok()
    } else if let Some(n) = t.strip_suffix('s') {
        n.parse::<f64>().ok().map(|s| s * 1000.0)
    } else {
        None
    }
}

const TIMING_FUNCTIONS: &[&str] = &[
    "ease",
    "linear",
    "ease-in",
    "ease-out",
    "ease-in-out",
    "step-start",
    "step-end",
];

fn is_function(t: &str) -> bool {
    t.contains('(')
}

// --- Kurzschreibweisen --------------------------------------------------------

/// Je Übergang: Eigenschaft und Dauer.
fn transitions(decls: &[Declaration]) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    if let Some(v) = value(decls, "transition") {
        for single in list(v) {
            let mut property = None;
            let mut duration = None;
            for t in parts(single) {
                let tl = t.to_ascii_lowercase();
                if let Some(ms) = time_ms(&tl) {
                    duration.get_or_insert(ms);
                } else if !is_function(&tl)
                    && !TIMING_FUNCTIONS.contains(&tl.as_str())
                    && !matches!(tl.as_str(), "normal" | "allow-discrete")
                {
                    property.get_or_insert(tl);
                }
            }
            out.push((
                property.unwrap_or_else(|| "all".into()),
                duration.unwrap_or(0.0),
            ));
        }
    }
    let properties = value(decls, "transition-property").map(list);
    let last = value(decls, "transition-duration").map(|d| {
        list(d)
            .into_iter()
            .map(|t| time_ms(t).unwrap_or(0.0))
            .collect::<Vec<_>>()
    });
    match (properties, last) {
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

/// Der Name aus einer einzelnen `animation`-Kurzschreibweise.
///
/// Nach CSS Animations geht ein Schlüsselwort, das auch Name sein könnte,
/// zuerst an die Eigenschaft, die es noch nicht hat — erst was übrig bleibt,
/// ist der Name. Das zählt, weil der Browser die Kurzschreibweise voll
/// ausschreibt: `2s linear 0s infinite normal none running spin`. Dort ist
/// `none` der Füllmodus, `spin` der Name. Ohne Rest heißt die Animation
/// `none`.
fn short_name(single: &str) -> Option<String> {
    let (mut timing_function, mut count, mut direction, mut fill, mut state) =
        (false, false, false, false, false);
    let mut name = None;
    for t in parts(single) {
        let tl = t.to_ascii_lowercase();
        let tl = tl.as_str();
        if time_ms(tl).is_some() || matches!(tl, "initial" | "inherit" | "unset" | "revert") {
            continue;
        }
        if is_function(tl) || (!timing_function && TIMING_FUNCTIONS.contains(&tl)) {
            timing_function = true;
        } else if !count && (tl == "infinite" || tl.parse::<f64>().is_ok()) {
            count = true;
        } else if !direction
            && matches!(tl, "normal" | "reverse" | "alternate" | "alternate-reverse")
        {
            direction = true;
        } else if !fill && matches!(tl, "none" | "forwards" | "backwards" | "both") {
            fill = true;
        } else if !state && matches!(tl, "running" | "paused") {
            state = true;
        } else if name.is_none() {
            name = Some(t.trim_matches(['"', '\'']).to_string());
        }
    }
    name
}

/// Die Animationsnamen einer Regel, ohne `none`.
fn animation_names(decls: &[Declaration]) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(v) = value(decls, "animation-name") {
        names.extend(list(v).into_iter().map(String::from));
    } else if let Some(v) = value(decls, "animation") {
        names.extend(list(v).into_iter().filter_map(short_name));
    }
    names.retain(|n| !n.eq_ignore_ascii_case("none"));
    names
}

/// Größte Animationsdauer einer Regel in ms, wenn angegeben.
fn animation_duration(decls: &[Declaration]) -> Option<f64> {
    if let Some(v) = value(decls, "animation-duration") {
        return list(v).into_iter().filter_map(time_ms).reduce(f64::max);
    }
    let v = value(decls, "animation")?;
    list(v)
        .into_iter()
        .filter_map(|e| parts(e).into_iter().find_map(time_ms))
        .reduce(f64::max)
}

// --- focus/outline-removed ----------------------------------------------------

fn is_zero(v: &str) -> bool {
    matches!(v.trim(), "0" | "0px" | "0em" | "0rem")
}

fn removes_frame(decls: &[Declaration]) -> bool {
    let removed = |v: &str| {
        let v = v.trim().to_ascii_lowercase();
        v == "none" || is_zero(&v) || v.split_whitespace().any(|t| t == "none")
    };
    value(decls, "outline").is_some_and(removed)
        || value(decls, "outline-style").is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
        || value(decls, "outline-width").is_some_and(is_zero)
}

/// Ein sichtbarer Hinweis in derselben Regel: ein wieder gesetzter
/// Fokusrahmen, Rahmen, Schatten, Hintergrund oder Unterstreichung.
///
/// Der wieder gesetzte Rahmen fehlte in auditmysite. sueddeutsche.de
/// (2026-10-03) entfernt ihn mit `[data-whatinput] *:focus` für alle und
/// setzt ihn mit `[data-whatintent='keyboard'] *:focus { outline: 2px solid }`
/// für die Tastatur wieder.
fn replaces_frame(decls: &[Declaration]) -> bool {
    let frame_set = ["outline", "outline-style", "outline-width"]
        .iter()
        .filter_map(|n| value(decls, n))
        .any(|v| !matches!(v.trim(), "initial" | "unset" | "revert" | "inherit"))
        && !removes_frame(decls);
    let set = |v: &str| {
        let v = v.trim().to_ascii_lowercase();
        !v.is_empty() && v != "none" && !is_zero(&v) && v != "transparent" && v != "inherit"
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
            && set(&d.value)
    }) || frame_set
}

fn focus_selector(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    s.contains(":focus") && !s.contains(":focus-within") && !s.contains(":focus-visible")
}

/// `:focus:not(:focus-visible)` trifft nur den Fokus, den der Browser ohnehin
/// nicht anzeigt (Maus, Berührung); der Hinweis unter `:focus-visible` bleibt.
fn only_without_keyboard(s: &str) -> bool {
    s.to_ascii_lowercase()
        .split_whitespace()
        .collect::<String>()
        .contains(":not(:focus-visible)")
}

/// Ein Selector, unter dem das Entfernen des Rahmens den Fokushinweis trifft:
/// `:focus` und auch `:focus-visible` (F78), nicht aber `:focus-within` und
/// nicht `:focus:not(:focus-visible)`.
fn removes_focus(s: &str) -> bool {
    let k = s.to_ascii_lowercase();
    k.contains(":focus") && !k.contains(":focus-within") && !only_without_keyboard(s)
}

/// 2.4.7: Ein Stylesheet entfernt den Fokusrahmen unter `:focus`, ohne dass
/// es irgendwo einen Ersatz gibt.
///
/// Wie auditmysite seitenweit: Ein `:focus-visible` irgendwo, das den Rahmen
/// nicht selbst entfernt, oder ein Ersatz in irgendeiner `:focus`-Regel,
/// nimmt den Befund zurück — ein
/// globaler Reset mit `outline: none` und ein Hinweis per `box-shadow` in
/// einer anderen Regel sind das verbreitete Muster.
pub(crate) fn outline_removed<D: Document>(
    doc: &D,
    sheets: &[Stylesheet],
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    let rules: Vec<Scoped<'_>> = sheets.iter().flat_map(|s| s.style_rules()).collect();
    // Ein `:focus-visible`, das selbst den Rahmen entfernt, ist kein Ersatz,
    // sondern das Entfernen für die Tastatur (F78).
    let with_focus_visible = rules.iter().any(|r| {
        !removes_frame(r.declarations)
            && r.selectors.iter().any(|s| {
                s.to_ascii_lowercase().contains(":focus-visible") && !only_without_keyboard(s)
            })
    });
    let with_replacement = rules
        .iter()
        .any(|r| r.selectors.iter().any(|s| focus_selector(s)) && replaces_frame(r.declarations));
    if with_focus_visible || with_replacement {
        return;
    }
    let removed: Vec<&str> = rules
        .iter()
        .filter(|r| removes_frame(r.declarations))
        .flat_map(|r| r.selectors.iter().filter(|s| removes_focus(s)))
        .map(String::as_str)
        .collect();
    if removed.is_empty() {
        return;
    }
    let examples = removed
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
                "A stylesheet removes the focus outline ({examples}) and no rule replaces it; \
                 keyboard users cannot see where focus is.",
                "Ein Stylesheet entfernt den Fokusrahmen ({examples}), und keine Regel ersetzt \
                 ihn; wer mit der Tastatur bedient, sieht nicht, wo der Fokus steht."
            ),
        )
        .with_severity(Severity::High)
        .with_wcag(["2.4.7"])
        .at(page(doc)),
    );
}

// --- motion/reduced-motion-ignored --------------------------------------------

/// Bewegung im Sinne von 2.3.3: Lage, Verschiebung, Drehung, Skalierung.
/// Farbe und Deckkraft gehören nicht dazu.
fn is_motion(p: &str) -> bool {
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

const STATE: &[&str] = &[
    ":hover",
    ":focus",
    ":active",
    ":checked",
    ":target",
    ":focus-within",
    ":focus-visible",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    Normal,
    Reduced,
    Shielded,
}

fn position(media: &[&str]) -> Position {
    let mut l = Position::Normal;
    for m in media {
        let m = m.to_ascii_lowercase();
        if m.contains("prefers-reduced-motion") {
            l = if m.contains("no-preference") || m.split_whitespace().any(|w| w == "not") {
                Position::Shielded
            } else {
                Position::Reduced
            };
        }
    }
    l
}

/// Neutralisiert ab dieser Dauer (ms): die üblichen Resets `0.01ms`/`1ms`.
const NEUTRALISED_UNTIL_MS: f64 = 50.0;

fn selector_parts(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 2.3.3 (AAA): Bewegung durch Animation oder Übergang, die unter
/// `prefers-reduced-motion: reduce` nicht abgeschaltet wird.
///
/// Wie auditmysite (#712): Beleg ist nur Bewegung, die auf dieser Seite
/// laufen kann — der Selector muss ein Element im Dokument treffen
/// ([`style_cascade::selector`]). `transition: all` zählt nur, wenn eine
/// Zustandsregel eine Bewegungs-Eigenschaft setzt.
pub(crate) fn reduced_motion_ignored<D: Document>(
    doc: &D,
    sheets: &[Stylesheet],
    locale: Locale,
    out: &mut Vec<Finding>,
) {
    let rules: Vec<Scoped<'_>> = sheets.iter().flat_map(|s| s.style_rules()).collect();
    let keyframes: Vec<_> = sheets.iter().flat_map(|s| s.keyframes()).collect();
    let moving_keyframes = |name: &str| -> Option<String> {
        keyframes
            .iter()
            .filter(|(k, ctx)| k.name == name && position(&ctx.media) != Position::Shielded)
            .flat_map(|(k, _)| k.frames.iter())
            .flat_map(|f| f.declarations.iter())
            .find(|d| is_motion(&d.name))
            .map(|d| d.name.clone())
    };
    let state_moving = rules.iter().any(|r| {
        position(&r.context.media) == Position::Normal
            && r.selectors.iter().any(|s| {
                let s = s.to_ascii_lowercase();
                STATE.iter().any(|z| s.contains(z))
            })
            && r.declarations.iter().any(|d| is_motion(&d.name))
    });

    // Abschaltungen im Block `prefers-reduced-motion: reduce`.
    let switch_offs: Vec<(Vec<String>, bool, bool)> = rules
        .iter()
        .filter(|r| position(&r.context.media) == Position::Reduced)
        .map(|r| {
            let d = r.declarations;
            let animation_off = animation_names(d).is_empty()
                && (value(d, "animation-name")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
                    || value(d, "animation")
                        .is_some_and(|v| v.trim().eq_ignore_ascii_case("none")))
                || animation_duration(d).is_some_and(|ms| ms <= NEUTRALISED_UNTIL_MS)
                || value(d, "animation-play-state")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("paused"));
            let transition_off = value(d, "transition-property")
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
                || value(d, "transition").is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
                || value(d, "transition-duration").is_some_and(|v| {
                    list(v)
                        .into_iter()
                        .filter_map(time_ms)
                        .reduce(f64::max)
                        .is_some_and(|ms| ms <= NEUTRALISED_UNTIL_MS)
                })
                || {
                    let u = transitions(d);
                    !u.is_empty() && u.iter().all(|(_, ms)| *ms <= NEUTRALISED_UNTIL_MS)
                };
            (
                r.selectors.iter().map(|s| selector_parts(s)).collect(),
                animation_off,
                transition_off,
            )
        })
        .collect();
    let switched_off = |selector: &str, animation: bool| {
        let s = selector_parts(selector);
        switch_offs.iter().any(|(sel, a, u)| {
            (if animation { *a } else { *u }) && sel.iter().any(|x| x.contains('*') || *x == s)
        })
    };

    let mut evidences: Vec<String> = Vec::new();
    for r in rules
        .iter()
        .filter(|r| position(&r.context.media) == Position::Normal)
    {
        for selector in &r.selectors {
            let mut evidence = None;
            for name in animation_names(r.declarations) {
                if let Some(p) = moving_keyframes(&name) {
                    if !switched_off(selector, true) {
                        evidence = Some(format!("{selector}: @keyframes {name} ({p})"));
                    }
                    break;
                }
            }
            if evidence.is_none() {
                let moving = transitions(r.declarations)
                    .into_iter()
                    .filter(|(_, ms)| *ms > 0.0)
                    .find_map(|(p, _)| {
                        if is_motion(&p) {
                            Some(p)
                        } else if p == "all" && state_moving {
                            Some("all".to_string())
                        } else {
                            None
                        }
                    });
                if let Some(p) = moving
                    && !switched_off(selector, false)
                {
                    evidence = Some(format!("{selector}: transition {p}"));
                }
            }
            if let Some(b) = evidence
                && any_match(doc, selector, Mode::Possible) != Match::No
            {
                evidences.push(b);
            }
        }
    }
    if evidences.is_empty() {
        return;
    }
    let count = evidences.len();
    let examples = evidences
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
                "{count} style rules move content and are not switched off under \
                 prefers-reduced-motion: reduce ({examples}).",
                "{count} Stilregeln bewegen Inhalt und werden unter \
                 prefers-reduced-motion: reduce nicht abgeschaltet ({examples})."
            ),
        )
        .with_severity(Severity::Medium)
        .with_wcag(["2.3.3"])
        .at(page(doc)),
    );
}

// --- orientation/content-hidden ---------------------------------------------

fn whole_content(s: &str) -> bool {
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
    let mut evidences: Vec<(String, String)> = Vec::new();
    let mut whole = false;
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
        let hidden = value(d, "display").is_some_and(|v| v.trim().eq_ignore_ascii_case("none"))
            || value(d, "visibility").is_some_and(|v| v.trim().eq_ignore_ascii_case("hidden"));
        let rotated = value(d, "transform")
            .is_some_and(|v| v.to_ascii_lowercase().contains("rotate"))
            || value(d, "rotate").is_some_and(|v| !v.trim().eq_ignore_ascii_case("none"));
        if !hidden && !rotated {
            continue;
        }
        for sel in &r.selectors {
            if sel.contains("::")
                || sel.to_ascii_lowercase().contains(":before")
                || sel.to_ascii_lowercase().contains(":after")
                || any_match(doc, sel, Mode::Possible) == Match::No
            {
                continue;
            }
            whole |= whole_content(sel);
            evidences.push((sel.clone(), media.to_string()));
        }
    }
    if evidences.is_empty() {
        return;
    }
    let count = evidences.len();
    let examples = evidences
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
                "{count} style rules hide or rotate content depending on the orientation \
                 ({examples}). Check that the content is available in portrait and landscape.",
                "{count} Stilregeln blenden je nach Ausrichtung Inhalt aus oder drehen ihn \
                 ({examples}). Prüfen, ob der Inhalt hoch- und querformatig erreichbar ist."
            ),
        )
        .with_severity(if whole {
            Severity::High
        } else {
            Severity::Medium
        })
        .with_wcag(["1.3.4"])
        .at(page(doc)),
    );
}

// --- text/justified, text/line-height-tight -----------------------------------

/// Die Angabe einer Eigenschaft im `style`-Attribut, samt `!important`.
fn in_style_attribute<'a, N: Node<'a>>(n: N, property: &str) -> Option<(String, bool)> {
    n.attr("style")?
        .split(';')
        .rev()
        .filter_map(|d| d.split_once(':'))
        .filter(|(k, _)| k.trim().eq_ignore_ascii_case(property))
        .map(|(_, v)| {
            let v = v.trim();
            match v.to_ascii_lowercase().strip_suffix("!important") {
                Some(without) => (v[..without.len()].trim().to_string(), true),
                None => (v.to_string(), false),
            }
        })
        .next()
}

/// Der wirksame Wert einer vererbten Eigenschaft an einem Element: die
/// letzte treffende Angabe am Element selbst, sonst am nächsten Vorfahren.
/// Spezifität bleibt außen vor; `!important` gewinnt. Das `style`-Attribut
/// schlägt jede Regel eines Stylesheets ohne `!important` (CSS Cascade 4,
/// „Cascade Sorting Order": Stilattribut vor Selektoren).
fn effective<'a, N: Node<'a>>(
    n: N,
    property: &str,
    rules: &[(Selector, String, bool)],
) -> Option<String> {
    for e in std::iter::once(n).chain(a11y_dom::ancestors(n)) {
        let fitting = rules
            .iter()
            .filter(|(sel, _, _)| sel.matches(e, Mode::Possible) == Match::Yes)
            .max_by_key(|(_, _, important)| *important);
        let attribute = in_style_attribute(e, property);
        match (attribute, fitting) {
            (Some((v, true)), _) => return Some(v),
            (_, Some((_, v, true))) => return Some(v.clone()),
            (Some((v, false)), _) => return Some(v),
            (None, Some((_, v, _))) => return Some(v.clone()),
            (None, None) => {}
        }
    }
    None
}

fn rules_for(sheets: &[Stylesheet], property: &str) -> Vec<(Selector, String, bool)> {
    sheets
        .iter()
        .flat_map(|s| s.style_rules())
        .filter(|r| r.context.media.is_empty())
        .flat_map(|r| {
            r.declarations
                .iter()
                .filter(|d| d.name == property)
                .map(|d| {
                    (
                        Selector::parse(&r.selectors.join(", ")),
                        d.value.clone(),
                        d.important,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Zeilenhöhe als Faktor der Schriftgröße, wenn ohne Schriftgröße bestimmbar.
fn line_factor(v: &str) -> Option<f64> {
    let v = v.trim().to_ascii_lowercase();
    if let Some(p) = v.strip_suffix('%') {
        return p.parse::<f64>().ok().map(|p| p / 100.0);
    }
    v.strip_suffix("em").unwrap_or(&v).parse().ok()
}

/// 1.4.8 (AAA): Absätze im Blocksatz oder mit Zeilenhöhe unter 1,5.
///
/// Anders als auditmysite, das jede Regel mit einem Selector ab `body`, `p`,
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
    let paragraphs: Vec<_> = a11y_dom::elements(doc)
        .filter(|n| n.is_element("p") && a11y_dom::has_text(*n))
        .collect();
    if paragraphs.is_empty() {
        return;
    }
    let alignment = rules_for(sheets, "text-align");
    let rows = rules_for(sheets, "line-height");
    let justified_text = paragraphs
        .iter()
        .filter(|p| {
            effective(**p, "text-align", &alignment)
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("justify"))
        })
        .count();
    let eng: Vec<String> = paragraphs
        .iter()
        .filter_map(|p| effective(*p, "line-height", &rows))
        .filter(|v| line_factor(v).is_some_and(|z| z > 0.0 && z < 1.5))
        .collect();
    if justified_text > 0 {
        out.push(
            Finding::review(
                "text/justified",
                tr!(
                    locale,
                    "{justified_text} paragraphs are set justified; uneven word spacing makes them \
                     harder to read.",
                    "{justified_text} Absätze stehen im Blocksatz; ungleiche Wortabstände erschweren \
                     das Lesen."
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.4.8"])
            .at(page(doc)),
        );
    }
    if let Some(lh) = eng.first() {
        let count = eng.len();
        out.push(
            Finding::review(
                "text/line-height-tight",
                tr!(
                    locale,
                    "{count} paragraphs have a line height below 1.5 (for example {lh}).",
                    "{count} Absätze haben eine Zeilenhöhe unter 1,5 (etwa {lh})."
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["1.4.8"])
            .at(page(doc)),
        );
    }
}

// --- Registrierung ------------------------------------------------------------

use a11y_dom::Tier;

use crate::registry::{Meta, StylesheetRule};
use crate::visibility::Scope;

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
        help_de: de_text!(
            "Einen sichtbaren Fokushinweis behalten: den Rahmen unter :focus nicht ohne \
                  Ersatz entfernen, oder das Entfernen auf :focus:not(:focus-visible) \
                  beschränken."
        ),
    },
    Meta {
        ids: &["motion/reduced-motion-ignored"],
        tier: Tier::Stylesheets,
        scope: Scope::Rendered,
        wcag: &["2.3.3"],
        severity: Severity::Medium,
        help: "Switch off movement under @media (prefers-reduced-motion: reduce).",
        help_de: de_text!("Bewegung unter @media (prefers-reduced-motion: reduce) abschalten."),
    },
    Meta {
        ids: &["orientation/content-hidden"],
        tier: Tier::Stylesheets,
        scope: Scope::Rendered,
        wcag: &["1.3.4"],
        severity: Severity::Medium,
        help: "Content must work in portrait and landscape; do not hide or rotate it per \
               orientation.",
        help_de: de_text!(
            "Inhalt muss hoch- und querformatig funktionieren; ihn nicht je nach \
                  Ausrichtung ausblenden oder drehen."
        ),
    },
    Meta {
        ids: &["text/justified", "text/line-height-tight"],
        tier: Tier::Stylesheets,
        scope: Scope::AccessibilityTree,
        wcag: &["1.4.8"],
        severity: Severity::Low,
        help: "Running text: no justified alignment, line height at least 1.5.",
        help_de: de_text!("Fließtext: kein Blocksatz, Zeilenhöhe mindestens 1,5."),
    },
];

pub(crate) fn rules<D: Document>() -> Vec<StylesheetRule<D>> {
    type Run<D> = fn(&D, &[Stylesheet], Locale, &mut Vec<Finding>);
    let functions: [Run<D>; 4] = [
        outline_removed,
        reduced_motion_ignored,
        orientation_content_hidden,
        visual_presentation,
    ];
    METAS
        .iter()
        .zip(functions)
        .map(|(meta, run)| StylesheetRule { meta: *meta, run })
        .collect()
}
