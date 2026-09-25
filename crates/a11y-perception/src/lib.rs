//! Was von einer Seite wahrnehmbar ist — als Aufnahme, als Projektion, als
//! Differenz zweier Aufnahmen.
//!
//! Dieses Crate berechnet, es erhebt nicht. Es öffnet keine Seite, spricht kein
//! Chrome DevTools Protocol und liest kein DOM. Ein Host übergibt einen
//! Accessibility-Tree samt Fokuszustand; was daraus folgt, steht hier:
//!
//! | Baustein | Frage, die er beantwortet |
//! |---|---|
//! | [`AXTree`], [`AXNode`] | was steht im Baum, und wie finde ich einen Knoten wieder |
//! | [`AXSnapshot`], [`FocusSnapshot`] | wie sah ein bestimmter Zeitpunkt aus |
//! | [`AXTreeDiff`] | was hat eine Handlung wahrnehmbar verändert |
//! | [`linearize`], [`ReadingItem`] | in welcher Reihenfolge fände ein Screenreader das vor |
//! | [`announce`], [`Announcement`] | woraus die Ansage zu einer Leseeinheit besteht |
//!
//! # Grenzen
//!
//! Die Projektion ist eine **Näherung über Browserdaten**. Sie belegt nicht,
//! was ein Screenreader tatsächlich ansagt: dass eine Live-Region Inhalt
//! bekommen hat, heißt, dass der Browser den Anlass zur Ansage hatte — nicht,
//! dass NVDA sie vorgelesen hat.
//!
//! Zwei Aufnahmen tragen außerdem nicht jeden Fall. Flüchtige Änderungen — eine
//! Fehlermeldung, die eingefügt, vorgelesen und wieder entfernt wird — sind
//! zwischen zwei Zeitpunkten unter Umständen nie sichtbar. Dafür braucht ein
//! Host eine Ereignisspur über die Zeit; der Diff allein genügt nicht.
//!
//! Knotenidentität gilt nur **innerhalb eines Dokuments**, über
//! `backend_dom_node_id`. Fehlt sie, wechselt der Frame oder wird navigiert,
//! ist die Identität unklar — und das gehört ausgewiesen, nicht geraten.
//!
//! # Herkunft
//!
//! Der Code lief zuerst in [auditmysite](https://github.com/casoon/auditmysite)
//! und ist an 133 Seiten mit 692 Journey-Instanzen gemessen worden. Die Messung
//! hat drei Dinge erzwungen, die hier eingebaut sind: „hinzugekommen" heißt
//! **wahrnehmbar geworden** (Chrome entfernt verborgene Knoten nicht, sie
//! bleiben als `ignored` mit Grund `notRendered` stehen), Knoten werden über
//! die Backend-ID zugeordnet statt über ihre Position, und eine unklare
//! Identität wird gemeldet statt überspielt.

#![forbid(unsafe_code)]

mod announcement;
mod diff;
mod linearizer;
mod reading;
mod snapshot;
mod tree;

pub use announcement::{AnnouncedRole, AnnouncedState, Announcement, announce};
pub use diff::*;
pub use linearizer::*;
pub use reading::{IgnoredReadingNode, ReadingItem};
pub use snapshot::{AXSnapshot, FocusIndicatorStatus, FocusSnapshot, Rect};
pub use tree::*;
