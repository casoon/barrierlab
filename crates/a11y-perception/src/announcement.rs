//! Woraus eine Ansage besteht — ohne ein Wort davon. Siehe [`announce`].
//!
//! Das Modul ist privat, seine Fläche wird in der Wurzel re-exportiert; die
//! Begründung steht deshalb an den öffentlichen Items, nicht hier.

use serde::{Deserialize, Serialize};

use crate::reading::ReadingItem;

/// Eine Ansage in ihren Teilen, in Ansagereihenfolge: Name, Rolle, Zustände.
///
/// Ein Host setzt daraus seinen Satz zusammen. Die Reihenfolge ist Teil der
/// Aussage und nicht dem Host überlassen — sie entspricht der, in der
/// Screenreader diese Teile nennen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Announcement {
    /// Der zugängliche Name, um Leerraum beschnitten.
    ///
    /// `None` heißt: es gibt keinen. Ein Host sagt dafür etwas aus eigener
    /// Feder („(kein Name)"), denn Schweigen wäre an dieser Stelle die eine
    /// Auskunft, die nicht stimmt — ein namenloser Schalter ist ein Befund.
    pub name: Option<String>,
    /// Die Rolle, wie sie angesagt wird.
    pub role: AnnouncedRole,
    /// Die ansagewürdigen Zustände, in der Reihenfolge, in der sie am Knoten
    /// stehen. Ein Zustand, der nichts hinzufügt, steht nicht darin.
    pub states: Vec<AnnouncedState>,
}

/// Die Rollen, für die eine Ansage eine eigene Benennung hat.
///
/// Nicht jede ARIA-Rolle steht hier. Was fehlt, kommt als [`Self::Other`]
/// durch und wird von einem Host unübersetzt genannt — besser der rohe
/// Rollenname als gar keine Rolle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncedRole {
    Button,
    Link,
    /// `textbox` und `searchbox`. Screenreader nennen beide als Eingabefeld;
    /// dass es ein Suchfeld ist, sagt der Name.
    TextBox,
    CheckBox,
    Radio,
    ComboBox,
    ListBox,
    Slider,
    SpinButton,
    Tab,
    /// Die Ebene gehört in die Rolle („Überschrift Ebene 2"), nicht in die
    /// Zustände. `None`, wenn der Knoten keine trägt oder sie keine Zahl ist.
    Heading {
        level: Option<u8>,
    },
    Navigation,
    Main,
    Banner,
    ContentInfo,
    /// Eine Rolle ohne eigene Benennung — der Rollenname selbst.
    ///
    /// Trägt der Knoten gar keine Rolle, steht hier `"generic"`: Chrome setzt
    /// diese Rolle für ein Element ohne eigene Semantik, und die Projektion
    /// unterscheidet nicht, ob sie im Baum stand oder fehlte.
    Other(String),
}

/// Die Zustände, die eine Ansage nennt.
///
/// Ein Zustand fehlt hier aus einem von zwei Gründen: weil sein Fehlen
/// selbstverständlich ist — `required=false` ist der Normalfall und wird nicht
/// angesagt —, oder weil er kein Zustand im Sinne der Ansage ist, wie die
/// Ebene einer Überschrift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncedState {
    Expanded,
    Collapsed,
    Checked,
    Unchecked,
    /// `checked=mixed` — teilweise gewählt.
    Mixed,
    Selected,
    NotSelected,
    Required,
    Invalid,
    Disabled,
    Pressed,
    NotPressed,
    /// Der Knoten ist mit der Tastatur erreichbar, obwohl seine Rolle das nicht
    /// erwarten lässt.
    ///
    /// Bei einem Schalter oder Link wäre das keine Nachricht. Bei einem `div`
    /// mit `tabindex` ist es eine: Der Nutzer landet auf etwas, dessen Rolle
    /// nicht verrät, was es tut.
    Focusable,
}

/// Woraus die Ansage zu einer Leseeinheit besteht — ohne ein Wort davon.
///
/// Was ein Screenreader zu einer Leseeinheit sagt, hat zwei Hälften. Die eine
/// ist Struktur: dass zuerst der Name kommt, dann die Rolle, dann die Zustände;
/// dass die Ebene einer Überschrift in die Rolle gehört und nicht daneben; dass
/// `required=false` nichts zu sagen gibt, `expanded=false` aber „eingeklappt".
/// Diese Hälfte ist überall dieselbe und steht hier.
///
/// Die andere Hälfte sind die Wörter. Ob eine `button`-Rolle „Schalter",
/// „button" oder „bouton" heißt, hängt an der Sprache des Berichts und damit am
/// Host. Deshalb gibt diese Funktion keine Zeichenkette zurück, sondern ein
/// [`Announcement`] — benannte Teile in Ansagereihenfolge, die ein Host durch
/// seine eigene Lokalisierung schickt und mit seinem eigenen Trennzeichen
/// zusammensetzt.
///
/// ```
/// use a11y_perception::{announce, AnnouncedRole, AnnouncedState, ReadingItem};
///
/// let item = ReadingItem {
///     seq: 0,
///     role: Some("heading".into()),
///     name: Some("Kontakt".into()),
///     description: None,
///     value: None,
///     states: vec!["level=1".into()],
///     tab_stop: false,
///     depth: 0,
///     node_id: "1".into(),
/// };
///
/// let ansage = announce(&item);
/// assert_eq!(ansage.name.as_deref(), Some("Kontakt"));
/// assert_eq!(ansage.role, AnnouncedRole::Heading { level: Some(1) });
/// assert_eq!(ansage.states, Vec::<AnnouncedState>::new());
/// ```
///
/// # Grenze
///
/// Das ist die Ansage, die aus den Browserdaten **folgt**, nicht die, die ein
/// Screenreader gesprochen hat. NVDA, JAWS und VoiceOver kürzen, fassen zusammen
/// und werten eigene Einstellungen aus; die Projektion tut nichts davon. Sie
/// taugt zum Vergleich zweier Zustände und zum Aufschreiben eines Durchlaufs,
/// nicht als Beleg über eine Sprachausgabe.
pub fn announce(item: &ReadingItem) -> Announcement {
    Announcement {
        name: item
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(String::from),
        role: role(item),
        states: states(item),
    }
}

fn role(item: &ReadingItem) -> AnnouncedRole {
    let role = item.role.as_deref().unwrap_or("generic");

    if role == "heading" {
        return AnnouncedRole::Heading {
            level: state_value(&item.states, "level").and_then(|level| level.parse().ok()),
        };
    }

    match role {
        "button" => AnnouncedRole::Button,
        "link" => AnnouncedRole::Link,
        "textbox" | "searchbox" => AnnouncedRole::TextBox,
        "checkbox" => AnnouncedRole::CheckBox,
        "radio" => AnnouncedRole::Radio,
        "combobox" => AnnouncedRole::ComboBox,
        "listbox" => AnnouncedRole::ListBox,
        "slider" => AnnouncedRole::Slider,
        "spinbutton" => AnnouncedRole::SpinButton,
        "tab" => AnnouncedRole::Tab,
        "navigation" => AnnouncedRole::Navigation,
        "main" => AnnouncedRole::Main,
        "banner" => AnnouncedRole::Banner,
        "contentinfo" => AnnouncedRole::ContentInfo,
        other => AnnouncedRole::Other(other.to_string()),
    }
}

fn states(item: &ReadingItem) -> Vec<AnnouncedState> {
    let mut states: Vec<AnnouncedState> = item
        .states
        .iter()
        .filter_map(|state| match split_state(state) {
            ("expanded", Some("false")) => Some(AnnouncedState::Collapsed),
            ("expanded", _) => Some(AnnouncedState::Expanded),
            ("checked", Some("false")) => Some(AnnouncedState::Unchecked),
            ("checked", Some("mixed")) => Some(AnnouncedState::Mixed),
            ("checked", _) => Some(AnnouncedState::Checked),
            ("selected", Some("false")) => Some(AnnouncedState::NotSelected),
            ("selected", _) => Some(AnnouncedState::Selected),
            ("required", Some("false")) => None,
            ("required", _) => Some(AnnouncedState::Required),
            ("invalid", Some("false")) => None,
            ("invalid", _) => Some(AnnouncedState::Invalid),
            ("disabled", Some("false")) => None,
            ("disabled", _) => Some(AnnouncedState::Disabled),
            ("pressed", Some("false")) => Some(AnnouncedState::NotPressed),
            ("pressed", _) => Some(AnnouncedState::Pressed),
            _ => None,
        })
        .collect();

    if item.tab_stop && !announces_its_own_focusability(item.role.as_deref()) {
        states.push(AnnouncedState::Focusable);
    }

    states
}

fn state_value<'a>(states: &'a [String], name: &str) -> Option<&'a str> {
    states
        .iter()
        .filter_map(|state| state.split_once('='))
        .find_map(|(state_name, value)| (state_name == name).then_some(value))
}

fn split_state(state: &str) -> (&str, Option<&str>) {
    match state.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (state, None),
    }
}

/// Rollen, bei denen „fokussierbar" nichts hinzufügt, weil die Rolle es schon
/// sagt.
///
/// Nicht [`crate::AXTree::form_controls`]: Das ist eine andere Frage — dort geht
/// es um Formularfelder, hier um jede Rolle, deren Bedienbarkeit eine Ansage
/// ohnehin mitteilt, also auch Schalter, Link und Reiter.
fn announces_its_own_focusability(role: Option<&str>) -> bool {
    matches!(
        role,
        Some(
            "button"
                | "link"
                | "textbox"
                | "searchbox"
                | "checkbox"
                | "radio"
                | "combobox"
                | "listbox"
                | "slider"
                | "spinbutton"
                | "tab"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::{AnnouncedRole, AnnouncedState, announce};
    use crate::reading::ReadingItem;

    fn item(role: Option<&str>, name: Option<&str>, states: Vec<&str>) -> ReadingItem {
        ReadingItem {
            seq: 0,
            role: role.map(String::from),
            name: name.map(String::from),
            description: None,
            value: None,
            states: states.into_iter().map(String::from).collect(),
            tab_stop: false,
            depth: 0,
            node_id: "1".into(),
        }
    }

    #[test]
    fn name_wird_beschnitten_und_leer_heisst_keiner() {
        assert_eq!(
            announce(&item(Some("link"), Some("  Mehr erfahren  "), vec![])).name,
            Some("Mehr erfahren".to_string())
        );
        assert_eq!(
            announce(&item(Some("button"), Some("   "), vec![])).name,
            None
        );
        assert_eq!(announce(&item(Some("button"), None, vec![])).name, None);
    }

    #[test]
    fn ebene_gehoert_in_die_rolle_nicht_in_die_zustaende() {
        let ansage = announce(&item(Some("heading"), Some("Willkommen"), vec!["level=1"]));

        assert_eq!(ansage.role, AnnouncedRole::Heading { level: Some(1) });
        assert!(ansage.states.is_empty(), "level ist kein Zustand");
    }

    #[test]
    fn ueberschrift_ohne_brauchbare_ebene_bleibt_ueberschrift() {
        // Chrome liefert `level` als Zahl. Kommt doch etwas anderes an, ist die
        // Rolle weiter sicher -- nur die Ebene fehlt, und das steht als `None`
        // da, statt „Ebene fuenf" zu erfinden.
        for states in [vec![], vec!["level=zwei"]] {
            assert_eq!(
                announce(&item(Some("heading"), Some("Kapitel"), states)).role,
                AnnouncedRole::Heading { level: None }
            );
        }
    }

    #[test]
    fn suchfeld_wird_als_eingabefeld_angesagt() {
        assert_eq!(
            announce(&item(Some("searchbox"), Some("Suche"), vec![])).role,
            AnnouncedRole::TextBox
        );
    }

    #[test]
    fn unbekannte_rolle_kommt_unuebersetzt_durch_fehlende_wird_generic() {
        assert_eq!(
            announce(&item(Some("marquee"), None, vec![])).role,
            AnnouncedRole::Other("marquee".to_string())
        );
        assert_eq!(
            announce(&item(None, None, vec![])).role,
            AnnouncedRole::Other("generic".to_string())
        );
    }

    #[test]
    fn zustaende_behalten_die_reihenfolge_des_knotens() {
        let ansage = announce(&item(
            Some("textbox"),
            Some("E-Mail"),
            vec!["required", "invalid"],
        ));

        assert_eq!(
            ansage.states,
            vec![AnnouncedState::Required, AnnouncedState::Invalid]
        );
    }

    #[test]
    fn verneintes_wird_angesagt_wo_es_etwas_aussagt_und_sonst_nicht() {
        // Eingeklappt, ungewaehlt, nicht gedrueckt: Das ist eine Auskunft --
        // der Nutzer muss wissen, dass es einen zweiten Zustand gibt.
        for (state, erwartet) in [
            ("expanded=false", AnnouncedState::Collapsed),
            ("checked=false", AnnouncedState::Unchecked),
            ("selected=false", AnnouncedState::NotSelected),
            ("pressed=false", AnnouncedState::NotPressed),
        ] {
            assert_eq!(
                announce(&item(Some("button"), Some("Filter"), vec![state])).states,
                vec![erwartet],
                "{state} sagt etwas aus"
            );
        }

        // Nicht erforderlich, nicht ungueltig, nicht deaktiviert: der Normalfall.
        for state in ["required=false", "invalid=false", "disabled=false"] {
            assert!(
                announce(&item(Some("textbox"), Some("Name"), vec![state]))
                    .states
                    .is_empty(),
                "{state} ist der Normalfall und wird nicht angesagt"
            );
        }
    }

    #[test]
    fn checked_mixed_ist_ein_eigener_zustand() {
        assert_eq!(
            announce(&item(Some("checkbox"), Some("Alle"), vec!["checked=mixed"])).states,
            vec![AnnouncedState::Mixed]
        );
    }

    #[test]
    fn unbekannter_zustand_wird_verschwiegen_nicht_durchgereicht() {
        assert!(
            announce(&item(Some("button"), Some("Los"), vec!["haspopup=menu"]))
                .states
                .is_empty()
        );
    }

    #[test]
    fn fokussierbar_nur_wo_die_rolle_es_nicht_schon_sagt() {
        let mut generisch = item(Some("generic"), Some("Karte"), vec![]);
        generisch.tab_stop = true;
        assert_eq!(
            announce(&generisch).states,
            vec![AnnouncedState::Focusable],
            "ein div mit tabindex ist eine Nachricht"
        );

        let mut schalter = item(Some("button"), Some("Senden"), vec![]);
        schalter.tab_stop = true;
        assert!(
            announce(&schalter).states.is_empty(),
            "bei einem Schalter fuegt fokussierbar nichts hinzu"
        );
    }

    #[test]
    fn fokussierbar_kommt_hinter_die_zustaende_des_knotens() {
        let mut generisch = item(Some("generic"), Some("Karte"), vec!["expanded=false"]);
        generisch.tab_stop = true;

        assert_eq!(
            announce(&generisch).states,
            vec![AnnouncedState::Collapsed, AnnouncedState::Focusable]
        );
    }
}
