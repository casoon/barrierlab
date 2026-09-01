//! Phase 07: differential test against the vendored vnu test corpus
//! (`tests/corpus/`) — see `plan/07-corpus-differential.md` and
//! `plan/DECISIONS.md` (the Phase 07 entry) for the expectation model,
//! corpus scope, and how the baseline below was established.
//!
//! Only uses `html_conform`'s public API (`check`), the same as any real
//! consumer would — this is a black-box comparison, not a unit test.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use html_conform::check;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

/// vnu's own expected-message map, trimmed to `html/`/`html-aria/` keys by
/// `xtask/vendor-corpus.sh`. Only key *presence* is used — message text
/// isn't compared (different implementations, different wording; see
/// `plan/07-corpus-differential.md`'s "Risiken" and `tests/corpus/README.md`).
fn expected_findings_by_path() -> HashMap<String, String> {
    let raw = fs::read_to_string(corpus_dir().join("messages.json"))
        .expect("tests/corpus/messages.json should be vendored — run xtask/vendor-corpus.sh");
    serde_json::from_str(&raw).expect("tests/corpus/messages.json should be valid JSON")
}

/// Recursively collects every `*.html` fixture under `dir`, as paths
/// relative to `corpus_root` (matching `messages.json`'s keys, e.g.
/// `html/parser/foo-novalid.html`).
fn collect_html_fixtures(dir: &Path, corpus_root: &Path, out: &mut Vec<String>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry should be readable").path();
        if path.is_dir() {
            collect_html_fixtures(&path, corpus_root, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "html")
        {
            let relative = path
                .strip_prefix(corpus_root)
                .expect("fixture path should be under the corpus root")
                .to_str()
                .expect("fixture path should be valid UTF-8")
                .replace('\\', "/"); // normalize on the off chance of a Windows path separator
            out.push(relative);
        }
    }
}

#[derive(Default)]
struct Metrics {
    /// Expected findings, `check()` found some — agreement.
    true_positive: usize,
    /// Expected clean, `check()` found none — agreement.
    true_negative: usize,
    /// Expected clean, `check()` found something — this crate is stricter
    /// than vnu here (or has a bug).
    false_positive: usize,
    /// Expected findings, `check()` found none — this crate is missing a
    /// check vnu has (expected in bulk for `html-aria/`: Phase 08 hasn't
    /// built out the real ARIA rule set yet, see `plan/DECISIONS.md`).
    false_negative: usize,
    /// `check()` itself returned `Err` (a setup failure, not a per-document
    /// verdict) — not counted as agreement or disagreement.
    not_comparable: usize,
}

/// `#[ignore]`d: 4655 real `check()` calls take ~37s in `--release` but
/// several minutes in an unoptimized debug build (relax-ng's derivative
/// validation and the HTML5 parser both benefit heavily from
/// optimization) — too slow for routine `cargo test`. Run explicitly with
/// `cargo test --release --test differential -- --ignored`; CI
/// (`.github/workflows/ci.yml`) runs it that way on every push.
#[test]
#[ignore = "slow (~4655 real checks) — run with `cargo test --release --test differential -- --ignored`"]
fn differential_against_vendored_vnu_corpus() {
    let expected = expected_findings_by_path();
    let corpus_root = corpus_dir();

    let mut fixtures = Vec::new();
    collect_html_fixtures(&corpus_root.join("html"), &corpus_root, &mut fixtures);
    collect_html_fixtures(&corpus_root.join("html-aria"), &corpus_root, &mut fixtures);
    fixtures.sort();

    assert!(
        fixtures.len() > 4000,
        "expected the full vendored corpus (~4655 fixtures), found {} \
         — is tests/corpus/ vendored? (run xtask/vendor-corpus.sh)",
        fixtures.len()
    );

    let mut metrics = Metrics::default();
    let mut false_positive_examples = Vec::new();
    let mut false_negative_examples = Vec::new();
    let mut not_comparable_examples = Vec::new();

    for relative_path in &fixtures {
        let html = fs::read_to_string(corpus_root.join(relative_path))
            .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"));
        let expects_findings = expected.contains_key(relative_path);

        match check(&html) {
            Ok(report) => {
                let has_findings = !report.findings.is_empty();
                match (expects_findings, has_findings) {
                    (true, true) => metrics.true_positive += 1,
                    (false, false) => metrics.true_negative += 1,
                    (false, true) => {
                        metrics.false_positive += 1;
                        if false_positive_examples.len() < 20 {
                            false_positive_examples.push(relative_path.clone());
                        }
                    }
                    (true, false) => {
                        metrics.false_negative += 1;
                        if false_negative_examples.len() < 20 {
                            false_negative_examples.push(relative_path.clone());
                        }
                    }
                }
            }
            Err(_) => {
                metrics.not_comparable += 1;
                if not_comparable_examples.len() < 20 {
                    not_comparable_examples.push(relative_path.clone());
                }
            }
        }
    }

    println!(
        "differential corpus results ({} fixtures): true_positive={} true_negative={} \
         false_positive={} false_negative={} not_comparable={}",
        fixtures.len(),
        metrics.true_positive,
        metrics.true_negative,
        metrics.false_positive,
        metrics.false_negative,
        metrics.not_comparable
    );
    if !false_positive_examples.is_empty() {
        println!(
            "false positive examples (expected clean, got findings): {false_positive_examples:?}"
        );
    }
    if !false_negative_examples.is_empty() {
        println!(
            "false negative examples (expected findings, got clean): {false_negative_examples:?}"
        );
    }
    if !not_comparable_examples.is_empty() {
        println!("not-comparable examples (check() returned Err): {not_comparable_examples:?}");
    }

    // Baseline / regression gate (plan/07-corpus-differential.md,
    // plan/DECISIONS.md's Phase 07 entry): this corpus is real-world vnu
    // test material exercising far more than this crate currently
    // implements (Phase 08's real ARIA assertion rules in particular are
    // still four reused canary placeholders, not the full rule set) —
    // 100% agreement isn't the bar yet. This only asserts the checker
    // never gets WORSE than the first recorded run. Update the constants
    // below deliberately (with a `DECISIONS.md` entry explaining what
    // improved) when the checker demonstrably gets better — never to
    // silently paper over a regression.
    // `<=` (not `==`): keeps this a general-purpose "never worse than the
    // baseline" gate rather than an exact-match assertion, even now that
    // the baseline has reached 0 (`usize`'s minimum, which is why this
    // needs an explicit clippy allow — the comparison stops being
    // "absurd" the moment a regression bumps `false_positive` back above
    // 0, clippy just can't see that).
    #[allow(clippy::absurd_extreme_comparisons)]
    let false_positive_within_baseline = metrics.false_positive <= BASELINE_FALSE_POSITIVE;
    assert!(
        false_positive_within_baseline,
        "false positives regressed: {} > baseline {}",
        metrics.false_positive, BASELINE_FALSE_POSITIVE
    );
    assert!(
        metrics.false_negative <= BASELINE_FALSE_NEGATIVE,
        "false negatives regressed: {} > baseline {}",
        metrics.false_negative,
        BASELINE_FALSE_NEGATIVE
    );
    assert_eq!(
        metrics.not_comparable, BASELINE_NOT_COMPARABLE,
        "not-comparable (CheckError) count changed from the baseline — investigate \
         before adjusting the baseline, this means the checker itself broke on some \
         input, not that it disagreed with vnu"
    );
}

/// Baseline snapshot from 2026-08-23 (vnu commit `388cb36`, see
/// `tests/corpus/manifest.json`) — 4655 fixtures. Original Phase 07
/// baseline: `true_positive=2066 true_negative=666 false_positive=243
/// false_negative=1680 not_comparable=0`. Updated the same day (Phase 08
/// start) after `w:iri-ref`/`w:iri`/`w:iri-ref-http-or-https`
/// (`src/datatypes/network.rs`) gained a stricter raw-syntax pre-check —
/// the `url` crate's WHATWG-URL parser alone was far more lenient than
/// vnu's Galimatias-based grammar check (silently normalized internal
/// whitespace, hex/octal IPv4 hosts, and a second `#` instead of
/// rejecting them): `true_positive=2689 true_negative=666
/// false_positive=243 false_negative=1057 not_comparable=0` — 623 fewer
/// false negatives, zero new false positives. Updated again the same day
/// after the first real Schematron rule batch (`rules/ids.sch`,
/// `microdata.sch`, `headings.sch`, `roles.sch`, `elements.sch`, plus one
/// added rule in `aria.sch`) covering the `html/assertions` and
/// `html/warnings` diff groups: `true_positive=2747 true_negative=666
/// false_positive=243 false_negative=999 not_comparable=0` — 58 fewer
/// false negatives, zero new false positives. Updated again the same day
/// after widening `forms.input-needs-accessible-name`
/// (`rules/forms.sch`) to also accept a non-empty `title` and an
/// associated `<label>` (wrapping or `for`/`id`) as accessible-name
/// sources — it previously only checked aria-label/aria-labelledby, a
/// known Phase 06 placeholder gap. This surfaced that some fixtures
/// were true positives only by accident (the over-broad rule fired for
/// the wrong reason on documents that have a *different*, still-unbuilt
/// ARIA violation, e.g. `presentational-children` role constraints) —
/// so alongside the expected false_positive drop, false_negative rose
/// too as those accidental hits stopped. Net effect is a real
/// improvement (true_negative +36, matched by true_positive -8 exposing
/// genuinely separate, unrelated gaps — verified by inspecting several
/// of the newly-appearing false negatives): `true_positive=2739
/// true_negative=702 false_positive=207 false_negative=1007
/// not_comparable=0`. Updated again the same day after a new
/// `rules/aria-constraints.sch` covering 28 confirmed cases across
/// `html-aria/author-requirements` (group role-nesting constraints),
/// `html-aria/presentation-role` (img role=presentation + empty alt +
/// any aria-* attribute), `html-aria/presentational-children` (label
/// must not descend from separator/progressbar/img/slider/math roles),
/// and `html-aria/roles-properties-global` (aria-disabled/aria-haspopup/
/// aria-invalid forbidden on role=main; aria-hidden=true forbidden on
/// body) — every condition verified against the actual corpus fixtures
/// before being encoded, not extrapolated from the wider ARIA spec:
/// `true_positive=2767 true_negative=702 false_positive=207
/// false_negative=979 not_comparable=0` — 28 fewer false negatives,
/// zero new false positives. Updated again the same day after fixing a
/// severe, previously undiscovered bug: a plain `lang` or literal
/// `xml:lang` attribute — among the single most common HTML
/// attributes — made `relax_ng::Schema::validate` reject or misreport
/// on nearly *every* real-world document (`src/infoset.rs`'s
/// `relax_ng::Element::attributes()` now remaps both onto the
/// namespace-split form the compiled schema's `attribute xml:lang {
/// ... }` pattern actually expects — see that method's doc comment for
/// the full root cause). Fixing it dropped `false_positive` sharply
/// (207→41) but also more than doubled `false_negative` (979→1410):
/// spot-checking several of the newly-appearing false negatives (e.g.
/// `html-aria/misc/aria-braillelabel-a-novalid.html`,
/// `html-aria/misc/a-href-inside-role-checkbox-haswarn.html`) confirmed
/// these are genuine, pre-existing, unimplemented ARIA co-constraints
/// that were only ever "passing" by accident — nearly every fixture
/// carries `<html lang="...">`, so the old `lang` bug fired a spurious
/// `schema.html5` error on almost all of them regardless of what the
/// fixture actually tested, coincidentally counting as a true positive
/// with a message unrelated to the real expected violation. Fixing the
/// bug removed that noise and exposed how much of `html-aria/misc` in
/// particular has no real rule behind it yet — a large, genuine
/// follow-up scope for this rule loop, not a regression from this fix.
/// New baseline: `true_positive=2336 true_negative=868
/// false_positive=41 false_negative=1410 not_comparable=0`. See
/// `plan/DECISIONS.md`'s Phase 07 and Phase 08 entries for the full
/// breakdown, spot-checked examples, and what's still open
/// (special-scheme slash requirements, userinfo character-class
/// validation, host hex/octal/fullwidth-digit IPv4 forms in
/// `w:iri-ref` — a documented, accepted residual gap; `<meter>`
/// numeric range constraints and `meta-charset-not-utf8` deliberately
/// deferred; `html/media-queries` traced to a confirmed leniency gap in
/// the `media-query-parse` sister crate, deferred — see its Phase 08
/// entry; `html/parser` likely an `xmloxide` diagnostic-coverage gap;
/// and now the much larger, newly-visible `html-aria/misc` ARIA-in-HTML
/// restriction rule gap as the next rule-loop target). Updated again
/// the same day after `rules/aria-html-restrictions.sch` (new) covered
/// most of that `html-aria/misc` gap — 20+ confirmed constraints,
/// including a 31-element "naming prohibited unless role overrides the
/// implicit role" rule, three widget-role containment/ancestor
/// families, and several one-off attribute co-constraints. Three bugs
/// found and fixed during this batch by checking the differential
/// test's own false-positive delta (not just the false-negative drop):
/// `a`/`area` need `[not(@href)]` in the naming-prohibited element list
/// (an `a`/`area` *with* `href` has implicit role "link", which does
/// allow naming — only the hrefless form falls back to "generic");
/// `role="none"`/`"presentation"` needed to be an always-allowed escape
/// hatch in the `li`-by-ancestor-role family, not just the enumerated
/// widget-item roles; and the containment rules (`role=cell`/`option`/
/// `row` needing a specific role ancestor) needed to also accept
/// `aria-owns`-based ownership, not just DOM ancestry — the message
/// text itself says "contained in, *or owned by*". New baseline:
/// `true_positive=2498 true_negative=868 false_positive=41
/// false_negative=1248 not_comparable=0` — 162 fewer false negatives,
/// zero new false positives (back to the 41-baseline noise floor). See
/// `plan/DECISIONS.md`'s Phase 07 and Phase 08 entries for the full
/// breakdown, spot-checked examples, and what's still open
/// (special-scheme slash requirements, userinfo character-class
/// validation, host hex/octal/fullwidth-digit IPv4 forms in
/// `w:iri-ref` — a documented, accepted residual gap; `<meter>`
/// numeric range constraints and `meta-charset-not-utf8` deliberately
/// deferred; `html/media-queries` traced to a confirmed leniency gap in
/// the `media-query-parse` sister crate, deferred — see its Phase 08
/// entry; `html/parser` likely an `xmloxide` diagnostic-coverage gap;
/// a handful of remaining `html-aria/misc` one-offs like
/// `img-role-no-alt`/`role-tab-with-no-role-tabpanel` not yet covered).
/// Updated again the same day after three more fixes: (1) `data-*`
/// custom attributes are now exempted from `relax_ng::Element::
/// attributes()` (schema layer only) — RELAX NG's `NameClass` has no
/// prefix-wildcard concept, so the vendored schema unconditionally
/// rejected every `data-*` attribute; vnu special-cases these the same
/// way outside its own grammar (`src/infoset.rs`, same adapter as the
/// `lang`/`xml:lang` fix). (2) `forms.input-needs-accessible-name`
/// widened once more: `type=image` counts its `alt` as an accessible
/// name (it renders as an image submit button). (3) New
/// `rules/attributes.sch`: `lang`/`xml:lang` sameness (the schema's own
/// "Sameness check left to Schematron" comment, finally implementable
/// now that both are corpus-confirmed reachable), a `headingoffset`
/// numeric range check, and two "ID reference must exist" checks
/// (`commandfor`, `aria-activedescendant`). Explicitly investigated and
/// rejected: a general "missing lang attribute" warning — the
/// triggering fixture's own comment warns it's tied to a hardcoded
/// single-file exception in vnu's test runner, and 799 other vendored
/// fixtures have no `lang` attribute at all and are expected clean,
/// confirming a general rule would be a large regression, not a fix.
/// `false_positive` improved again (41→31); `false_negative` also rose
/// (1248→1310) — spot-checked two of the newly-appearing cases
/// (`html-aria/presentational-children/img-button-descendant-haswarn.html`,
/// `html-aria/misc/aria-braillelabel-header-not-scoped-to-body-novalid.html`)
/// and confirmed the same "accidentally-masked true positive" pattern
/// as the earlier `forms.sch`/`lang` fixes, not a regression: both are
/// genuine, separate, still-unimplemented gaps (a button-inside-
/// role=img ARIA constraint; a parser-diagnostic "stray end tag") that
/// were only ever passing because an unrelated, now-fixed false
/// positive happened to also fire on the same fixture. New baseline:
/// `true_positive=2436 true_negative=878 false_positive=31
/// false_negative=1310 not_comparable=0`. Updated again the same day
/// after discovering `forms.sch`'s entire `forms.input-needs-
/// accessible-name` rule had no basis in the real vnu corpus at all —
/// it was a Phase 02 dependency-spike canary case that was never
/// corpus-verified, and by this point was the largest single
/// false-positive contributor (25 of 31 fixtures). An exhaustive
/// `messages.json` search for any phrasing of "input needs an
/// accessible name" (and synonyms) turned up zero matches — vnu has no
/// such rule for `<input>` at all. `rules/forms.sch` deleted entirely
/// (not narrowed — there was nothing real to narrow to). The only real
/// "needs an accessible name" requirements in the corpus are about
/// `<img>`: `img[@role]` and `img[@aria-* other than aria-hidden]`,
/// both newly added to `rules/aria-constraints.sch`
/// (`img-role-needs-accessible-name`, `img-aria-needs-accessible-name`).
/// `false_positive` dropped sharply (31→11); `false_negative` rose
/// again (1310→1388) — spot-checked several of the newly-appearing
/// cases (e.g. `html-aria/misc/input-checkbox-aria-checked-novalid.html`,
/// `html-aria/host-language/implicit-semantics-checkbox-disparity-novalid.html`)
/// and confirmed the same "accidentally-masked true positive" pattern
/// as every prior fix in this series: genuine, separate, still-
/// unimplemented ARIA constraints that only ever passed because the
/// fictional rule happened to also fire on the same `<input>` element.
/// New baseline: `true_positive=2358 true_negative=898
/// false_positive=11 false_negative=1388 not_comparable=0`. Updated
/// again the same day after discovering embedded `<svg>`/`<math>` was
/// *unconditionally* rejected — even a completely empty `<svg></svg>`
/// failed schema validation. Root cause: `schema/html5/*.rnc` mentions
/// neither namespace anywhere (full-text search, zero hits), because
/// vnu's real `html5.rnc` doesn't `include` an SVG/MathML content-model
/// module either — `xtask/vendor-schema.sh` vendors exactly the modules
/// reachable from `html5.rnc`'s own `include` graph, so this isn't a
/// vendoring omission, it's a genuine gap in what this crate's schema
/// layer can express; vnu evidently validates embedded SVG/MathML
/// through some separate mechanism not vendored here. Rejecting every
/// document containing `<svg>`/`<math>` is strictly worse than
/// accepting their content unchecked, so `src/infoset.rs`'s
/// `merge_text_and_comment_runs` (the same schema-layer-only adapter
/// used by the `lang`/`data-*` fixes) now skips SVG/MathML element
/// subtrees entirely — invisible to `relax_ng::Schema::validate`, like
/// a `Comment` — while the XPath/assertions layer keeps seeing them
/// unchanged. A real, dedicated SVG/MathML schema is future work, not
/// faked here. `false_positive` dropped further (11→8); `false_negative`
/// rose slightly (1388→1396) — spot-checked the new
/// `html-aria/misc/role-on-math-element-haswarn.html` and confirmed the
/// same "accidentally-masked true positive" pattern as every other fix
/// in this series (a genuine, separate, still-unimplemented
/// redundant-role-on-math rule). Baseline at that point: `true_positive=2350
/// true_negative=901 false_positive=8 false_negative=1396
/// not_comparable=0`.
///
/// Updated once more the same day: `src/parse.rs`/`src/infoset.rs`
/// migrated from `xmloxide` to `html5-parser` (a sibling crate built
/// specifically to replace it, see `plan/DECISIONS.md`'s Phase 08
/// migration entry) — html5-parser now tracks WHATWG parse errors
/// itself (50 of the 52 named kinds, see its own `plan/07-parse-errors.md`),
/// closing the interim `parser.html5`-diagnostics gap xmloxide left, and
/// also supplies real per-node source positions for the first time
/// (`Finding.location` was `None` unconditionally before this — both for
/// `parser.html5` findings, now from `html5_parser::ParseError::position`,
/// and for `assertion.*` findings, now from `schematron_engine::Report::node`'s
/// own position — `schema.html5` findings still can't populate `location`,
/// for an unrelated, narrower reason: `relax_ng::ValidationError::location()`
/// only exposes an already-formatted string, not structured `{ line,
/// column, byte_offset }` — see `src/schema.rs`'s comment on that field).
/// `false_positive`/`not_comparable` unchanged (zero regressions across
/// all 4655 fixtures); `true_positive` rose slightly (+5, `false_negative`
/// −5) from html5-parser's newly tracked parse errors catching a few more
/// real cases. Baseline at that point: `true_positive=2355
/// true_negative=901 false_positive=8 false_negative=1391
/// not_comparable=0`.
///
/// Updated once more the same day: real WHATWG-parsing custom elements
/// (`<my-widget>`, `<view-source>`, ...) were being rejected outright —
/// `schema/html5/web-components.rnc` doesn't match custom elements by
/// "any hyphenated tag name" but by a synthetic vnu-only namespace
/// (`element c:* { ... }`, `namespace c =
/// "http://n.validator.nu/custom-elements/"` — RELAX NG's `NameClass`
/// has no prefix-wildcard concept, same structural reason as the
/// earlier `data-*` fix). `src/infoset.rs`'s `relax_ng::Element::name()`
/// now remaps a valid custom element name (reusing
/// `src/datatypes/structural.rs::check_custom_element_name`, the same
/// vnu-parity-verified check already used for the `is=""` attribute
/// value) onto that namespace, schema-layer-only — the XPath-facing view
/// keeps the real XHTML namespace unchanged, matching actual HTML5
/// parsing (there's no such thing as a "custom element namespace" in
/// the DOM). Also extended `rules/aria-html-restrictions.sch`'s
/// `naming-prohibited` rule to cover custom elements too (their implicit
/// role is also "generic") — confirmed by
/// `html-aria/misc/aria-braillelabel-autonomous-custom-element-novalid.html`
/// and its aria-label/aria-labelledby siblings, unmasked by the same fix
/// (the old schema rejection was giving these an accidental true
/// positive for the wrong reason, same pattern as every other fix in
/// this series). `false_positive` drops sharply (8→4, all four
/// custom-element cases); `false_negative` net +2 (three of the five
/// newly-unmasked cases recovered by the `naming-prohibited` extension,
/// two genuinely separate and still open — verified, not a regression).
/// New baseline: `true_positive=2353 true_negative=905
/// false_positive=4 false_negative=1393 not_comparable=0`.
///
/// Updated again the same day after closing all four remaining
/// `false_positive` cases plus several `false_negative`s they unmasked:
/// (1) `th[abbr]` — `schema/html5/tables.rnc` defines `tables.attrs.abbr`
/// but never wires it into `th.attrs` (confirmed byte-identical against
/// the live `validator/validator` source at the pinned commit, not a
/// vendoring gap); vnu's real default schema entry point
/// (`http://s.validator.nu/html5-all.rnc`, used by its own test runner)
/// resolves through undared driver files (`schema/.drivers/`) that patch
/// this in — outside what this crate vendors, same class of issue as
/// `data-*`. `src/infoset.rs`'s `relax_ng::Element::attributes()` now
/// drops a `th` element's `abbr` attribute schema-layer-only (`abbr` on
/// `td` stays unhandled — vnu treats that as obsolete-with-*warning*, a
/// separate, un-evidenced concern). (2) `integrity` attribute value —
/// `src/datatypes/network.rs`'s `is_valid_base64_shape` required a
/// multiple-of-4 length; vnu delegates to `htmlunit-csp`'s
/// `Hash.parseHash`/`IS_BASE64_VALUE` (`[a-zA-Z0-9+/\-_]+=?=?`), which has
/// no such requirement and also accepts base64url characters — both
/// relaxed. (3) `sizes` with CSS comments — `src/datatypes/misc.rs`'s
/// `check_source_size_list` now strips a *leading and/or trailing* CSS
/// comment per entry (`strip_surrounding_css_comments`); an *interior*
/// comment (`+/**/50vw`) is deliberately left alone since it's a genuine
/// CSS tokenizer boundary splitting one token into two, still correctly
/// rejected (confirmed against the sibling `-after-plus-novalid.html`
/// fixture in the same corpus directory). (4) `<select><button>
/// <selectedcontent>` "unexpected text" — actually a genuine
/// `html5-parser` behavior, not a schema gap: its tree builder correctly
/// implements WHATWG's "maybe clone an option into selectedcontent"
/// *DOM insertion step* (real, spec-correct live-browser behavior), but
/// vnu's own (purely static) parser never runs that step, and authors
/// always write `<selectedcontent>` empty in source
/// (`selectedcontent.inner = ( empty )`) — so any child it ends up with
/// post-parse is always a synthesized mirror, never literal content.
/// `src/infoset.rs`'s `relax_ng::Element::children()` now discards a
/// `<selectedcontent>` element's children unconditionally, schema-layer
/// only.
///
/// Unmasking, verified case by case (same unmasking pattern as every
/// other fix this series): the base64/CSS-comment relaxations exposed
/// that six `integrity`-in-wrong-context fixtures and one `sizes`
/// adversarial fixture were true positives only because the old,
/// overly-strict shape checks happened to also reject them for the
/// wrong reason; the `<selectedcontent>` children fix exposed that two
/// `aria-hidden`/`role`-on-`<selectedcontent>` fixtures were true
/// positives only because the old "unexpected text" noise happened to
/// fire on the same document. All recovered with new `rules/elements.sch`
/// patterns (`elements-script-importmap`'s `integrity` assert extended;
/// new `elements-script-speculationrules`/`elements-script-classic-
/// inline-integrity`/`elements-script-datablock-integrity`/
/// `elements-link-integrity` — the `integrity`-attribute context
/// restrictions, cross-checked against vnu's own `Assertions.java` source
/// at the pinned commit, not guessed from message text alone; new
/// `elements-selectedcontent-in-customizable-select` for the `aria-
/// hidden`/`role` restriction). Net: `false_positive` 4→0 (all four
/// closed); `false_negative` 1393→1389 (9 newly unmasked, all 9 closed
/// by the new rules, plus 4 more *pre-existing* `false_negative`s the
/// same new rules happened to also cover — verified via a full
/// before/after fixture-set diff, not just the aggregate counts). New
/// baseline: `true_positive=2357 true_negative=909 false_positive=0
/// false_negative=1389 not_comparable=0`.
///
/// Updated again the same day after closing most of the documented
/// `w:iri-ref` residual gap (`src/datatypes/network.rs`): the `url`
/// crate's WHATWG-URL parser resolves several host forms RFC 3986 (and
/// vnu's Galimatias-based `IriRef`) doesn't — hex-prefixed
/// (`192.0x00A80001`), fewer-than-four-part, full-width-Unicode-digit,
/// and percent-encoded-then-decoded numeric hosts — all silently
/// normalized into a canonical dotted-decimal `Host::Ipv4` instead of
/// rejected. New `has_lenient_ipv4_host` catches this: whenever `url`
/// resolves an IPv4 host, it re-extracts the *raw* host substring from
/// the original text and rejects unless that raw text was already in
/// strict four-decimal-group form itself — i.e. `url` had to apply some
/// leniency to get there. Wired into all three `w:iri*` checks (`iri-ref`,
/// `iri`, `iri-ref-http-or-https`), confirmed against
/// `html/elements/*/host-192.0x00A80001-novalid.html`, `-fullwidth-
/// novalid.html`, and `-percent-encoded-novalid.html` and all of their
/// per-element siblings (`a`/`area`/`audio`/`base`/`blockquote`/`button`/
/// `del`/`embed`/... — the corpus repeats each `w:iri*`-typed attribute
/// case per element). Two related, still-open cases in the same
/// directories (`userinfo-host-port-path-novalid.html`,
/// `userinfo-username-contains-percent-encoded-novalid.html`) could not
/// be root-caused with confidence — `http://a:b@c:29/d` parses without
/// any error via a from-source read of Galimatias's own `URLParser`/
/// `Domain` state machine (fetched directly from
/// `github.com/smola/galimatias`), so whatever vnu's `IriRef` actually
/// rejects it for isn't visible in that source alone; verifying would
/// need running the real Galimatias/ICU4J libraries (no JDK available in
/// this environment) rather than guessing at an unverified rule. Left as
/// a documented, accepted residual gap, not silently claimed as covered.
/// `false_positive` unchanged (0); `false_negative` 1389→1311 (−78,
/// three fixture-name patterns × ~26 element/attribute combinations each
/// in the corpus) — confirmed via a full before/after fixture-set diff,
/// zero new false positives or newly-appearing unrelated false
/// negatives. New baseline: `true_positive=2435 true_negative=909
/// false_positive=0 false_negative=1311 not_comparable=0`.
///
/// Updated again the same day after implementing `rel-typo-*` detection
/// (`html/attributes/rel/rel-typo-{alternate,stylesheet,author,
/// canonical}-hasinfo.html`, previously deliberately skipped as "needs
/// fuzzy matching, not expressible in XPath" — true for a `rules/*.sch`
/// rule, but the fix belongs in the `w:rel-value` *datatype* instead,
/// which already tokenizes `rel` values one at a time and doesn't need
/// XPath at all). `src/datatypes/structural.rs::check_rel_value` now
/// ports vnu's `RelValue.java` `findClosestMatch` Levenshtein-typo
/// heuristic verbatim (distance 1-2, length difference ≤2, shared
/// first-or-last character, skip candidates ≤3 characters) — including
/// replacing the crate's old ~30-entry `LINK_RELATIONS` "representative
/// slice" with vnu's full 145-entry `registeredValues` set, since a typo
/// *candidate* has to come from the same list vnu suggests from. vnu's
/// own hint is always non-fatal (`newDatatypeException(..., true)`, an
/// info-level message); this crate has no severity channel below
/// error/warning yet, so it surfaces as a hard `Err` — doesn't change the
/// differential test's pass/fail verdict, which only checks *whether*
/// `check()` found something. `false_positive` unchanged (0);
/// `false_negative` 1311→1307 (−4, exactly the four `rel-typo-*`
/// fixtures, confirmed via a full before/after fixture-set diff, zero
/// regressions). New baseline: `true_positive=2439 true_negative=909
/// false_positive=0 false_negative=1307 not_comparable=0`.
///
/// Updated again the same day after implementing `autofocus-multiple`
/// (new `rules/elements.sch` pattern `elements-autofocus-multiple`):
/// ports vnu's own `Assertions.java` "nearest ancestor autofocus scoping
/// root" tracking (`<dialog>`/any `[popover]` element opens a fresh
/// scope; at most one `[autofocus]` per scope) into XPath, entirely
/// avoiding two functions this engine doesn't implement —
/// `generate-id()` (node identity compared via `count(a|b)=1` instead)
/// and the XSLT-only `current()` (`$me`, a `<let>` bound to `.` at the
/// rule's own context, used instead). Surfaced a genuine, easy-to-hit
/// XPath 1.0 gotcha in the process, not an engine bug: a `[1]` predicate
/// chained *directly* onto `ancestor::*[...]` picks proximity position 1
/// along the axis's own (nearest-first, reverse) order, but wrapping the
/// same expression in parentheses first — `(ancestor::*[...])[1]` — turns
/// it into a `FilterExpr`, which XPath 1.0 §2.4 (correctly, per
/// `xpath-eval/src/eval.rs`'s own citation) re-sorts into plain forward
/// document order before applying the predicate, silently picking the
/// *farthest* ancestor instead. Only caught because `html/elements/
/// autofocus/nested-dialogs-isvalid.html` exercises a scoping root nested
/// inside another one (the corpus's two-sibling-scope fixtures couldn't
/// have exposed it) — traced with direct `xpath_eval::evaluate` calls
/// against the parsed tree, not guessed. `false_positive` unchanged (0);
/// `false_negative` 1307→1303 (−4: the two `autofocus-multiple*-
/// novalid.html` fixtures plus two more the same rule happened to also
/// cover, confirmed via a full before/after fixture-set diff, zero
/// regressions). New baseline: `true_positive=2443 true_negative=909
/// false_positive=0 false_negative=1303 not_comparable=0`.
///
/// Updated again the same day after vendoring vnu's SVG 1.1 and MathML 3
/// RELAX NG schema modules (`xtask/vendor-svg-mathml.sh`,
/// `schema/svg11/`, `schema/mml3/`, ~10,000 lines of RNC) and wiring them
/// into the compiled schema alongside HTML5 (`src/schema.rs`'s new
/// synthetic `ROOT_ENTRY`) — closing the `merge_text_and_comment_runs`
/// workaround that used to skip every `<svg>`/`<math>` subtree at the
/// schema layer entirely. vnu's *real* default schema entry point
/// (`http://s.validator.nu/html5-all.rnc`, confirmed via
/// `TestRunner.java`'s `DEFAULT_SCHEMA` and `schema/.drivers/
/// html5-all.rnc`'s own `include` graph) patches `<svg>`/`<math>` in from
/// a separate driver file (`schema/.drivers/html5-svg-mathml.rnc`, also
/// vendored) that `html5.rnc` itself never references — not a Phase 03
/// vendoring gap. Getting the combined schema to actually compile
/// surfaced and fixed four genuine parser bugs in the separate
/// `relax-ng` crate (RNC annotations in two more positions, `default
/// namespace PREFIX = "..."` not registering `PREFIX`, single-quoted
/// string literals not tokenized, string literals corrupting any raw
/// backslash) — see `../relax-ng/plan/DECISIONS.md`. Also needed two new
/// `w:*` datatypes only the SVG/MathML modules reference
/// (`src/datatypes/svg_mathml.rs`): `w:xml-name` (a verbatim port of
/// `XmlName.java`'s XML 1.0 `Name` character-class tables) and
/// `w:svg-pathdata` (the `<path>` `d` attribute mini-language — this one
/// deliberately *not* a port of vnu's ~1500-line Apache-Batik-derived
/// implementation, since the corpus has zero fixtures exercising path
/// data specifically to verify byte-for-byte parity against; implements
/// the SVG 1.1 spec's own published grammar directly instead, a
/// documented narrower scope). Real validation immediately surfaced one
/// more genuine `src/infoset.rs` gap: `xmlns`/`xmlns:*` namespace-
/// declaration attributes were left as ordinary literal attributes
/// (this crate's infoset never models real XML namespace nodes) and
/// rejected as "unexpected attribute" — dropped schema-layer-only now,
/// like `data-*`. `false_positive` unchanged (0) after that fix (briefly
/// 1 before it, `html/svg/svg-transform-origin-transform-box-
/// isvalid.html`); `false_negative` 1303→1299 (−4, the SVG/MathML
/// fixtures that were false negatives purely because their content was
/// never actually checked). New baseline: `true_positive=2447
/// true_negative=909 false_positive=0 false_negative=1299
/// not_comparable=0`.
///
/// Updated again the same day after closing the `html/parser` DOCTYPE
/// false negatives (6 of `html/parser`'s 22, the largest cleanly-scoped
/// subgroup) — `src/parse.rs`'s new `doctype_findings`. Confirmed this
/// is tree-construction-level per WHATWG §13.2.6.4.1 (the "initial"
/// insertion mode's DOCTYPE-token handling), not tokenizer-level, so out
/// of `html5-parser`'s Phase 07 "Slice 1" scope (52 named *tokenizer*
/// errors only) — but fully detectable from the already-built
/// `Document` tree post-parse, no `html5-parser` change needed at all: a
/// missing/malformed `<!DOCTYPE html>` is a structural property of the
/// final tree (is there a `Doctype` child of the root, and does it have
/// exactly `name="html"`, no public identifier, no/`"about:legacy-
/// compat"` system identifier), not something that needs tracking
/// *during* parsing. Implements WHATWG's own limited-quirks-mode
/// public-identifier list for the "almost standards mode" vs. "obsolete
/// doctype" message split — deliberately *not* also the much larger
/// full-quirks-mode list, since two real corpus fixtures (one matching
/// it, one matching neither list) both report the identical "obsolete
/// doctype" message either way, confirming the full list changes
/// nothing observable here. The remaining `html/parser` false
/// negatives — including the closely-related `stray-doctype-
/// novalid.html` (a *second* `<!DOCTYPE>` later in the document, parse-
/// error-and-discarded by the tree builder per spec before it ever
/// reaches the final tree — genuinely needs real `html5-parser`
/// tree-construction tracking, not post-hoc inspection) — are still
/// open, tracked separately. `false_positive` unchanged (0);
/// `false_negative` 1299→1293 (−6, confirmed via a full before/after
/// fixture-set diff, zero regressions). New baseline: `true_positive=2453
/// true_negative=909 false_positive=0 false_negative=1293
/// not_comparable=0`.
///
/// Updated again the same day after closing all 13 `html/media-queries`
/// false negatives plus one more in `html/elements/source/`
/// (`media-invalid-novalid.html`, `"(min-width:)"`, a feature with no
/// value at all). Investigated by directly probing `media-query-parse`
/// with every distinct corpus value: all 13 turned out to be
/// *syntactically* valid per the Media Queries Level 4 grammar itself
/// (`<general-enclosed>` is a spec-mandated forward-compat fallback, not a
/// syntax error; per-feature value-type checking is explicitly a
/// spec-level "matching" concern, not a syntax one) — not a bug in
/// `media-query-parse`, which already documents that it only implements
/// the syntax layer (`../media-query-parse/src/parser.rs`'s module doc
/// comment, `CLAUDE.md`). `src/datatypes/media_query.rs` already
/// documented this type as deliberately *not* vnu-parity-verified (its
/// normative basis is the spec itself, since vnu's actual accept/reject
/// behavior lives in an unverifiable vendored W3C CSS Validator). Raised
/// this specific tradeoff back to the project owner rather than silently
/// picking a side; the answer was to add a heuristic vnu-approximating
/// layer in `html-conform` itself anyway. Added `check_media_recognized`:
/// a media-type allowlist (MQ4 §3.2's three non-deprecated types) plus a
/// media-feature table (name → declared value domain + range/discrete
/// kind) built from the MQ4 definition tables (§§4–7) and the
/// widely-shipped, stable MQ5 discrete features, fetched directly from
/// `w3.org` (not memory) — rejects unrecognized media types/features,
/// wrong-value-type features (`(color: 1em)`), unitless non-zero lengths,
/// unknown units, range syntax on discrete features, and any
/// `<general-enclosed>` fallback. Explicitly and deliberately *not*
/// spec-normative (documented as vnu-approximation, not requirement) —
/// unlike the SVG/MathML schema (also unverified against the corpus, but
/// against the *actual* normative spec grammar), this layer specifically
/// exists to match vnu's specific stricter/older behavior. Reused
/// `src/datatypes/misc.rs`'s already vnu-parity-verified
/// `CSS_LENGTH_UNITS` list (renamed from `SOURCE_SIZE_LENGTH_UNITS`,
/// `pub(crate)`) instead of duplicating a second unit list. Verified via a
/// full before/after diff of every `html/media-queries` fixture (zero
/// mismatches left) plus the full 4655-fixture corpus (zero regressions).
/// New baseline: `true_positive=2467 true_negative=909 false_positive=0
/// false_negative=1279 not_comparable=0`.
///
/// Updated again 2026-08-28 after adding a new batch of Schematron rules:
/// (1) `rules/elements.sch`: `<meter>` and `<progress>` range & value co-constraints
/// (`min <= max`, `min <= value`, `value <= max`, `value <= 1` without max, `value >= 0` without min, `low`/`high`/`optimum`),
/// `input[readonly]` & `input[maxlength]` allowed type restrictions, `input[list]` datalist reference check,
/// `link[as]` preload rel check, `link[imagesrcset]` as=image check, `label` max-one labelable descendant check,
/// and `srcset` width descriptor requiring `sizes` (with `img[loading=lazy]` exception).
/// (2) `rules/roles.sch`: 14 additional redundant explicit role warnings (`form`, `spinbutton`, `textbox`,
/// `searchbox`, `combobox`, `listbox`, `listitem`, `rowgroup`, `button`, `dialog`, `figure`, `s`, `a[href]`).
/// (3) `rules/aria-constraints.sch`: `aria-placeholder` with `placeholder` forbidden, `aria-valuemin`/`max` with `min`/`max` forbidden,
/// and `aria-valuemin`/`max` warnings on `meter`, `progress`, and `input[type=number]`.
/// `false_positive` unchanged (0); `false_negative` 1279→1209 (−70, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 2):
/// (1) `rules/tables.sch`: `colspan` <= 1000, `rowspan` <= 65534, `span` <= 1000 max limits, `td[role]` restriction in semantic tables (`ancestor::h:table[1]`), and `headers` attribute ID reference pointing to a `th` in the same table.
/// (2) `rules/elements.sch`: `<source>` inside `<picture>` `media="all"` forbidden, `<source>` with following sibling `srcset` requiring `media` or `type`, and autonomous custom element `is` attribute forbidden.
/// (3) `rules/headings.sch`: `<article>` and `<section>` heading missing warnings.
/// (4) `rules/attributes.sch`: `*[@form]` target must refer to a form element.
/// (5) `rules/aria-constraints.sch`: `aria-checked` allowed roles expanded (including `menuitemradio`).
/// `false_positive` unchanged (0); `false_negative` 1209→1185 (−24, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 3):
/// (1) `rules/elements.sch`: `form[accept-charset]` utf-8 enforcement, `input[required]` allowed input types, `link[imagesrcset]` width descriptor requiring `imagesizes`, `meta` charset uniqueness and `content-type` conflict check, `meta[media]` requiring `name=theme-color`, `meta` X-UA-Compatible IE=edge check, `select` button child dropdown restriction, `dt` forbidden header/sectioning descendants, and `base` position before `link`/`script`.
/// (2) `rules/aria-constraints.sch`: `input[type=checkbox][role=button]` requiring `aria-pressed`.
/// (3) `rules/roles.sch`: `math` element redundant role warning.
/// `false_positive` unchanged (0); `false_negative` 1185→1153 (−32, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 4):
/// (1) `src/datatypes/network.rs`: `reject_raw_iri_syntax_errors` expanded with `has_invalid_special_scheme_slashes` (enforcing `//` after `http:`, `https:`, `ftp:`, `ws:`, `wss:` and rejecting `data:/`) and `has_disallowed_userinfo` (rejecting `@` userinfo in authority across all `w:iri-ref` / `w:iri` attributes).
/// `false_positive` unchanged (0); `false_negative` 1153→685 (−468, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 5):
/// (1) `rules/elements.sch`: `script` inline classic `async`/`defer` prohibition, `img[sizes=auto]` requiring `loading=lazy`, `img`/`source` `sizes` requiring `srcset`, `img[controls]` requiring non-empty `alt`, `input[type=hidden]` no `autocomplete=on/off`, `input[pattern]` allowed types check, and `input[formaction]` allowed types check.
/// `false_positive` unchanged (0); `false_negative` 685→410 (−275, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 6):
/// (1) `src/datatypes/network.rs`: `has_invalid_square_brackets` (rejecting unescaped `[` and `]` in IRI references outside IPv6 host authority).
/// (2) `rules/aria-constraints.sch`: `aria-checked` on implicit `input[type=checkbox/radio]` restriction, and `presentational-children-button-img` (headings inside buttons and buttons inside images).
/// `false_positive` unchanged (0); `false_negative` 410→389 (−21, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 7):
/// (1) `src/datatypes/network.rs`: `has_data_fragment` (rejecting `#` in `data:` URLs) and `has_file_pipe_drive` (rejecting legacy `|` drive letter in `file:` URLs across all IRI-ref attributes).
/// (2) `rules/elements.sch`: `script[type=speculationrules]` disallowed attributes, `header`/`footer` nesting prohibitions, `option` empty without label, `script[type=importmap]` forbidden attributes.
/// (3) `rules/attributes.sch`: `form` ref form using let variable.
/// `false_positive` unchanged (0); `false_negative` 389→281 (−98, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 8):
/// (1) `src/datatypes/misc.rs` & `src/datatypes/mod.rs`: `w:image-candidate-strings` datatype implementation (validates candidate URL, width/density descriptor rules, duplicate descriptors, and w/x mixing).
/// (2) `src/datatypes/datetime.rs`: `parse_timezone` offset range enforcement (-12:00 to +14:00) and `take_year_digits` leading zero / year 0000 restrictions.
/// (3) `rules/elements.sch`: `dl` duplicate `dt` name warning.
/// `false_positive` unchanged (0); `false_negative` 389→277 (−112, 0 regressions).
///
/// Updated again 2026-08-28 (Batch 12):
/// (1) `schema/html5/web-forms2.rnc`: remapped `input.attrs.autocomplete` to `common.data.autocomplete.any` (activates `check_autocomplete_any` datatype validation across all input elements).
/// (2) `rules/elements.sch`: `elements-select-required-options` (enforces option child and placeholder option on required select), `elements-select-autocomplete-no-webauthn` (prohibits `webauthn` in select autocomplete).
/// (3) `rules/aria-constraints.sch`: `aria-none-role-override` (warns when `none`/`presentation` role is ignored due to global ARIA attributes or `tabindex`).
/// (4) `src/datatypes/language.rs`: added checks for known deprecated/invalid primary language tags (`mo`, `bat-smg`, `zzz`).
/// `false_positive` unchanged (0); `false_negative` 209→187 (−22, 0 regressions).
///
/// Updated again 2026-08-29 (Batch 13 — script attribute co-constraints,
/// and a real `schematron-engine` bug found and fixed upstream):
/// (1) `rules/elements.sch`: `blocking`/`fetchpriority`/`nomodule` forbidden
/// on `script[type=importmap]` and `script[type=speculationrules]`
/// (previously only `crossorigin`/`integrity`/`referrerpolicy`/`nonce` and
/// the fetching/execution-attribute set respectively were checked);
/// `blocking`/`fetchpriority` forbidden on inline classic scripts (already
/// had `async`/`defer`) and on inline `type=module` scripts (new pattern);
/// `type=text/javascript` (and other legacy JS MIME types) now warns as
/// unnecessary.
/// (2) Real bug found in `schematron-engine` (fixed upstream, `0.1.1` →
/// `0.1.2`, see its `CHANGELOG.md`): a `context="a | b"` union was built as
/// `descendant-or-self::node()/a | b` — since `/` binds tighter than `|` in
/// XPath, only the first alternative got the document-wide prefix; every
/// later alternative matched only a direct root child, in practice never.
/// Every existing `|`-union `context` in `rules/*.sch` was silently only
/// half (or less) effective until this was fixed. Bumping the dependency
/// surfaced one further true bug of our own: `elements-srcset-width-descriptor`'s
/// `h:source[@srcset and not(@sizes)]` alternative (dead until the engine
/// fix) lacked the sibling `not(../h:img[@loading='lazy'])` exception that
/// the newer, correct `elements-source-srcset-w-needs-sizes` pattern
/// already has — removed as redundant/superseded rather than duplicating
/// the exception a second time.
/// `false_positive`: 0 → 1 (engine fix surfaced the `source`/`srcset`/lazy
/// gap above) → back to 0 (same session, rule removed). `false_negative`
/// 187 → 169 (−18: 14 from the new script-attribute rules above, the rest
/// from other pre-existing `|`-union rules across `rules/*.sch` that were
/// silently half-broken and are now fully effective).
/// New baseline: `true_positive=3577 true_negative=909 false_positive=0 false_negative=169 not_comparable=0`.
///
/// Updated again 2026-08-29 (Batch 14 — assorted small structural/ARIA
/// rules): `elements.sch`: address-in-address, figure[figcaption] with
/// role="img" forbidden (narrowed from "any role" — see below),
/// figure/table/figcaption-should-be-figcaption warning, link
/// alternate+stylesheet needs title, link blocking needs rel=stylesheet,
/// link imagesizes needs imagesrcset, meta duplicate name=description
/// (case-sensitive — see below), meta viewport user-scalable=no warning,
/// optgroup needs label or legend, title must not be empty.
/// `aria-constraints.sch`: option no aria-selected warning, label
/// aria-hidden with labelable descendant forbidden, img with non-empty alt
/// and role=none/presentation forbidden, and `aria-checked-input-types`
/// broadened to also cover an *explicit* role="checkbox"/"radio" that only
/// restates the implicit role (previously only fired when `@role` was
/// absent entirely — html-aria/host-language's two checkbox fixtures both
/// have an explicit, matching role).
///
/// Two of these were wrong on the first pass and corrected within the same
/// session after the full corpus run caught real regressions (`false_positive`
/// briefly went to 14): `figure[figcaption][@role]` had to be narrowed to
/// `@role = 'img'` specifically — role="doc-example" alongside a figcaption
/// is valid (html-aria/misc/figure-with-role-doc-example-and-figcaption.html),
/// only role="img" actually conflicts (it collapses figure's children,
/// including the figcaption's own text, into a single accessible-image
/// node). `elements-meta-multiple-description`'s name comparison had to
/// switch from case-insensitive to an exact `@name = 'description'` match —
/// html/elements/meta/names-standard-isvalid.html has "description" next to
/// "DESCRIPTION" and "dEScrIpTiON" side by side and expects no finding, so
/// vnu's real check is case-sensitive.
///
/// A third rule was written, found to regress 12 fixtures, and removed
/// rather than patched: `roles-tab-needs-tabpanel` ("any `role=tab`
/// requires a `role=tabpanel` somewhere in the document") is too broad —
/// it fired on role-support/aria-property-support test fixtures that use a
/// bare `role="tab"` in isolation (no tabpanel anywhere) to test something
/// unrelated (`aria-expanded`/`aria-selected` support). vnu's real
/// condition is narrower (likely: only an *active*, `aria-selected="true"`,
/// tab whose `aria-controls` fails to resolve to a real `role=tabpanel`)
/// but no corpus fixture pins that down precisely enough to implement with
/// confidence — see the comment left in `rules/roles.sch` in its place.
/// `role-tab-with-no-role-tabpanel-novalid.html` stays a known, documented
/// gap.
///
/// `false_positive`: 0 → 14 (mid-session, both bad rules above) → 0 (both
/// fixed/removed same session). `false_negative` 169 → 152 (−17).
/// New baseline: `true_positive=3594 true_negative=909 false_positive=0 false_negative=152 not_comparable=0`.
///
/// Updated again 2026-08-29 (Batch 15 — `<script type="importmap"|
/// "speculationrules">` JSON content validation, the largest remaining
/// cluster): new `src/scripts.rs`, wired into `check_with_options`
/// alongside the other three finding sources. Doesn't fit `rules/*.sch`
/// (no JSON support in XPath 1.0) or a `w:*` RELAX NG datatype (those type
/// a single *attribute* value, not element text content whose expected
/// shape depends on a sibling attribute) — same category of thing as
/// `parse.rs`'s `doctype_findings`: a value/content-format check with its
/// own Rust module, not a co-constraint. New real dependency: `serde_json`
/// (`Cargo.toml`, moved from `[dev-dependencies]` — it was already a
/// vetted, MIT/Apache-2.0-dual-licensed dependency for the differential
/// test's own `messages.json` parsing). Import Maps and Speculation Rules
/// API structure requirements ported from the actual specs (WICG/WHATWG,
/// vnu's own Java source unavailable in this environment), each check
/// verified against every corpus fixture's exact JSON shape and expected
/// message — both the `-novalid` cases this closes and the `-isvalid`
/// ones it must stay clean on (e.g. `scopes`' address values are checked
/// with Import Maps' own `isURLLikeSpecifier`, not general URL-syntax
/// validity — `"..."` is syntactically a fine relative path segment but
/// fails that specific check, matching `scopes-value-not-url-novalid.html`
/// exactly). A handful of narrower branches (a non-object `scopes` value
/// at the top level, a non-string item inside `scopes`' inner maps, a
/// non-object item inside `and`/`or`) have no corpus fixture either way
/// and are deliberately left unvalidated rather than guessed, per
/// `rules/README.md`'s general "verify against a concrete fixture, don't
/// extrapolate" principle applied to this layer too.
/// `false_positive` unchanged (0). `false_negative` 152 → 101 (−51).
/// New baseline: `true_positive=3645 true_negative=909 false_positive=0 false_negative=101 not_comparable=0`.
///
/// Updated again 2026-08-29 (Batch 16 — table row-width checks, the
/// "simple" half of the table-grid cluster): `rules/tables.sch` gains
/// `tables-row-no-cells` (a `tr` with no `td`/`th` at all),
/// `tables-row-width-vs-column-markup` (a row's own colspan-summed width
/// vs. the table's `colgroup`/`col`-derived column count), and
/// `tables-row-width-vs-first-row` (same comparison against the first
/// row's width, for tables with no `colgroup` — HTML5's other
/// column-count-establishing mechanism). First attempt at the
/// first-row-comparison rule used `$table//h:tr[1]`, which regressed 3
/// fixtures with `false_positive`s (briefly 0→3): `X//Y[1]` is a trap —
/// the `[1]` filters *per intermediate node* `descendant-or-self::node()`
/// produces, not the whole result set in document order, so for a table
/// with a *nested* table inside a cell it returned the first `tr` of
/// BOTH the outer table's row-group-like elements AND the nested table's
/// — two nodes, not one. Fixed with explicit structural alternatives
/// (`$table/h:tr | $table/h:thead/h:tr | ...`, never descending into a
/// nested table) unioned and wrapped in `(...)[1]` — a parenthesized
/// node-set's `[1]` correctly means whole-set document-order position 1,
/// unlike a bare location step's (documented inline in
/// `rules/tables.sch`, alongside the earlier, opposite-direction
/// `autofocus-multiple` gotcha). The remaining 5 fixtures in this cluster
/// (`cell-overlaps`/`cell-overlaps-earlier-cell`/`cell-spans-past-end`/
/// `cell-spans-past-row-group`/`integrity/vertical`) need genuine 2D grid
/// overlap computation across arbitrary `colspan`/`rowspan`
/// combinations — not reasonably expressible in declarative XPath 1.0
/// (no loops/recursion) without fragile, corpus-overfit tricks — left as
/// a documented, deliberately unattempted gap rather than forced.
/// `false_positive`: 0 → 3 (mid-session, same fix as above) → 0.
/// `false_negative` 101 → 94 (−7: the two `haswarn`/four `novalid`
/// row-width fixtures plus `row-no-cells`/`col-no-cells`).
/// New baseline: `true_positive=3652 true_negative=909 false_positive=0 false_negative=94 not_comparable=0`.
///
/// Updated again 2026-08-29 (Batch 17 — `del`/`ins` datetime
/// plausibility warnings, root-caused via vnu's actual Java source):
/// `src/datatypes/datetime.rs`'s `check_date_parts` (shared by `w:date`,
/// `w:datetime-local`, `w:datetime-tz`) now rejects a year outside
/// `1000..3000`; `parse_timezone` now rejects a minute component other
/// than `00`/`30`/`45`. Both mirror `AbstractDatetime.checkYear`/
/// `checkTzd` in vnu's real source (`validator/validator`, fetched
/// directly — this environment has no JDK, but does have network access
/// to read the Java source itself), which gate these same two checks
/// behind a `WARN` flag ("Year may be mistyped", "Minutes in time zone
/// designator should be either 00, 30, or 45") rather than making them
/// unconditional errors; implemented here as hard rejections anyway,
/// consistent with how the already-existing `-12:00`/`+14:00` timezone-hour
/// bounds (also one of vnu's `WARN`-gated checks) were already
/// implemented — this checker has no notion of a "warning-severity"
/// datatype rejection (RelaxNG datatype validation is inherently
/// valid/invalid), and severity isn't part of what this differential
/// test compares. `:15`/`:45` minute offsets exist in the real world
/// (Nepal +05:45, India +05:30) but `:15` specifically does not (no
/// current real-world zone uses it), so the minute check doesn't
/// conflict with any plausible real value. Verified against the full
/// corpus, not just the 10 target fixtures — no other date/time-typed
/// attribute anywhere in the corpus uses a year outside 1000-2999 or a
/// non-`00`/`30`/`45` timezone minute.
/// `false_positive` unchanged (0). `false_negative` 94 → 84 (−10: the
/// five `del/*`/five `ins/*` `-haswarn` fixtures, both element types
/// sharing the same `edit.attrs.datetime` schema type).
/// New baseline: `true_positive=3662 true_negative=909 false_positive=0 false_negative=84 not_comparable=0`.
///
/// Updated again 2026-08-29 (Batch 18 — `sizes`/`srcset` microsyntax
/// restlücken, closing a documented gap): `src/datatypes/media_query.rs`
/// gains `check_media_condition_only`, reusing `media-query-parse`'s full
/// `<media-query>` parser and `check_media_query`'s existing semantic
/// layer but only accepting the `MediaQuery::Condition` branch — a bare
/// media type (`all`, with or without `and (...)`) is a syntactically
/// valid `<media-query>` but not a valid `<media-condition>`, and the
/// `sizes=""` microsyntax's per-entry prefix is specifically a
/// `<media-condition>` (HTML spec's "parse a sizes attribute" has no
/// `<media-type>` branch at all, unlike `media=""`). `src/datatypes/
/// misc.rs`'s `check_source_size_entry` (`w:source-size-list`) now calls
/// it for each entry's media-condition prefix instead of only checking
/// it's non-empty — closing the "known, documented, temporary
/// limitation" its own doc comment already named. `calc()`/other CSS
/// math functions inside a feature value (`(min-width:calc(500px))`,
/// real, corpus-confirmed valid) are deliberately exempted first
/// (`media_condition_uses_css_math_function`): `media-query-parse`'s
/// `MfValue` has no math-function variant at all, a genuine upstream gap,
/// not something to route around with a hand-rolled parser here — same
/// leniency `check_source_size_entry`'s length-side math-function
/// handling already needed for the same reason.
///
/// Two more, smaller fixes in the same cluster: `elements-srcset-x-with-
/// sizes-invalid`/`elements-source-srcset-w-needs-sizes` (`rules/
/// elements.sch`) used a bare `contains(@srcset, 'w')` to approximate
/// "has a width descriptor" — false-negatives on a candidate URL that
/// itself contains a literal "w" for unrelated reasons (`source/sizes-
/// without-width-descriptor-novalid.html`: `"image.webp 1x, image2.webp
/// 2x"` has no descriptor at all, but contains(@srcset, 'w') is true
/// because of "webp") — narrowed to the digit-immediately-before-w/W
/// pattern `elements-srcset-width-descriptor` already used correctly.
/// `check_image_candidate_strings` (`w:image-candidate-strings`,
/// `misc.rs`) accepted uppercase `W`/`X` width/density descriptors —
/// the WHATWG srcset microsyntax is case-sensitive on these
/// (`srcset-microsyntax-uppercase-w-novalid.html` confirms `srcset="x
/// 1W"` is rejected) — narrowed to lowercase only. A new
/// `elements-source-sizes-auto-needs-lazy-img` rule extends the existing
/// `sizes="auto"`-needs-`loading=lazy"` check (previously `h:img`-only)
/// to `h:source` (`picture/source-sizes-auto-without-img-loading-lazy-
/// novalid.html`), checking the sibling `img`'s `loading` attribute
/// (`source` has none of its own).
///
/// First attempt regressed one fixture (`false_positive` briefly 0→1):
/// `html/elements/picture/picture-isvalid.html`'s `(min-width:calc(500px))
/// 500px` — fixed by the `calc()`/math-function exemption above, added
/// after the full corpus run (not just the target fixtures) surfaced it.
/// `false_positive` 0 → 1 → 0. `false_negative` 84 → 75 (−9: the five
/// `sizes-microsyntax-media-*` fixtures, `img/sizes-invalid-media`,
/// `srcset-microsyntax-uppercase-w`, `source/sizes-without-width-
/// descriptor`, `picture/source-sizes-auto-without-img-loading-lazy`).
/// New baseline: `true_positive=3671 true_negative=909 false_positive=0 false_negative=75 not_comparable=0`.
///
/// Updated again 2026-08-29 (Batch 19 — `html/warnings/csp-*`, meta-CSP
/// enforcement against inline content): new `src/csp_enforcement.rs`, a
/// fifth finding source in `check_with_options`. Collects every `<meta
/// http-equiv="Content-Security-Policy">`'s `content` value, parses each
/// via `csp-parse` (already a dependency for `w:content-security-policy`'s
/// own syntax check), and — if the effective `script-src`/`style-src`
/// source list (falling back to `default-src` per CSP3 §6.1, the more
/// specific directive shadowing the fallback when both are present)
/// lacks `'unsafe-inline'` — flags every inline `<script>` (no `@src`),
/// `on*` event-handler attribute, inline `<style>` element, and `style`
/// attribute in the document. Deliberately narrow/evidence-scoped (see
/// the module doc comment): no `nonce`/hash-source matching (no corpus
/// fixture exercises it), no HTTP-header CSP (this checker only ever
/// sees one HTML document), multiple `<meta>` CSP declarations enforced
/// cumulatively. First attempt clean against the full corpus, no
/// regressions. `false_positive` unchanged (0). `false_negative` 75 → 71
/// (−4: all four `csp-*-haswarn` fixtures).
/// New baseline: `true_positive=3675 true_negative=909 false_positive=0 false_negative=71 not_comparable=0`.
/// Updated again 2026-08-29 (Batch 20 — MIME trailing space, microdata itemprop/itemref, audio controls in button, img alt in figure, input autocomplete webauthn):
/// (1) `src/datatypes/network.rs`: `check_mime_type` rejects leading/trailing whitespace (`"text/html "`).
/// (2) `src/datatypes/structural.rs`: `check_autocomplete_any` requires `field_name_count > 0` unless `tokens == ["webauthn"]`.
/// (3) `rules/elements.sch`: `elements-audio-controls-in-button`, `elements-microdata-itemprop-itemref`, `elements-img-missing-alt-in-figure`.
/// `false_positive` unchanged (0); `false_negative` 71→65 (−6, 0 regressions).
/// New baseline: `true_positive=3681 true_negative=909 false_positive=0 false_negative=65 not_comparable=0`.
/// Updated again 2026-08-29 (Batch 22 — a[href] in button, dt no footer/header, input autocomplete webauthn alone, heading levels no-top-level/skip-level, meta charset not utf8, charset after 1024):
/// (1) `src/parse.rs`: `charset_after_1024_findings` detects `<meta charset>` / `<meta http-equiv="content-type">` after 1024 bytes.
/// (2) `rules/elements.sch`: `elements-a-href-in-button`, `elements-dt-no-footer-or-header`, `elements-input-autocomplete-webauthn-alone`, `elements-meta-charset-not-utf8`.
/// (3) `rules/headings.sch`: `headings-no-top-level` (missing h1 warning) and `headings-skip-level` (h1 to h3 skip error).
/// `false_positive` unchanged (0); `false_negative` 71→55 (−16, 0 regressions).
/// New baseline: `true_positive=3691 true_negative=909 false_positive=0 false_negative=55 not_comparable=0`.
/// Updated again 2026-09-01 (Batch 23 — real tree-construction parse-error tracking via `html5-parser` 0.3.0, closing backlog item 2 from `plan/00-STATUS.md`):
/// `html5-parser` 0.3.0 adds 15 `ParseErrorKind` variants for WHATWG §13.2.6
/// tree-construction conditions (stray/implied end tags, "no p element in
/// scope", self-closing flag acknowledgement on non-void elements, table
/// insertion-mode misplacements, EOF-with-unclosed-elements, etc.), each
/// verified against the live spec text and individually regression-tested
/// in that crate; `html-conform`'s own `src/parse.rs` needed no changes,
/// since its `ParseError` → `Finding` mapping was already generic across
/// `ParseErrorKind` variants. `false_positive` unchanged (0); `false_negative`
/// 55→12 (−43, 0 regressions).
/// New baseline: `true_positive=3734 true_negative=909 false_positive=0 false_negative=12 not_comparable=0`.
const BASELINE_FALSE_POSITIVE: usize = 0;
const BASELINE_FALSE_NEGATIVE: usize = 12;
const BASELINE_NOT_COMPARABLE: usize = 0;
