//! Gruppe „sonden“: Regeln über Messungen, die ein Host an der laufenden
//! Seite vornimmt (`probes.rs`, `heuristics.rs` für `focus/obscured`).
//!
//! Die Dokumente tragen die Werte des Hosts als eigene Eigenschaften im
//! `style`-Attribut (siehe `host.rs`). Die Vorlagen stammen aus dem
//! auditmysite-Korpus `tests/fixtures/detection_corpus/` (nur gelesen) und
//! sind hier als eigene, kleine Dokumente nachgestellt.

use super::super::{Fall, na, negative, not_run, untested};

const FRAME_FIXTURE: &str = "auditmysite tests/fixtures/detection_corpus/content_frames_cross_origin (frame-tested): booking, small-map, shadow-calendar and late-reviews are reported; pixel (1x1), collapsed (display:none), silent (aria-hidden) and local-help (readable) are not";
const FRAME_NORM: &str = "no WCAG criterion applies to a document the tool cannot read (coverage note); host contract of Rendering::frame_reachable, casoon/barrierlab#132";

pub fn cases() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(frames());
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
