//! Messungen an der laufenden Seite, die der Collector je Knoten liefern kann:
//! Rahmen, Drehung der Wurzel, Einfügen, Fokusabdeckung, Textabstand
//! (casoon/barrierlab#130). Sie erreichen `a11y-dom` über die
//! [`Rendering`](a11y_dom::Rendering)-Methoden `frame_reachable`,
//! `box_transform`, `paste_probe`, `focus_cover` und `text_spacing_clip`.
//!
//! **Fehlt eine Spalte, ist die Messung nicht gemacht** (`None`), und die Regel
//! erscheint als nicht gelaufen. Ist sie da, gilt sie als gemessen — an den
//! Knoten, die sie nennt. Alle Spalten sind dünn besetzt: ein Index-Array
//! (`nodes`, streng aufsteigend, Knotenindizes der Arena) plus je Wert ein
//! paralleles Array. Die Proben berühren nur wenige Knoten (Rahmen, Felder,
//! Bedienelemente), eine Spalte je Knoten wäre überwiegend leer.

use a11y_dom::{BoxTransform, FocusCover, NodeId, PasteProbe, SpacingClip};

/// Bit 0 der Einfüge-Spalte: das Ereignis wurde abgebrochen.
pub const PASTE_CANCELLED: u8 = 1;
/// Bit 1 der Einfüge-Spalte: der Probetext stand danach im Feld.
pub const PASTE_INSERTED: u8 = 2;
/// Bit 0 der Textabstands-Spalte: vor dem Überschreiben abgeschnitten.
pub const SPACING_BEFORE: u8 = 1;
/// Bit 1 der Textabstands-Spalte: mit überschriebenem Textabstand abgeschnitten.
pub const SPACING_AFTER: u8 = 2;

/// Eine dünn besetzte Spalte: Werte für einige Knoten, nach Index sortiert.
pub struct Sparse<T> {
    nodes: Vec<u32>,
    values: Vec<T>,
}

impl<T> Sparse<T> {
    /// Wie viele Knoten die Spalte nennt. Eine Spalte, die ein Lauf nicht
    /// gefüllt hat, ist leer — Tests prüfen das.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn get(&self, node: NodeId) -> Option<&T> {
        let i = self.nodes.binary_search(&node.0).ok()?;
        self.values.get(i)
    }
}

/// Prüft den Indexteil einer Spalte und dass jedes Wert-Array so lang ist.
fn check(
    column: &str,
    node_count: usize,
    nodes: &[u32],
    lens: &[(&str, usize)],
) -> Result<(), String> {
    for (name, len) in lens {
        if *len != nodes.len() {
            return Err(format!(
                "{column}: {name} has {len} entries, nodes has {}",
                nodes.len()
            ));
        }
    }
    for (i, w) in nodes.windows(2).enumerate() {
        if w[1] <= w[0] {
            return Err(format!(
                "{column}: nodes must be strictly ascending, but nodes[{}] = {} follows {}",
                i + 1,
                w[1],
                w[0]
            ));
        }
    }
    if let Some(&last) = nodes.last()
        && last as usize >= node_count
    {
        return Err(format!(
            "{column}: node {last} is outside the arena ({node_count} nodes)"
        ));
    }
    Ok(())
}

/// Prüft ein Bit-Array: nur Bits aus `mask`.
fn check_bits(column: &str, name: &str, values: &[u8], mask: u8) -> Result<(), String> {
    match values.iter().position(|v| v & !mask != 0) {
        Some(i) => Err(format!(
            "{column}: {name}[{i}] = {} has bits outside {mask:#b}",
            values[i]
        )),
        None => Ok(()),
    }
}

/// Die Probenspalten eines Scans. Jede ist einzeln optional.
#[derive(Default)]
pub struct ProbeColumns {
    pub frames: Option<Sparse<bool>>,
    pub transforms: Option<Sparse<BoxTransform>>,
    pub paste: Option<Sparse<PasteProbe>>,
    pub focus: Option<Sparse<FocusCover>>,
    pub spacing: Option<Sparse<SpacingClip>>,
}

impl ProbeColumns {
    /// `frame_reachable` je `<iframe>`/`<frame>`: `reachable` ist 0 (nicht
    /// lesbar) oder 1 (lesbar).
    pub fn set_frames(
        &mut self,
        node_count: usize,
        nodes: Vec<u32>,
        reachable: &[u8],
    ) -> Result<(), String> {
        check(
            "frames",
            node_count,
            &nodes,
            &[("reachable", reachable.len())],
        )?;
        check_bits("frames", "reachable", reachable, 1)?;
        let values = reachable.iter().map(|v| *v == 1).collect();
        self.frames = Some(Sparse { nodes, values });
        Ok(())
    }

    /// `box_transform` je Knoten (mindestens `html` und `body`), so wie der
    /// Browser `transform` und `rotate` serialisiert. Eine leere Zeichenkette
    /// heißt: dieses Feld nicht gemessen.
    pub fn set_transforms(
        &mut self,
        node_count: usize,
        nodes: Vec<u32>,
        transform: Vec<String>,
        rotate: Vec<String>,
    ) -> Result<(), String> {
        check(
            "transforms",
            node_count,
            &nodes,
            &[("transform", transform.len()), ("rotate", rotate.len())],
        )?;
        let some = |s: String| (!s.is_empty()).then_some(s);
        let values = transform
            .into_iter()
            .zip(rotate)
            .map(|(transform, rotate)| BoxTransform {
                transform: some(transform),
                rotate: some(rotate),
            })
            .collect();
        self.transforms = Some(Sparse { nodes, values });
        Ok(())
    }

    /// `paste_probe` je Passwort- oder Codefeld: Bits [`PASTE_CANCELLED`] und
    /// [`PASTE_INSERTED`].
    pub fn set_paste(
        &mut self,
        node_count: usize,
        nodes: Vec<u32>,
        flags: &[u8],
    ) -> Result<(), String> {
        check("paste", node_count, &nodes, &[("flags", flags.len())])?;
        check_bits("paste", "flags", flags, PASTE_CANCELLED | PASTE_INSERTED)?;
        let values = flags
            .iter()
            .map(|f| PasteProbe {
                cancelled: f & PASTE_CANCELLED != 0,
                inserted: f & PASTE_INSERTED != 0,
            })
            .collect();
        self.paste = Some(Sparse { nodes, values });
        Ok(())
    }

    /// `focus_cover` je fokussiertes Bedienelement. `blocker` ist der
    /// Knotenindex des obersten überdeckenden Elements oder -1.
    pub fn set_focus(
        &mut self,
        node_count: usize,
        nodes: Vec<u32>,
        points: &[u32],
        covered: &[u32],
        blocker: &[i32],
    ) -> Result<(), String> {
        check(
            "focus",
            node_count,
            &nodes,
            &[
                ("points", points.len()),
                ("covered", covered.len()),
                ("blocker", blocker.len()),
            ],
        )?;
        let mut values = Vec::with_capacity(nodes.len());
        for i in 0..nodes.len() {
            if covered[i] > points[i] {
                return Err(format!(
                    "focus: covered[{i}] = {} exceeds points[{i}] = {}",
                    covered[i], points[i]
                ));
            }
            if blocker[i] < -1 || blocker[i] as i64 >= node_count as i64 {
                return Err(format!(
                    "focus: blocker[{i}] = {} must be -1 or a node of the arena ({node_count} nodes)",
                    blocker[i]
                ));
            }
            values.push(FocusCover {
                points: points[i],
                covered: covered[i],
                blocker: (blocker[i] >= 0).then_some(NodeId(blocker[i] as u32)),
            });
        }
        self.focus = Some(Sparse { nodes, values });
        Ok(())
    }

    /// `text_spacing_clip` je Textcontainer: Bits [`SPACING_BEFORE`] und
    /// [`SPACING_AFTER`].
    pub fn set_spacing(
        &mut self,
        node_count: usize,
        nodes: Vec<u32>,
        flags: &[u8],
    ) -> Result<(), String> {
        check("spacing", node_count, &nodes, &[("flags", flags.len())])?;
        check_bits("spacing", "flags", flags, SPACING_BEFORE | SPACING_AFTER)?;
        let values = flags
            .iter()
            .map(|f| SpacingClip {
                before: f & SPACING_BEFORE != 0,
                after: f & SPACING_AFTER != 0,
            })
            .collect();
        self.spacing = Some(Sparse { nodes, values });
        Ok(())
    }

    pub(crate) fn frame(&self, node: NodeId) -> Option<bool> {
        self.frames.as_ref()?.get(node).copied()
    }

    pub(crate) fn transform(&self, node: NodeId) -> Option<BoxTransform> {
        self.transforms.as_ref()?.get(node).cloned()
    }

    pub(crate) fn paste_of(&self, node: NodeId) -> Option<PasteProbe> {
        self.paste.as_ref()?.get(node).copied()
    }

    pub(crate) fn focus_of(&self, node: NodeId) -> Option<FocusCover> {
        self.focus.as_ref()?.get(node).copied()
    }

    pub(crate) fn spacing_of(&self, node: NodeId) -> Option<SpacingClip> {
        self.spacing.as_ref()?.get(node).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fehler(r: Result<(), String>) -> String {
        r.expect_err("muss abgelehnt werden")
    }

    #[test]
    fn indexspalte_muss_aufsteigend_und_in_der_arena_sein() {
        let mut p = ProbeColumns::default();
        assert!(
            fehler(p.set_frames(5, vec![2, 2], &[1, 1])).contains("strictly ascending"),
            "doppelter Index"
        );
        assert!(fehler(p.set_frames(5, vec![3, 1], &[1, 1])).contains("strictly ascending"));
        assert!(fehler(p.set_frames(5, vec![1, 5], &[1, 1])).contains("node 5 is outside"));
        assert!(
            p.frames.is_none(),
            "eine abgelehnte Spalte bleibt ungesetzt"
        );
        assert_eq!(p.set_frames(5, vec![1, 4], &[1, 0]), Ok(()));
    }

    #[test]
    fn wertspalten_muessen_so_lang_sein_wie_nodes() {
        let mut p = ProbeColumns::default();
        assert!(
            fehler(p.set_paste(3, vec![0, 1], &[1])).contains("flags has 1 entries, nodes has 2")
        );
        assert!(
            fehler(p.set_transforms(3, vec![0], vec!["none".into()], vec![]))
                .contains("rotate has 0")
        );
        assert!(fehler(p.set_focus(3, vec![0], &[9], &[1], &[])).contains("blocker has 0 entries"));
    }

    #[test]
    fn werte_ausserhalb_des_bereichs_werden_abgelehnt() {
        let mut p = ProbeColumns::default();
        assert!(fehler(p.set_frames(3, vec![1], &[2])).contains("reachable[0] = 2"));
        assert!(fehler(p.set_paste(3, vec![1], &[4])).contains("flags[0] = 4"));
        assert!(fehler(p.set_spacing(3, vec![1], &[8])).contains("flags[0] = 8"));
        assert!(fehler(p.set_focus(3, vec![1], &[9], &[10], &[-1])).contains("exceeds points"));
        assert!(fehler(p.set_focus(3, vec![1], &[9], &[1], &[3])).contains("blocker[0] = 3"));
        assert!(fehler(p.set_focus(3, vec![1], &[9], &[1], &[-2])).contains("blocker[0] = -2"));
    }

    #[test]
    fn fehlende_spalte_ist_nicht_gemessen_vorhandene_nennt_nur_ihre_knoten() {
        let mut p = ProbeColumns::default();
        assert_eq!(p.frame(NodeId(1)), None, "ohne Spalte");
        p.set_frames(4, vec![1], &[0]).unwrap();
        assert_eq!(p.frame(NodeId(1)), Some(false));
        assert_eq!(p.frame(NodeId(2)), None, "Knoten außerhalb der Spalte");
    }

    #[test]
    fn leere_zeichenketten_sind_nicht_gemessene_felder() {
        let mut p = ProbeColumns::default();
        p.set_transforms(
            3,
            vec![0, 1],
            vec!["none".into(), "".into()],
            vec!["".into(), "90deg".into()],
        )
        .unwrap();
        assert_eq!(
            p.transform(NodeId(0)),
            Some(BoxTransform {
                transform: Some("none".into()),
                rotate: None
            })
        );
        assert_eq!(
            p.transform(NodeId(1)).unwrap().rotate.as_deref(),
            Some("90deg")
        );
    }

    #[test]
    fn bits_und_blocker_werden_entpackt() {
        let mut p = ProbeColumns::default();
        p.set_paste(3, vec![2], &[PASTE_CANCELLED]).unwrap();
        assert_eq!(
            p.paste_of(NodeId(2)),
            Some(PasteProbe {
                cancelled: true,
                inserted: false
            })
        );
        p.set_spacing(3, vec![1], &[SPACING_AFTER]).unwrap();
        assert_eq!(
            p.spacing_of(NodeId(1)),
            Some(SpacingClip {
                before: false,
                after: true
            })
        );
        p.set_focus(3, vec![1, 2], &[9, 9], &[3, 0], &[0, -1])
            .unwrap();
        assert_eq!(p.focus_of(NodeId(1)).unwrap().blocker, Some(NodeId(0)));
        assert_eq!(p.focus_of(NodeId(2)).unwrap().blocker, None);
    }
}
