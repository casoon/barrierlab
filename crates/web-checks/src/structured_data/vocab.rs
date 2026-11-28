//! Gebündeltes schema.org-Vokabular: Typhierarchie, Typ-/Property-/Range-Prüfung
//! und Entitäts-Zusammenfassung für seitenübergreifende Prüfungen.
//!
//! Offline-Snapshot ohne Netzabruf zur Laufzeit. Aktualisierungsweg:
//! Typ- und Subtyp-Listen gegen `https://schema.org/version/latest/schemaorg-current-https.jsonld`
//! abgleichen und [`SCHEMA_ORG_VERSION`] hochziehen.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version des gebündelten schema.org-Vokabulars.
pub const SCHEMA_ORG_VERSION: &str = "29.0";

/// Alle offiziellen `LocalBusiness`-Unterarten aus schema.org (ohne veraltete Typen).
const LOCAL_BUSINESS_SUBTYPES: &[&str] = &[
    "AccountingService",
    "AdultEntertainment",
    "AmusementPark",
    "AnimalShelter",
    "ArchiveOrganization",
    "ArtGallery",
    "Attorney",
    "Audiology",
    "AutoBodyShop",
    "AutoDealer",
    "AutoPartsStore",
    "AutoRental",
    "AutoRepair",
    "AutoWash",
    "AutomatedTeller",
    "AutomotiveBusiness",
    "Bakery",
    "BankOrCreditUnion",
    "BarOrPub",
    "BeautySalon",
    "BedAndBreakfast",
    "BikeStore",
    "BookStore",
    "BowlingAlley",
    "Brewery",
    "CafeOrCoffeeShop",
    "Campground",
    "Casino",
    "ChildCare",
    "ClothingStore",
    "ComedyClub",
    "CommunityHealth",
    "ComputerStore",
    "ConvenienceStore",
    "CovidTestingFacility",
    "DaySpa",
    "Dentist",
    "DepartmentStore",
    "Dermatology",
    "DietNutrition",
    "Distillery",
    "DryCleaningOrLaundry",
    "Electrician",
    "ElectronicsStore",
    "Emergency",
    "EmergencyService",
    "EmploymentAgency",
    "EntertainmentBusiness",
    "ExerciseGym",
    "FastFoodRestaurant",
    "FinancialService",
    "FireStation",
    "Florist",
    "FoodEstablishment",
    "FurnitureStore",
    "GardenStore",
    "GasStation",
    "GeneralContractor",
    "Geriatric",
    "GolfCourse",
    "GovernmentOffice",
    "GroceryStore",
    "Gynecologic",
    "HVACBusiness",
    "HairSalon",
    "HardwareStore",
    "HealthAndBeautyBusiness",
    "HealthClub",
    "HobbyShop",
    "HomeAndConstructionBusiness",
    "HomeGoodsStore",
    "Hospital",
    "Hostel",
    "Hotel",
    "HousePainter",
    "IceCreamShop",
    "IndividualPhysician",
    "InsuranceAgency",
    "InternetCafe",
    "JewelryStore",
    "LegalService",
    "Library",
    "LiquorStore",
    "Locksmith",
    "LodgingBusiness",
    "MedicalBusiness",
    "MedicalClinic",
    "MensClothingStore",
    "Midwifery",
    "MobilePhoneStore",
    "Motel",
    "MotorcycleDealer",
    "MotorcycleRepair",
    "MovieRentalStore",
    "MovieTheater",
    "MovingCompany",
    "MusicStore",
    "NailSalon",
    "NightClub",
    "Notary",
    "Nursing",
    "Obstetric",
    "OfficeEquipmentStore",
    "Oncologic",
    "Ophthalmology",
    "Optician",
    "Optometric",
    "Otolaryngologic",
    "OutletStore",
    "PawnShop",
    "Pediatric",
    "PetStore",
    "Pharmacy",
    "Physician",
    "PhysiciansOffice",
    "Physiotherapy",
    "PlasticSurgery",
    "Plumber",
    "Podiatric",
    "PoliceStation",
    "PostOffice",
    "PrimaryCare",
    "ProfessionalService",
    "Psychiatric",
    "PublicHealth",
    "PublicSwimmingPool",
    "RadioStation",
    "RealEstateAgent",
    "RecyclingCenter",
    "Resort",
    "Restaurant",
    "RoofingContractor",
    "SelfStorage",
    "ShoeStore",
    "ShoppingCenter",
    "SkiResort",
    "SportingGoodsStore",
    "SportsActivityLocation",
    "SportsClub",
    "StadiumOrArena",
    "Store",
    "TattooParlor",
    "TelevisionStation",
    "TennisComplex",
    "TireShop",
    "TouristInformationCenter",
    "ToyStore",
    "TravelAgency",
    "VacationRental",
    "WholesaleStore",
    "Winery",
];

/// Kern-Typhierarchie `(Typ, direkte Obertypen)`.
const TYPE_PARENTS: &[(&str, &[&str])] = &[
    ("Thing", &[]),
    // Organization & Place
    ("Organization", &["Thing"]),
    ("Place", &["Thing"]),
    ("LocalBusiness", &["Organization", "Place"]),
    ("Corporation", &["Organization"]),
    ("EducationalOrganization", &["Organization"]),
    ("CollegeOrUniversity", &["EducationalOrganization"]),
    ("School", &["EducationalOrganization"]),
    ("GovernmentOrganization", &["Organization"]),
    ("NGO", &["Organization"]),
    ("PerformingGroup", &["Organization"]),
    ("Project", &["Organization"]),
    ("ResearchOrganization", &["Organization"]),
    ("SportsOrganization", &["Organization"]),
    ("WorkersUnion", &["Organization"]),
    ("Airline", &["Organization"]),
    ("Consortium", &["Organization"]),
    ("FundingScheme", &["Organization"]),
    ("LibrarySystem", &["Organization"]),
    ("MedicalOrganization", &["Organization"]),
    ("NewsMediaOrganization", &["Organization"]),
    ("OnlineBusiness", &["Organization"]),
    ("OnlineStore", &["OnlineBusiness"]),
    ("PoliticalParty", &["Organization"]),
    ("SearchRescueOrganization", &["Organization"]),
    ("AdministrativeArea", &["Place"]),
    ("City", &["AdministrativeArea"]),
    ("State", &["AdministrativeArea"]),
    ("Country", &["AdministrativeArea"]),
    ("CivicStructure", &["Place"]),
    ("TouristAttraction", &["Place"]),
    ("Residence", &["Place"]),
    // Person
    ("Person", &["Thing"]),
    // CreativeWork, Article, WebPage, WebSite
    ("CreativeWork", &["Thing"]),
    ("Article", &["CreativeWork"]),
    ("BlogPosting", &["SocialMediaPosting"]),
    ("SocialMediaPosting", &["Article"]),
    ("NewsArticle", &["Article"]),
    ("TechArticle", &["Article"]),
    ("ScholarlyArticle", &["Article"]),
    ("Report", &["Article"]),
    ("SatiricalArticle", &["Article"]),
    ("AdvertiserContentArticle", &["Article"]),
    ("AnalysisNewsArticle", &["NewsArticle"]),
    ("AskPublicNewsArticle", &["NewsArticle"]),
    ("BackgroundNewsArticle", &["NewsArticle"]),
    ("OpinionNewsArticle", &["NewsArticle"]),
    ("ReportageNewsArticle", &["NewsArticle"]),
    ("ReviewNewsArticle", &["NewsArticle"]),
    ("WebPage", &["CreativeWork"]),
    ("AboutPage", &["WebPage"]),
    ("CheckoutPage", &["WebPage"]),
    ("CollectionPage", &["WebPage"]),
    ("ContactPage", &["WebPage"]),
    ("FAQPage", &["WebPage"]),
    ("ItemPage", &["WebPage"]),
    ("MedicalWebPage", &["WebPage"]),
    ("ProfilePage", &["WebPage"]),
    ("QAPage", &["WebPage"]),
    ("RealEstateListing", &["WebPage"]),
    ("SearchResultsPage", &["WebPage"]),
    ("WebSite", &["CreativeWork"]),
    ("Blog", &["CreativeWork"]),
    ("Book", &["CreativeWork"]),
    ("Comment", &["CreativeWork"]),
    ("Course", &["CreativeWork"]),
    ("CreativeWorkSeries", &["CreativeWork"]),
    ("Dataset", &["CreativeWork"]),
    ("HowTo", &["CreativeWork"]),
    ("HowToStep", &["CreativeWork"]),
    ("HowToSection", &["CreativeWork"]),
    ("Recipe", &["HowTo"]),
    ("Review", &["CreativeWork"]),
    ("Question", &["Comment"]),
    ("Answer", &["Comment"]),
    ("Menu", &["CreativeWork"]),
    ("Map", &["CreativeWork"]),
    ("Photograph", &["CreativeWork"]),
    ("SoftwareSourceCode", &["CreativeWork"]),
    ("SoftwareApplication", &["CreativeWork"]),
    ("WebApplication", &["SoftwareApplication"]),
    ("MobileApplication", &["SoftwareApplication"]),
    ("VideoGame", &["SoftwareApplication"]),
    ("MediaObject", &["CreativeWork"]),
    ("ImageObject", &["MediaObject"]),
    ("VideoObject", &["MediaObject"]),
    ("AudioObject", &["MediaObject"]),
    // Product & Offer
    ("Product", &["Thing"]),
    ("ProductGroup", &["Product"]),
    ("IndividualProduct", &["Product"]),
    ("ProductModel", &["Product"]),
    ("SomeProducts", &["Product"]),
    ("Vehicle", &["Product"]),
    // Event
    ("Event", &["Thing"]),
    ("BusinessEvent", &["Event"]),
    ("ChildrensEvent", &["Event"]),
    ("ComedyEvent", &["Event"]),
    ("CourseInstance", &["Event"]),
    ("DanceEvent", &["Event"]),
    ("EducationEvent", &["Event"]),
    ("ExhibitionEvent", &["Event"]),
    ("Festival", &["Event"]),
    ("FoodEvent", &["Event"]),
    ("Hackathon", &["Event"]),
    ("LiteraryEvent", &["Event"]),
    ("MusicEvent", &["Event"]),
    ("PublicationEvent", &["Event"]),
    ("SaleEvent", &["Event"]),
    ("ScreeningEvent", &["Event"]),
    ("SocialEvent", &["Event"]),
    ("SportsEvent", &["Event"]),
    ("TheaterEvent", &["Event"]),
    ("VisualArtsEvent", &["Event"]),
    // Intangible & Structured Values
    ("Intangible", &["Thing"]),
    ("Action", &["Thing"]),
    ("SearchAction", &["Action"]),
    ("Offer", &["Intangible"]),
    ("AggregateOffer", &["Offer"]),
    ("Brand", &["Intangible"]),
    ("JobPosting", &["Intangible"]),
    ("ItemList", &["Intangible"]),
    ("BreadcrumbList", &["ItemList"]),
    ("OfferCatalog", &["ItemList"]),
    ("ListItem", &["Intangible"]),
    ("Rating", &["Intangible"]),
    ("AggregateRating", &["Rating"]),
    ("Service", &["Intangible"]),
    ("StructuredValue", &["Intangible"]),
    ("ContactPoint", &["StructuredValue"]),
    ("PostalAddress", &["ContactPoint"]),
    ("GeoCoordinates", &["StructuredValue"]),
    ("OpeningHoursSpecification", &["StructuredValue"]),
    ("OfferShippingDetails", &["StructuredValue"]),
    ("MerchantReturnPolicy", &["Intangible"]),
    ("DefinedRegion", &["StructuredValue"]),
    ("MonetaryAmount", &["StructuredValue"]),
    ("QuantitativeValue", &["StructuredValue"]),
    ("PriceSpecification", &["StructuredValue"]),
    ("UnitPriceSpecification", &["PriceSpecification"]),
    ("ShippingDeliveryTime", &["StructuredValue"]),
    ("NutritionInformation", &["StructuredValue"]),
    ("InteractionCounter", &["StructuredValue"]),
    ("PropertyValue", &["StructuredValue"]),
    ("EntryPoint", &["Intangible"]),
    ("SpeakableSpecification", &["Intangible"]),
    ("VirtualLocation", &["Intangible"]),
];

/// Gängige schema.org-Properties für Tippfehler-Erkennung (Domain-unabhängig).
const KNOWN_PROPERTIES: &[&str] = &[
    "about",
    "acceptedAnswer",
    "address",
    "addressCountry",
    "addressLocality",
    "addressRegion",
    "aggregateRating",
    "alternateName",
    "applicableCountry",
    "applicantLocationRequirements",
    "applicationCategory",
    "areaServed",
    "articleBody",
    "articleSection",
    "author",
    "availability",
    "availableLanguage",
    "baseSalary",
    "bestRating",
    "brand",
    "breadcrumb",
    "calories",
    "caption",
    "closes",
    "contactPoint",
    "contactType",
    "contentUrl",
    "cookTime",
    "copyrightHolder",
    "copyrightYear",
    "creator",
    "currency",
    "dateCreated",
    "dateModified",
    "datePosted",
    "datePublished",
    "dayOfWeek",
    "deliveryTime",
    "description",
    "directApply",
    "doesNotShip",
    "duration",
    "email",
    "embedUrl",
    "employmentType",
    "encodingFormat",
    "endDate",
    "eventAttendanceMode",
    "eventStatus",
    "expires",
    "founder",
    "foundingDate",
    "geo",
    "gtin",
    "gtin8",
    "gtin12",
    "gtin13",
    "gtin14",
    "handlingTime",
    "hasMerchantReturnPolicy",
    "hasOfferCatalog",
    "hasPart",
    "hasVariant",
    "headline",
    "height",
    "highPrice",
    "hiringOrganization",
    "identifier",
    "image",
    "inLanguage",
    "interactionStatistic",
    "interactionType",
    "isAccessibleForFree",
    "isAccessoryOrSparePartFor",
    "isConsumableFor",
    "isPartOf",
    "isRelatedTo",
    "isSimilarTo",
    "isVariantOf",
    "item",
    "itemCondition",
    "itemListElement",
    "itemOffered",
    "jobLocation",
    "jobLocationType",
    "jobTitle",
    "keywords",
    "knowsAbout",
    "latitude",
    "legalName",
    "location",
    "logo",
    "longitude",
    "lowPrice",
    "mainEntity",
    "mainEntityOfPage",
    "maxValue",
    "mentions",
    "merchantReturnDays",
    "merchantReturnLink",
    "minValue",
    "mpn",
    "name",
    "numberOfEmployees",
    "numberOfItems",
    "nutrition",
    "offerCount",
    "offers",
    "openingHours",
    "openingHoursSpecification",
    "opens",
    "operatingSystem",
    "organizer",
    "parentOrganization",
    "performer",
    "position",
    "postalCode",
    "potentialAction",
    "prepTime",
    "previousStartDate",
    "price",
    "priceCurrency",
    "priceRange",
    "priceSpecification",
    "priceType",
    "priceValidUntil",
    "primaryImageOfPage",
    "publisher",
    "publishingPrinciples",
    "query-input",
    "ratingCount",
    "ratingValue",
    "recipeCategory",
    "recipeCuisine",
    "recipeIngredient",
    "recipeInstructions",
    "recipeYield",
    "regionsAllowed",
    "returnFees",
    "returnMethod",
    "returnPolicyCategory",
    "returnShippingFeesAmount",
    "review",
    "reviewBody",
    "reviewCount",
    "reviewRating",
    "sameAs",
    "serviceType",
    "shippingDestination",
    "shippingDetails",
    "shippingRate",
    "sku",
    "slogan",
    "speakable",
    "startDate",
    "streetAddress",
    "subEvent",
    "subOrganization",
    "superEvent",
    "target",
    "taxID",
    "telephone",
    "text",
    "thumbnail",
    "thumbnailUrl",
    "timeRequired",
    "title",
    "totalTime",
    "transitTime",
    "unitCode",
    "unitText",
    "uploadDate",
    "url",
    "urlTemplate",
    "userInteractionCount",
    "validFrom",
    "validThrough",
    "value",
    "vatID",
    "width",
    "wordCount",
    "workTranslation",
    "worksFor",
    "worstRating",
];

/// Vokabular-Problem an einem JSON-LD-Knoten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VocabIssue {
    /// `@type` existiert nicht im gebündelten schema.org-Vokabular.
    UnknownType {
        schema_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        suggestion: Option<String>,
    },
    /// Property ist unbekannt (oft Tippfehler wie `adress` oder `telephon`).
    UnknownProperty {
        property: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        suggestion: Option<String>,
    },
    /// Property gehört nicht zur Domain des angegebenen `@type` (z. B. `publishingPrinciples` auf `Person`).
    InvalidPropertyDomain {
        property: String,
        schema_type: String,
        expected_domain: String,
    },
    /// Wert einer Property passt nicht zur erwarteten Range (z. B. `geo` als Text oder `address` als Zahl).
    InvalidRange {
        property: String,
        expected: String,
    },
}

/// Zusammenfassung eines Entitäts-Knotens für seitenübergreifende Konsistenzprüfungen.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntitySummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<(f64, f64)>,
}

/// Ob `schema_type` im gebündelten schema.org-Vokabular bekannt ist.
pub fn is_known_type(schema_type: &str) -> bool {
    LOCAL_BUSINESS_SUBTYPES.contains(&schema_type)
        || TYPE_PARENTS.iter().any(|(t, _)| *t == schema_type)
}

/// Direkte Obertypen eines bekannten schema.org-Typs.
pub fn parent_types(schema_type: &str) -> &'static [&'static str] {
    if LOCAL_BUSINESS_SUBTYPES.contains(&schema_type) {
        return &["LocalBusiness"];
    }
    TYPE_PARENTS
        .iter()
        .find(|(t, _)| *t == schema_type)
        .map_or(&[], |(_, parents)| *parents)
}

/// Ob `schema_type` gleich `ancestor` oder eine (direkte oder indirekte) Unterart davon ist.
pub fn is_subtype_of(schema_type: &str, ancestor: &str) -> bool {
    if schema_type == ancestor {
        return true;
    }
    let mut stack: Vec<&str> = parent_types(schema_type).to_vec();
    let mut visited: Vec<&str> = vec![schema_type];
    while let Some(current) = stack.pop() {
        if current == ancestor {
            return true;
        }
        if visited.contains(&current) {
            continue;
        }
        visited.push(current);
        stack.extend_from_slice(parent_types(current));
    }
    false
}

/// Schlägt den nächstliegenden bekannten schema.org-Typ vor (Levenshtein-Distanz ≤ 4).
pub fn suggest_type(unknown: &str) -> Option<&'static str> {
    let target = unknown.to_ascii_lowercase();
    let mut best: Option<(usize, &'static str)> = None;
    for &candidate in LOCAL_BUSINESS_SUBTYPES
        .iter()
        .chain(TYPE_PARENTS.iter().map(|(t, _)| t))
    {
        let dist = levenshtein(&target, &candidate.to_ascii_lowercase());
        if dist <= 4 && best.is_none_or(|(d, _)| dist < d) {
            best = Some((dist, candidate));
        }
    }
    best.map(|(_, name)| name)
}

/// Schlägt die nächstliegende bekannte schema.org-Property vor (Levenshtein-Distanz ≤ 2).
pub fn suggest_property(unknown: &str) -> Option<&'static str> {
    let target = unknown.to_ascii_lowercase();
    let mut best: Option<(usize, &'static str)> = None;
    for &candidate in KNOWN_PROPERTIES {
        let dist = levenshtein(&target, &candidate.to_ascii_lowercase());
        if dist <= 2 && best.is_none_or(|(d, _)| dist < d) {
            best = Some((dist, candidate));
        }
    }
    best.map(|(_, name)| name)
}

/// Prüft einen JSON-LD-Knoten auf Vokabularfehler: unbekannter Typ, Tippfehler in
/// Property-Namen, Domain-Verstöße (z. B. `publishingPrinciples` auf `Person`) und
/// grobe Range-Verstöße (`geo` als Text, `address` als Zahl, etc.).
pub fn validate_vocabulary(schema_type: &str, content: &Value) -> Vec<VocabIssue> {
    let mut issues = Vec::new();
    if !is_known_type(schema_type) {
        issues.push(VocabIssue::UnknownType {
            schema_type: schema_type.to_string(),
            suggestion: suggest_type(schema_type).map(str::to_string),
        });
    }

    let Some(obj) = content.as_object() else {
        return issues;
    };

    for (key, val) in obj {
        if key.starts_with('@') {
            continue;
        }
        if !KNOWN_PROPERTIES.contains(&key.as_str()) {
            if let Some(suggestion) = suggest_property(key) {
                issues.push(VocabIssue::UnknownProperty {
                    property: key.clone(),
                    suggestion: Some(suggestion.to_string()),
                });
            }
            continue;
        }

        // Bekannte Domain-Verstöße
        if key == "publishingPrinciples"
            && is_known_type(schema_type)
            && !is_subtype_of(schema_type, "CreativeWork")
            && !is_subtype_of(schema_type, "Organization")
        {
            issues.push(VocabIssue::InvalidPropertyDomain {
                property: key.clone(),
                schema_type: schema_type.to_string(),
                expected_domain: "CreativeWork | Organization".to_string(),
            });
        }

        // Range-Prüfungen für strukturierte Kernfelder
        match key.as_str() {
            "geo" => {
                if !val.is_object() {
                    issues.push(VocabIssue::InvalidRange {
                        property: key.clone(),
                        expected: "GeoCoordinates (object)".to_string(),
                    });
                }
            }
            "address" => {
                if !(val.is_object() || val.is_string() || val.is_array()) {
                    issues.push(VocabIssue::InvalidRange {
                        property: key.clone(),
                        expected: "PostalAddress (object) or Text".to_string(),
                    });
                }
            }
            "aggregateRating" => {
                if !val.is_object() {
                    issues.push(VocabIssue::InvalidRange {
                        property: key.clone(),
                        expected: "AggregateRating (object)".to_string(),
                    });
                }
            }
            "offers" => {
                if !(val.is_object() || val.is_array()) {
                    issues.push(VocabIssue::InvalidRange {
                        property: key.clone(),
                        expected: "Offer | AggregateOffer (object or array)".to_string(),
                    });
                }
            }
            "openingHoursSpecification" => {
                if !(val.is_object() || val.is_array()) {
                    issues.push(VocabIssue::InvalidRange {
                        property: key.clone(),
                        expected: "OpeningHoursSpecification (object or array)".to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    issues
}

/// Liest Kernfelder einer Entität (`@id`, `name`, `url`, `address`, `geo`) für
/// seitenübergreifende Konsistenzprüfungen aus.
pub fn extract_entity_summary(node: &Value) -> EntitySummary {
    let id = node
        .get("@id")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let url = node
        .get("url")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let address = node.get("address").and_then(|addr| match addr {
        Value::String(s) => Some(s.trim().to_string()).filter(|v| !v.is_empty()),
        Value::Object(map) => {
            let parts: Vec<&str> = [
                "streetAddress",
                "postalCode",
                "addressLocality",
                "addressRegion",
                "addressCountry",
            ]
            .iter()
            .filter_map(|k| map.get(*k).and_then(Value::as_str).map(str::trim))
            .filter(|s| !s.is_empty())
            .collect();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        _ => None,
    });
    let geo = node.get("geo").and_then(|g| {
        let lat = g
            .get("latitude")
            .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))?;
        let lon = g
            .get("longitude")
            .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))?;
        Some((lat, lon))
    });

    EntitySummary {
        id,
        name,
        url,
        address,
        geo,
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn subtype_hierarchy_resolves_local_business_and_webpage() {
        assert!(is_subtype_of("ProfessionalService", "LocalBusiness"));
        assert!(is_subtype_of("ProfessionalService", "Organization"));
        assert!(is_subtype_of("Physiotherapy", "LocalBusiness"));
        assert!(is_subtype_of("Dentist", "LocalBusiness"));
        assert!(is_subtype_of("AboutPage", "WebPage"));
        assert!(is_subtype_of("BlogPosting", "Article"));
        assert!(!is_subtype_of("WebDesignCompany", "LocalBusiness"));
        assert!(!is_known_type("WebDesignCompany"));
    }

    #[test]
    fn validates_unknown_type_typo_property_domain_and_range() {
        let unknown = validate_vocabulary("WebDesignCompany", &json!({"name": "Casoon"}));
        assert!(matches!(unknown[0], VocabIssue::UnknownType { .. }));

        let issues = validate_vocabulary(
            "LocalBusiness",
            &json!({
                "name": "Praxis",
                "adress": "Musterstr. 1",
                "geo": "54.0, 12.1",
                "address": 12345
            }),
        );
        assert!(issues.iter().any(|i| matches!(
            i,
            VocabIssue::UnknownProperty { property, suggestion: Some(s) }
            if property == "adress" && s == "address"
        )));
        assert!(issues.iter().any(|i| matches!(
            i,
            VocabIssue::InvalidRange { property, .. } if property == "geo"
        )));
        assert!(issues.iter().any(|i| matches!(
            i,
            VocabIssue::InvalidRange { property, .. } if property == "address"
        )));

        let person_issues = validate_vocabulary(
            "Person",
            &json!({"name": "Ada", "publishingPrinciples": "https://example.com/policy"}),
        );
        assert!(person_issues.iter().any(|i| matches!(
            i,
            VocabIssue::InvalidPropertyDomain { property, .. } if property == "publishingPrinciples"
        )));
    }

    #[test]
    fn extracts_entity_summary() {
        let summary = extract_entity_summary(&json!({
            "@id": "https://example.com/#org",
            "name": "Example",
            "url": "https://example.com/",
            "address": {"streetAddress": "Hauptstr. 1", "postalCode": "18055", "addressLocality": "Rostock"},
            "geo": {"latitude": 54.0887, "longitude": 12.1405}
        }));
        assert_eq!(summary.id.as_deref(), Some("https://example.com/#org"));
        assert_eq!(summary.address.as_deref(), Some("Hauptstr. 1, 18055, Rostock"));
        assert_eq!(summary.geo, Some((54.0887, 12.1405)));
    }
}
