//! Muster aus echten Stylesheets, als eigene Nachbildung auf wenige Zeilen
//! verkürzt (kein übernommener Text). Erhoben am 2026-10-03; je Fall steht
//! die Seite, auf der das Muster vorkam.

use stylesheet_parse::{Declaration, parse_stylesheet};

fn decl(name: &str, value: &str, important: bool) -> Declaration {
    Declaration {
        name: name.into(),
        value: value.into(),
        important,
    }
}

/// https://www.yahoo.com — Nesting im ausgelieferten CSS: `@media` direkt in
/// einer Stilregel mit zwei Selektoren, `!important` ohne Leerraum.
#[test]
fn nested_media_in_rule_with_selector_list() {
    let sheet = parse_stylesheet(
        "#home .box,#home [class*=stream-] .card{background-color:rgb(var(--bg-2))!important;\
         @media (min-width:640px){background-color:rgb(var(--bg-1))!important;border-radius:.5rem}}",
    );
    let rules = sheet.style_rules();
    assert_eq!(rules.len(), 2);
    assert_eq!(
        rules[1].selectors,
        ["#home .box", "#home [class*=stream-] .card"]
    );
    assert_eq!(rules[1].context.media, ["(min-width:640px)"]);
    assert_eq!(
        rules[1].declarations,
        [
            decl("background-color", "rgb(var(--bg-1))", true),
            decl("border-radius", "0.5rem", false),
        ]
    );
}

/// https://www.chefkoch.de — Svelte-Klassen, `:focus` ohne Umriss neben
/// `::-moz-focus-inner`.
#[test]
fn focus_outline_none_minified() {
    let sheet = parse_stylesheet(
        ".btn.svelte-x1{border:0}.btn.svelte-x1::-moz-focus-inner{border:0;padding:0}\
         .btn.svelte-x1:focus{outline:none}.btn.svelte-x1:focus-visible{outline:2px solid #212018;outline-offset:2px}",
    );
    let rules = sheet.style_rules();
    assert_eq!(rules[2].selectors, [".btn.svelte-x1:focus"]);
    assert_eq!(rules[2].declarations, [decl("outline", "none", false)]);
    assert_eq!(
        rules[3].declarations[0],
        decl("outline", "2px solid #212018", false)
    );
}

/// https://www.welt.de — `@media(` ohne Leerraum, universelle Selektoren mit
/// Pseudo-Elementen, `!important` mit Leerraum.
#[test]
fn reduced_motion_reset() {
    let sheet = parse_stylesheet(
        "@media(prefers-reduced-motion: reduce){body *,body ::before,body ::after\
         {animation-delay:-1ms !important;animation-duration:1ms !important}}",
    );
    let r = &sheet.style_rules()[0];
    assert_eq!(r.context.media, ["(prefers-reduced-motion: reduce)"]);
    assert_eq!(r.selectors, ["body *", "body ::before", "body ::after"]);
    assert_eq!(
        r.declarations,
        [
            decl("animation-delay", "-1ms", true),
            decl("animation-duration", "1ms", true),
        ]
    );
}

/// https://www.check24.de — `@keyframes` und Stilregeln in `@scope` mit
/// Attributselektoren im Präludium.
#[test]
fn keyframes_inside_scope() {
    let sheet = parse_stylesheet(
        "@scope ([data-slug=\"home\"][data-slide=\"lp\"]) {\n\
         .sk .skeleton{animation:pulse 1.1s ease-in-out infinite}\
         @keyframes pulse{0%{background-color:var(--dark)}50%{background-color:var(--light)}to{background-color:var(--dark)}}\
         @media (prefers-reduced-motion: reduce){.sk .skeleton{animation:none}}}",
    );
    let k = sheet.keyframes();
    assert_eq!(k.len(), 1);
    assert_eq!(k[0].0.name, "pulse");
    assert_eq!(k[0].0.frames.len(), 3);
    assert_eq!(
        k[0].1.groups,
        [("scope", "([data-slug=\"home\"][data-slide=\"lp\"])")]
    );
    let rules = sheet.style_rules();
    assert_eq!(rules[1].context.media, ["(prefers-reduced-motion: reduce)"]);
    assert_eq!(rules[1].context.groups[0].0, "scope");
    assert_eq!(rules[1].declarations, [decl("animation", "none", false)]);
}

/// https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.css —
/// maskierte Klassennamen mit `:` und `.`.
#[test]
fn tailwind_escapes() {
    let sheet = parse_stylesheet(
        r".focus\:sr-only:focus{position:absolute}.inset-0\.5{top:.125rem}@media (min-width:640px){.sm\:w-1\/2{width:50%}}",
    );
    let sel: Vec<String> = sheet
        .style_rules()
        .into_iter()
        .map(|s| s.selectors.join(","))
        .collect();
    assert_eq!(
        sel,
        [r".focus\:sr-only:focus", r".inset-0\.5", r".sm\:w-1\/2"]
    );
}

/// https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.css —
/// `An+B` mit Vorzeichen, Custom Properties, `@media` mit Bereichssyntax.
#[test]
fn bootstrap_like() {
    let sheet = parse_stylesheet(
        ".btn-group > .btn:nth-child(n+3), .x > :not(caption) > tr > :nth-child(even) { --bs-gap: .5rem; margin-left: calc(-1 * var(--bs-gap)) }\n\
         @media (prefers-reduced-motion: reduce) { .btn { transition: none; } }",
    );
    let rules = sheet.style_rules();
    assert_eq!(
        rules[0].selectors,
        [
            ".btn-group > .btn:nth-child(n+3)",
            ".x > :not(caption) > tr > :nth-child(even)"
        ]
    );
    assert_eq!(
        rules[0].declarations,
        [
            decl("--bs-gap", "0.5rem", false),
            decl("margin-left", "calc(-1 * var(--bs-gap))", false),
        ]
    );
    assert_eq!(rules[1].declarations, [decl("transition", "none", false)]);
}
