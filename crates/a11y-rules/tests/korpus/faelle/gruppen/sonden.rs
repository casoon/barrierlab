//! Gruppe „sonden“: Regeln über Messungen, die ein Host an der laufenden
//! Seite vornimmt (`probes.rs`, `heuristics.rs` für `focus/obscured`).
//!
//! Die Dokumente tragen die Werte des Hosts als eigene Eigenschaften im
//! `style`-Attribut (siehe `host.rs`). Die Vorlagen stammen aus dem
//! auditmysite-Korpus `tests/fixtures/detection_corpus/` (nur gelesen) und
//! sind hier als eigene, kleine Dokumente nachgestellt.

use super::super::{Fall, fail, na, negative, not_run, untested};

const FRAME_FIXTURE: &str = "auditmysite tests/fixtures/detection_corpus/content_frames_cross_origin (frame-tested): booking, small-map, shadow-calendar and late-reviews are reported; pixel (1x1), collapsed (display:none), silent (aria-hidden) and local-help (readable) are not";
const FRAME_NORM: &str = "no WCAG criterion applies to a document the tool cannot read (coverage note); host contract of Rendering::frame_reachable, casoon/barrierlab#132";

pub fn cases() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(frames());
    v.extend(orientation());
    v.extend(paste());
    v
}

fn frames() -> Vec<Fall> {
    let id = "frames/cross-origin";
    vec![
        untested(
            id,
            r#"<iframe id="booking" title="Room booking" src="https://other.example/booking" style="--w:400px;--h:200px;--frame-reachable:0"></iframe>
<iframe id="small-map" title="Location map" src="https://other.example/map" style="--w:2px;--h:2px;--frame-reachable:0"></iframe>
<iframe id="pixel" title="Visitor counter" src="https://other.example/pixel" style="--w:1px;--h:1px;--frame-reachable:0"></iframe>
<iframe id="collapsed" title="Chat" src="https://other.example/chat" style="display:none;--frame-reachable:0"></iframe>
<iframe id="silent" title="Analytics" src="https://other.example/analytics" aria-hidden="true" style="--w:300px;--h:100px;--frame-reachable:0"></iframe>
<iframe id="local-help" title="Booking help" style="--w:400px;--h:120px"></iframe>
<booking-widget><template shadowrootmode="open"><iframe id="shadow-calendar" title="Room calendar" src="https://other.example/calendar" style="--w:300px;--h:150px;--frame-reachable:0"></iframe></template></booking-widget>
<div id="late-frame"><iframe id="late-reviews" title="Reviews" src="https://other.example/reviews" style="--w:300px;--h:150px;--frame-reachable:0"></iframe></div>"#,
            FRAME_FIXTURE,
        )
        .variant("Fixture nachgestellt: vier gemeldet, vier nicht, ein Rahmen im Shadow Root")
        .count(4),
        untested(
            id,
            r#"<!DOCTYPE html><html lang="en"><head><title>T</title></head><frameset><frame src="https://other.example/nav" style="--w:200px;--h:300px;--frame-reachable:0"></frameset></html>"#,
            FRAME_NORM,
        )
        .variant("<frame> statt <iframe>"),
        negative(
            id,
            r#"<iframe title="Booking help" style="--w:400px;--h:120px"></iframe>"#,
            "auditmysite content_frames_cross_origin: local-help (srcdoc) is readable and not reported",
        )
        .variant("lesbarer Rahmen"),
        na(
            id,
            r#"<iframe title="Counter" src="https://other.example/pixel" style="--w:1px;--h:1px;--frame-reachable:0"></iframe>"#,
            FRAME_FIXTURE,
        )
        .variant("Grenze: 1 px ist ausgenommen"),
        na(
            id,
            r#"<iframe title="Chat" src="https://other.example/chat" style="display:none;--frame-reachable:0"></iframe>"#,
            FRAME_FIXTURE,
        )
        .variant("display: none"),
        na(
            id,
            r#"<iframe title="Analytics" src="https://other.example/a" aria-hidden="true" style="--w:300px;--h:100px;--frame-reachable:0"></iframe>"#,
            FRAME_FIXTURE,
        )
        .variant("aria-hidden"),
        na(
            id,
            r#"<iframe src="https://other.example/a" role="presentation" style="--w:300px;--h:100px;--frame-reachable:0"></iframe>"#,
            FRAME_NORM,
        )
        .variant("role=presentation"),
        na(
            id,
            r#"<iframe src="https://other.example/a" hidden style="--w:300px;--h:100px;--frame-reachable:0"></iframe>"#,
            FRAME_NORM,
        )
        .variant("hidden"),
        na(id, r#"<p>No frames here.</p>"#, FRAME_NORM),
        not_run(
            id,
            r#"<!DOCTYPE html><html lang="en" style="--unmeasured: frames"><head><title>T</title></head><body><iframe title="Booking" src="https://other.example/booking" style="--w:400px;--h:200px"></iframe></body></html>"#,
            "docs/conventions.md: not measured is not passed; issue #132: None for every frame yields NotRun",
        ),
    ]
}

const ORIENT_FIXTURES: &str = "auditmysite tests/fixtures/detection_corpus/visual_orientation_{lock,script_lock,ok,hide}: computed transform of html and body (css-orientation-lock)";
const ORIENT_NORM: &str = "WCAG 2.2 SC 1.3.4 Orientation: content does not restrict its view and operation to a single display orientation; a quarter-turn rotation of the root is such a restriction";

/// Ein vollständiges Dokument, dessen `html`- und `body`-Element die
/// berechneten Werte des Hosts tragen.
macro_rules! root {
    ($html:literal, $body:literal) => {
        concat!(
            r#"<!DOCTYPE html><html lang="en" style=""#,
            $html,
            r#""><head><title>T</title></head><body style=""#,
            $body,
            r#""><p>Content</p></body></html>"#
        )
    };
}

fn orientation() -> Vec<Fall> {
    let id = "orientation/root-rotated";
    vec![
        fail(
            id,
            root!("", "transform: matrix(6.12323e-17, 1, -1, 6.12323e-17, 0, 0)"),
            ORIENT_FIXTURES,
        )
        .variant("visual_orientation_lock: body rotate(90deg)")
        .count(1),
        fail(
            id,
            root!("transform: matrix(6.12323e-17, -1, 1, 6.12323e-17, 0, 0)", ""),
            ORIENT_FIXTURES,
        )
        .variant("visual_orientation_script_lock: html rotate(-90deg), Script setzt es")
        .count(1),
        fail(
            id,
            root!(
                "",
                "transform: matrix3d(0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)"
            ),
            ORIENT_NORM,
        )
        .variant("matrix3d: rotateZ(90deg)")
        .count(1),
        fail(
            id,
            root!("rotate: 90deg", ""),
            "issue #134: a rotate angle off a multiple of 180 degrees counts",
        )
        .variant("rotate: 90deg an html")
        .count(1),
        fail(
            id,
            root!("rotate: 0.25turn", ""),
            "CSS Transforms 2: angles in turn are 360 degrees",
        )
        .variant("rotate in turn")
        .count(1),
        fail(
            id,
            root!("rotate: 0 0 1 90deg", ""),
            "CSS Transforms 2: axis (0, 0, 1) is the z axis, a rotation in the picture plane",
        )
        .variant("rotate um die Z-Achse in Langform")
        .count(1),
        fail(
            id,
            root!(
                "transform: matrix(1, 0.0101, -0.0101, 1, 0, 0)",
                "transform: matrix(6.12323e-17, 1, -1, 6.12323e-17, 0, 0)"
            ),
            "issue #134: threshold |b| > 0.01 and |b + c| < 0.01",
        )
        .variant("Grenze: b = 0,0101 darüber; html und body je ein Befund")
        .count(2),
        negative(
            id,
            root!("transform: matrix(1, 0.01, -0.01, 1, 0, 0)", ""),
            "issue #134: threshold |b| > 0.01; exactly 0.01 does not count",
        )
        .variant("Grenze: b = 0,01 nicht darüber"),
        negative(
            id,
            root!(
                "transform: matrix(1, -2.44929e-16, 2.44929e-16, 1, 0, 0)",
                "transform: matrix(1, 0, 0, 1, 0, 0)"
            ),
            ORIENT_FIXTURES,
        )
        .variant("visual_orientation_ok: rotate(360deg) und translateX(0)"),
        negative(
            id,
            root!("transform: matrix(-1, 1.22465e-16, -1.22465e-16, -1, 0, 0)", ""),
            "issue #134: a half turn gives b = 0 and does not force an orientation",
        )
        .variant("rotate(180deg)"),
        negative(
            id,
            root!("transform: matrix(1, 1, 0, 1, 0, 0)", ""),
            "issue #134: skew alone does not produce b = -c",
        )
        .variant("skewY(45deg)"),
        negative(
            id,
            root!("transform: matrix(2, 0, 0, 2, 0, 0)", "rotate: 180deg"),
            "issue #134: scale alone, and a rotate that is a multiple of 180 degrees",
        )
        .variant("scale und rotate: 180deg"),
        negative(
            id,
            root!("rotate: x 90deg", ""),
            "CSS Transforms 2: a rotation about the x axis tilts, it does not turn in the picture plane",
        )
        .variant("rotate um die X-Achse"),
        negative(
            id,
            r#"<div style="transform: matrix(6.12323e-17, 1, -1, 6.12323e-17, 0, 0)">Badge</div>"#,
            "issue #134: only html and body are read; a rotated component is not an orientation lock",
        )
        .variant("gedrehtes Bauteil im Inhalt"),
        negative(id, root!("", ""), ORIENT_FIXTURES)
            .variant("visual_orientation_hide: nichts gedreht"),
        na(
            id,
            r#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"></svg>"#,
            "issue #134: the rule reads html and body; a foreign document has neither",
        ),
        not_run(
            id,
            concat!(
                r#"<!DOCTYPE html><html lang="en" style="--unmeasured: transform"><head><title>T</title></head>"#,
                r#"<body style="transform: matrix(6.12323e-17, 1, -1, 6.12323e-17, 0, 0)"><p>Content</p></body></html>"#
            ),
            "docs/conventions.md: not measured is not passed; issue #134: neither field measured on html/body gives NotRun",
        ),
    ]
}

const AUTH_FIXTURE: &str = "auditmysite tests/fixtures/detection_corpus/accessible_authentication (accessible-auth-paste-blocked): pw-blocked and otp-blocked are violations, pw-ok and pw-sanitised pass";
const AUTH_NORM: &str = "WCAG 2.2 SC 3.3.8 Accessible Authentication (Minimum): a cognitive function test such as remembering a password needs a mechanism to assist; paste / a password manager is that mechanism (Understanding 3.3.8)";

fn paste() -> Vec<Fall> {
    let id = "auth/paste-blocked";
    vec![
        fail(
            id,
            r#"<input id="pw-blocked" name="password" type="password" autocomplete="current-password" style="--paste-cancelled:1;--paste-inserted:0">"#,
            AUTH_FIXTURE,
        )
        .variant("pw-blocked: addEventListener + preventDefault")
        .count(1),
        fail(
            id,
            r#"<input id="otp-blocked" name="otp" type="text" inputmode="numeric" autocomplete="one-time-code" onpaste="return false" style="--paste-cancelled:1;--paste-inserted:0">"#,
            AUTH_FIXTURE,
        )
        .variant("otp-blocked: onpaste=return false, Einmalcode")
        .count(1),
        fail(
            id,
            r#"<input type="password" autocomplete="off" style="--paste-cancelled:1;--paste-inserted:0">"#,
            AUTH_NORM,
        )
        .variant("Passwortfeld mit autocomplete=off wird trotzdem geprüft")
        .count(1),
        fail(
            id,
            r#"<input name="pw" autocomplete="section-login current-password" style="--paste-cancelled:1;--paste-inserted:0">"#,
            "HTML autocomplete: a space-separated token list; current-password selects the field",
        )
        .variant("Tokenliste, Feld ohne type=password")
        .count(1),
        fail(
            id,
            r#"<input type="password" style="--paste-probe:unmeasured"><input type="password" style="--paste-cancelled:1;--paste-inserted:0">"#,
            "issue #133: a field without a probe result is not judged, a probed one is",
        )
        .variant("ein Feld ohne Messung, eines blockiert")
        .count(1),
        negative(
            id,
            r#"<input id="pw-ok" name="password2" type="password" autocomplete="current-password">"#,
            AUTH_FIXTURE,
        )
        .variant("pw-ok: Einfügen erlaubt"),
        negative(
            id,
            r#"<input id="pw-sanitised" name="password3" type="password" autocomplete="current-password" style="--paste-cancelled:1;--paste-inserted:1">"#,
            AUTH_FIXTURE,
        )
        .variant("pw-sanitised: Handler bricht ab, fügt aber selbst ein"),
        negative(
            id,
            r#"<input type="password" style="--paste-cancelled:0;--paste-inserted:0">"#,
            "issue #133: blocked means cancelled and not inserted; not cancelled is not blocked",
        )
        .variant("nicht abgebrochen, aber nichts eingefügt"),
        na(
            id,
            r#"<input id="user-a" name="username" type="text" autocomplete="username" style="--paste-cancelled:1;--paste-inserted:0">"#,
            AUTH_FIXTURE,
        )
        .variant("Benutzername: kein Passwort- oder Codefeld"),
        na(
            id,
            r#"<input type="password" disabled style="--paste-cancelled:1;--paste-inserted:0">"#,
            "issue #133: fields are visible, not disabled and not read-only",
        )
        .variant("disabled"),
        na(
            id,
            r#"<input type="password" readonly style="--paste-cancelled:1;--paste-inserted:0">"#,
            "issue #133: fields are visible, not disabled and not read-only",
        )
        .variant("readonly"),
        na(
            id,
            r#"<input type="password" style="display:none;--paste-cancelled:1;--paste-inserted:0">"#,
            "issue #133: fields are visible",
        )
        .variant("display: none"),
        na(id, r#"<p>No fields.</p>"#, AUTH_NORM),
        not_run(
            id,
            r#"<!DOCTYPE html><html lang="en" style="--unmeasured: paste"><head><title>T</title></head><body><input type="password" autocomplete="current-password" style="--paste-cancelled:1;--paste-inserted:0"></body></html>"#,
            "docs/conventions.md: not measured is not passed; issue #133: no probe on any candidate gives NotRun",
        ),
    ]
}
