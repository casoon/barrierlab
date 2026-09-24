//! Prüfungen über Webseiten, die mehr als ein Werkzeug braucht.
//!
//! Dieses Crate ist die gemeinsame Mitte zweier Auditoren, die dasselbe an
//! derselben Seite prüfen: [auditmysite](https://crates.io/crates/auditmysite)
//! über eine laufende Seite in Chrome, `astro-post-audit` über ein gebautes
//! `dist/`. Was sich unterscheidet, ist die **Erhebung**; was sich gleicht, ist
//! die **Auswertung** — und die steht hier.
//!
//! Zwei Regeln halten das Crate klein:
//!
//! 1. **Nichts wird geholt.** Kein HTTP, kein Dateisystem, kein Browser. Ein
//!    Host übergibt Text oder Daten, dieses Crate rechnet.
//! 2. **Keine Formulierungen.** Kennungen, Schweregrade und Texte für Berichte
//!    bleiben beim Host — sie sind dort verschieden und mehrsprachig. Hier
//!    stehen Daten und Prädikate, aus denen ein Host seine Befunde bildet.
//!
//! # Umfang
//!
//! [`robots`] — Grammatik der robots.txt, Einordnung der Bots, Pfadauswertung
//! nach der Längsten-Regel.
//!
//! Weitere Familien folgen einzeln. Jede wird vorher darauf geprüft, ob sie
//! wirklich dieselbe Frage stellt: bei „Security" etwa sah die Namensgleichheit
//! nach Doppelung aus, tatsächlich prüft ein Werkzeug HTTP-Header und das
//! andere HTML.

#![forbid(unsafe_code)]

pub mod robots;
