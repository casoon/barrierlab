//! Gruppe „darstellung“: Kontrast (`rendering.rs`), Heuristiken
//! (`heuristik.rs`) und gemessene Darstellungsregeln (`darstellung.rs`).
//!
//! Die Farbpaare an den Schwellen sind nachgerechnet (relative Leuchtdichte
//! nach WCAG 2.2, Grautöne):
//!
//! | Paar (Text auf Grund) | Verhältnis |
//! |---|---|
//! | `#6d6d6d` auf `#efefef` | 4,49995:1 |
//! | `#282828` auf `#8e8e8e` | 4,50017:1 |
//! | `#1b1b1b` auf `#666666` | 2,99980:1 |
//! | `#242424` auf `#6d6d6d` | 3,000004:1 |
//! | `#313131` auf `#bebebe` | 6,99905:1 |
//! | `#2a2a2a` auf `#b5b5b5` | 7,00005:1 |

use super::super::{Fall, fail, na, negativ, review, untested};

const U143: &str = "WCAG 2.2 SC 1.4.3 (4.5:1, large text 3:1); Understanding 1.4.3: ratios are not rounded, 4.499:1 does not meet 4.5:1";
const U146: &str =
    "WCAG 2.2 SC 1.4.6 (7:1, large text 4.5:1); Understanding 1.4.6: ratios are not rounded";
const U1411: &str = "WCAG 2.2 SC 1.4.11: visual information needed to identify a UI component needs 3:1 against adjacent colors; Understanding 1.4.11";
const U258: &str = "WCAG 2.2 SC 2.5.8: targets at least 24x24 CSS px, or a 24 px circle on the target does not intersect another target or another undersized target's circle; exceptions inline and equivalent";
const U255: &str =
    "WCAG 2.2 SC 2.5.5 (AAA): targets at least 44x44 CSS px; exceptions inline and equivalent";

pub fn faelle() -> Vec<Fall> {
    let mut v = Vec::new();
    v.extend(kontrast());
    v.extend(bedienelement());
    v.extend(reihenfolge());
    v.extend(bewegung());
    v.extend(reflow());
    v.extend(nur_zeiger());
    v.extend(verdeckt());
    v.extend(ziele());
    v.extend(ziele_erhoeht());
    v.extend(fokus());
    v.extend(listen());
    v.extend(links());
    v.extend(scrollbereich());
    v
}

fn kontrast() -> Vec<Fall> {
    let ti = "contrast/text-insufficient";
    let tu = "contrast/text-undetermined";
    let te = "contrast/text-enhanced";
    vec![
        // --- 1.4.3 ---
        fail(ti, r#"<p style="color:#6d6d6d;background-color:#efefef">Opening hours</p>"#, U143)
            .variante("Grenze: 4,49995:1, normaler Text; Regression: keine Rundungstoleranz")
            .anzahl(1),
        negativ(ti, r#"<p style="color:#282828;background-color:#8e8e8e">Opening hours</p>"#, U143)
            .variante("Grenze: 4,50017:1, normaler Text"),
        fail(ti, r#"<p style="font-size:24px;color:#1b1b1b;background-color:#666666">Opening hours</p>"#, U143)
            .variante("Grenze: 2,9998:1, großer Text 24 px"),
        negativ(ti, r#"<p style="font-size:24px;color:#242424;background-color:#6d6d6d">Opening hours</p>"#, U143)
            .variante("Grenze: 3,000004:1, großer Text 24 px"),
        fail(ti, r#"<p style="font-size:23.9px;color:#242424;background-color:#6d6d6d">Opening hours</p>"#,
            "WCAG 2.2 1.4.3 and definition 'large scale': at least 18 pt (24 px) or 14 pt bold; 23.9 px regular is normal text and needs 4.5:1")
            .variante("Grenze: 23,9 px ist kein großer Text"),
        negativ(ti, r#"<p style="font-size:14pt;font-weight:bold;color:#242424;background-color:#6d6d6d">Opening hours</p>"#,
            "WCAG 2.2 definition 'large scale': 14 pt bold counts as large text, 3:1 suffices")
            .variante("Grenze: 14 pt fett ist großer Text"),
        fail(ti, r#"<p style="font-size:14pt;color:#242424;background-color:#6d6d6d">Opening hours</p>"#,
            "WCAG 2.2 definition 'large scale': 14 pt regular is not large text")
            .variante("14 pt normal ist kein großer Text"),
        fail(ti, r#"<p aria-hidden="true" style="color:#777777;background-color:#888888">Decorative caption</p>"#,
            "WCAG 2.2 1.4.3 applies to the visual presentation of text; aria-hidden does not remove it from view")
            .variante("aria-hidden, aber sichtbar"),
        negativ(ti, r#"<p style="display:none;color:#777777;background-color:#888888">Hidden note</p><p hidden style="color:#777777;background-color:#888888">Hidden note</p><p style="visibility:hidden;color:#777777;background-color:#888888">Hidden note</p>"#,
            "ACT afw4f7: applies only to visible text; display:none / hidden / visibility:hidden text is not presented")
            .variante("versteckt per CSS und hidden-Attribut"),
        negativ(ti, r#"<a href="/skip" style="--visually-hidden:1;color:#777777;background-color:#888888">Skip to content</a>"#,
            "ACT afw4f7: text that is not visible (clipped sr-only pattern) is not applicable")
            .variante("optisch verborgen (sr-only)"),
        negativ(ti, r#"<button disabled style="color:#777777;background-color:#888888">Send</button><div role="button" aria-disabled="true" style="color:#777777;background-color:#888888">Send</div><fieldset disabled><button style="color:#777777;background-color:#888888">Save</button></fieldset>"#,
            "WCAG 2.2 1.4.3 exception: text that is part of an inactive user interface component; ACT afw4f7 excludes text of disabled widgets")
            .variante("Regression: inaktive Bedienelemente (disabled, aria-disabled, fieldset)"),
        fail(ti, r#"<button style="color:#777777;background-color:#888888">Send</button>"#, U143)
            .variante("aktives Bedienelement"),
        fail(ti, r#"<div><template shadowrootmode="open"><p style="color:#777777;background-color:#888888">Shadow text</p></template></div>"#,
            "WCAG 2.2 1.4.3; shadow DOM content is rendered like light DOM")
            .variante("Shadow DOM"),
        fail(ti, r#"<p style="color:#000000;--bg-unknown:1;--backdrop:0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1">Over a dark photo</p>"#,
            "WCAG 2.2 1.4.3 over a sampled backdrop: black text over luminance 0.1 reaches 3:1, below 4.5:1")
            .variante("abgetasteter Hintergrund, durchgehend zu dunkel"),
        na(ti, r#"<img src="logo.png" alt=""><div></div>"#, "ACT afw4f7: applies to text nodes only; no text here"),
        // --- nicht bestimmbar ---
        untested(tu, r#"<p style="--bg-unknown:1">Text over an unknown background</p>"#,
            "WCAG 2.2 1.4.3 needs both colours; without a resolvable background the ratio cannot be determined automatically — not passed (docs/conventions.md)")
            .anzahl(1),
        untested(tu, r#"<p style="background-image:url(hero.jpg)">Text over a photo</p>"#,
            "Understanding 1.4.3: text over background images must be checked against the image; unresolved here")
            .variante("background-image ohne Abtastung"),
        untested(tu, r#"<p style="--obscured:1;color:#777777;background-color:#888888">Covered by a banner</p>"#,
            "WCAG 2.2 1.4.3: a fixed overlay covering the text makes its rendered contrast unmeasurable — not passed")
            .variante("überdeckt"),
        review(tu, r#"<p style="color:#000000;--bg-unknown:1;--backdrop:0.1,0.1,0.1,0.1,0.1,1,1,1,1,1">Over a gradient</p>"#,
            "Understanding 1.4.3 (gradients/images): the ratio varies over the box; median meets 4.5:1, darker parts reach 3:1 — a human decides")
            .variante("abgetasteter Hintergrund, teils zu dunkel"),
        negativ(tu, r#"<p style="color:#000000;--bg-unknown:1;--backdrop:1,1,1,1,1,1,1,1,1,1">Over a bright photo</p>"#,
            "WCAG 2.2 1.4.3: sampled backdrop gives 21:1 throughout — determinable and sufficient"),
        negativ(tu, r#"<p style="color:#000000;background-color:#ffffff">Plain text</p>"#,
            "WCAG 2.2 1.4.3: both colours known — determinable"),
        negativ(tu, r#"<p hidden style="--bg-unknown:1">Hidden</p>"#,
            "ACT afw4f7: hidden text is not applicable")
            .variante("versteckt"),
        na(tu, r#"<div></div>"#, "ACT afw4f7: no text"),
        // --- 1.4.6 ---
        fail(te, r#"<p style="color:#313131;background-color:#bebebe">Opening hours</p>"#, U146)
            .variante("Grenze: 6,99905:1, normaler Text")
            .anzahl(1),
        negativ(te, r#"<p style="color:#2a2a2a;background-color:#b5b5b5">Opening hours</p>"#, U146)
            .variante("Grenze: 7,00005:1, normaler Text"),
        fail(te, r#"<p style="font-size:24px;color:#6d6d6d;background-color:#efefef">Opening hours</p>"#, U146)
            .variante("Grenze: 4,49995:1, großer Text"),
        negativ(te, r#"<p style="font-size:24px;color:#282828;background-color:#8e8e8e">Opening hours</p>"#, U146)
            .variante("Grenze: 4,50017:1, großer Text"),
        negativ(te, r#"<button disabled style="color:#313131;background-color:#bebebe">Send</button>"#,
            "WCAG 2.2 1.4.6 exception: text of an inactive user interface component")
            .variante("Regression: inaktives Bedienelement"),
        na(te, r#"<div></div>"#, "WCAG 2.2 1.4.6 applies to text; no text"),
    ]
}

fn bedienelement() -> Vec<Fall> {
    let ci = "contrast/control-insufficient";
    let cu = "contrast/control-undetermined";
    vec![
        fail(ci, r#"<div style="background-color:#666666"><input type="checkbox" aria-label="Agree" style="appearance:none;border-top-style:solid;border-top-width:1px;border-top-color:#1b1b1b"></div>"#, U1411)
            .variante("Grenze: Rahmen 2,9998:1")
            .anzahl(1),
        negativ(ci, r#"<div style="background-color:#6d6d6d"><input type="checkbox" aria-label="Agree" style="appearance:none;border-top-style:solid;border-top-width:1px;border-top-color:#242424"></div>"#, U1411)
            .variante("Grenze: Rahmen 3,000004:1"),
        fail(ci, r#"<div style="background-color:#666666"><span role="switch" aria-checked="false" tabindex="0" aria-label="Dark mode" style="appearance:none;background-color:#1b1b1b"></span></div>"#, U1411)
            .variante("ARIA-Schalter, eigene Fläche statt Rahmen"),
        negativ(ci, r#"<div style="background-color:#666666"><input type="checkbox" aria-label="Agree" style="border-top-style:solid;border-top-width:1px;border-top-color:#1b1b1b"></div>"#,
            "Understanding 1.4.11: controls drawn by the user agent (appearance not none) are not modified by the author and are exempt")
            .variante("natives Element"),
        negativ(ci, r#"<div style="background-color:#666666"><input type="checkbox" disabled aria-label="Agree" style="appearance:none;border-top-style:solid;border-top-width:1px;border-top-color:#1b1b1b"></div>"#,
            "WCAG 2.2 1.4.11 exception: inactive user interface components")
            .variante("Regression: inaktives Bedienelement"),
        negativ(ci, r#"<div style="background-color:#666666"><input type="checkbox" hidden aria-label="Agree" style="appearance:none;border-top-style:solid;border-top-width:1px;border-top-color:#1b1b1b"></div>"#,
            "WCAG 2.2 1.4.11: a control that is not rendered has no visual boundary")
            .variante("versteckt"),
        na(ci, r#"<div style="background-color:#666666"><p>No controls</p></div>"#, "WCAG 2.2 1.4.11 (UI components): no checkbox, radio, switch or slider"),
        untested(cu, r#"<div style="--bg-unknown:1"><input type="radio" name="r" aria-label="One" style="appearance:none;border-top-style:solid;border-top-width:1px;border-top-color:#1b1b1b"></div>"#,
            "WCAG 2.2 1.4.11 needs the adjacent colour; an unresolved background is not a pass (docs/conventions.md)")
            .anzahl(1),
        negativ(cu, r#"<div style="background-color:#ffffff"><input type="radio" name="r" aria-label="One" style="appearance:none;border-top-style:solid;border-top-width:1px;border-top-color:#1b1b1b"></div>"#,
            "WCAG 2.2 1.4.11: adjacent colour known — determinable"),
        negativ(cu, r#"<div style="--bg-unknown:1"><input type="radio" name="r" aria-label="One"></div>"#,
            "Understanding 1.4.11: native (user-agent drawn) control, nothing for the author to measure")
            .variante("natives Element"),
        na(cu, r#"<div style="--bg-unknown:1"><p>No controls</p></div>"#, "WCAG 2.2 1.4.11: no custom control"),
    ]
}

fn reihenfolge() -> Vec<Fall> {
    let id = "order/visual-mismatch";
    let q = "WCAG 2.2 SC 1.3.2 / 2.4.3; C27: DOM order should match visual order; reordering via flex-direction *-reverse or order over content is a hint, REVIEW per docs/packages/a11y-rules.md (heuristics)";
    vec![
        review(id, r#"<div style="display:flex;flex-direction:row-reverse"><a href="/a">First</a><a href="/b">Second</a></div>"#, q)
            .anzahl(1),
        review(id, r#"<div style="display:flex"><a href="/a" style="order:2">First</a><a href="/b">Second</a></div>"#, q)
            .variante("order"),
        review(id, r#"<div style="display:flex;flex-direction:column-reverse"><p>Step one</p><p>Step two</p></div>"#, q)
            .variante("nur Text: 1.3.2 Lesereihenfolge"),
        negativ(id, r#"<div style="display:flex;flex-direction:row"><a href="/a">First</a><a href="/b">Second</a></div>"#,
            "C27: DOM order matches visual order"),
        negativ(id, r#"<div style="display:flex"><a href="/a" style="order:0">First</a><a href="/b">Second</a></div>"#,
            "C27: order 0 keeps source order")
            .variante("order: 0"),
        negativ(id, r#"<div style="display:flex;flex-direction:row-reverse"><a href="/story"><img src="t.jpg" alt=""></a><a href="/story">Headline</a></div>"#,
            "WCAG 2.2 2.4.3: a focus order that leads to the same destination preserves meaning and operability")
            .variante("Teaserkarte, ein Ziel"),
        negativ(id, r#"<div style="display:flex;flex-direction:row-reverse"><a href="/a">Only</a></div>"#,
            "WCAG 2.2 1.3.2: one item has no sequence to change")
            .variante("ein Kind"),
        na(id, r#"<p>Plain paragraph</p>"#, "1.3.2 / 2.4.3: no CSS reordering"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: flex-reversed order"><head><title>T</title></head><body><p>Text</p></body></html>"#,
            "docs/conventions.md: not measured is not passed")
            .anzahl(1),
    ]
}

fn bewegung() -> Vec<Fall> {
    let id = "motion/infinite-animation";
    let q = "WCAG 2.2 SC 2.2.2: moving content that starts automatically and lasts more than five seconds needs a pause/stop/hide mechanism; REVIEW per docs (heuristic)";
    vec![
        review(id, r#"<div style="animation-iteration-count:infinite">Spinning badge</div>"#, q).anzahl(1),
        review(id, r#"<marquee>Breaking news</marquee>"#, q).variante("marquee"),
        review(id, r#"<div><span style="--infinite-animation:1">a</span><span style="--infinite-animation:1">b</span></div>"#, q)
            .variante("Gruppe: eine Bewegung")
            .anzahl(1),
        review(id, r#"<div aria-hidden="true" style="--infinite-animation:1">Decorative loop</div>"#,
            "WCAG 2.2 2.2.2 concerns visible movement; aria-hidden does not stop it")
            .variante("aria-hidden, aber sichtbar"),
        negativ(id, r#"<div style="animation-iteration-count:infinite">Spinning badge</div><button>Pause animation</button>"#,
            "WCAG 2.2 2.2.2: a mechanism to pause is provided (G4)"),
        negativ(id, r#"<div style="--infinite-animation:1">Loop</div><input type="checkbox" id="m"><label for="m">Reduce motion</label>"#,
            "WCAG 2.2 2.2.2 (G4): a control to stop motion")
            .variante("Schalter über label for"),
        negativ(id, r#"<div style="animation-iteration-count:3">Short pulse</div>"#,
            "WCAG 2.2 2.2.2 applies to movement lasting more than five seconds; a finite animation ends")
            .variante("endliche Animation"),
        negativ(id, r#"<div style="display:none;--infinite-animation:1">Loop</div>"#,
            "WCAG 2.2 2.2.2: content not displayed does not move")
            .variante("versteckt"),
        na(id, r#"<p>Static text</p>"#, "WCAG 2.2 2.2.2: no moving content"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: animation"><head><title>T</title></head><body><p>Text</p></body></html>"#,
            "docs/conventions.md: not measured is not passed")
            .anzahl(1),
    ]
}

fn reflow() -> Vec<Fall> {
    let id = "reflow/min-width";
    let q = "WCAG 2.2 SC 1.4.10: content reflows at 320 CSS px without horizontal scrolling; a min-width above 320 px forces it; REVIEW per docs (heuristic)";
    vec![
        review(id, r#"<div style="min-width:321px">Wide box</div>"#, q).variante("Grenze: 321 px").anzahl(1),
        negativ(id, r#"<div style="min-width:320px">Box</div>"#, q).variante("Grenze: 320 px passt"),
        review(id, r#"<div style="min-width:400px"><div style="min-width:400px">Inner</div></div>"#, q)
            .variante("verschachtelt: nur das äußerste")
            .anzahl(1),
        negativ(id, r#"<table style="min-width:600px"><tr><td>a</td></tr></table><pre style="min-width:600px"><span style="min-width:600px">code</span></pre>"#,
            "Understanding 1.4.10: content requiring two-dimensional layout (data tables, code, images, video) is exempt")
            .variante("Ausnahme 2D-Inhalt"),
        negativ(id, r#"<div hidden style="min-width:600px">Hidden</div>"#, "1.4.10: content not rendered does not scroll")
            .variante("versteckt"),
        na(id, r#"<p>Text</p>"#, "1.4.10: no min-width"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: min-width"><head><title>T</title></head><body><p>Text</p></body></html>"#,
            "docs/conventions.md: not measured is not passed")
            .anzahl(1),
    ]
}

fn nur_zeiger() -> Vec<Fall> {
    let id = "keyboard/pointer-only";
    let q = "WCAG 2.2 SC 2.1.1 / 4.1.2: something that looks clickable without role or focusability cannot be operated by keyboard (F54, F59); REVIEW per docs (heuristic)";
    vec![
        review(id, r#"<div style="cursor:pointer">Search</div>"#, q).anzahl(1),
        review(id, r#"<div style="cursor:pointer"><span>Search</span> <span>now</span></div>"#, q)
            .variante("vererbtes cursor: nur der Ursprung")
            .anzahl(1),
        negativ(id, r#"<div style="cursor:pointer"><a href="/s">Search</a></div>"#,
            "WCAG 2.2 2.1.1: the card's click lands on a keyboard-operable link inside")
            .variante("Kartenmuster"),
        negativ(id, r#"<button style="cursor:pointer">Search</button>"#, "WCAG 2.2 2.1.1: native button is keyboard operable")
            .variante("nativ"),
        negativ(id, r#"<div role="button" tabindex="0" style="cursor:pointer">Search</div>"#,
            "WAI-ARIA 1.2 button role with tabindex: focusable widget")
            .variante("ARIA-Rolle"),
        negativ(id, r#"<div style="cursor:pointer"></div>"#, "WCAG 2.2 2.1.1: an empty box offers no function to reach")
            .variante("ohne Inhalt"),
        negativ(id, r#"<div hidden style="cursor:pointer">Search</div>"#, "2.1.1: not rendered").variante("versteckt"),
        na(id, r#"<p>Text</p>"#, "2.1.1: nothing looks clickable"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: cursor"><head><title>T</title></head><body><p>Text</p></body></html>"#,
            "docs/conventions.md: not measured is not passed")
            .anzahl(1),
    ]
}

fn verdeckt() -> Vec<Fall> {
    let id = "focus/obscured";
    let q = "WCAG 2.2 SC 2.4.11: a focused component is not entirely hidden by author-created content (sticky/fixed); REVIEW per docs (heuristic)";
    vec![
        review(id, r#"<button style="--obscured:1">Buy</button>"#, q).anzahl(1),
        review(id, r#"<header style="--hides-focus:1">Sticky bar</header>"#, q).variante("Leiste ohne scroll-padding"),
        negativ(id, r#"<p style="--obscured:1">Covered text</p>"#, "WCAG 2.2 2.4.11 applies to components that receive focus")
            .variante("nicht fokussierbar"),
        negativ(id, r#"<div tabindex="-1" style="--obscured:1">Panel</div>"#, "2.4.11: not in the sequential focus order")
            .variante("tabindex=-1"),
        negativ(id, r#"<button>Buy</button>"#, "2.4.11: nothing covers the control"),
        na(id, r#"<p>Text</p>"#, "2.4.11: no focusable content, nothing fixed"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: obscured hides-focus"><head><title>T</title></head><body><button>Buy</button></body></html>"#,
            "docs/conventions.md: not measured is not passed")
            .anzahl(1),
    ]
}

fn ziele() -> Vec<Fall> {
    let id = "targets/size";
    vec![
        review(id, r#"<button style="--w:16px;--h:16px">A</button><button style="--x:20px;--w:16px;--h:16px">B</button>"#, U258)
            .anzahl(2),
        review(id, r#"<button style="--w:16px;--h:16px">A</button><button style="--x:23px;--w:16px;--h:16px">B</button>"#, U258)
            .variante("Grenze Abstand: Mitten 23 px, Kreise schneiden sich")
            .anzahl(2),
        negativ(id, r#"<button style="--w:16px;--h:16px">A</button><button style="--x:24px;--w:16px;--h:16px">B</button>"#, U258)
            .variante("Grenze Abstand: Mitten 24 px, Kreise berühren sich nur"),
        review(id, r#"<button style="--w:23.5px;--h:24px">A</button><button style="--x:23.5px;--w:23.5px;--h:24px">B</button>"#, U258)
            .variante("Grenze Größe: 23,5 × 24 px"),
        negativ(id, r#"<button style="--w:24px;--h:24px">A</button><button style="--x:24px;--w:24px;--h:24px">B</button>"#, U258)
            .variante("Grenze Größe: 24 × 24 px"),
        negativ(id, r#"<button style="--w:16px;--h:16px">A</button><button style="--x:100px;--w:16px;--h:16px">B</button>"#, U258)
            .variante("Abstandsausnahme"),
        review(id, r#"<div role="button" tabindex="0" style="--w:16px;--h:16px">A</div><button style="--x:16px;--w:16px;--h:16px">B</button>"#, U258)
            .variante("ARIA-Rolle"),
        negativ(id, r#"<p>Read the <a href="/a" style="--w:18px;--h:16px">terms</a> and the <a href="/b" style="--x:20px;--w:18px;--h:16px">privacy notice</a> now.</p>"#,
            "WCAG 2.2 2.5.8 exception 'Inline': target in a sentence")
            .variante("Ausnahme inline"),
        negativ(id, r#"<a href="/cart" style="--w:16px;--h:16px">C</a><button style="--x:16px;--w:100px;--h:30px">Search</button><a href="/cart" style="--y:100px">Cart</a>"#,
            "WCAG 2.2 2.5.8 exception 'Equivalent': the function is available through a different control that meets the criterion")
            .variante("Ausnahme gleichwertig"),
        review(id, r#"<a href="/cart" style="--w:16px;--h:16px">C</a><button style="--x:16px;--w:100px;--h:30px">Search</button><a href="/cart" aria-hidden="true" tabindex="-1" style="--y:100px">Cart</a>"#,
            "WCAG 2.2 2.5.8 'Equivalent' requires a usable equivalent; one under aria-hidden is not (auditmysite#706)")
            .variante("Gegenstück unter aria-hidden zählt nicht")
            .anzahl(1),
        negativ(id, r#"<label><input type="checkbox" style="--w:13px;--h:13px"> Remember me</label><button style="--x:13px;--w:100px;--h:30px">Go</button>"#,
            "Understanding 2.5.8: the label of a checkbox is part of its target")
            .variante("Ziel durch Label"),
        negativ(id, r#"<button style="--w:16px;--h:16px">A</button><button style="--x:16px;--w:16px;--h:16px;pointer-events:none">B</button><button inert style="--x:-16px;--w:16px;--h:16px">C</button>"#,
            "Understanding 2.5.8: a target is a region that responds to pointer input; pointer-events:none and inert elements are not targets (auditmysite#705)")
            .variante("Nachbarn ohne Zeigerereignisse"),
        negativ(id, r#"<button style="--w:16px;--h:16px">A</button><button hidden style="--x:16px;--w:16px;--h:16px">B</button>"#,
            "2.5.8: a hidden element is not a target")
            .variante("versteckter Nachbar"),
        untested(id, r#"<button style="--w:16px;--h:16px;--animating:1">A</button><button style="--x:16px;--w:16px;--h:16px;--animating:1">B</button>"#,
            "docs/conventions.md: a size measured mid-animation is not the final size — not judged (auditmysite#706)")
            .variante("noch in Bewegung")
            .anzahl(2),
        na(id, r#"<p>No controls</p>"#, "2.5.8 applies to pointer targets only"),
    ]
}

fn ziele_erhoeht() -> Vec<Fall> {
    let id = "targets/size-enhanced";
    vec![
        review(id, r#"<button style="--w:44px;--h:43.5px">Save</button>"#, U255)
            .variante("Grenze: 44 × 43,5 px")
            .anzahl(1),
        negativ(id, r#"<button style="--w:44px;--h:44px">Save</button>"#, U255).variante("Grenze: 44 × 44 px"),
        review(id, r#"<button style="--w:16px;--h:16px">X</button>"#, "WCAG 2.2 SC 2.5.5 has no spacing exception")
            .variante("allein, aber zu klein")
            .anzahl(1),
        negativ(id, r#"<p>Read the <a href="/a" style="--w:30px;--h:16px">terms</a> now.</p>"#,
            "WCAG 2.2 2.5.5 exception 'Inline'")
            .variante("Ausnahme inline"),
        negativ(id, r#"<a href="/a" style="--w:30px;--h:30px">H</a><a href="/a" style="--y:100px;--w:44px;--h:44px">Home</a>"#,
            "WCAG 2.2 2.5.5 exception 'Equivalent'")
            .variante("Ausnahme gleichwertig"),
        untested(id, r#"<button style="--w:30px;--h:30px;--animating:1">Save</button>"#,
            "docs/conventions.md: size measured mid-animation is not judged")
            .variante("noch in Bewegung")
            .anzahl(1),
        na(id, r#"<p>No controls</p>"#, "2.5.5 applies to pointer targets only"),
    ]
}

fn fokus() -> Vec<Fall> {
    let im = "focus/indicator-missing";
    let iu = "focus/indicator-unmeasured";
    let q = "WCAG 2.2 SC 2.4.7: keyboard focus indicator visible (F78, F55); REVIEW per docs (heuristic)";
    vec![
        review(im, r#"<button style="--focus-visible:0">Save</button><button>Cancel</button>"#, q).anzahl(1),
        review(im, r#"<div role="button" tabindex="0" style="--focus-visible:0">Save</div>"#, q).variante("ARIA-Rolle mit tabindex"),
        negativ(im, r#"<button>Save</button><a href="/x">Link</a>"#, "2.4.7: focus change is visible"),
        negativ(im, r#"<div tabindex="-1" style="--focus-visible:0">Panel</div><button disabled style="--focus-visible:0">Off</button>"#,
            "2.4.7 applies to keyboard-operable components in the focus order")
            .variante("nicht per Tab erreichbar"),
        negativ(im, r#"<button hidden style="--focus-visible:0">Save</button>"#, "2.4.7: not rendered").variante("versteckt"),
        na(im, r#"<p>Text only</p>"#, "2.4.7: no focusable component"),
        untested(iu, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: focus-visible"><head><title>T</title></head><body><button>Save</button><a href="/x">Link</a></body></html>"#,
            "docs/conventions.md: focus visibility not measured is not passed")
            .anzahl(1),
        untested(iu, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: layout"><head><title>T</title></head><body><button>Save</button></body></html>"#,
            "docs/conventions.md: host without layout data")
            .variante("kein Layout"),
        negativ(iu, r#"<button>Save</button>"#, "2.4.7 measured on the page"),
        na(iu, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: focus-visible"><head><title>T</title></head><body><p>Text</p></body></html>"#,
            "2.4.7: nothing focusable, nothing to measure"),
    ]
}

fn listen() -> Vec<Fall> {
    let id = "lists/role-redundant";
    let q = "ARIA in HTML: role=list on ul/ol is redundant (best practice, no SC; docs/packages/a11y-rules.md, auditmysite#779)";
    vec![
        fail(id, r#"<ul role="list"><li>One</li><li>Two</li></ul>"#, q).anzahl(1),
        fail(id, r#"<ol role="list"><li>One</li></ol>"#, q).variante("ol"),
        negativ(id, r#"<ul role="list" style="list-style-type:none"><li>One</li></ul>"#,
            "auditmysite#644: WebKit drops list semantics with list-style-type:none; the explicit role restores them, so it is not redundant")
            .variante("list-style-type: none"),
        negativ(id, r#"<ul><li>One</li></ul>"#, "ARIA in HTML: implicit role, nothing redundant"),
        na(id, r#"<div role="list"><div role="listitem">One</div></div>"#, "ARIA in HTML: only ul/ol have an implicit list role"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: style"><head><title>T</title></head><body><ul role="list"><li>One</li></ul></body></html>"#,
            "docs/conventions.md: list-style-type not measured is not passed")
            .anzahl(1),
    ]
}

fn links() -> Vec<Fall> {
    let id = "color/link-indistinct";
    let q = "WCAG 2.2 SC 1.4.1; F73: links in running text not visually evident without colour";
    vec![
        fail(id, r#"<p>Please read the <a href="/terms" style="text-decoration:none;color:#0000ee">terms of service</a> before you continue with the order.</p>"#, q)
            .anzahl(1),
        negativ(id, r#"<p>Please read the <a href="/terms">terms of service</a> before you continue with the order.</p>"#,
            "WCAG 2.2 1.4.1, C15/G183: underline distinguishes the link"),
        negativ(id, r#"<p>Please read the <a href="/terms" style="text-decoration:none;font-weight:bold">terms of service</a> before you continue with the order.</p>"#,
            "WCAG 2.2 1.4.1: a different weight distinguishes the link")
            .variante("fett"),
        negativ(id, r#"<p>Please read the <a href="/terms" style="text-decoration:none;background-color:#ffff00">terms of service</a> before you continue with the order.</p>"#,
            "WCAG 2.2 1.4.1: a background distinguishes the link")
            .variante("Hintergrund"),
        negativ(id, r#"<nav><p>Go to <a href="/a" style="text-decoration:none">news and articles</a> or elsewhere today.</p></nav>"#,
            "Understanding 1.4.1: link purpose evident from location (navigation), not inline in a text block")
            .variante("Navigation"),
        negativ(id, r#"<p><a href="/terms" style="text-decoration:none">Terms of service</a></p>"#,
            "F73 applies to links in text blocks; a link alone is not surrounded by text")
            .variante("kein Fließtext"),
        negativ(id, r#"<p style="display:none">Please read the <a href="/terms" style="text-decoration:none">terms of service</a> before you continue.</p>"#,
            "1.4.1: not rendered")
            .variante("versteckt"),
        na(id, r#"<p>A paragraph without links.</p>"#, "1.4.1 / F73: no link"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: style"><head><title>T</title></head><body><p>Please read the <a href="/terms">terms of service</a> before you continue.</p></body></html>"#,
            "docs/conventions.md: text decoration not measured is not passed")
            .anzahl(1),
    ]
}

fn scrollbereich() -> Vec<Fall> {
    let id = "keyboard/scrollable-region-not-focusable";
    let q = "WCAG 2.2 SC 2.1.1; ACT 0ssw9k: scrollable content must be reachable with sequential focus navigation";
    let p = "axe scrollable-region-focusable: overflow up to 13 px is layout noise (subpixel, padding), not scrollable content";
    vec![
        fail(id, r#"<div style="--scroll-overflow:100px">Long terms and conditions text</div>"#, q).anzahl(1),
        fail(id, r#"<div style="--scroll-overflow:13.5px">Long text</div>"#, p).variante("Grenze: 13,5 px"),
        negativ(id, r#"<div style="--scroll-overflow:13px">Long text</div>"#, p).variante("Grenze: 13 px"),
        negativ(id, r#"<div tabindex="0" style="--scroll-overflow:100px">Long text</div>"#,
            "ACT 0ssw9k: the region itself is in the focus order")
            .variante("tabindex=0"),
        negativ(id, r#"<div style="--scroll-overflow:100px">Long text <a href="/more">more</a></div>"#,
            "ACT 0ssw9k: a focusable descendant makes the content reachable")
            .variante("fokussierbarer Nachfahre"),
        negativ(id, r#"<div style="--scroll-overflow:100px"><img src="a.png" alt=""></div>"#,
            "ACT 0ssw9k: no visible text content to reach")
            .variante("ohne Text"),
        negativ(id, r#"<div inert style="--scroll-overflow:100px">Long text</div>"#,
            "HTML inert: the region is not operable at all")
            .variante("inert"),
        na(id, r#"<p>Short text</p>"#, "ACT 0ssw9k: no scroll container"),
        untested(id, r#"<!DOCTYPE html><html lang="en" style="--unmeasured: scroll"><head><title>T</title></head><body><p>Text</p></body></html>"#,
            "docs/conventions.md: scroll extents not measured is not passed")
            .anzahl(1),
    ]
}
