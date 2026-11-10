# relax-ng

RELAX NG in reiner Rust: Schema lesen (Compact und XML), vereinfachen, Instanzen mit
Brzozowski-Ableitungen prüfen. Norm: RELAX NG Specification. Der Kern enthält keinen XML-Parser;
Instanzen kommen über das Trait `Element` (Baum) oder über Ereignisse (`StreamValidator`) herein.

## Streaming-Validierung

`StreamValidator` prüft ein Dokument aus Ereignissen, ohne dass der Aufrufer einen `Element`-Baum
aufbaut (barrierlab#77). Der Baumpfad (`validate`, `Schema::validate`) bleibt unverändert.

```rust
let mut v = schema.stream_validator::<()>(&registry)?; // oder StreamValidator::new
v.start_element(name, attributes, namespace_bindings, location)?;
v.text("…")?;           // beliebig oft, wird bis zum nächsten Tag zusammengefasst
v.end_element()?;
let findings = v.finish()?; // Vec<ValidationError<L>>, leer = gültig
```

### Ereignismodell

| Ereignis | Ableitungsschritt |
|---|---|
| `start_element(name, attributes, bindings, location)` | `startTagOpen` (Ableitung des Elternmusters nach dem Namen; Inhalt des Kindes bestimmen), dann je Attribut `attDeriv` (ungeordnet), `startTagClose` ist implizit: die Kindinhalte starten mit dem Rest-Muster |
| `text(&str)` | wird im Rahmen des offenen Elements gesammelt |
| nächstes `start_element` / `end_element` | gesammelter Text wird als **ein** Textknoten abgeleitet (`textDeriv`) |
| `end_element` | Textrest ableiten, dann Nullable-Prüfung (`endTag`); sonst `MissingContent` |
| `finish` | prüft, dass genau ein Wurzelelement vollständig war |

Attribute stehen gebündelt am `start_element`, nicht als einzelne Ereignisse: ein Pull-Parser
liefert sie ohnehin mit dem Start-Tag, und so gibt es keinen Zustand „Tag offen, Attribute
unvollständig".

### Text

- `data`, `value` und `list` brauchen den vollständigen Text eines Textknotens. Deshalb wird Text
  je offenem Element gepuffert und erst an einer Kindgrenze (Start- oder End-Tag) abgeleitet.
  Aufeinanderfolgende `text`-Aufrufe sind ein Lauf; die Zusammenführung über Kommentare hinweg
  bleibt Sache des Aufrufers (wie bei `Content::Text`).
- Reiner Leerraum (§6.2.7, schwacher Vergleich): Steht er neben anderen Kindern, wird er nicht
  abgeleitet. „Neben anderen Kindern" heißt: ein früheres Kind existierte, oder ein Kind beginnt
  direkt danach. Das ist das Kriterium `Kindanzahl > 1` des Baumpfads, nur ohne Vorausblick auf
  das Elementende. Ein Element, dessen einziges Kind ein Leerraumlauf ist, bekommt ihn wie bisher
  angeboten.
- Passt Text nicht, ist es `UnexpectedText`, außer bei reinem Leerraum, wenn nichts im Muster
  Text verlangt (`wants_text`) — identisch zum Baumpfad.

### Zustand

Je offenem Element ein Frame: Rest-Muster (interniert, `Id`), Datentyp-Kontext (Namensraum-
bindungen), Ort, gepufferter Text, Kindzähler. Dazu der Pfad der Elementnamen, die Befundliste
und ein Zähler für übersprungene Teilbäume. Speicher wächst mit Verschachtelungstiefe, Befunden,
der Länge des längsten ungeteilten Textlaufs und dem Musterinterner des Schemas, nicht mit der
Dokumentgröße. (Der Interner wächst mit der Zahl verschiedener Muster, die vorkommen.)

### Positionen

Der Aufrufer gibt `location: Option<L>` je Element mit; `L: Clone`. Ein Befund trägt den Ort des
Elements, auf dem er entsteht (Attribute, Text, `MissingContent`) bzw. des unerwarteten Kindes
(`UnexpectedElement`) — wie im Baumpfad. Die Pfade sind dieselben `ExpandedName`-Listen.

### Unerwartete Elemente

Ein unerwartetes Element erzeugt `UnexpectedElement` und sein Teilbaum wird übersprungen
(Ereignisse darin werden gezählt, nicht geprüft), wie im Baumpfad. Das Muster des Elternknotens
rückt dabei nicht weiter.

### Fehler im Ereignisstrom

Ein ungültiger Ereignisstrom (End-Tag ohne Start, zweite Wurzel, Text außerhalb der Wurzel
außer Leerraum, `finish` vor dem Ende der Wurzel) ist ein `StreamError`, nie ein Befund und nie
ein stiller Erfolg. Danach ist der Zustand unbestimmt; der Validator wird verworfen. Ein Schema
mit nicht unterstütztem Datentyp scheitert wie im Baumpfad schon in `new` mit `DatatypeError`.

### Grenzen

- Ein einzelner Textlauf wird ganz gepuffert.
- `StreamValidator` prüft ein Dokument; für das nächste ein neuer (der Schema-Interner wird nicht
  geteilt).
- Keine Wiederherstellung nach einem unerwarteten Element: `ValidationOptions::recover_after_unexpected_element`
  (#76) wirkt nur im Baumpfad (`validate_with_options`). Ohne die Option liefern beide Pfade dasselbe.
- Kein Parser: Aufrufer liefern Namen bereits expandiert, ohne `xmlns`-Attribute.

### Gleichwertigkeit

Der Baumpfad und der Ereignispfad sind zwei Implementierungen derselben Schritte
(`src/validator/stream.rs` und `validate_element_content`); der Baumpfad kennt zusätzlich die
opt-in Wiederherstellung (`ValidationOptions`), die nur dort existiert; ohne sie ist er
unverändert. Abgesichert wird das durch Tests: der
Spectest (`tests/spectest.rs`) treibt jede `valid`/`invalid`-Instanz durch beide Pfade, Textläufe
in zwei Teile zerlegt, und verlangt **gleiche Befunde in gleicher Reihenfolge**, nicht nur gleiche
Urteile. Dasselbe prüft ein ignorierter Test in `html-conform` über das ganze vnu-Korpus
(`cargo test --release -p html-conform --lib stream_and_tree -- --ignored`).
