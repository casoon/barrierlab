//! Korpus der Merkmalsprofile für strukturierte Daten: direkte Funktionstests
//! für jedes [`SchemaFeature`] (casoon/auditmysite#783).
//!
//! Jeder Fall läuft wie ein Host: `parse_block` → je Knoten und Typ
//! `assess_node` → die Bewertungen des Merkmals. Je Merkmal verlangt das Gate:
//!
//! - **positiv** — der fehlerhafte Zustand: `MissingRequiredProperties` mit
//!   genau den erwarteten Pfaden. Ein Merkmal ohne Pflichtangaben
//!   (`RecommendationsOnly`) hat keinen fehlerhaften Zustand; positiv ist dort
//!   `RecommendationsOnly` mit fehlenden Empfehlungen.
//! - **negativ** — vollständig: `MeetsRequiredProperties` ohne fehlende
//!   Pflichtpfade, bzw. `RecommendationsOnly` ohne fehlende Empfehlungen.
//! - **nicht anwendbar** — kein Knoten eines Typs mit diesem Merkmal: keine
//!   Bewertung.
//! - **nicht bewertet** — nur bei Merkmalen, die vom Seitenkontext abhängen
//!   (Merchant Listing): `NotEvaluated`. Für alle anderen prüft jeder Fall,
//!   dass jeder `ProductRuleContext` dasselbe ergibt.
//!
//! Die Erwartungen stammen aus Google Search Central (Quelle je Merkmal in
//! `source_url`) und schema.org, nicht aus der Ausgabe der Regeln.
//!
//! `KORPUS_ID=<schlüssel>[,…]` beschränkt `korpus_faelle` auf Merkmale; der
//! Mutationslauf (`xtask/mutate.py`) benutzt das.

mod korpus {
    pub mod katalog;
}

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use web_checks::structured_data::{
    ManualReview, ProductRuleContext, SchemaFeature, SchemaRequirementStatus, SchemaRuleAssessment,
    assess_node, parse_block,
};

use ManualReview as M;
use SchemaRequirementStatus as S;

/// Ob ein Merkmal Pflichtangaben kennt und ob es vom Seitenkontext abhängt.
/// Beides prüft der Korpus nach, statt es zu glauben.
fn profil(f: SchemaFeature) -> (bool, bool) {
    use SchemaFeature::*;
    match f {
        Article | Organization | WebPage | WebSite | Person => (false, false),
        MerchantListing => (true, true),
        _ => (true, false),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Art {
    Positiv,
    Negativ,
    NichtAnwendbar,
    NichtBewertet,
}

struct Fall {
    feature: SchemaFeature,
    art: Art,
    json: &'static str,
    /// `None`: jeder Kontext muss dasselbe ergeben.
    kontext: Option<ProductRuleContext>,
    status: Option<S>,
    /// Genau diese Pflichtpfade fehlen (bei positiven Fällen).
    pflicht: &'static [&'static str],
    /// Diese Empfehlungen fehlen mindestens.
    empfohlen: &'static [&'static str],
    /// Diese Empfehlungen dürfen nicht fehlen.
    nicht_empfohlen: &'static [&'static str],
    manuell: &'static [ManualReview],
    quelle: &'static str,
}

fn fall(feature: SchemaFeature, art: Art, json: &'static str, quelle: &'static str) -> Fall {
    Fall {
        feature,
        art,
        json,
        kontext: None,
        status: None,
        pflicht: &[],
        empfohlen: &[],
        nicht_empfohlen: &[],
        manuell: &[],
        quelle,
    }
}

/// Pflichtangaben fehlen, genau diese.
fn fehlt(
    f: SchemaFeature,
    json: &'static str,
    pflicht: &'static [&'static str],
    quelle: &'static str,
) -> Fall {
    Fall {
        status: Some(S::MissingRequiredProperties),
        pflicht,
        ..fall(f, Art::Positiv, json, quelle)
    }
}

/// Alle Pflichtangaben da.
fn erfuellt(f: SchemaFeature, json: &'static str, quelle: &'static str) -> Fall {
    Fall {
        status: Some(S::MeetsRequiredProperties),
        ..fall(f, Art::Negativ, json, quelle)
    }
}

/// Ohne Pflichtangaben: Empfehlungen fehlen.
fn nur_empfehlung(
    f: SchemaFeature,
    json: &'static str,
    empfohlen: &'static [&'static str],
    quelle: &'static str,
) -> Fall {
    Fall {
        status: Some(S::RecommendationsOnly),
        empfohlen,
        ..fall(f, Art::Positiv, json, quelle)
    }
}

/// Ohne Pflichtangaben und vollständig.
fn vollstaendig(f: SchemaFeature, json: &'static str, quelle: &'static str) -> Fall {
    Fall {
        status: Some(S::RecommendationsOnly),
        ..fall(f, Art::Negativ, json, quelle)
    }
}

fn na(f: SchemaFeature, json: &'static str, quelle: &'static str) -> Fall {
    fall(f, Art::NichtAnwendbar, json, quelle)
}

impl Fall {
    fn kontext(mut self, k: ProductRuleContext) -> Self {
        self.kontext = Some(k);
        self
    }
    fn empfohlen(mut self, e: &'static [&'static str]) -> Self {
        self.empfohlen = e;
        self
    }
    fn nicht_empfohlen(mut self, e: &'static [&'static str]) -> Self {
        self.nicht_empfohlen = e;
        self
    }
    fn manuell(mut self, m: &'static [ManualReview]) -> Self {
        self.manuell = m;
        self
    }
}

const KONTEXTE: [ProductRuleContext; 3] = [
    ProductRuleContext::MerchantProductDetail,
    ProductRuleContext::EditorialProduct,
    ProductRuleContext::Indeterminate,
];

fn bewerte(json: &str, kontext: ProductRuleContext, f: SchemaFeature) -> Vec<SchemaRuleAssessment> {
    let block = parse_block(json);
    block
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(i, n)| {
            n.types
                .iter()
                .flat_map(move |t| assess_node(i, t, &n.content, kontext))
        })
        .filter(|a| a.feature == f)
        .collect()
}

fn pruefe(fall: &Fall) -> Result<(), String> {
    let kontexte: Vec<_> = match fall.kontext {
        Some(k) => vec![k],
        None => KONTEXTE.to_vec(),
    };
    let ergebnisse: Vec<_> = kontexte
        .iter()
        .map(|k| bewerte(fall.json, *k, fall.feature))
        .collect();
    if ergebnisse.windows(2).any(|w| w[0] != w[1]) {
        return Err("Ergebnis hängt vom Produktkontext ab".into());
    }
    let r = &ergebnisse[0];
    if fall.art == Art::NichtAnwendbar {
        return if r.is_empty() {
            Ok(())
        } else {
            Err(format!("keine Bewertung erwartet, erhalten {r:?}"))
        };
    }
    let [a] = r.as_slice() else {
        return Err(format!(
            "genau eine Bewertung erwartet, erhalten {}",
            r.len()
        ));
    };
    let status = match fall.art {
        Art::NichtBewertet => S::NotEvaluated,
        _ => fall.status.expect("Status"),
    };
    if a.requirement_status != status {
        return Err(format!(
            "Status {:?} erwartet, erhalten {:?} (fehlend {:?})",
            status, a.requirement_status, a.missing_required
        ));
    }
    let pflicht: Vec<&str> = a.missing_required.iter().map(String::as_str).collect();
    if pflicht != fall.pflicht {
        return Err(format!(
            "fehlende Pflichtpfade {:?} erwartet, erhalten {pflicht:?}",
            fall.pflicht
        ));
    }
    let empfohlen: Vec<&str> = a.missing_recommended.iter().map(String::as_str).collect();
    if fall.art == Art::Negativ && !profil(fall.feature).0 && !empfohlen.is_empty() {
        return Err(format!(
            "keine fehlende Empfehlung erwartet, erhalten {empfohlen:?}"
        ));
    }
    for e in fall.empfohlen {
        if !empfohlen.contains(e) {
            return Err(format!("Empfehlung {e:?} fehlt nicht in {empfohlen:?}"));
        }
    }
    for e in fall.nicht_empfohlen {
        if empfohlen.contains(e) {
            return Err(format!(
                "Empfehlung {e:?} sollte nicht fehlen: {empfohlen:?}"
            ));
        }
    }
    for m in fall.manuell {
        if !a.manual_review.contains(m) {
            return Err(format!("Handprüfung {m:?} fehlt in {:?}", a.manual_review));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn faelle() -> Vec<Fall> {
    use ProductRuleContext as K;
    use SchemaFeature as F;
    const PS: &str = "Google: Product snippet, required properties (name; review | aggregateRating | offers; Offer.price, AggregateOffer.lowPrice + priceCurrency)";
    const ML: &str = "Google: Merchant listing, required properties (name, image, offers: Offer with price and priceCurrency)";
    const AR: &str = "Google: Article — no required properties, only recommended";
    const BC: &str = "Google: Breadcrumb — BreadcrumbList with at least two ListItem; ListItem name, position; item except for the last";
    const OR: &str = "Google: Organization — no required properties";
    const LB: &str = "Google: Local business — required name, address";
    const FQ: &str = "Google: FAQ — FAQPage.mainEntity (Question); Question.name and acceptedAnswer; Answer.text";
    const EV: &str = "Google: Event — required location, name, startDate; previousStartDate for a rescheduled event";
    const RC: &str =
        "Google: Recipe — required image, name; recipeYield when nutrition.calories is given";
    const VD: &str = "Google: Video — required name, thumbnailUrl, uploadDate";
    const JP: &str = "Google: Job posting — required datePosted, description, hiringOrganization, title, jobLocation; applicantLocationRequirements for TELECOMMUTE";
    const SA: &str = "Google: Software app — required name, offers.price, aggregateRating | review";
    const PP: &str = "Google: Profile page — required mainEntity (Person | Organization) with name";
    const IL: &str = "Google: Carousel (ItemList) — itemListElement with ListItem position and url";
    const WP: &str = "schema.org/WebPage — no Google feature, recommendations only";
    const WS: &str = "schema.org/WebSite — no Google feature, recommendations only";
    const PE: &str = "schema.org/Person — no Google feature, recommendations only";
    vec![
        // --- product_snippet ---
        fehlt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget"}"#,
            &["review | aggregateRating | offers"], PS),
        fehlt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","offers":{"@type":"Offer","priceCurrency":"EUR"}}"#,
            &["offers.price | offers.priceSpecification.price"], PS),
        fehlt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","offers":{"@type":"AggregateOffer","priceCurrency":"EUR"}}"#,
            &["offers.lowPrice"], PS),
        fehlt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"","review":{"@type":"Review","author":"A"}}"#,
            &["name"], "Google: name required; an empty string is no value"),
        fehlt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","offers":[{"@type":"Offer","price":"5"},{"@type":"Offer"}]}"#,
            &["offers[1].price | offers[1].priceSpecification.price"], PS),
        erfuellt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","offers":{"@type":"Offer","price":"29.99","priceCurrency":"EUR"}}"#, PS),
        erfuellt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","offers":{"@type":"Offer","priceSpecification":{"price":"29.99"}}}"#, PS),
        erfuellt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","review":{"@type":"Review","author":"A"}}"#, PS),
        erfuellt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","aggregateRating":{"ratingValue":"4"}}"#, PS),
        erfuellt(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","offers":{"@type":"AggregateOffer","lowPrice":"10","priceCurrency":"EUR"}}"#, PS),
        erfuellt(F::ProductSnippet, r#"{"@context":"https://schema.org","@graph":[{"@type":"https://schema.org/Product","name":"Gadget","review":{"@type":"Review"}}]}"#,
            "JSON-LD @graph and full IRI type are the same Product"),
        na(F::ProductSnippet, r#"{"@context":"https://schema.org","@type":"Organization","name":"Shop"}"#, PS),
        na(F::ProductSnippet, r#"{"@context":"https://example.org/vocab","@type":"Product","name":"X"}"#,
            "JSON-LD 1.1 §4.1/§3.5: under a foreign @context Product is not https://schema.org/Product (regression)"),
        // --- merchant_listing ---
        Fall { kontext: Some(K::Indeterminate), manuell: &[M::MerchantListingContext], ..fall(F::MerchantListing, Art::NichtBewertet,
            r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget"}"#, "Google: merchant listings apply only to pages where the product can be bought") },
        Fall { kontext: Some(K::EditorialProduct), ..fall(F::MerchantListing, Art::NichtBewertet,
            r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","image":"https://example.com/a.jpg","offers":{"@type":"Offer","price":"1","priceCurrency":"EUR"}}"#, "Google: an editorial product review page is not a merchant listing") },
        fehlt(F::MerchantListing, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","image":"https://example.com/a.jpg","offers":{"@type":"Offer","price":"19.99"}}"#,
            &["offers.priceCurrency | offers.priceSpecification.priceCurrency"], ML).kontext(K::MerchantProductDetail),
        fehlt(F::MerchantListing, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","offers":{"@type":"Offer","price":"19.99","priceCurrency":"EUR"}}"#,
            &["image"], ML).kontext(K::MerchantProductDetail),
        fehlt(F::MerchantListing, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","image":"https://example.com/a.jpg","offers":{"@type":"AggregateOffer","lowPrice":"1","priceCurrency":"EUR"}}"#,
            &["offers.@type = Offer", "offers.price | offers.priceSpecification.price"], ML).kontext(K::MerchantProductDetail),
        fehlt(F::MerchantListing, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","image":"https://example.com/a.jpg","offers":[{"@type":"Offer","price":"1","priceCurrency":"EUR"},{"@type":"Offer","price":"2"}]}"#,
            &["offers[1].priceCurrency | offers[1].priceSpecification.priceCurrency"], ML).kontext(K::MerchantProductDetail),
        erfuellt(F::MerchantListing, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","image":"https://example.com/a.jpg","offers":{"@type":"Offer","price":"19.99","priceCurrency":"EUR"}}"#, ML).kontext(K::MerchantProductDetail),
        erfuellt(F::MerchantListing, r#"{"@context":"https://schema.org","@type":"Product","name":"Gadget","image":"https://example.com/a.jpg","offers":{"@type":"Offer","priceSpecification":{"price":"19.99","priceCurrency":"EUR"}}}"#, ML).kontext(K::MerchantProductDetail),
        na(F::MerchantListing, r#"{"@context":"https://schema.org","@type":"Article","headline":"H"}"#, ML).kontext(K::MerchantProductDetail),
        // --- article ---
        nur_empfehlung(F::Article, r#"{"@context":"https://schema.org","@type":"Article","headline":"Audit"}"#,
            &["author", "dateModified", "datePublished", "image"], AR),
        nur_empfehlung(F::Article, r#"{"@context":"https://schema.org","@type":"NewsArticle","headline":"Audit","author":"A","dateModified":"2026-01-02","datePublished":"2026-01-01","image":"https://example.com/a.png"}"#,
            &["publisher"], "Google Article: publisher recommended for NewsArticle"),
        vollstaendig(F::Article, r#"{"@context":"https://schema.org","@type":"BlogPosting","headline":"Audit","author":{"@type":"Person","name":"Ada"},"dateModified":"2026-01-02","datePublished":"2026-01-01","image":"https://example.com/a.png"}"#, AR),
        vollstaendig(F::Article, r#"{"@context":"https://schema.org","@type":"Article","headline":"Audit","author":"A","dateModified":"2026-01-02","datePublished":"2026-01-01","image":"https://example.com/a.png"}"#, AR),
        na(F::Article, r#"{"@context":"https://schema.org","@type":"WebPage","name":"P"}"#, AR),
        // --- breadcrumb ---
        fehlt(F::Breadcrumb, r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"name":"Only"}]}"#,
            &["itemListElement (minimum 2 entries)"], "Google Breadcrumb: at least two ListItems (boundary: one)"),
        fehlt(F::Breadcrumb, r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"name":"Home"},{"@type":"ListItem","position":2,"name":"Here"}]}"#,
            &["itemListElement[0].item"], "Google Breadcrumb: item required except for the last ListItem"),
        fehlt(F::Breadcrumb, r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","name":"Home","item":"https://example.com"},{"@type":"ListItem","position":2}]}"#,
            &["itemListElement[0].position", "itemListElement[1].name"], BC),
        fehlt(F::Breadcrumb, r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[]}"#,
            &["itemListElement"], "Google Breadcrumb: itemListElement required; [] is no value"),
        erfuellt(F::Breadcrumb, r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"name":"Home","item":"https://example.com"},{"@type":"ListItem","position":2,"name":"Here"}]}"#,
            "Google Breadcrumb: two items, the last without item (boundary: two)"),
        erfuellt(F::Breadcrumb, r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{"@type":"ListItem","position":1,"item":{"@id":"https://example.com","name":"Home"}},{"@type":"ListItem","position":2,"name":"Here"}]}"#,
            "Google Breadcrumb: name may sit on item"),
        na(F::Breadcrumb, r#"{"@context":"https://schema.org","@type":"ItemList","itemListElement":[]}"#, BC),
        // --- organization ---
        nur_empfehlung(F::Organization, r#"{"@context":"https://schema.org","@type":"Organization","name":"ACME"}"#,
            &["url", "logo", "address", "telephone", "sameAs"], OR).nicht_empfohlen(&["name"]),
        vollstaendig(F::Organization, r#"{"@context":"https://schema.org","@type":"Organization","name":"ACME","url":"https://example.com","logo":"https://example.com/l.png","address":"Main St 1","telephone":"+49 1","sameAs":["https://x.example/acme"]}"#, OR),
        na(F::Organization, r#"{"@context":"https://schema.org","@type":"LocalBusiness","name":"ACME","address":"Main St 1"}"#,
            "LocalBusiness is assessed as local_business, not organization"),
        // --- local_business ---
        fehlt(F::LocalBusiness, r#"{"@context":"https://schema.org","@type":"LocalBusiness","name":"Bakery"}"#, &["address"], LB),
        fehlt(F::LocalBusiness, r#"{"@context":"https://schema.org","@type":"LocalBusiness","address":{"streetAddress":"Main St 1"},"name":null}"#, &["name"], "Google: null is no value"),
        erfuellt(F::LocalBusiness, r#"{"@context":"https://schema.org","@type":"LocalBusiness","name":"Bakery","address":{"@type":"PostalAddress","streetAddress":"Main St 1"}}"#, LB),
        na(F::LocalBusiness, r#"{"@context":"https://schema.org","@type":"Organization","name":"ACME"}"#, LB),
        // --- faq ---
        fehlt(F::Faq, r#"{"@context":"https://schema.org","@type":"FAQPage","mainEntity":[{"@type":"Question","name":"A?","acceptedAnswer":{"@type":"Answer","text":"Yes"}},{"@type":"Question","name":"B?"}]}"#,
            &["mainEntity[1].acceptedAnswer"], FQ),
        fehlt(F::Faq, r#"{"@context":"https://schema.org","@type":"FAQPage","mainEntity":{"@type":"Question","acceptedAnswer":{"@type":"Answer","text":"Yes"}}}"#,
            &["mainEntity[0].name"], "Google FAQ: Question.name required (regression)"),
        fehlt(F::Faq, r#"{"@context":"https://schema.org","@type":"FAQPage","mainEntity":{"@type":"Question","name":"A?","acceptedAnswer":{"@type":"Answer"}}}"#,
            &["mainEntity[0].acceptedAnswer.text"], "Google FAQ: Answer.text required (regression)"),
        fehlt(F::Faq, r#"{"@context":"https://schema.org","@type":"FAQPage"}"#, &["mainEntity"], FQ),
        erfuellt(F::Faq, r#"{"@context":"https://schema.org","@type":"FAQPage","mainEntity":[{"@type":"Question","name":"A?","acceptedAnswer":{"@type":"Answer","text":"Yes"}}]}"#, FQ),
        na(F::Faq, r#"{"@context":"https://schema.org","@type":"WebPage","name":"FAQ"}"#, FQ),
        // --- event ---
        fehlt(F::Event, r#"{"@context":"https://schema.org","@type":"Event","name":"Concert","location":{"@type":"Place","name":"Hall"}}"#,
            &["startDate"], EV).manuell(&[M::EventIsSinglePublicEvent]),
        fehlt(F::Event, r#"{"@context":"https://schema.org","@type":"Event","name":"Concert","startDate":"2026-05-01","location":"Hall","eventStatus":"https://schema.org/EventRescheduled"}"#,
            &["previousStartDate (when eventStatus is EventRescheduled)"], EV),
        erfuellt(F::Event, r#"{"@context":"https://schema.org","@type":"Event","name":"Concert","startDate":"2026-05-01","location":"Hall","eventStatus":"https://schema.org/EventRescheduled","previousStartDate":"2026-04-01"}"#, EV),
        erfuellt(F::Event, r#"{"@context":"https://schema.org","@type":"Event","name":"Concert","startDate":"2026-05-01","location":"Hall","eventStatus":"https://schema.org/EventCancelled"}"#,
            "Google Event: a cancelled event keeps its date and place — a manual check").manuell(&[M::CancelledEventKeepsDetails, M::EventIsSinglePublicEvent]),
        na(F::Event, r#"{"@context":"https://schema.org","@type":"Organization","name":"Hall"}"#, EV),
        // --- recipe ---
        fehlt(F::Recipe, r#"{"@context":"https://schema.org","@type":"Recipe","name":"Soup"}"#, &["image"], RC)
            .manuell(&[M::RecipeMatchesVisibleRecipe]),
        fehlt(F::Recipe, r#"{"@context":"https://schema.org","@type":"Recipe","name":"Soup","image":"https://example.com/s.jpg","nutrition":{"calories":"200 kcal"}}"#,
            &["recipeYield (when nutrition.calories is present)"], RC),
        erfuellt(F::Recipe, r#"{"@context":"https://schema.org","@type":"Recipe","name":"Soup","image":"https://example.com/s.jpg","nutrition":{"calories":"200 kcal"},"recipeYield":"4"}"#, RC),
        erfuellt(F::Recipe, r#"{"@context":"https://schema.org","@type":"Recipe","name":"Soup","image":"https://example.com/s.jpg","cookTime":"PT20M"}"#,
            "Google Recipe: cookTime and prepTime together — one alone is a manual check").manuell(&[M::RecipeCookAndPrepTime]),
        na(F::Recipe, r#"{"@context":"https://schema.org","@type":"Article","headline":"Soup"}"#, RC),
        // --- video ---
        fehlt(F::Video, r#"{"@context":"https://schema.org","@type":"VideoObject","name":"Intro","thumbnailUrl":"https://example.com/t.jpg"}"#,
            &["uploadDate"], VD),
        erfuellt(F::Video, r#"{"@context":"https://schema.org","@type":"VideoObject","name":"Intro","thumbnailUrl":"https://example.com/t.jpg","uploadDate":"2026-01-01"}"#, VD)
            .empfohlen(&["contentUrl | embedUrl"]),
        erfuellt(F::Video, r#"{"@context":"https://schema.org","@type":"VideoObject","name":"Intro","thumbnailUrl":"https://example.com/t.jpg","uploadDate":"2026-01-01","embedUrl":"https://example.com/e"}"#,
            "Google Video: contentUrl or embedUrl, either suffices").nicht_empfohlen(&["contentUrl | embedUrl"]),
        na(F::Video, r#"{"@context":"https://schema.org","@type":"WebPage","name":"Intro"}"#, VD),
        // --- job_posting ---
        fehlt(F::JobPosting, r#"{"@context":"https://schema.org","@type":"JobPosting","title":"Dev","datePosted":"2026-01-01","description":"Code","hiringOrganization":"ACME"}"#,
            &["jobLocation | remote-job properties"], JP).manuell(&[M::JobPostingIsOpenPosition]),
        fehlt(F::JobPosting, r#"{"@context":"https://schema.org","@type":"JobPosting","title":"Dev","datePosted":"2026-01-01","description":"Code","hiringOrganization":"ACME","jobLocationType":"TELECOMMUTE"}"#,
            &["applicantLocationRequirements (for a fully remote job)"], JP),
        erfuellt(F::JobPosting, r#"{"@context":"https://schema.org","@type":"JobPosting","title":"Dev","datePosted":"2026-01-01","description":"Code","hiringOrganization":"ACME","jobLocation":{"@type":"Place","address":"Berlin"}}"#, JP),
        erfuellt(F::JobPosting, r#"{"@context":"https://schema.org","@type":"JobPosting","title":"Dev","datePosted":"2026-01-01","description":"Code","hiringOrganization":"ACME","jobLocationType":"TELECOMMUTE","applicantLocationRequirements":{"@type":"Country","name":"DE"}}"#, JP),
        na(F::JobPosting, r#"{"@context":"https://schema.org","@type":"Organization","name":"ACME"}"#, JP),
        // --- software_application ---
        fehlt(F::SoftwareApplication, r#"{"@context":"https://schema.org","@type":"SoftwareApplication","name":"App","offers":{"price":"0"}}"#,
            &["aggregateRating | review"], SA),
        fehlt(F::SoftwareApplication, r#"{"@context":"https://schema.org","@type":"MobileApplication","name":"App","aggregateRating":{"ratingValue":"4"}}"#,
            &["offers.price"], SA),
        erfuellt(F::SoftwareApplication, r#"{"@context":"https://schema.org","@type":"SoftwareApplication","name":"App","offers":{"price":0},"aggregateRating":{"ratingValue":"4"}}"#,
            "Google Software app: price 0 for a free app is a value (boundary 0)")
            .nicht_empfohlen(&["offers.priceCurrency (when price is greater than 0)"]).manuell(&[M::SoftwareRatingIsGenuine]),
        erfuellt(F::SoftwareApplication, r#"{"@context":"https://schema.org","@type":"WebApplication","name":"App","offers":{"price":"0.01"},"review":{"@type":"Review"}}"#,
            "Google Software app: priceCurrency when price > 0 (boundary 0.01)")
            .empfohlen(&["offers.priceCurrency (when price is greater than 0)"]),
        na(F::SoftwareApplication, r#"{"@context":"https://schema.org","@type":"Product","name":"App"}"#, SA),
        // --- profile_page ---
        fehlt(F::ProfilePage, r#"{"@context":"https://schema.org","@type":"ProfilePage","mainEntity":{"@type":"Thing","name":"Ada"}}"#,
            &["mainEntity.@type = Person | Organization"], PP).manuell(&[M::ProfilePageSubject]),
        fehlt(F::ProfilePage, r#"{"@context":"https://schema.org","@type":"ProfilePage","dateCreated":"2026-01-01"}"#,
            &["mainEntity", "mainEntity.name"], PP),
        fehlt(F::ProfilePage, r#"{"@context":"https://schema.org","@type":"ProfilePage","mainEntity":{"@type":"Person"}}"#,
            &["mainEntity.name"], PP),
        erfuellt(F::ProfilePage, r#"{"@context":"https://schema.org","@type":"ProfilePage","mainEntity":{"@type":"Person","name":"Ada"}}"#, PP),
        erfuellt(F::ProfilePage, r#"{"@context":"https://schema.org","@type":"ProfilePage","mainEntity":{"@type":"https://schema.org/Organization","name":"ACME"}}"#, PP),
        na(F::ProfilePage, r#"{"@context":"https://schema.org","@type":"Person","name":"Ada"}"#, PP),
        // --- item_list ---
        fehlt(F::ItemList, r#"{"@context":"https://schema.org","@type":"ItemList","itemListElement":[{"@type":"ListItem","position":1}]}"#,
            &["itemListElement[0].url"], IL).manuell(&[M::ItemListCanonicalUrls]),
        fehlt(F::ItemList, r#"{"@context":"https://schema.org","@type":"ItemList","name":"Top"}"#, &["itemListElement"], IL),
        erfuellt(F::ItemList, r#"{"@context":"https://schema.org","@type":"ItemList","itemListElement":[{"@type":"ListItem","position":1,"url":"https://example.com/a"}]}"#, IL),
        Fall { status: Some(S::RecommendationsOnly), ..fall(F::ItemList, Art::Negativ,
            r#"{"@context":"https://schema.org","@type":"CollectionPage","name":"All","url":"https://example.com/all","hasPart":[{"@type":"WebPage"}]}"#,
            "CollectionPage: carousel context only, no required properties") },
        na(F::ItemList, r#"{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[]}"#, IL),
        // --- web_page ---
        nur_empfehlung(F::WebPage, r#"{"@context":"https://schema.org","@type":"WebPage","name":"Home"}"#,
            &["url", "description", "inLanguage", "isPartOf"], WP).nicht_empfohlen(&["name"]),
        vollstaendig(F::WebPage, r#"{"@context":"https://schema.org","@type":"WebPage","name":"Home","url":"https://example.com","description":"D","inLanguage":"en","isPartOf":{"@type":"WebSite"}}"#, WP),
        na(F::WebPage, r#"{"@context":"https://schema.org","@type":"WebSite","name":"Site"}"#, WP),
        // --- web_site ---
        nur_empfehlung(F::WebSite, r#"{"@context":"https://schema.org","@type":"WebSite","name":"Site"}"#,
            &["url", "inLanguage", "publisher", "potentialAction"], WS).nicht_empfohlen(&["name"]),
        vollstaendig(F::WebSite, r#"{"@context":"https://schema.org","@type":"WebSite","name":"Site","url":"https://example.com","inLanguage":"en","publisher":{"@type":"Organization"},"potentialAction":{"@type":"SearchAction"}}"#, WS),
        na(F::WebSite, r#"{"@context":"https://schema.org","@type":"WebPage","name":"Home"}"#, WS),
        // --- person ---
        nur_empfehlung(F::Person, r#"{"@context":"https://schema.org","@type":"Person","name":"Ada"}"#,
            &["url", "image", "jobTitle", "worksFor", "sameAs"], PE).nicht_empfohlen(&["name"]),
        vollstaendig(F::Person, r#"{"@context":"https://schema.org","@type":"Person","name":"Ada","url":"https://example.com/ada","image":"https://example.com/a.jpg","jobTitle":"Engineer","worksFor":{"@type":"Organization"},"sameAs":"https://x.example/ada"}"#, PE),
        na(F::Person, r#"{"@context":"https://schema.org","@type":"ProfilePage","mainEntity":{"@type":"Organization","name":"ACME"}}"#,
            "a Person only nested in mainEntity is not a node of its own"),
    ]
}

#[test]
fn korpus_faelle() {
    let nur = std::env::var("KORPUS_ID").ok();
    let auswahl: Option<Vec<&str>> = nur.as_deref().map(|s| s.split(',').collect());
    let mut fehler = Vec::new();
    let mut gelaufen = 0;
    for f in faelle() {
        if auswahl
            .as_ref()
            .is_some_and(|ids| !ids.contains(&f.feature.key()))
        {
            continue;
        }
        gelaufen += 1;
        if let Err(grund) = pruefe(&f) {
            fehler.push(format!(
                "{} [{:?}]: {grund}\n    Quelle: {}\n    {}",
                f.feature.key(),
                f.art,
                f.quelle,
                f.json
            ));
        }
    }
    assert!(gelaufen > 0, "keine Fälle gelaufen (KORPUS_ID = {nur:?})");
    assert!(
        fehler.is_empty(),
        "{} von {gelaufen} Fällen scheitern:\n{}",
        fehler.len(),
        fehler.join("\n")
    );
}

#[test]
fn gate_jedes_merkmal_hat_alle_arten() {
    let mut arten: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for f in faelle() {
        assert!(!f.quelle.trim().is_empty());
        let art = match (f.art, f.status) {
            (Art::Positiv, Some(S::RecommendationsOnly)) => "positiv (nur Empfehlungen)",
            (Art::Positiv, _) => "positiv",
            (Art::Negativ, _) => "negativ",
            (Art::NichtAnwendbar, _) => "n/a",
            (Art::NichtBewertet, _) => "nicht bewertet",
        };
        arten
            .entry(f.feature.key())
            .or_default()
            .insert(art.to_string());
    }
    let mut luecken = Vec::new();
    for feature in SchemaFeature::ALL {
        let (pflicht, kontext) = profil(feature);
        let mut noetig = vec![
            if pflicht {
                "positiv"
            } else {
                "positiv (nur Empfehlungen)"
            },
            "negativ",
            "n/a",
        ];
        if kontext {
            noetig.push("nicht bewertet");
        }
        let da = arten.get(feature.key()).cloned().unwrap_or_default();
        for n in noetig {
            if !da.contains(n) {
                luecken.push(format!("{}: {n} fehlt", feature.key()));
            }
        }
        // Ein Merkmal ohne Pflichtangaben darf nie als fehlerhaft gelten.
        if !pflicht && da.contains("positiv") {
            luecken.push(format!(
                "{}: ohne Pflichtangaben, aber Fehlerfall",
                feature.key()
            ));
        }
    }
    assert!(luecken.is_empty(), "{}", luecken.join("\n"));
}

/// Was `profil` behauptet, muss stimmen: Ohne Pflichtangaben gibt es keinen
/// fehlerhaften Zustand, und nur kontextabhängige Merkmale bleiben unbewertet.
#[test]
fn gate_profil_stimmt() {
    for f in faelle() {
        let (pflicht, kontext) = profil(f.feature);
        for k in KONTEXTE {
            for a in bewerte(f.json, k, f.feature) {
                if !pflicht {
                    assert_eq!(a.requirement_status, S::RecommendationsOnly, "{}", f.json);
                }
                if !kontext {
                    assert_ne!(a.requirement_status, S::NotEvaluated, "{}", f.json);
                }
            }
        }
    }
}

#[test]
fn gate_mutationskatalog_deckt_jedes_merkmal() {
    let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let eintraege = korpus::katalog::lade(&wurzel.join("tests/mutations"));
    let keys: BTreeSet<&str> = SchemaFeature::ALL.iter().map(|f| f.key()).collect();
    let mut fehler = Vec::new();
    for e in &eintraege {
        if !keys.contains(e.id.as_str()) {
            fehler.push(format!("{}: kein Merkmal", e.id));
        }
        let text = std::fs::read_to_string(wurzel.join(&e.file)).unwrap_or_default();
        let n = text.matches(e.find.as_str()).count();
        if n != 1 {
            fehler.push(format!("{}: `find` kommt {n}-mal in {} vor", e.id, e.file));
        }
        if e.find == e.replace {
            fehler.push(format!("{}: `find` und `replace` sind gleich", e.id));
        }
        if e.expect == "equivalent" && e.note.trim().is_empty() {
            fehler.push(format!("{}: äquivalente Mutation ohne Begründung", e.id));
        }
    }
    for k in &keys {
        if !eintraege.iter().any(|e| e.id == *k && e.expect == "killed") {
            fehler.push(format!("{k}: keine Mutation, die ein Test töten muss"));
        }
    }
    assert!(fehler.is_empty(), "{}", fehler.join("\n"));
}

/// Die Fälle sind gültiges JSON — sonst prüften sie nur den Parserfehler.
#[test]
fn faelle_sind_json() {
    for f in faelle() {
        serde_json::from_str::<Value>(f.json).unwrap_or_else(|e| panic!("{e}: {}", f.json));
    }
}
