//! Der Katalog deckt jede erzeugbare Kennung genau einmal ab, und die
//! Regelreferenz der Doku listet dieselbe Menge.

use std::collections::{BTreeSet, HashSet};

use a11y_rules::{
    catalogue, meta_for, rendering_metas, semantics_metas, structure_metas, stylesheet_metas,
};

fn declared_ids() -> Vec<&'static str> {
    [
        structure_metas(),
        semantics_metas(),
        rendering_metas(),
        stylesheet_metas(),
    ]
    .iter()
    .flat_map(|metas| metas.iter())
    .flat_map(|m| m.ids.iter().copied())
    .collect()
}

#[test]
fn every_declared_id_appears_exactly_once() {
    let catalogue_ids: Vec<&str> = catalogue()
        .iter()
        .flat_map(|m| m.ids.iter().copied())
        .collect();
    let unique: HashSet<&str> = catalogue_ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        catalogue_ids.len(),
        "Kennung doppelt im Katalog"
    );
    assert_eq!(catalogue_ids, declared_ids());
}

#[test]
fn lookup_finds_every_id_and_nothing_else() {
    for id in declared_ids() {
        assert!(meta_for(id).is_some_and(|m| m.ids.contains(&id)), "{id}");
    }
    assert!(meta_for("no/such-rule").is_none());
}

#[test]
fn rule_reference_page_lists_the_catalogue_ids() {
    let page = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/a11y/reference/regeln.mdx"
    ))
    .unwrap();
    let generated = page
        .split("generiert:anfang")
        .nth(1)
        .and_then(|rest| rest.split("generiert:ende").next())
        .expect("Marker generiert:anfang/ende fehlen");
    let on_page: BTreeSet<&str> = generated
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split('`').next())
        .collect();
    let in_catalogue: BTreeSet<&str> = declared_ids().into_iter().collect();
    let hint = "neu erzeugen mit `cargo run --manifest-path examples/a11y/Cargo.toml`";
    let missing: Vec<_> = in_catalogue.difference(&on_page).collect();
    let extra: Vec<_> = on_page.difference(&in_catalogue).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "Seite und Katalog weichen ab ({hint}); fehlt auf der Seite: {missing:?}; zu viel: {extra:?}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn meta_is_serialisable() {
    fn assert_serialize<T: serde::Serialize>() {}
    assert_serialize::<a11y_rules::Meta>();
}
