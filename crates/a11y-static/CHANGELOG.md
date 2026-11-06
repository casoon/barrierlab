# Changelog

Alle nennenswerten Änderungen an `a11y-static` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

## [0.1.0] - 2026-11-06

### Added

- **Fassade:** `audit(html, sheets, env)` und `Page::audit` prüfen statisches HTML mit allen Regel-Tiers (Struktur, Semantik, Darstellung, Stylesheets) in der richtigen Reihenfolge; der Host holt nur noch die Stylesheets.
- **Seite:** `Page::parse` und `Page::stylesheet_refs` finden `<link rel=stylesheet>` (ohne `alternate`, ohne `disabled`, mit `href`) und `<style>` in Dokumentreihenfolge samt `media`; die Kaskade wertet `media` gegen die `Environment` aus.
- **Eingaben:** Ein `<link>` ohne mitgeliefertes `Sheet` ergibt einen `InputNote` `StylesheetUnavailable`, ein nicht aufgelöstes `@import` einen `StylesheetUnresolved`; solange ein solcher Vermerk vorliegt, meldet der Host keine Farben, die Kontrastregeln ergeben `untested` statt bestanden oder verfehlt.
- **Host:** Der `html5-parser`-Baum wird ohne Kopie gelesen; `aria-hidden`, `inert`, das `hidden`-Attribut und `display: none` aus der Kaskade zählen als nicht dargestellt.
- **Feature `de`:** deutsche Befundtexte, durchgereicht an `a11y-rules`.
