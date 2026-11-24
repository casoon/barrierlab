//! Bilder, die ein Stylesheet einfügt: die Adressen aus einem Wert lesen.

/// Die Adressen in `url(…)`-Funktionen eines Wertes, in Reihenfolge. Quoted
/// und unquoted; ein Verlauf (`linear-gradient(…)`) hat keine.
pub(crate) fn urls_in(wert: &str) -> Vec<String> {
    let mut out = Vec::new();
    let klein = wert.to_ascii_lowercase();
    let mut ab = 0;
    while let Some(i) = klein[ab..].find("url(") {
        // `url(` als eigener Funktionsname, nicht Ende eines längeren
        // (`-webkit-image-url(` o. ä. gibt es nicht, aber `curl(` ebenso wenig).
        let start = ab + i;
        let vorher_ok = start == 0
            || !klein.as_bytes()[start - 1].is_ascii_alphanumeric()
                && klein.as_bytes()[start - 1] != b'-';
        let inhalt = start + 4;
        let rest = &wert[inhalt..];
        let rest_t = rest.trim_start();
        let (adresse, verbraucht) = match rest_t.chars().next() {
            Some(q @ ('"' | '\'')) => {
                let tiefer = &rest_t[1..];
                match tiefer.find(q) {
                    Some(e) => (
                        tiefer[..e].to_string(),
                        rest.len() - rest_t.len() + 1 + e + 1,
                    ),
                    None => break,
                }
            }
            Some(_) => match rest_t.find(')') {
                Some(e) => (
                    rest_t[..e].trim().to_string(),
                    rest.len() - rest_t.len() + e,
                ),
                None => break,
            },
            None => break,
        };
        if vorher_ok && !adresse.is_empty() {
            out.push(adresse);
        }
        ab = inhalt + verbraucht;
    }
    out
}

/// Ob ein `content`-Wert einen Textersatz nach dem Schrägstrich trägt
/// (`url(x.svg) / "Logo"`, CSS Generated Content Level 3).
pub(crate) fn hat_textersatz(wert: &str) -> bool {
    let mut ohne = String::new();
    let mut tiefe = 0u32;
    let mut quote: Option<char> = None;
    for c in wert.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => quote = Some(c),
                '(' => tiefe += 1,
                ')' => tiefe = tiefe.saturating_sub(1),
                _ if tiefe == 0 => ohne.push(c),
                _ => {}
            },
        }
    }
    ohne.contains('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adressen_quoted_und_unquoted() {
        assert_eq!(urls_in("url(a.svg)"), ["a.svg"]);
        assert_eq!(urls_in("url( 'b c.png' )"), ["b c.png"]);
        assert_eq!(
            urls_in("URL(\"x)y.png\"), url(z.png)"),
            ["x)y.png", "z.png"]
        );
    }

    #[test]
    fn verlaeufe_haben_keine_adresse() {
        assert!(urls_in("linear-gradient(red, blue)").is_empty());
        assert!(urls_in("none").is_empty());
    }

    #[test]
    fn textersatz_nach_schraegstrich() {
        assert!(hat_textersatz("url(x.svg) / \"Logo\""));
        assert!(!hat_textersatz("url(a/b.svg)"));
        assert!(!hat_textersatz("\"a / b\" url(x.svg)"));
    }
}
