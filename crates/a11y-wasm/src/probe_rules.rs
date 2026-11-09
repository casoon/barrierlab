//! Die Messspalten bis zur Regel: je Messung ein Dokument, das die Regel
//! auslöst, und der Bericht über denselben Pfad wie `Scan::run`. Eine Spalte
//! zählt als vorhanden, wenn ein Lauf sie gefüllt gesehen hat — jeder Test
//! prüft das, nicht nur den Befund.

use a11y_dom::{Node, elements};
use a11y_report::{Finding, NotRun, Outcome, Report};

use crate::Scan;
use crate::arena::Arena;
use crate::probes::{PASTE_CANCELLED, PASTE_INSERTED, ProbeColumns, SPACING_AFTER, SPACING_BEFORE};
use crate::rendering::RenderingColumns;
use crate::testing::build;

const FUENF: [&str; 5] = [
    "frames/cross-origin",
    "orientation/root-rotated",
    "auth/paste-blocked",
    "focus/obscured-enhanced",
    "text-spacing/clipped",
];

fn dokument() -> Arena {
    build(|b| {
        b.open("html")
            .attr("lang", "de")
            .open("head")
            .open("title")
            .text("Seite")
            .close()
            .close()
            .open("body")
            .open("iframe")
            .attr("src", "https://fremd.example/")
            .attr("title", "Fremd")
            .close()
            .open("input")
            .attr("type", "password")
            .attr("aria-label", "Passwort")
            .close()
            .open("button")
            .text("Los")
            .close()
            .open("div")
            .text("Ein langer Text")
            .close()
            .close()
            .close();
    })
}

fn scan() -> Scan {
    let arena = dokument();
    let n = arena.len();
    Scan {
        rendering: Some(RenderingColumns {
            color: vec![0x3333_33ff; n],
            background: vec![0xffff_ffff; n],
            font_size_px: vec![16.0; n],
            font_weight: vec![400; n],
            flags: vec![4; n],
        }),
        layout: None,
        probes: ProbeColumns::default(),
        arena,
    }
}

fn index(scan: &Scan, tag: &str) -> u32 {
    elements(&scan.arena)
        .find(|n| n.local_name() == tag)
        .map(|n| n.id().0)
        .unwrap()
}

fn findings<'r>(report: &'r Report, id: &str) -> Vec<&'r Finding> {
    report.findings.iter().filter(|f| f.rule_id == id).collect()
}

fn lief(report: &Report, id: &str) -> bool {
    report
        .rule_runs
        .iter()
        .find(|r| r.rule_id == id)
        .is_some_and(|r| r.did_run())
}

#[test]
fn ohne_spalten_sind_die_fuenf_regeln_nicht_gelaufen_mit_grund() {
    let report = scan().report();
    for id in FUENF {
        let run = report.rule_runs.iter().find(|r| r.rule_id == id).unwrap();
        assert_eq!(run.not_run, Some(NotRun::CapabilityMissing), "{id}");
        assert!(run.reason.as_deref().is_some_and(|r| !r.is_empty()), "{id}");
        assert!(findings(&report, id).is_empty(), "{id}");
    }
    assert_eq!(report.summary.pass, 0);
}

#[test]
fn frames_unlesbarer_rahmen_ist_untested() {
    let mut scan = scan();
    let frame = index(&scan, "iframe");
    scan.with_frames(vec![frame], vec![0]).unwrap();
    assert!(scan.probes.frames.as_ref().is_some_and(|c| !c.is_empty()));
    let report = scan.report();
    let f = findings(&report, "frames/cross-origin");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].outcome, Outcome::Untested);
    // Lesbar: gelaufen, kein Befund.
    scan.with_frames(vec![frame], vec![1]).unwrap();
    let report = scan.report();
    assert!(lief(&report, "frames/cross-origin"));
    assert!(findings(&report, "frames/cross-origin").is_empty());
}

#[test]
fn transforms_gedrehte_wurzel_ist_ein_befund() {
    let mut scan = scan();
    let (html, body) = (index(&scan, "html"), index(&scan, "body"));
    scan.with_transforms(
        vec![html, body],
        vec!["none".into(), "matrix(0, 1, -1, 0, 0, 0)".into()],
        vec!["none".into(), String::new()],
    )
    .unwrap();
    assert!(
        scan.probes
            .transforms
            .as_ref()
            .is_some_and(|c| !c.is_empty())
    );
    let report = scan.report();
    let f = findings(&report, "orientation/root-rotated");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].outcome, Outcome::Fail);
    // Ungedreht: gelaufen, kein Befund.
    scan.with_transforms(
        vec![html, body],
        vec!["none".into(), "none".into()],
        vec!["none".into(), "none".into()],
    )
    .unwrap();
    let report = scan.report();
    assert!(lief(&report, "orientation/root-rotated"));
    assert!(findings(&report, "orientation/root-rotated").is_empty());
}

#[test]
fn paste_abgebrochen_ohne_einfuegen_ist_ein_befund() {
    let mut scan = scan();
    let input = index(&scan, "input");
    scan.with_paste_probes(vec![input], vec![PASTE_CANCELLED])
        .unwrap();
    assert!(scan.probes.paste.as_ref().is_some_and(|c| !c.is_empty()));
    let report = scan.report();
    let f = findings(&report, "auth/paste-blocked");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].outcome, Outcome::Fail);
    // Abgebrochen, aber vom Handler selbst eingefügt: kein Befund.
    scan.with_paste_probes(vec![input], vec![PASTE_CANCELLED | PASTE_INSERTED])
        .unwrap();
    let report = scan.report();
    assert!(lief(&report, "auth/paste-blocked"));
    assert!(findings(&report, "auth/paste-blocked").is_empty());
}

#[test]
fn focus_ueberdeckung_erreicht_beide_fokusregeln_samt_blocker() {
    let mut scan = scan();
    let (button, blocker) = (index(&scan, "button"), index(&scan, "div"));
    // Teilweise überdeckt: nur die erhöhte Stufe.
    scan.with_focus_cover(vec![button], vec![9], vec![3], vec![blocker as i32])
        .unwrap();
    assert!(scan.probes.focus.as_ref().is_some_and(|c| !c.is_empty()));
    let report = scan.report();
    assert_eq!(findings(&report, "focus/obscured-enhanced").len(), 1);
    assert!(findings(&report, "focus/obscured").is_empty());
    let beleg = &findings(&report, "focus/obscured-enhanced")[0].evidence;
    assert!(
        beleg
            .iter()
            .any(|e| e.field.as_deref() == Some("focus_covered") && e.value.as_deref() == Some("3"))
    );
    // Ganz überdeckt: auch 2.4.11, der Blocker steht im Beleg.
    scan.with_focus_cover(vec![button], vec![9], vec![9], vec![blocker as i32])
        .unwrap();
    let report = scan.report();
    let f = findings(&report, "focus/obscured");
    assert_eq!(f.len(), 1);
    let erwartet = blocker.to_string();
    assert!(f[0].evidence.iter().any(|e| {
        e.field.as_deref() == Some("focus_blocker") && e.value.as_deref() == Some(&erwartet)
    }));
}

#[test]
fn textabstand_neu_abgeschnitten_ist_ein_review() {
    let mut scan = scan();
    let div = index(&scan, "div");
    scan.with_text_spacing(vec![div], vec![SPACING_AFTER])
        .unwrap();
    assert!(scan.probes.spacing.as_ref().is_some_and(|c| !c.is_empty()));
    let report = scan.report();
    let f = findings(&report, "text-spacing/clipped");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].outcome, Outcome::Review);
    // Schon vorher abgeschnitten: liegt nicht am Textabstand.
    scan.with_text_spacing(vec![div], vec![SPACING_BEFORE | SPACING_AFTER])
        .unwrap();
    let report = scan.report();
    assert!(lief(&report, "text-spacing/clipped"));
    assert!(findings(&report, "text-spacing/clipped").is_empty());
}

#[test]
fn jede_spalte_bedient_nur_ihre_regel() {
    let mut scan = scan();
    let frame = index(&scan, "iframe");
    scan.with_frames(vec![frame], vec![1]).unwrap();
    let report = scan.report();
    for id in FUENF {
        assert_eq!(lief(&report, id), id == "frames/cross-origin", "{id}");
    }
}

#[test]
fn spalten_ohne_rendering_bleiben_wirkungslos() {
    let mut scan = scan();
    scan.rendering = None;
    let frame = index(&scan, "iframe");
    scan.with_frames(vec![frame], vec![0]).unwrap();
    let report = scan.report();
    assert!(findings(&report, "frames/cross-origin").is_empty());
}
