# Changelog

Alle nennenswerten Änderungen an `a11y-static` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

### Added

- **Ausfallgrund:** `Sheet::failed(source, reason)`; der Grund steht im `detail` des `StylesheetUnavailable`-Vermerks, die Kontrastregeln bleiben `untested`.
- **`@import`:** Ein Host, der importierte Blätter lädt, übergibt sie in `Sheet::imports`; `StylesheetUnresolved` entfällt dann. `layer`, `layer(x)`, `supports()` und Medienliste am `@import` gelten für das übergebene Blatt (Cascade 5 §6.4.1). Ohne Treffer, mit Fehlschlag und bei Zyklen bleibt der Import unaufgelöst.
- **Leeres `href`:** Ein `<link rel=stylesheet>` ohne brauchbares `href` ergibt einen `StylesheetUnavailable`-Vermerk statt still zu fehlen.
- **Doku:** URL-Auflösung und Abruf liegen beim Host.

### Changed

- **`Sheet` (inkompatibel zu 0.1.0):** `#[non_exhaustive]` mit den neuen Feldern `failure` und `imports`; Aufbau über `Sheet::new`, `Sheet::failed`, `with_media`, `with_imports`. `media` am `Sheet` ist jetzt das `media` des `<link>`, das es beantwortet, kein Überschreiben mehr; ohne `media` passt es auf jeden Link mit dem `href`.
- **Zuordnung:** Ein `<link>` wird über `href` plus `media` zugeordnet; zwei Links mit gleichem `href` und verschiedenem `media` bekommen je ihr Blatt.

## [0.1.0] - 2026-11-06

### Added

- **Fassade:** `audit(html, sheets, env)` und `Page::audit` prüfen statisches HTML mit allen Regel-Tiers (Struktur, Semantik, Darstellung, Stylesheets) in der richtigen Reihenfolge; der Host holt nur noch die Stylesheets.
- **Seite:** `Page::parse` und `Page::stylesheet_refs` finden `<link rel=stylesheet>` (ohne `alternate`, ohne `disabled`, mit `href`) und `<style>` in Dokumentreihenfolge samt `media`; die Kaskade wertet `media` gegen die `Environment` aus.
- **Eingaben:** Ein `<link>` ohne mitgeliefertes `Sheet` ergibt einen `InputNote` `StylesheetUnavailable`, ein nicht aufgelöstes `@import` einen `StylesheetUnresolved`; solange ein solcher Vermerk vorliegt, meldet der Host keine Farben, die Kontrastregeln ergeben `untested` statt bestanden oder verfehlt.
- **Host:** Der `html5-parser`-Baum wird ohne Kopie gelesen; `aria-hidden`, `inert`, das `hidden`-Attribut und `display: none` aus der Kaskade zählen als nicht dargestellt.
- **Feature `de`:** deutsche Befundtexte, durchgereicht an `a11y-rules`.
