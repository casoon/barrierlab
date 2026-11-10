# Changelog

Alle nennenswerten Änderungen an `xml-adapters` stehen hier. Das Format folgt
[Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

### Added

- **Tests:** `RngElement` besteht den vendorten jing-spectest-Korpus (553 Instanzen, ohne `include`/`externalRef`-Fälle) mit denselben Urteilen wie der Test von `relax-ng`; Fehler tragen dabei eine Position (casoon/barrierlab#75).

## [0.1.0] - 2026-11-08

### Added

- **roxmltree:** `RngElement` implementiert `relax_ng::Element` für `roxmltree`-Elemente (Text über Kommentare hinweg zusammengefasst, Namensraumbindungen, Fehlerposition) (casoon/barrierlab#75).
- **roxmltree:** `XmlDoc` und `XmlNode` implementieren `xpath_eval::Document` und `Node` und damit `schematron_engine::evaluate`: Namensraum-Achse einschließlich `xml`, Kommentare, Verarbeitungsanweisungen, `xml:id` als ID-Attribut, `XmlNode::position()` (casoon/barrierlab#75).
- **Features:** `relax-ng` und `xpath`, beide standardmäßig an.
