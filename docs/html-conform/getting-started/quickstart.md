---
title: "Quickstart"
description: "Check a document, print its findings, and fail when it has errors."
order: 2
---

## Check a document

`check()` takes the complete document as a `&str` and returns a `CheckReport`:

```rust
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let html = std::fs::read_to_string("index.html")?;
    let report = html_conform::check(&html)?;

    for finding in &report.findings {
        let at = finding.location.map(|l| l.to_string()).unwrap_or_default();
        println!("{:?} {at} {} {}", finding.severity, finding.rule_id, finding.message);
    }

    if report.has_errors() {
        std::process::exit(1);
    }
    Ok(())
}
```

`SourceLocation` displays as `line:column`. For this document:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Parse errors</title>
</head>
<body>
  <p>Fish &chips; for dinner.</p>
  <p class="intro" class="lead">Duplicate attribute</p>
</body>
</html>
```

the program prints:

```text
Error 8:17 parser.html5 an unknown named character reference
Error 9:25 parser.html5 a duplicate attribute on a tag
```

and exits with status 1.

## What `?` on `check()` means

`check()` returns `Err` only when the checker itself cannot start: the embedded schema or
rule set failed to load. A non-conforming document is never an error; its problems are in
`report.findings`. See [Using html-conform as a dependency](../../guides/dependency/) for
the details.

## Next steps

- [Findings and rule IDs](../../guides/rule-ids/) explains what each finding source reports.
- The [showcase](../../../showcase/) shows the findings for seven sample documents.
