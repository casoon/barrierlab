//! Regeln über Messungen, die ein Host an der laufenden Seite vornimmt
//! (casoon/barrierlab#130): Rahmen, Drehung der Wurzel, Einfügen in
//! Passwortfelder, Fokus unter fixierten Elementen, Textabstand.
//!
//! Jede dieser Regeln liest ein Feld, das ein Host liefern *kann* und das die
//! Vorgabe von [`Rendering`] mit `None` („nicht gemessen") beantwortet. Fehlt
//! die Messung an allen Elementen, auf die die Regel zielt, entsteht ein
//! Vermerk `NotRun::CapabilityMissing` mit Grund (`crate::not_run`), nie ein
//! stilles Bestehen. Gibt es nichts, worauf die Regel zielt, lief sie und
//! meldet nichts.

use a11y_dom::{FocusCover, Node, NodeId, NodeKind, Rendering, Tier, elements};
use a11y_report::{Evidence, Finding, Location, Severity};

use crate::locale::{Locale, de_text, pick, tr};
use crate::not_run;
use crate::registry::Meta;
use crate::structure::per_tab_reachable;
use crate::visibility::Scope;

fn at(id: NodeId) -> Location {
    Location::node(id.to_string())
}

// --- frames/cross-origin ------------------------------------------------------

/// Ob ein Rahmen im Sinne der Abdeckungsnotiz „dargestellt" ist: nicht
/// präsentational, nicht versteckt, größer als 1 px in beiden Richtungen
/// (auditmysite-Umfang).
fn frame_in_scope<'a, D: Rendering>(doc: &'a D, n: D::N<'a>) -> bool {
    let role = n
        .attr("role")
        .and_then(|r| r.split_whitespace().next())
        .map(str::to_ascii_lowercase);
    if matches!(role.as_deref(), Some("none" | "presentation"))
        || n.has_attr("hidden")
        || n.attr("aria-hidden")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
    {
        return false;
    }
    if let Some(style) = doc.computed_style(n)
        && (style.display.as_deref() == Some("none")
            || matches!(style.visibility.as_deref(), Some("hidden" | "collapse")))
    {
        return false;
    }
    !doc.bounds(n)
        .is_some_and(|b| b.width <= 1.0 || b.height <= 1.0)
}

/// Kein Kriterium: ein Dokument, das das Werkzeug nicht lesen kann, lässt
/// sich an keinem messen. Das ist eine Abdeckungsnotiz, keine Konformitätsprüfung.
fn frames<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut measured = false;
    let mut candidates = false;
    for n in elements(doc).filter(|n| matches!(n.local_name(), "iframe" | "frame")) {
        if !frame_in_scope(doc, n) {
            continue;
        }
        candidates = true;
        match doc.frame_reachable(n) {
            None => {}
            Some(true) => measured = true,
            Some(false) => {
                measured = true;
                let src = n.attr("src").unwrap_or("");
                out.push(
                    Finding::untested(
                        "frames/cross-origin",
                        tr!(
                            locale,
                            "The document in this frame (src: {src}) cannot be read, so it was \
                             not tested. It is cross-origin, or its navigation had not finished. \
                             Check it by hand or test its address on its own.",
                            "Das Dokument in diesem Rahmen (src: {src}) ist nicht lesbar und \
                             blieb deshalb ungeprüft. Es hat einen fremden Ursprung, oder seine \
                             Navigation war nicht abgeschlossen. Von Hand prüfen oder die \
                             Adresse für sich prüfen."
                        ),
                    )
                    .with_severity(Severity::Low)
                    .with_evidence([Evidence::dom_attribute(
                        "src",
                        n.attr("src").map(str::to_string),
                    )])
                    .at(at(n.id())),
                );
            }
        }
    }
    if candidates && !measured {
        out.push(not_run::note(
            "frames/cross-origin",
            pick!(
                locale,
                "host does not report frame access",
                "Host meldet den Zugriff auf Rahmen nicht",
            ),
        ));
    }
}

// --- orientation/root-rotated -------------------------------------------------

/// Ob eine berechnete `transform`-Matrix um eine Vierteldrehung dreht:
/// `b = sin θ`, `c = −sin θ` (bei `matrix3d` Eintrag 2 und 5 der
/// Spaltenfolge). Neigung und Skalierung allein ergeben dieses Paar nicht.
/// `None`: nicht auswertbar (weder `none` noch eine Matrix).
fn matrix_rotates(value: &str) -> Option<bool> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Some(false);
    }
    let (inner, b, c) = if let Some(rest) = value.strip_prefix("matrix3d(") {
        (rest, 1, 4)
    } else {
        (value.strip_prefix("matrix(")?, 1, 2)
    };
    let numbers: Vec<f64> = inner
        .strip_suffix(')')?
        .split(',')
        .map(|v| v.trim().parse().ok())
        .collect::<Option<_>>()?;
    let (b, c) = (*numbers.get(b)?, *numbers.get(c)?);
    Some(b.abs() > 0.01 && (b + c).abs() < 0.01)
}

/// Ob die Eigenschaft `rotate` um einen Winkel abseits eines Vielfachen von
/// 180° um die Z-Achse dreht. Achsen in X oder Y kippen und drehen nicht in
/// der Bildebene. `None`: nicht auswertbar.
fn rotate_turns(value: &str) -> Option<bool> {
    let tokens: Vec<&str> = value.split_whitespace().collect();
    let (angle, axis) = match tokens.as_slice() {
        ["none"] => return Some(false),
        [angle] => (*angle, true),
        ["z", angle] => (*angle, true),
        ["x" | "y", angle] => (*angle, false),
        [x, y, z, angle] => {
            let axis: Vec<f64> = [x, y, z]
                .iter()
                .map(|v| v.parse().ok())
                .collect::<Option<_>>()?;
            (*angle, axis[0] == 0.0 && axis[1] == 0.0 && axis[2] > 0.0)
        }
        _ => return None,
    };
    let degrees = if let Some(v) = angle.strip_suffix("deg") {
        v.parse::<f64>().ok()?
    } else if let Some(v) = angle.strip_suffix("grad") {
        v.parse::<f64>().ok()? * 0.9
    } else if let Some(v) = angle.strip_suffix("rad") {
        v.parse::<f64>().ok()?.to_degrees()
    } else if let Some(v) = angle.strip_suffix("turn") {
        v.parse::<f64>().ok()? * 360.0
    } else {
        return None;
    };
    let off = degrees.rem_euclid(180.0);
    Some(axis && off > 0.01 && off < 179.99)
}

/// 1.3.4: Die Seite dreht ihre Wurzel, um nur eine Ausrichtung nutzbar zu
/// machen. Sichtbar ist das an `html` und `body`; ob die Ausrichtung
/// wesentlich ist, entscheidet ein Mensch.
fn root_rotated<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut roots = false;
    let mut measured = false;
    for n in elements(doc).filter(|n| matches!(n.local_name(), "html" | "body")) {
        roots = true;
        let Some(value) = doc.box_transform(n) else {
            continue;
        };
        let by_matrix = value.transform.as_deref().and_then(matrix_rotates);
        let by_angle = value.rotate.as_deref().and_then(rotate_turns);
        measured |= by_matrix.is_some() || by_angle.is_some();
        if by_matrix != Some(true) && by_angle != Some(true) {
            continue;
        }
        let tag = n.local_name();
        out.push(
            Finding::fail(
                "orientation/root-rotated",
                tr!(
                    locale,
                    "The <{tag}> element is rotated, which can force a single display                      orientation. Unless a specific orientation is essential, content must not                      restrict its view and operation to one.",
                    "Das Element <{tag}> ist gedreht, was eine einzige Bildschirmausrichtung                      erzwingen kann. Außer eine bestimmte Ausrichtung ist wesentlich, darf der                      Inhalt Ansicht und Bedienung nicht auf eine beschränken."
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["1.3.4"])
            .with_evidence(
                [
                    ("transform", &value.transform),
                    ("rotate", &value.rotate),
                ]
                .into_iter()
                .filter_map(|(field, v)| v.as_deref().map(|v| Evidence::computed(field, v))),
            )
            .at(at(n.id())),
        );
    }
    if roots && !measured {
        out.push(not_run::note(
            "orientation/root-rotated",
            pick!(
                locale,
                "host does not report transform or rotate of html and body",
                "Host meldet transform und rotate von html und body nicht",
            ),
        ));
    }
}

// --- text-spacing/clipped -----------------------------------------------------

/// 1.4.12: Der Host hat den Textabstand auf die Mindestwerte des Kriteriums
/// gesetzt und je Element gemessen, ob es seinen Text abschneidet. Gemeldet
/// wird, was *neu* abgeschnitten ist (`after && !before`); schon vorher
/// Abgeschnittenes liegt nicht am Textabstand. Überlappung und
/// verschwindender Inhalt sieht die Regel nicht, deshalb `REVIEW`.
fn text_spacing_clipped<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut candidates = false;
    let mut measured = false;
    for n in elements(doc).filter(|n| {
        n.children()
            .any(|k| k.kind() == NodeKind::Text && !k.text().trim().is_empty())
    }) {
        // Ein dauerhaftes Clip eines Kastens von höchstens 2 px ist kein
        // Befund (das übliche „nur für Screenreader").
        if doc
            .bounds(n)
            .is_some_and(|b| b.width <= 2.0 || b.height <= 2.0)
        {
            continue;
        }
        candidates = true;
        let Some(clip) = doc.text_spacing_clip(n) else {
            continue;
        };
        measured = true;
        if !(clip.after && !clip.before) {
            continue;
        }
        out.push(
            Finding::review(
                "text-spacing/clipped",
                pick!(
                    locale,
                    "With the text spacing of WCAG 1.4.12 applied (line height 1.5, paragraph \
                     spacing 2, letter spacing 0.12, word spacing 0.16), this element cuts off \
                     its own text, which it did not before. Check by hand that no content or \
                     function is lost.",
                    "Mit dem Textabstand aus WCAG 1.4.12 (Zeilenhöhe 1,5, Absatzabstand 2, \
                     Zeichenabstand 0,12, Wortabstand 0,16) schneidet dieses Element seinen \
                     eigenen Text ab, was es vorher nicht tat. Von Hand prüfen, dass kein \
                     Inhalt und keine Funktion verloren geht.",
                ),
            )
            .with_severity(Severity::Medium)
            .with_wcag(["1.4.12"])
            .with_evidence([
                Evidence::computed("clipped_before", "false"),
                Evidence::computed("clipped_after", "true"),
            ])
            .at(at(n.id())),
        );
    }
    if candidates && !measured {
        out.push(not_run::note(
            "text-spacing/clipped",
            pick!(
                locale,
                "host did not apply the text-spacing override (no text_spacing_clip)",
                "Host hat den Textabstand nicht überschrieben (kein text_spacing_clip)",
            ),
        ));
    }
}

// --- focus/obscured-enhanced --------------------------------------------------

pub(crate) fn cover_evidence(c: FocusCover) -> Vec<Evidence> {
    let mut e = vec![
        Evidence::computed("focus_points", c.points.to_string()),
        Evidence::computed("focus_covered", c.covered.to_string()),
    ];
    if let Some(b) = c.blocker {
        e.push(Evidence::computed("focus_blocker", b.to_string()));
    }
    e
}

/// 2.4.12 (AAA): kein Teil des fokussierten Bedienelements ist verdeckt. Die
/// Abtastung sieht „irgendein Punkt überdeckt"; ob das ein Teil im Sinne des
/// Kriteriums ist, entscheidet ein Mensch — `REVIEW`. Dieselbe Messung wie bei
/// `focus/obscured`, andere Schwelle.
fn focus_obscured_enhanced<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut candidates = false;
    let mut measured = false;
    for n in
        elements(doc).filter(|n| per_tab_reachable(*n) && doc.visually_hidden(*n) != Some(true))
    {
        candidates = true;
        let Some(c) = doc.focus_cover(n) else {
            continue;
        };
        measured = true;
        if c.covered == 0 {
            continue;
        }
        out.push(
            Finding::review(
                "focus/obscured-enhanced",
                tr!(
                    locale,
                    "After receiving focus, {} of {} sample points of this control lie under a \
                     fixed or sticky element. Enhanced (AAA): no part of a focused control may \
                     be hidden. Check it by hand.",
                    "Nach dem Fokussieren liegen {} von {} Abtastpunkten dieses Bedienelements \
                     unter einem fixierten oder klebenden Element. Erhöht (AAA): kein Teil \
                     eines fokussierten Bedienelements darf verdeckt sein. Von Hand prüfen.",
                    c.covered,
                    c.points,
                ),
            )
            .with_severity(Severity::Low)
            .with_wcag(["2.4.12"])
            .with_evidence(cover_evidence(c))
            .at(at(n.id())),
        );
    }
    if candidates && !measured {
        out.push(not_run::note(
            "focus/obscured-enhanced",
            pick!(
                locale,
                "host did not run a focus pass (no focus_cover)",
                "Host hat keinen Fokuslauf gemacht (kein focus_cover)",
            ),
        ));
    }
}

// --- auth/paste-blocked -------------------------------------------------------

/// Ein Feld, auf das 3.3.8 zielt und das der Host auf Einfügen prüfen soll:
/// ein Passwortfeld oder ein Feld mit `autocomplete` `one-time-code` oder
/// `current-password`, sichtbar, nicht gesperrt, nicht schreibgeschützt.
/// `autocomplete="off"` oder fehlendes `autocomplete` macht ein Passwortfeld
/// weder zum Befund noch zur Ausnahme — Browser ignorieren es.
fn is_paste_candidate<'a, D: Rendering>(doc: &'a D, n: D::N<'a>) -> bool {
    if !n.is_element("input") || n.has_attr("disabled") || n.has_attr("readonly") {
        return false;
    }
    let kind = n
        .attr("type")
        .map(|t| t.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(
        kind.as_str(),
        "hidden"
            | "checkbox"
            | "radio"
            | "file"
            | "submit"
            | "button"
            | "reset"
            | "image"
            | "range"
            | "color"
    ) {
        return false;
    }
    let token = |wanted: &str| {
        n.attr("autocomplete")
            .is_some_and(|a| a.split_whitespace().any(|t| t.eq_ignore_ascii_case(wanted)))
    };
    (kind == "password" || token("one-time-code") || token("current-password"))
        && !doc.bounds(n).is_some_and(|b| b.is_empty())
}

/// 3.3.8: Einfügen in ein Passwort- oder Codefeld ist der Mechanismus, mit dem
/// ein Passwortmanager hilft. Blockiert ist es, wenn ein Handler das
/// Ereignis abbricht und den Text nicht selbst einfügt.
fn paste_blocked<D: Rendering>(doc: &D, locale: Locale, out: &mut Vec<Finding>) {
    let mut candidates = false;
    let mut measured = false;
    for n in elements(doc).filter(|n| is_paste_candidate(doc, *n)) {
        candidates = true;
        let Some(probe) = doc.paste_probe(n) else {
            continue;
        };
        measured = true;
        if !(probe.cancelled && !probe.inserted) {
            continue;
        }
        out.push(
            Finding::fail(
                "auth/paste-blocked",
                pick!(
                    locale,
                    "Pasting into this field is blocked: a handler cancels the paste event and \
                     does not insert the text itself. A password manager or paste is the \
                     mechanism that keeps remembering a password from being required.",
                    "Einfügen in dieses Feld ist blockiert: Ein Handler bricht das \
                     paste-Ereignis ab und fügt den Text nicht selbst ein. Ein \
                     Passwortmanager oder Einfügen ist der Mechanismus, der verhindert, dass \
                     man sich ein Passwort merken muss.",
                ),
            )
            .with_severity(Severity::High)
            .with_wcag(["3.3.8"])
            .with_evidence([
                Evidence::computed("paste_cancelled", "true"),
                Evidence::computed("paste_inserted", "false"),
            ])
            .at(at(n.id())),
        );
    }
    if candidates && !measured {
        out.push(not_run::note(
            "auth/paste-blocked",
            pick!(
                locale,
                "host did not probe paste",
                "Host hat das Einfügen nicht untersucht",
            ),
        ));
    }
}

pub(crate) const METAS: &[Meta] = &[
    Meta {
        ids: &["text-spacing/clipped"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["1.4.12"],
        severity: Severity::Medium,
        help: "No content or function may be lost when line height is at least 1.5, paragraph \
               spacing 2, letter spacing 0.12 and word spacing 0.16 times the font size. Seen: \
               text that hidden overflow newly cuts off after the host applied that override \
               (Rendering::text_spacing_clip, a before/after pair; already clipped boxes are not \
               findings). Overlapping text and content that disappears in other ways are not \
               detected, so the result is a review. If the host could not apply the override (for \
               example blocked by a CSP style-src), the rule does not run.",
        help_de: de_text!(
            "Bei einer Zeilenhöhe von mindestens 1,5, einem Absatzabstand von 2, einem \
                  Zeichenabstand von 0,12 und einem Wortabstand von 0,16 der Schriftgröße darf \
                  kein Inhalt und keine Funktion verloren gehen. Gesehen wird Text, den \
                  verstecktes Overflow nach dem Überschreiben durch den Host neu abschneidet \
                  (Rendering::text_spacing_clip, ein Vorher/Nachher-Paar; schon vorher \
                  abgeschnittene Kästen sind kein Befund). Überlappender Text und anderweitig \
                  verschwindender Inhalt werden nicht erkannt, das Ergebnis ist deshalb ein \
                  Review. Konnte der Host das Überschreiben nicht anwenden (etwa durch eine CSP \
                  style-src), läuft die Regel nicht."
        ),
    },
    Meta {
        ids: &["focus/obscured-enhanced"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["2.4.12"],
        severity: Severity::Low,
        help: "Enhanced (AAA): no part of a focused control may be hidden by fixed or sticky \
               content. Seen: Rendering::focus_cover, sample points under such an element after \
               the host focused and scrolled the control; any covered point is reported. \
               Sampling approximates \"any part\", so the result is a review. Without a focus \
               pass the rule does not run: static geometry is not a probe.",
        help_de: de_text!(
            "Erhöht (AAA): Kein Teil eines fokussierten Bedienelements darf von fixiertem oder \
                  klebendem Inhalt verdeckt sein. Gesehen wird Rendering::focus_cover, \
                  Abtastpunkte unter einem solchen Element, nachdem der Host das Bedienelement \
                  fokussiert und gescrollt hat; jeder überdeckte Punkt wird gemeldet. Die \
                  Abtastung nähert \"irgendein Teil\" nur an, das Ergebnis ist ein Review. Ohne \
                  Fokuslauf läuft die Regel nicht: statische Geometrie ist keine Messung."
        ),
    },
    Meta {
        ids: &["auth/paste-blocked"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["3.3.8"],
        severity: Severity::High,
        help: "A password or one-time-code field must accept pasted text, so that a password \
               manager can fill it: a handler that cancels the paste event without inserting \
               the text itself blocks it. Seen: a synthetic, cancelable paste event the host \
               dispatched; it is not trusted (isTrusted is false), so a handler that reacts \
               only to trusted events or to key presses is not seen. autocomplete=\"off\" on a \
               password field is not a finding: browsers ignore it.",
        help_de: de_text!(
            "Ein Passwort- oder Einmalcode-Feld muss eingefügten Text annehmen, damit ein \
                  Passwortmanager es füllen kann: Ein Handler, der das paste-Ereignis abbricht, \
                  ohne den Text selbst einzufügen, blockiert das. Gesehen wird ein synthetisches, \
                  abbrechbares paste-Ereignis des Hosts; es ist nicht vertrauenswürdig \
                  (isTrusted ist false), ein Handler, der nur auf vertrauenswürdige Ereignisse \
                  oder Tastendrücke reagiert, bleibt unsichtbar. autocomplete=\"off\" an einem \
                  Passwortfeld ist kein Befund: Browser ignorieren es."
        ),
    },
    Meta {
        ids: &["orientation/root-rotated"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &["1.3.4"],
        severity: Severity::High,
        help: "A page that rotates its root (html or body) by a quarter turn to force one \
               display orientation fails 1.3.4. Seen: the computed transform matrix and the \
               rotate property of html and body, in the one viewport the host measured; a lock \
               that applies only in the other orientation needs a second snapshot there \
               (RuleRun::in_viewport). That an orientation is essential is a human judgement.",
        help_de: de_text!(
            "Eine Seite, die ihre Wurzel (html oder body) um eine Vierteldrehung dreht, um \
                  eine Ausrichtung zu erzwingen, verfehlt 1.3.4. Gesehen werden die berechnete \
                  transform-Matrix und die Eigenschaft rotate von html und body, in dem einen \
                  Viewport, den der Host gemessen hat; eine Sperre nur in der anderen \
                  Ausrichtung braucht dort einen zweiten Schnappschuss (RuleRun::in_viewport). \
                  Ob eine Ausrichtung wesentlich ist, entscheidet ein Mensch."
        ),
    },
    Meta {
        ids: &["frames/cross-origin"],
        tier: Tier::Rendering,
        scope: Scope::Rendered,
        wcag: &[],
        severity: Severity::Low,
        help: "A frame whose document the tool cannot read (cross-origin, or still navigating) \
           was not tested. Not a conformance check and tied to no success criterion: a \
           coverage note. Frames that are presentational, hidden or 1 px or smaller are \
           skipped.",
        help_de: de_text!(
            "Ein Rahmen, dessen Dokument das Werkzeug nicht lesen kann (fremder Ursprung oder \
              Navigation läuft noch), blieb ungeprüft. Keine Konformitätsprüfung und an kein \
              Erfolgskriterium gebunden: eine Abdeckungsnotiz. Präsentationale, versteckte und \
              höchstens 1 px große Rahmen bleiben außen vor."
        ),
    },
];

/// Die Auswertungsfunktionen, in derselben Reihenfolge wie [`METAS`].
pub(crate) fn functions<D: Rendering>() -> [fn(&D, Locale, &mut Vec<Finding>); 5] {
    [
        text_spacing_clipped,
        focus_obscured_enhanced,
        paste_blocked,
        root_rotated,
        frames,
    ]
}

const _: () = assert!(METAS.len() == 5);
