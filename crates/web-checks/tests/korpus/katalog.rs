//! Liest den Mutationskatalog (`tests/mutations/*.toml`).
//!
//! Ohne TOML-Abhängigkeit: Der Katalog benutzt nur eine kleine Teilmenge —
//! `[[mutation]]`-Tabellen mit Zeichenketten als Werten, einfach (`'…'`,
//! `"…"`) oder mehrzeilig (`'''…'''`), und `#`-Kommentare. Was davon
//! abweicht, lässt den Test scheitern, statt still übergangen zu werden.
//! `xtask/mutate.py` liest dieselben Dateien mit `tomllib`. Gleich wie in
//! `a11y-rules/tests/korpus/katalog.rs`; jedes Paket bringt seine Kopie mit,
//! damit es sich allein veröffentlichen lässt.

use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct Mutation {
    pub id: String,
    /// Pfad relativ zum Crate.
    pub file: String,
    pub find: String,
    pub replace: String,
    /// `killed` oder `equivalent`.
    pub expect: String,
    pub note: String,
}

pub fn lade(verzeichnis: &Path) -> Vec<Mutation> {
    let mut dateien: Vec<_> = std::fs::read_dir(verzeichnis)
        .unwrap_or_else(|e| panic!("{}: {e}", verzeichnis.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    dateien.sort();
    dateien
        .iter()
        .flat_map(|p| {
            let text = std::fs::read_to_string(p).unwrap();
            lies(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        })
        .collect()
}

fn lies(text: &str) -> Result<Vec<Mutation>, String> {
    let mut out: Vec<Mutation> = Vec::new();
    let mut rest = text;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        if let Some(r) = rest.strip_prefix('#') {
            rest = r.split_once('\n').map_or("", |(_, r)| r);
            continue;
        }
        if let Some(r) = rest.strip_prefix("[[mutation]]") {
            out.push(Mutation::default());
            rest = r;
            continue;
        }
        let (schluessel, r) = rest
            .split_once('=')
            .ok_or_else(|| format!("erwartet `schlüssel = wert` bei {:?}", anfang(rest)))?;
        let schluessel = schluessel.trim();
        let r = r.trim_start_matches([' ', '\t']);
        let (wert, r) = zeichenkette(r)?;
        let m = out
            .last_mut()
            .ok_or("Schlüssel vor dem ersten [[mutation]]")?;
        match schluessel {
            "id" => m.id = wert,
            "file" => m.file = wert,
            "find" => m.find = wert,
            "replace" => m.replace = wert,
            "expect" => m.expect = wert,
            "note" => m.note = wert,
            // Nur für Leser und den Läufer; der Test braucht sie nicht.
            "kind" | "test" | "filter" => {}
            k => return Err(format!("unbekannter Schlüssel {k:?}")),
        }
        rest = r;
    }
    for m in &out {
        if m.id.is_empty() || m.file.is_empty() || m.find.is_empty() || m.expect.is_empty() {
            return Err(format!("unvollständiger Eintrag: {m:?}"));
        }
    }
    Ok(out)
}

fn anfang(s: &str) -> &str {
    &s[..s.len().min(40)]
}

/// Liest eine Zeichenkette am Anfang von `s`; gibt sie und den Rest zurück.
fn zeichenkette(s: &str) -> Result<(String, &str), String> {
    if let Some(r) = s.strip_prefix("'''") {
        // TOML: Ein Zeilenumbruch direkt nach den öffnenden Zeichen gehört
        // nicht zum Wert.
        let r = r.strip_prefix('\n').unwrap_or(r);
        let ende = r.find("'''").ok_or("''' nicht geschlossen")?;
        return Ok((r[..ende].to_string(), &r[ende + 3..]));
    }
    if let Some(r) = s.strip_prefix('\'') {
        let ende = r.find('\'').ok_or("' nicht geschlossen")?;
        if r[..ende].contains('\n') {
            return Err("Zeilenumbruch in '…'".into());
        }
        return Ok((r[..ende].to_string(), &r[ende + 1..]));
    }
    if let Some(r) = s.strip_prefix('"') {
        let mut wert = String::new();
        let mut zeichen = r.char_indices();
        while let Some((i, c)) = zeichen.next() {
            match c {
                '"' => return Ok((wert, &r[i + 1..])),
                '\\' => match zeichen.next() {
                    Some((_, 'n')) => wert.push('\n'),
                    Some((_, 't')) => wert.push('\t'),
                    Some((_, '"')) => wert.push('"'),
                    Some((_, '\\')) => wert.push('\\'),
                    other => return Err(format!("Escape {other:?} nicht unterstützt")),
                },
                '\n' => return Err("Zeilenumbruch in \"…\"".into()),
                c => wert.push(c),
            }
        }
        return Err("\" nicht geschlossen".into());
    }
    Err(format!("erwartet eine Zeichenkette bei {:?}", anfang(s)))
}

#[test]
fn liest_die_teilmenge() {
    let m = lies(
        "# Kommentar\n[[mutation]]\nid = \"a/b\"\nfile = 'src/x.rs'\nfind = '''\nif a {\n'''\nreplace = 'if !a {'\nexpect = \"killed\"\n",
    )
    .unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].find, "if a {\n");
    assert_eq!(m[0].replace, "if !a {");
}
