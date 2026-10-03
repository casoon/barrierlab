//! Fälle zu jedem Zweig von CSS Syntax Level 3 §5 und CSS Nesting, wie
//! `css-parse` sie umsetzt.

use css_parse::{Declaration, Rule, StyleRule, Stylesheet, parse_stylesheet};

fn style(rule: &Rule) -> &StyleRule {
    match rule {
        Rule::Style(s) => s,
        other => panic!("keine Stilregel: {other:?}"),
    }
}

fn decl(name: &str, value: &str) -> Declaration {
    Declaration {
        name: name.into(),
        value: value.into(),
        important: false,
    }
}

fn only_style(css: &str) -> StyleRule {
    let sheet = parse_stylesheet(css);
    assert_eq!(sheet.rules.len(), 1, "{sheet:?}");
    style(&sheet.rules[0]).clone()
}

// --- Stilregeln und Deklarationen -------------------------------------------

#[test]
fn simple_rule() {
    let r = only_style("a:focus { outline: none; color: red }");
    assert_eq!(r.selectors, ["a:focus"]);
    assert_eq!(
        r.declarations,
        [decl("outline", "none"), decl("color", "red")]
    );
    assert!(r.nested.is_empty());
}

#[test]
fn selector_list_split_at_top_level_commas_only() {
    let r = only_style("a,\n  b > c ,:is(d, e) f {}");
    assert_eq!(r.selectors, ["a", "b > c", ":is(d, e) f"]);
}

#[test]
fn names_lowercased_custom_properties_and_values_not() {
    let r = only_style("A { COLOR: Red; --Brand-Color: #ABC; Outline-Style: NONE }");
    assert_eq!(r.selectors, ["A"]);
    assert_eq!(
        r.declarations,
        [
            decl("color", "Red"),
            decl("--Brand-Color", "#ABC"),
            decl("outline-style", "NONE"),
        ]
    );
}

#[test]
fn important_with_space_and_case() {
    let r = only_style("a { color: red ! IMPORTANT ; b: c!important; d: e !important x }");
    assert_eq!(r.declarations.len(), 3);
    assert_eq!(r.declarations[0].value, "red");
    assert!(r.declarations[0].important);
    assert_eq!(r.declarations[1].value, "c");
    assert!(r.declarations[1].important);
    // Nicht am Ende: gehört zum Wert.
    assert_eq!(r.declarations[2].value, "e !important x");
    assert!(!r.declarations[2].important);
}

#[test]
fn value_serialization() {
    let r = only_style(
        "a { transform: rotate(45deg)  scale( 1.50 ); margin: 0 .5em -1px 10%; \
         color: #FFF; content: 'it''s \"x\"'; background: url(img/a.png) no-repeat; \
         font: 12px/1.5 \"Helvetica Neue\", sans-serif; width: calc(100% - 2*1rem); \
         grid-area: 1 / 2; z: 1e3 +2 }",
    );
    let v: Vec<&str> = r.declarations.iter().map(|d| d.value.as_str()).collect();
    assert_eq!(
        v,
        [
            "rotate(45deg) scale( 1.5 )",
            "0 0.5em -1px 10%",
            "#FFF",
            "\"it\"\"s \\\"x\\\"\"",
            "url(img/a.png) no-repeat",
            "12px/1.5 \"Helvetica Neue\", sans-serif",
            "calc(100% - 2*1rem)",
            "1 / 2",
            "1000 2",
        ]
    );
}

#[test]
fn escaped_identifiers_round_trip() {
    // Tailwind-artige Klassen.
    let r = only_style(r".md\:flex, .w-1\/2, .\31 0, #\#x { a: b }");
    assert_eq!(r.selectors, [r".md\:flex", r".w-1\/2", r".\31 0", r"#\#x"]);
}

#[test]
fn an_plus_b_keeps_its_sign() {
    let r = only_style("li:nth-child(2n+1), li:nth-child(-n+3), li:nth-child(2n - 1) {}");
    assert_eq!(
        r.selectors,
        [
            "li:nth-child(2n+1)",
            "li:nth-child(-n+3)",
            "li:nth-child(2n - 1)"
        ]
    );
}

#[test]
fn comment_between_tokens_that_would_merge() {
    let r = only_style("a { b: x/**/y; c: 1/**/-2 }");
    assert_eq!(r.declarations[0].value, "x/**/y");
    assert_eq!(r.declarations[1].value, "1/**/-2");
}

#[test]
fn whitespace_and_comments_collapse() {
    let r = only_style("a  /* c */\n\t b {  color :\n red  /* x */ ; }");
    assert_eq!(r.selectors, ["a b"]);
    assert_eq!(r.declarations, [decl("color", "red")]);
}

#[test]
fn empty_value_is_kept() {
    let r = only_style("a { --x: ; color: }");
    assert_eq!(r.declarations, [decl("--x", ""), decl("color", "")]);
}

// --- Fehlerbehandlung -------------------------------------------------------

#[test]
fn invalid_declaration_dropped_up_to_semicolon() {
    let r = only_style("a { color red; 12px: x; : y; background: blue }");
    assert_eq!(r.declarations, [decl("background", "blue")]);
}

#[test]
fn double_semicolons() {
    let r = only_style("a { ;; color: red;; ; margin: 0;; }");
    assert_eq!(r.declarations, [decl("color", "red"), decl("margin", "0")]);
}

#[test]
fn missing_closing_brace_at_eof() {
    let sheet = parse_stylesheet("@media screen { a { color: red; transform: rotate(1deg");
    let Rule::Media(m) = &sheet.rules[0] else {
        panic!()
    };
    let r = style(&m.rules[0]);
    assert_eq!(
        r.declarations,
        [decl("color", "red"), decl("transform", "rotate(1deg)")]
    );
}

#[test]
fn unclosed_prelude_block_is_closed() {
    let sheet = parse_stylesheet("@media (max-width: 600px");
    let Rule::Other(o) = &sheet.rules[0] else {
        panic!("{sheet:?}")
    };
    assert_eq!(o.prelude, "(max-width: 600px)");
}

#[test]
fn stray_closing_brace_at_top_level() {
    // Das `}` gehört zum Präludium der folgenden Regel; die ist damit
    // ungültig. Die Regel danach bleibt.
    let sheet = parse_stylesheet("a { color: red } } b { color: blue } c { color: green }");
    let sel: Vec<&Vec<String>> = sheet.rules.iter().map(|r| &style(r).selectors).collect();
    assert_eq!(sel, [&vec!["a".to_string()], &vec!["c".to_string()]]);
}

#[test]
fn garbage_does_not_take_down_the_sheet() {
    let sheet = parse_stylesheet("}}}{{{ ]]) ;; @ # ! a { color: red } ");
    // Alles nach dem ersten `{` ist ein einziger unvollständiger Block.
    assert!(sheet.rules.len() <= 2);
    let sheet = parse_stylesheet("a { color: red } ; b { color: blue }");
    // `; b` ist das Präludium der zweiten Regel — kein gültiger Selektor
    // ist es nicht, aber syntaktisch bleibt es eine Regel.
    assert_eq!(style(&sheet.rules[1]).selectors, ["; b"]);
}

#[test]
fn bad_tokens_drop_declaration_and_rule() {
    let r = only_style("a { content: \"abc\n; color: red; background: url(a b); x: y) }");
    assert_eq!(r.declarations, [decl("color", "red")]);
    let sheet = parse_stylesheet("a[title=\"x\n] { color: red } b { color: blue }");
    assert!(
        sheet
            .style_rules()
            .iter()
            .all(|s| !s.selectors[0].starts_with('a'))
    );
}

#[test]
fn empty_selector_part_invalidates_rule() {
    let sheet = parse_stylesheet("a,, b { color: red } c, { color: blue } d { }");
    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(style(&sheet.rules[0]).selectors, ["d"]);
}

#[test]
fn cdo_cdc_ignored_at_top_level() {
    let sheet = parse_stylesheet("<!-- a { color: red } --> <!-- b { c: d } -->");
    let sel: Vec<String> = sheet
        .rules
        .iter()
        .map(|r| style(r).selectors.join(","))
        .collect();
    assert_eq!(sel, ["a", "b"]);
}

#[test]
fn custom_property_lookalike_rule_dropped() {
    let sheet = parse_stylesheet("--x: { a: b } c { d: e }");
    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(style(&sheet.rules[0]).selectors, ["c"]);
}

#[test]
fn curly_block_mixed_into_value_is_a_rule() {
    // `a:hover { … }` in einem Block ist keine Deklaration `a: hover {…}`.
    let r = only_style("p { a:hover { color: red } color: blue }");
    assert_eq!(r.declarations, [decl("color", "blue")]);
    assert_eq!(style(&r.nested[0]).selectors, ["a:hover"]);
}

#[test]
fn no_panic_on_pathological_input() {
    let deep_blocks = "@media x{".repeat(100_000);
    let deep_parens = "a{b:".to_string() + &"(".repeat(100_000);
    let deep_nesting = "a{".repeat(100_000);
    let amp_bomb = "a,b{".to_string() + &"& & & &{".repeat(200);
    let list_bomb = "a{".to_string() + &"&,&,&{".repeat(120);
    for css in [
        "",
        "{",
        "}",
        "@",
        "@media",
        "a",
        "a{",
        "a{b",
        "a{b:",
        "a{b:c!",
        "\\",
        "a{b:\\\n}",
        "url(",
        "\"",
        "\u{0}",
        &deep_blocks,
        &deep_parens,
        &deep_nesting,
        &amp_bomb,
        &list_bomb,
    ] {
        let sheet = parse_stylesheet(css);
        let _ = sheet.style_rules();
        let _ = sheet.keyframes();
    }
}

#[test]
fn depth_limit_drops_only_the_too_deep_rule() {
    let css = "a{".repeat(200) + &"}".repeat(200) + " z { color: red }";
    let sheet = parse_stylesheet(&css);
    let rules = sheet.style_rules();
    assert_eq!(rules.last().unwrap().selectors, ["z"]);
}

// --- At-Regeln --------------------------------------------------------------

#[test]
fn media_rule() {
    let sheet = parse_stylesheet(
        "@MEDIA screen and (prefers-reduced-motion:reduce) { * { animation: none } }",
    );
    let Rule::Media(m) = &sheet.rules[0] else {
        panic!()
    };
    assert_eq!(m.condition, "screen and (prefers-reduced-motion:reduce)");
    assert_eq!(style(&m.rules[0]).selectors, ["*"]);
}

#[test]
fn declarations_directly_in_top_level_media_are_dropped() {
    let sheet = parse_stylesheet("@media print { color: red; a { b: c } }");
    let Rule::Media(m) = &sheet.rules[0] else {
        panic!()
    };
    assert_eq!(m.rules.len(), 1);
    assert_eq!(style(&m.rules[0]).selectors, ["a"]);
}

#[test]
fn keyframes_and_vendor_prefixes() {
    let sheet = parse_stylesheet(
        "@keyframes Spin { FROM { transform: rotate(0) } 50%, 75% { opacity: .5 } to { transform: rotate(360deg) } }
         @-webkit-keyframes spin { 0% { opacity: 0 } }
         @keyframes \"quoted name\" { }",
    );
    let k = sheet.keyframes();
    assert_eq!(k.len(), 3);
    assert_eq!(k[0].0.name, "Spin");
    assert_eq!(k[0].0.vendor, None);
    let sels: Vec<&Vec<String>> = k[0].0.frames.iter().map(|f| &f.selectors).collect();
    assert_eq!(
        sels,
        [
            &vec!["from".to_string()],
            &vec!["50%".to_string(), "75%".to_string()],
            &vec!["to".to_string()],
        ]
    );
    assert_eq!(k[0].0.frames[1].declarations, [decl("opacity", "0.5")]);
    assert_eq!(k[1].0.name, "spin");
    assert_eq!(k[1].0.vendor.as_deref(), Some("webkit"));
    assert_eq!(k[2].0.name, "quoted name");
    // Keyframes erscheinen nicht unter den Stilregeln.
    assert!(sheet.style_rules().is_empty());
}

#[test]
fn keyframes_in_media_have_context() {
    let sheet =
        parse_stylesheet("@media (prefers-reduced-motion: no-preference) { @keyframes a { } }");
    let k = sheet.keyframes();
    assert_eq!(k[0].1.media, ["(prefers-reduced-motion: no-preference)"]);
}

#[test]
fn layer_statement_vs_block() {
    let sheet = parse_stylesheet("@layer base, components; @layer base { a { b: c } } @layer;");
    match &sheet.rules[0] {
        Rule::Other(o) => {
            assert_eq!(o.name, "layer");
            assert_eq!(o.prelude, "base, components");
        }
        r => panic!("{r:?}"),
    }
    match &sheet.rules[1] {
        Rule::Group(g) => {
            assert_eq!(g.name, "layer");
            assert_eq!(g.prelude, "base");
            assert_eq!(style(&g.rules[0]).selectors, ["a"]);
        }
        r => panic!("{r:?}"),
    }
    assert!(matches!(&sheet.rules[2], Rule::Other(o) if o.prelude.is_empty()));
}

#[test]
fn import_charset_font_face_page_are_other() {
    let sheet = parse_stylesheet(
        "@charset \"utf-8\"; @import url(a.css) screen; @IMPORT \"b.css\" layer(x);
         @font-face { font-family: X; src: url(x.woff2) format(\"woff2\") }
         @page :first { margin: 1cm; @top-left { content: 'x' } }",
    );
    let others: Vec<(&str, &str, usize)> = sheet
        .rules
        .iter()
        .map(|r| match r {
            Rule::Other(o) => (o.name.as_str(), o.prelude.as_str(), o.declarations.len()),
            r => panic!("{r:?}"),
        })
        .collect();
    assert_eq!(
        others,
        [
            ("charset", "\"utf-8\"", 0),
            ("import", "url(a.css) screen", 0),
            ("import", "\"b.css\" layer(x)", 0),
            ("font-face", "", 2),
            ("page", ":first", 1),
        ]
    );
}

#[test]
fn groups_and_media_nest_outer_to_inner() {
    let sheet = parse_stylesheet(
        "@supports (display: grid) { @layer l { @media (min-width: 1px) { @container card (width > 1px) { a { b: c } } } } }",
    );
    let r = &sheet.style_rules()[0];
    assert_eq!(r.selectors, ["a"]);
    assert_eq!(r.context.media, ["(min-width: 1px)"]);
    assert_eq!(
        r.context.groups,
        [
            ("supports", "(display: grid)"),
            ("layer", "l"),
            ("container", "card (width > 1px)"),
        ]
    );
}

#[test]
fn other_group_names() {
    let sheet = parse_stylesheet(
        "@scope (.card) to (.content) { img { a: b } } @starting-style { a { b: c } } \
         @-moz-document url-prefix() { a { b: c } } @document url(x) { a { b: c } }",
    );
    let names: Vec<&str> = sheet
        .rules
        .iter()
        .map(|r| match r {
            Rule::Group(g) => g.name.as_str(),
            r => panic!("{r:?}"),
        })
        .collect();
    assert_eq!(
        names,
        ["scope", "starting-style", "-moz-document", "document"]
    );
}

// --- Verschachtelung --------------------------------------------------------

fn flat(sheet: &Stylesheet) -> Vec<(String, Vec<&str>)> {
    sheet
        .style_rules()
        .into_iter()
        .map(|s| (s.selectors.join(" | "), s.context.media.clone()))
        .collect()
}

#[test]
fn nesting_resolution() {
    let sheet = parse_stylesheet(
        ".a {
           color: red;
           &:focus { outline: none }
           .b { x: y }
           > .c { x: y }
           .d & { x: y }
           & + & { x: y }
         }",
    );
    let f: Vec<String> = flat(&sheet).into_iter().map(|(s, _)| s).collect();
    assert_eq!(
        f,
        [
            ".a",
            ".a:focus",
            ".a .b",
            ".a > .c",
            ".d :is(.a)",
            ".a + :is(.a)",
        ]
    );
}

#[test]
fn nesting_with_multiple_parents_uses_is() {
    let sheet = parse_stylesheet("a, b { &:hover { x: y } c { x: y } }");
    let f: Vec<String> = flat(&sheet).into_iter().map(|(s, _)| s).collect();
    assert_eq!(f, ["a | b", ":is(a, b):hover", ":is(a, b) c"]);
}

#[test]
fn deep_nesting_resolves_through_levels() {
    let sheet = parse_stylesheet(".a { .b { &:hover { .c { x: y } } } }");
    let f: Vec<String> = flat(&sheet).into_iter().map(|(s, _)| s).collect();
    assert_eq!(f, [".a", ".a .b", ".a .b:hover", ".a .b:hover .c"]);
}

#[test]
fn nested_media_applies_to_parent_selector() {
    let sheet = parse_stylesheet(
        ".btn { color: red; @media (prefers-reduced-motion: reduce) { transition: none; &:hover { x: y } } }",
    );
    let f = flat(&sheet);
    assert_eq!(
        f,
        [
            (".btn".to_string(), vec![]),
            (".btn".to_string(), vec!["(prefers-reduced-motion: reduce)"]),
            (
                ".btn:hover".to_string(),
                vec!["(prefers-reduced-motion: reduce)"]
            ),
        ]
    );
    let rules = sheet.style_rules();
    assert_eq!(rules[1].declarations, [decl("transition", "none")]);
}

#[test]
fn nested_non_group_at_rules_are_dropped() {
    let r = only_style("a { @font-face { x: y } @keyframes k { } color: red }");
    assert!(r.nested.is_empty());
    assert_eq!(r.declarations, [decl("color", "red")]);
}

#[test]
fn nested_rule_stops_at_semicolon() {
    // In einem Block endet eine verunglückte qualifizierte Regel am `;`.
    let r = only_style("a { 1px; color: red }");
    assert_eq!(r.declarations, [decl("color", "red")]);
    assert!(r.nested.is_empty());
}

#[test]
fn top_level_ampersand_is_left_alone() {
    let sheet = parse_stylesheet("& { a: b }");
    assert_eq!(sheet.style_rules()[0].selectors, ["&"]);
}

#[test]
fn no_panic_on_fragment_soup() {
    // Deterministischer Zufall (LCG) über Bruchstücke, die die Zweige des
    // Parsers treffen.
    const PARTS: &[&str] = &[
        "{",
        "}",
        "(",
        ")",
        "[",
        "]",
        ";",
        ":",
        "@media",
        "@keyframes",
        "@layer",
        "a",
        "&",
        "!",
        "important",
        ",",
        "\"",
        "'",
        "\\",
        "url(",
        "/*",
        "*/",
        "--x",
        "\n",
        "1.5e3",
        "#",
        ".",
        " ",
        "<!--",
        "-->",
        "-",
        "+",
        "e",
        "\u{0}",
    ];
    let mut seed: u64 = 0x2026_1003;
    for _ in 0..5000 {
        let mut css = String::new();
        for _ in 0..40 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            css.push_str(PARTS[(seed >> 33) as usize % PARTS.len()]);
        }
        let sheet = parse_stylesheet(&css);
        for s in sheet.style_rules() {
            // Was herauskommt, lässt sich erneut parsen.
            let again = parse_stylesheet(&format!("{} {{}}", s.selectors.join(",")));
            let _ = again.style_rules();
        }
        let _ = sheet.keyframes();
    }
}
