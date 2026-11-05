//! Liest den Mutationskatalog (`tests/mutations/*.toml`).
//!
//! Ohne TOML-Abhängigkeit: Der Katalog benutzt nur eine kleine Teilmenge —
//! `[[mutation]]`-Tabellen mit Zeichenketten als Werten, einfach (`'…'`,
//! `"…"`) oder mehrzeilig (`'''…'''`), und `#`-Kommentare. Was davon
//! abweicht, lässt den Test scheitern, statt still übergangen zu werden.
//! `xtask/mutate.py` liest dieselben Dateien mit `tomllib`.

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

pub fn load(directory: &Path) -> Vec<Mutation> {
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("{}: {e}", directory.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|p| {
            let text = std::fs::read_to_string(p).unwrap();
            read(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        })
        .collect()
}

fn read(text: &str) -> Result<Vec<Mutation>, String> {
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
        let (key, r) = rest
            .split_once('=')
            .ok_or_else(|| format!("erwartet `schlüssel = wert` bei {:?}", start(rest)))?;
        let key = key.trim();
        let r = r.trim_start_matches([' ', '\t']);
        let (value, r) = string(r)?;
        let m = out
            .last_mut()
            .ok_or("Schlüssel vor dem ersten [[mutation]]")?;
        match key {
            "id" => m.id = value,
            "file" => m.file = value,
            "find" => m.find = value,
            "replace" => m.replace = value,
            "expect" => m.expect = value,
            "note" => m.note = value,
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

fn start(s: &str) -> &str {
    &s[..s.len().min(40)]
}

/// Liest eine Zeichenkette am Anfang von `s`; gibt sie und den Rest zurück.
fn string(s: &str) -> Result<(String, &str), String> {
    if let Some(r) = s.strip_prefix("'''") {
        // TOML: Ein Zeilenumbruch direkt nach den öffnenden Zeichen gehört
        // nicht zum Wert.
        let r = r.strip_prefix('\n').unwrap_or(r);
        let end = r.find("'''").ok_or("''' nicht geschlossen")?;
        return Ok((r[..end].to_string(), &r[end + 3..]));
    }
    if let Some(r) = s.strip_prefix('\'') {
        let end = r.find('\'').ok_or("' nicht geschlossen")?;
        if r[..end].contains('\n') {
            return Err("Zeilenumbruch in '…'".into());
        }
        return Ok((r[..end].to_string(), &r[end + 1..]));
    }
    if let Some(r) = s.strip_prefix('"') {
        let mut value = String::new();
        let mut character = r.char_indices();
        while let Some((i, c)) = character.next() {
            match c {
                '"' => return Ok((value, &r[i + 1..])),
                '\\' => match character.next() {
                    Some((_, 'n')) => value.push('\n'),
                    Some((_, 't')) => value.push('\t'),
                    Some((_, '"')) => value.push('"'),
                    Some((_, '\\')) => value.push('\\'),
                    other => return Err(format!("Escape {other:?} nicht unterstützt")),
                },
                '\n' => return Err("Zeilenumbruch in \"…\"".into()),
                c => value.push(c),
            }
        }
        return Err("\" nicht geschlossen".into());
    }
    Err(format!("erwartet eine Zeichenkette bei {:?}", start(s)))
}

#[test]
fn reads_the_subset() {
    let m = read(
        "# Kommentar\n[[mutation]]\nid = \"a/b\"\nfile = 'src/x.rs'\nfind = '''\nif a {\n'''\nreplace = 'if !a {'\nexpect = \"killed\"\n",
    )
    .unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].find, "if a {\n");
    assert_eq!(m[0].replace, "if !a {");
}
