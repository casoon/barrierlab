//! `@import` (CSS Cascade 5 §6.4.1, CSS Conditional 3 §7.1): der Vorspann und
//! was ein Host damit macht, der das importierte Blatt liefert.
//!
//! Die Kaskade holt nichts. Liefert der Host das Blatt, ersetzt er die
//! `@import`-Regel durch [`Import::wrap`]: die Regeln des Imports stehen dann
//! in der Ebene (`layer`, `layer(x)`), unter der Bedingung (`supports()`)
//! und im Medienblock, die der Import nennt — dieselben Konstrukte, die auch
//! ein Block im Blatt selbst hätte. Solange er das nicht tut, bleibt die
//! `@import`-Regel stehen und die Kaskade antwortet `Unknown`
//! ([`Cascade::unresolved_imports`](crate::Cascade::unresolved_imports)).

use stylesheet_parse::{AtRule, GroupRule, MediaRule, Rule};

/// Die Ebene, in die ein Import seine Regeln legt (Cascade 5 §6.4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportLayer {
    /// `layer`: eine eigene, anonyme Ebene.
    Anonymous,
    /// `layer(x)` oder `layer(a.b)`: die benannte Ebene, wie sie im Blatt
    /// steht (Escapes nicht aufgelöst; die Kaskade normalisiert beim
    /// Vergleichen).
    Named(String),
}

/// Ein gelesener `@import`-Vorspann:
/// `[ <url> | <string> ] [ layer | layer(<name>) ]? [ supports(…) ]? <medien>?`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Import {
    /// Die URL, so wie sie im Blatt steht (nicht aufgelöst).
    pub url: String,
    /// Die Ebene, falls der Import eine nennt.
    pub layer: Option<ImportLayer>,
    /// Der Inhalt von `supports(…)`, falls vorhanden.
    pub supports: Option<String>,
    /// Die Medienbedingung, falls vorhanden.
    pub media: Option<String>,
}

impl Import {
    /// Liest den Vorspann einer `@import`-Regel (`AtRule::prelude`). `None`,
    /// wenn er mit keiner URL beginnt oder eine Klammer offen bleibt.
    pub fn parse(prelude: &str) -> Option<Import> {
        let p = prelude.trim();
        let (url, mut rest) = url(p)?;
        let mut layer = None;
        let mut supports = None;
        let mut media = None;
        loop {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }
            if layer.is_none()
                && supports.is_none()
                && let Some(r) = keyword(rest, "layer")
            {
                match r.strip_prefix('(') {
                    Some(inner) => {
                        let end = paren_end(inner)?;
                        layer = Some(ImportLayer::Named(inner[..end].trim().to_string()));
                        rest = &inner[end + 1..];
                    }
                    None => {
                        layer = Some(ImportLayer::Anonymous);
                        rest = r;
                    }
                }
            } else if supports.is_none()
                && let Some(inner) = keyword_call(rest, "supports")
            {
                let end = paren_end(inner)?;
                supports = Some(inner[..end].trim().to_string());
                rest = &inner[end + 1..];
            } else {
                media = Some(rest.to_string());
                break;
            }
        }
        Some(Import {
            url,
            layer,
            supports,
            media,
        })
    }

    /// Die Regeln, die an die Stelle der `@import`-Regel treten, wenn der
    /// Host `rules` (das Blatt des Imports) liefert: außen die Ebene, darin
    /// `@supports`, darin `@media`. Ein unaufgelöster `@import` im
    /// importierten Blatt steht vorn auf oberster Ebene, wo die Kaskade ihn
    /// findet (er kann jede Angabe überstimmen, gleich unter welcher
    /// Bedingung).
    pub fn wrap(&self, rules: Vec<Rule>) -> Vec<Rule> {
        let (mut out, mut inner): (Vec<_>, Vec<_>) = rules.into_iter().partition(is_import);
        if let Some(cond) = self.media.as_deref().filter(|m| !m.is_empty()) {
            inner = vec![Rule::Media(MediaRule {
                condition: cond.to_string(),
                rules: inner,
            })];
        }
        if let Some(s) = &self.supports {
            // `supports(display: grid)` ist die Kurzform der Deklaration; als
            // Bedingung braucht `@supports` die Klammern.
            let t = s.trim_start().to_ascii_lowercase();
            let is_condition = t.starts_with('(') || t.starts_with("not ") || t.starts_with("not(");
            let prelude = if is_condition {
                s.clone()
            } else {
                format!("({s})")
            };
            inner = vec![Rule::Group(GroupRule {
                name: "supports".into(),
                prelude,
                rules: inner,
            })];
        }
        match &self.layer {
            None => out.extend(inner),
            Some(l) => out.push(Rule::Group(GroupRule {
                name: "layer".into(),
                prelude: match l {
                    ImportLayer::Anonymous => String::new(),
                    ImportLayer::Named(n) => n.clone(),
                },
                rules: inner,
            })),
        }
        out
    }
}

fn is_import(r: &Rule) -> bool {
    matches!(r, Rule::Other(AtRule { name, .. }) if name == "import")
}

/// `url(…)`, `url("…")` oder ein String am Anfang; Rest dahinter.
fn url(p: &str) -> Option<(String, &str)> {
    let (raw, rest) = if p.len() >= 4 && p[..4].eq_ignore_ascii_case("url(") {
        let r = p[4..].trim_start();
        if let Some(q) = r.chars().next().filter(|c| matches!(c, '"' | '\'')) {
            let end = string_end(r, q)?;
            let after = r[end + 1..].trim_start().strip_prefix(')')?;
            (&r[..=end], after)
        } else {
            let end = r.find(')')?;
            (r[..end].trim_end(), &r[end + 1..])
        }
    } else {
        let q = p.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        let end = string_end(p, q)?;
        (&p[..=end], &p[end + 1..])
    };
    let url = match raw.chars().next()? {
        q @ ('"' | '\'') => raw[1..].strip_suffix(q)?,
        _ => raw,
    };
    Some((url.to_string(), rest))
}

/// Byte-Position des schließenden Anführungszeichens (`s` beginnt mit `q`).
fn string_end(s: &str, q: char) -> Option<usize> {
    let mut esc = false;
    for (i, c) in s.char_indices().skip(1) {
        match c {
            _ if esc => esc = false,
            '\\' => esc = true,
            c if c == q => return Some(i),
            _ => {}
        }
    }
    None
}

/// `kw` am Anfang als eigenes Wort (danach Ende, Leerraum oder `(`);
/// liefert den Rest ab dort.
fn keyword<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    let head = s.get(..kw.len())?;
    let rest = &s[kw.len()..];
    (head.eq_ignore_ascii_case(kw)
        && rest
            .chars()
            .next()
            .is_none_or(|c| c == '(' || c.is_whitespace()))
    .then_some(rest)
}

/// `kw(` am Anfang; liefert den Rest hinter der Klammer.
fn keyword_call<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    keyword(s, kw)?.strip_prefix('(')
}

/// Position der Klammer, die eine schon geöffnete schließt; Strings und
/// Escapes zählen nicht.
fn paren_end(s: &str) -> Option<usize> {
    let mut depth = 1u32;
    let mut quote: Option<char> = None;
    let mut esc = false;
    for (i, c) in s.char_indices() {
        if esc {
            esc = false;
            continue;
        }
        match (c, quote) {
            ('\\', _) => esc = true,
            (c, Some(q)) if c == q => quote = None,
            (_, Some(_)) => {}
            ('"' | '\'', None) => quote = Some(c),
            ('(', None) => depth += 1,
            (')', None) => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Import {
        Import::parse(s).unwrap()
    }

    #[test]
    fn formen() {
        let i = p("url(x.css)");
        assert_eq!(i.url, "x.css");
        assert!(i.layer.is_none() && i.supports.is_none() && i.media.is_none());
        assert_eq!(p("\"a b.css\"").url, "a b.css");
        assert_eq!(p("url(\"a)b.css\") screen").url, "a)b.css");
        assert_eq!(p("url(x.css) layer").layer, Some(ImportLayer::Anonymous));
        assert_eq!(
            p("url(x.css) layer(a.b) supports(display: grid) screen and (min-width: 1px)"),
            Import {
                url: "x.css".into(),
                layer: Some(ImportLayer::Named("a.b".into())),
                supports: Some("display: grid".into()),
                media: Some("screen and (min-width: 1px)".into()),
            }
        );
        assert_eq!(p("url(x.css) print").media.as_deref(), Some("print"));
        assert_eq!(Import::parse("layer(x)"), None);
        assert_eq!(Import::parse("url(x.css) layer(x"), None);
    }
}
