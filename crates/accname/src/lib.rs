//! WAI-ARIA Accessible Name and Description Computation, generisch über das
//! Dokumentmodell aus [`a11y_dom`].
//!
//! Umsetzung von
//! [Accessible Name and Description Computation 1.2](https://w3c.github.io/accname/)
//! und den Namensregeln aus
//! [HTML-AAM 1.0](https://www.w3.org/TR/html-aam-1.0/). Gegen die
//! Spezifikation geschrieben, nicht gegen eine vorhandene Implementierung —
//! die Schrittnummern im Quelltext entsprechen denen des Normtexts.
//!
//! # Warum das nicht nebenbei geht
//!
//! Eine Näherung aus „Teilbaumtext plus `aria-label`" liegt in genau den
//! Fällen falsch, in denen es darauf ankommt: `aria-labelledby` löst
//! Verweisketten auf und kann Knoten heranziehen, die sonst versteckt sind;
//! ein eingebettetes Steuerelement steuert innerhalb einer Rekursion seinen
//! *Wert* bei, nicht seine Beschriftung; und für Rollen wie `generic` oder
//! `paragraph` ist ein Name schlicht verboten.
//!
//! ```
//! use a11y_dom::{elements, Arena, Document, Node};
//! use accname::{name, IdIndex};
//!
//! let doc = Arena::builder()
//!     .open("form")
//!         .open("span").attr("id", "l").text("Vorname").close()
//!         .open("input").attr("id", "v").attr("aria-labelledby", "l").close()
//!     .close()
//!     .build();
//!
//! let ids = IdIndex::build(doc.root());
//! let feld = elements(&doc).find(|n| n.local_name() == "input").unwrap();
//! assert_eq!(name(feld, &ids).as_deref(), Some("Vorname"));
//! ```
//!
//! # Grenze ohne Rendering
//!
//! Schritt 2A der Spezifikation schließt versteckte Knoten aus. Ohne
//! Rendering-Daten sind nur `aria-hidden="true"` und das `hidden`-Attribut
//! erkennbar — `display: none` und `visibility: hidden` nicht. Ein Host mit
//! [`a11y_dom::Rendering`] kann hier genauer sein; dieses Crate bleibt
//! bewusst bei dem, was ohne Layout entscheidbar ist.

#![forbid(unsafe_code)]

mod index;
mod name;
mod role;

pub use index::IdIndex;
pub use name::{description, name};
pub use role::{allows_name_from_content, implicit, name_is_prohibited, role};

#[cfg(test)]
mod tests {
    use super::*;
    use a11y_dom::{elements, Arena, ArenaNode, Document, Node};

    fn finde<'a>(doc: &'a Arena, tag: &str) -> ArenaNode<'a> {
        elements(doc).find(|n| n.local_name() == tag).unwrap()
    }

    fn mit_id<'a>(doc: &'a Arena, id: &str) -> ArenaNode<'a> {
        elements(doc).find(|n| n.attr("id") == Some(id)).unwrap()
    }

    // --- Schritt 2B: aria-labelledby ------------------------------------

    #[test]
    fn labelledby_gewinnt_gegen_alles_andere() {
        let doc = Arena::builder()
            .open("div")
            .open("span")
            .attr("id", "l")
            .text("Aus dem Verweis")
            .close()
            .open("button")
            .attr("aria-labelledby", "l")
            .attr("aria-label", "Aus aria-label")
            .attr("title", "Aus title")
            .text("Aus dem Inhalt")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Aus dem Verweis")
        );
    }

    #[test]
    fn labelledby_setzt_mehrere_verweise_zusammen() {
        let doc = Arena::builder()
            .open("div")
            .open("span")
            .attr("id", "a")
            .text("Datei")
            .close()
            .open("span")
            .attr("id", "b")
            .text("löschen")
            .close()
            .open("button")
            .attr("aria-labelledby", "a b")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Datei löschen")
        );
    }

    #[test]
    fn labelledby_zieht_auch_versteckte_knoten_heran() {
        // Schritt 2A macht eine Ausnahme fuer ausdrueckliche Verweise.
        let doc = Arena::builder()
            .open("div")
            .open("span")
            .attr("id", "l")
            .attr("hidden", "")
            .text("Versteckt, aber gemeint")
            .close()
            .open("button")
            .attr("aria-labelledby", "l")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Versteckt, aber gemeint")
        );
    }

    #[test]
    fn labelledby_ring_endet_statt_zu_haengen() {
        let doc = Arena::builder()
            .open("div")
            .open("span")
            .attr("id", "a")
            .attr("aria-labelledby", "b")
            .close()
            .open("span")
            .attr("id", "b")
            .attr("aria-labelledby", "a")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        // Darf nicht in eine Endlosschleife laufen.
        assert!(name(mit_id(&doc, "a"), &ids).is_none());
    }

    #[test]
    fn labelledby_auf_fehlende_id_faellt_zurueck() {
        let doc = Arena::builder()
            .open("button")
            .attr("aria-labelledby", "gibtsnicht")
            .attr("aria-label", "Rückfall")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Rückfall")
        );
    }

    // --- Schritt 2C: aria-label -----------------------------------------

    #[test]
    fn aria_label_gewinnt_gegen_inhalt() {
        let doc = Arena::builder()
            .open("button")
            .attr("aria-label", "Schließen")
            .text("X")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Schließen")
        );
    }

    #[test]
    fn leeres_aria_label_wird_uebergangen() {
        let doc = Arena::builder()
            .open("button")
            .attr("aria-label", "   ")
            .text("Senden")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(name(finde(&doc, "button"), &ids).as_deref(), Some("Senden"));
    }

    // --- Schritt 2D: HTML-eigene Textalternativen ------------------------

    #[test]
    fn bild_nutzt_alt() {
        let doc = Arena::builder()
            .open("img")
            .attr("src", "h.png")
            .attr("alt", "Ein Hund")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(name(finde(&doc, "img"), &ids).as_deref(), Some("Ein Hund"));
    }

    #[test]
    fn label_ueber_for_attribut() {
        let doc = Arena::builder()
            .open("form")
            .open("label")
            .attr("for", "v")
            .text("Vorname")
            .close()
            .open("input")
            .attr("id", "v")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(name(finde(&doc, "input"), &ids).as_deref(), Some("Vorname"));
    }

    #[test]
    fn umschliessendes_label_ohne_den_eigenen_wert() {
        let doc = Arena::builder()
            .open("label")
            .text("Suche")
            .open("input")
            .attr("value", "Katzen")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        // Der Wert des Feldes darf nicht in sein eigenes Label zurueckfliessen.
        assert_eq!(name(finde(&doc, "input"), &ids).as_deref(), Some("Suche"));
    }

    #[test]
    fn submit_hat_einen_vorgabenamen() {
        let doc = Arena::builder()
            .open("input")
            .attr("type", "submit")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(name(finde(&doc, "input"), &ids).as_deref(), Some("Submit"));
    }

    #[test]
    fn fieldset_nutzt_legend_table_nutzt_caption() {
        let doc = Arena::builder()
            .open("div")
            .open("fieldset")
            .open("legend")
            .text("Anschrift")
            .close()
            .close()
            .open("table")
            .open("caption")
            .text("Umsätze")
            .close()
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "fieldset"), &ids).as_deref(),
            Some("Anschrift")
        );
        assert_eq!(name(finde(&doc, "table"), &ids).as_deref(), Some("Umsätze"));
    }

    #[test]
    fn svg_nutzt_sein_title_kind() {
        let doc = Arena::builder()
            .open("svg")
            .open("title")
            .text("Logo")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(name(finde(&doc, "svg"), &ids).as_deref(), Some("Logo"));
    }

    // --- Schritt 2F: Name aus dem Inhalt ---------------------------------

    #[test]
    fn inhalt_wird_verschachtelt_zusammengesetzt() {
        let doc = Arena::builder()
            .open("a")
            .attr("href", "/x")
            .text("Zum")
            .open("strong")
            .text("Bericht")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(name(finde(&doc, "a"), &ids).as_deref(), Some("Zum Bericht"));
    }

    #[test]
    fn bild_im_link_steuert_sein_alt_bei() {
        let doc = Arena::builder()
            .open("a")
            .attr("href", "/")
            .open("img")
            .attr("src", "l.png")
            .attr("alt", "Startseite")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(name(finde(&doc, "a"), &ids).as_deref(), Some("Startseite"));
    }

    #[test]
    fn versteckter_inhalt_zaehlt_nicht_mit() {
        let doc = Arena::builder()
            .open("button")
            .text("Sichtbar")
            .open("span")
            .attr("aria-hidden", "true")
            .text("Versteckt")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Sichtbar")
        );
    }

    #[test]
    fn div_bekommt_keinen_namen_aus_seinem_inhalt() {
        // generic erlaubt keinen Namen aus dem Inhalt.
        let doc = Arena::builder()
            .open("div")
            .text("Nur Text")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert!(name(finde(&doc, "div"), &ids).is_none());
    }

    #[test]
    fn rollen_mit_namensverbot_ignorieren_aria_label() {
        let doc = Arena::builder()
            .open("p")
            .attr("aria-label", "Verboten")
            .text("Absatz")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert!(name(finde(&doc, "p"), &ids).is_none());
    }

    // --- Schritt 2E: eingebettete Steuerelemente ------------------------

    #[test]
    fn eingebettetes_textfeld_steuert_seinen_wert_bei() {
        let doc = Arena::builder()
            .open("div")
            .open("span")
            .attr("id", "l")
            .text("Ich möchte")
            .open("input")
            .attr("type", "text")
            .attr("value", "drei")
            .close()
            .text("Stück")
            .close()
            .open("button")
            .attr("aria-labelledby", "l")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Ich möchte drei Stück")
        );
    }

    // --- Schritt 2I und Flattening --------------------------------------

    #[test]
    fn title_ist_die_letzte_rueckfallebene() {
        let doc = Arena::builder()
            .open("a")
            .attr("href", "/x")
            .attr("title", "Mehr erfahren")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "a"), &ids).as_deref(),
            Some("Mehr erfahren")
        );
    }

    #[test]
    fn whitespace_wird_zusammengefasst() {
        let doc = Arena::builder()
            .open("button")
            .text("  Mehr \n\t  erfahren  ")
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        assert_eq!(
            name(finde(&doc, "button"), &ids).as_deref(),
            Some("Mehr erfahren")
        );
    }

    #[test]
    fn ohne_jede_quelle_gibt_es_keinen_namen() {
        let doc = Arena::builder().open("button").close().build();
        let ids = IdIndex::build(doc.root());
        assert!(name(finde(&doc, "button"), &ids).is_none());
    }

    // --- Beschreibung ----------------------------------------------------

    #[test]
    fn beschreibung_ueber_describedby() {
        let doc = Arena::builder()
            .open("div")
            .open("p")
            .attr("id", "h")
            .text("Mindestens acht Zeichen")
            .close()
            .open("input")
            .attr("id", "p")
            .attr("aria-label", "Passwort")
            .attr("aria-describedby", "h")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        let feld = finde(&doc, "input");
        assert_eq!(name(feld, &ids).as_deref(), Some("Passwort"));
        assert_eq!(
            description(feld, &ids).as_deref(),
            Some("Mindestens acht Zeichen")
        );
    }

    #[test]
    fn title_ist_nur_beschreibung_wenn_der_name_woanders_herkommt() {
        let doc = Arena::builder()
            .open("div")
            .open("a")
            .attr("id", "eins")
            .attr("href", "/x")
            .attr("title", "Nur title")
            .close()
            .open("a")
            .attr("id", "zwei")
            .attr("href", "/y")
            .attr("aria-label", "Bericht")
            .attr("title", "Zusatzinfo")
            .close()
            .close()
            .build();
        let ids = IdIndex::build(doc.root());
        // Erster Link: title stellt den Namen, also keine Beschreibung.
        assert!(description(mit_id(&doc, "eins"), &ids).is_none());
        // Zweiter Link: der Name kommt aus aria-label, title wird Beschreibung.
        assert_eq!(
            description(mit_id(&doc, "zwei"), &ids).as_deref(),
            Some("Zusatzinfo")
        );
    }

    // --- Rollen -----------------------------------------------------------

    #[test]
    fn implizite_rollen() {
        let doc = Arena::builder()
            .open("div")
            .open("a")
            .attr("href", "/")
            .close()
            .open("a")
            .close()
            .open("h2")
            .close()
            .open("img")
            .attr("alt", "")
            .close()
            .open("img")
            .attr("alt", "x")
            .close()
            .close()
            .build();
        let mut rollen = elements(&doc).map(role);
        assert_eq!(rollen.next(), Some(Some("generic"))); // div
        assert_eq!(rollen.next(), Some(Some("link"))); // a[href]
        assert_eq!(rollen.next(), Some(Some("generic"))); // a ohne href
        assert_eq!(rollen.next(), Some(Some("heading"))); // h2
        assert_eq!(rollen.next(), Some(Some("presentation"))); // img alt=""
        assert_eq!(rollen.next(), Some(Some("img"))); // img mit alt
    }

    #[test]
    fn input_rollen_haengen_am_typ() {
        for (ty, erwartet) in [
            ("text", Some("textbox")),
            ("search", Some("searchbox")),
            ("checkbox", Some("checkbox")),
            ("radio", Some("radio")),
            ("range", Some("slider")),
            ("number", Some("spinbutton")),
            ("submit", Some("button")),
            ("password", None),
            ("hidden", None),
        ] {
            let doc = Arena::builder()
                .open("input")
                .attr("type", ty)
                .close()
                .build();
            assert_eq!(role(finde(&doc, "input")), erwartet, "type={ty}");
        }
    }

    #[test]
    fn explizite_rolle_gewinnt_ungueltige_nicht() {
        let doc = Arena::builder()
            .open("div")
            .open("div")
            .attr("role", "button")
            .close()
            .open("div")
            .attr("role", "buton")
            .close()
            .open("div")
            .attr("role", "buton button")
            .close()
            .close()
            .build();
        let mut r = elements(&doc).skip(1).map(role);
        assert_eq!(r.next(), Some(Some("button")));
        // Ungueltig: faellt auf die implizite Rolle zurueck.
        assert_eq!(r.next(), Some(Some("generic")));
        // Fallback-Stapel: die erste gueltige gewinnt.
        assert_eq!(r.next(), Some(Some("button")));
    }

    #[test]
    fn header_ist_nur_ausserhalb_von_sektionen_eine_landmark() {
        let doc = Arena::builder()
            .open("body")
            .open("header")
            .attr("id", "oben")
            .close()
            .open("article")
            .open("header")
            .attr("id", "innen")
            .close()
            .close()
            .close()
            .build();
        assert_eq!(role(mit_id(&doc, "oben")), Some("banner"));
        assert_eq!(role(mit_id(&doc, "innen")), Some("generic"));
    }

    #[test]
    fn section_ist_nur_benannt_eine_region() {
        let doc = Arena::builder()
            .open("body")
            .open("section")
            .attr("id", "ohne")
            .close()
            .open("section")
            .attr("id", "mit")
            .attr("aria-label", "Zahlen")
            .close()
            .close()
            .build();
        assert_eq!(role(mit_id(&doc, "ohne")), Some("generic"));
        assert_eq!(role(mit_id(&doc, "mit")), Some("region"));
    }
}
