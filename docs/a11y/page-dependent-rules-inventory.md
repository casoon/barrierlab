---
title: "Seitenabhängige Regeln: Bestandsaufnahme"
description: "Welche Regeln von auditmysite die laufende Seite brauchen, welche Daten ein Host dafür liefern müsste und welche davon nach a11y-rules wandern."
order: 5
---

Bestandsaufnahme zu [#130](https://github.com/casoon/barrierlab/issues/130), Voraussetzung 5 des
Konzepts [#127](https://github.com/casoon/barrierlab/issues/127). Reine Analyse, kein Code.
Stand: `main` bei `e951e1e` (2026-11-08), `a11y-dom` und `a11y-rules` 0.26.1,
auditmysite und liveaudit auf dem jeweiligen `main` ihrer Checkouts. Alles ist am Code
gelesen, nicht aus Dateinamen geschlossen.

Die Zeilen 2, 3, 4, 5, 7 und 10 haben je ein Folgeissue (#132 bis #136); dieses Dokument
führt sie mit, damit die Tabelle vollständig ist, und widerspricht ihnen nicht.

## Umfang

auditmysite hat in `src/wcag/rules` **16 Dateien mit einer `chromiumoxide::Page`-Signatur und
18 Einstiegspunkte** (`media_rules.rs` und `timing_adjustable.rs` haben je zwei). Die „17" in
#127 und #130 zählt `page_rules.rs` mit, den Katalog `PAGE_RULES`, der keine Regel ist.
`check_reflow_with_page` läuft nicht über den Katalog, sondern direkt aus der Pipeline.

Nicht alle 18 brauchen die Seite:

- Drei nehmen `_page` und fassen es nie an (Zeilen 15, 16, 17).
- Drei lesen nur das DOM, das auditmysite ohnehin als `Document` hält (Zeilen 9, 14, 18).
- Die übrigen zwölf brauchen Daten, die nur eine laufende oder gesteuerte Seite hat.

## Was es schon gibt

`a11y-dom` `Rendering` (Tier 3) kennt als Vorgabemethoden mit `None` = „nicht gemessen":
`computed_style`, `bounds`, `is_rendered`, `layout` (`Layout`: `flex_reversed`, `order`,
`min_width_px`, `cursor_pointer`, `infinite_animation`, `obscured`, `hides_focus`,
`focus_visible`, `pointer_events_none`, `animating`), `scroll_overflow_px`,
`visually_hidden`, `sampled_backdrop`. Jede bisherige Erweiterung von Tier 3 war eine neue
Vorgabemethode auf `Rendering`; die Vorschläge unten folgen dem. `Interaction` (Tier 4) hat
`tab_order` und `has_visible_focus`, aber keine Regel und kein Runner benutzt es.

Die Hosts:

- **auditmysite** implementiert `Rendering` über CDP und füllt `bounds`,
  `scroll_overflow_px`, `visually_hidden`, `sampled_backdrop` und `layout`.
- **liveaudit** liefert Farbe, effektiven Hintergrund, Schrift, `display`/`visibility`, die
  Layout-Bitspalte, `order`, `min-width` und `bounds` nur für per Tab erreichbare Elemente.
  Das **Layout-Byte ist voll** (8 von 8 Bits). Jede neue Boolesche braucht eine neue Spalte in
  `a11y-wasm` und im Collector; neue Messungen je Knoten sind deshalb ausdrückliche
  Durchgänge wie `focus: true`. Für nicht erreichbare Frames liefert `DocumentScan.untested`
  schon `cross-origin-frame` (liveaudit `docs/decisions.md`). liveaudit kann den Viewport
  nicht setzen.
- **Agent-Plugin** (geplant, #127): Browser-Werkzeug plus Collector plus `a11y-wasm`; es läuft je
  konfiguriertem Viewport (`RuleRun::in_viewport`). Es zählt als zweiter Host erst, wenn es
  die jeweilige Fähigkeit tatsächlich hat (#127 sagt das für Fokusspuren ausdrücklich).

## Die Bestandsaufnahme

Die Spalte „Hosts" nennt, wer die Daten braucht: **am** = auditmysite, **la** = liveaudit,
**ag** = Agent-Plugin. „Mutiert" heißt: der Host ändert die Seite für die Messung (eigener
Durchgang, opt-in). Größe: S = eine Vorgabemethode und eine kleine Regel; M = neue Datenform in
zwei Hosts plus Regel mit Schwellen; L = neue Stufe oder neues Protokoll.

| # | Regel (auditmysite, Datei) | WCAG, dort emittierte ids | Liest von der Seite | Passt zu `Rendering`? Neue Spalten | Mutiert? | Hosts | Vorgeschlagene id | Blocker, offene Fragen | Issue |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `check_reflow_with_page` (`reflow.rs`) | 1.4.10 / `css-overflow-hidden` | Viewport-Override 320 x 256, 350 ms Wartezeit; `scrollWidth` gegen `innerWidth`; `getBoundingClientRect().right` aller Elemente; Ausnahmen per `closest()` (Tabellen, `img`, `svg`, `canvas`, `video`, `iframe`, Rollen `img`, `table`, `grid`, `application` …) | Teils. `bounds` gibt es, liveaudit füllt es nur für erreichbare Elemente. Neu: `snapshot_viewport() -> Option<(f32, f32)>`, `document_scroll_width() -> Option<f32>`, `bounds` für Elemente jenseits des Randes | Ja (Viewport setzen, danach zurück) | am, ag (la kann den Viewport nicht setzen) | `reflow/overflow-x` (`reflow/min-width` bleibt der statische Stellvertreter) | Zweiter Host existiert noch nicht. Regel darf auf einem Snapshot, der nicht 320 CSS-px breit ist, nichts behaupten: `NotRun` mit gemessener Breite. Geprüft wird nur die Breite, nicht die 256-px-Höhe | neues Issue nötig, **zurückgestellt** (Entwurf unten) |
| 2 | `check_text_spacing_with_page` (`text_spacing.rs`) | 1.4.12 / `text-spacing` | Eingespritztes Stylesheet (Zeilenhöhe 1,5, Zeichenabstand 0,12 em, Wortabstand 0,16 em, Absatzabstand 2 em); zwei Messungen (erste 2000 Elemente): `overflow` hidden, `scrollWidth/Height` gegen `clientWidth/Height` (+2 px), `backface-visibility` bis 5 Vorfahren, Text nicht leer, Kasten über 2 px | Nein. `scroll_overflow_px` deckt nur `auto`/`scroll`. Neu: `text_spacing_clip(node) -> Option<SpacingClip { before, after }>` | **Ja** (Stylesheet einspritzen, entfernen) | am, la, ag | `text-spacing/clipped` (REVIEW) | Wirkung einer CSP (`style-src`) auf das Einspritzen nicht geprüft; dann `NotRun` mit Grund. Überlappung und verschwindender Inhalt nicht erkennbar, daher nie FAIL | #136 |
| 3 | `check_orientation_with_page` (`orientation.rs`) | 1.3.4 / `css-orientation-lock` | Berechnetes `transform`-Matrix und `rotate` von `html` und `body` | Nein. `ComputedStyle` hat kein `transform`/`rotate`. Neu: zwei `Option<String>`-Felder auf `ComputedStyle` | Nein | am, la, ag | `orientation/root-rotated` | Regel sieht einen Viewport; ein Lock nur in der anderen Ausrichtung braucht einen zweiten Snapshot (`RuleRun::in_viewport`). „Wesentlich"-Ausnahme bleibt menschliches Urteil. liveaudit: Layout-Byte voll, also neue Spalten | #134 |
| 4 | `check_focus_not_obscured_minimum_with_page` (`focus_not_obscured_minimum.rs`) | 2.4.11 / `focus-not-obscured-minimum` | Fokuslauf bis 60 Steuerelemente: `focus({preventScroll})`, `scrollIntoView({block:'nearest'})`, `elementsFromPoint` an 5 Punkten; Blocker = Nicht-Vorfahr mit `position: fixed/sticky`, darüber gemalt. Rückfall: Überlagerungen und Fokussierbare als Rechtecke, Überlappung 0,98 | Teils. `Layout.obscured` und `hides_focus` speisen `focus/obscured`. Die Messung je Steuerelement nach dem Fokussieren fehlt. Neu: `focus_cover(node) -> Option<FocusCover { points, covered, blocker }>` | **Ja** (Fokus, Scroll; danach wiederherstellen) | am, la, ag | `focus/obscured` (besteht), erweitert um `FocusCover` | Welcher Scroll ein echtes Tab erzeugt, ist offen (auditmysite nimmt `nearest`); der Vertrag muss festhalten, wie der Host scrollte. Statische Geometrie darf nicht als Messung durchgehen. liveaudit: neue Spalte | #135 |
| 5 | `check_focus_not_obscured_enhanced_with_page` (`focus_not_obscured_enhanced.rs`) | 2.4.12 (AAA) / `focus-not-obscured-enhanced` | Nur statische Geometrie, kein Fokus, kein Scroll; Überlappung 0,05, Malreihenfolge in der Mitte der Überlappung | Dieselben Daten wie Zeile 4 | wie 4 | am, la, ag | `focus/obscured-enhanced` | Beide auditmysite-Regeln gehen unterschiedlich vor (2.4.11 läuft, 2.4.12 misst statisch); nach der Migration gleiche Datenquelle, andere Schwelle (`covered > 0`) | #135 |
| 6 | `check_video_caption_tracks_with_page` (`media_rules.rs`) | 1.2.2 / `video-caption` (nur Pass oder UNTESTED, nie FAIL) | Sichtbare `<video>`; `<track>` (kind, `src`, `srclang`, `label`); `iframe[src]` gegen 7 Einbettungs-Hosts; **Netz**: HEAD auf bis zu 10 gleich-ursprüngliche Tracks, `.vtt`/`.srt` oder Content-Type `vtt` | Attribute und Sichtbarkeit: ja (Tier 1). Ergebnis des Abrufs: nein, bräuchte eine Ressourcen-Statusangabe | Nein (aber Netzzugriff) | nur am | (`media/caption-track-unresolved`, falls je) | Braucht E/A, das eine Bibliothek nicht tut; nur ein Host. Die statische Hälfte deckt `manual/media-alternatives` | bleibt in auditmysite |
| 7 | `check_frame_tested_with_page` (`media_rules.rs`) | `frame-tested` (UNTESTED je Cross-Origin-Frame; mit 4.1.2 getaggt, ist aber eine Abdeckungsnotiz) | `iframe`/`frame` inkl. offener Shadow-Roots; überspringt role none/presentation, `hidden`, `aria-hidden`, `display:none`, `visibility`, Kasten bis 1 px; Cross-Origin = `contentDocument === null` oder wirft | Erreichbarkeit: nein. Neu: `frame_reachable(node) -> Option<bool>` | Nein | am, la (`DocumentScan.untested` liefert es schon) | `frames/cross-origin` (kein WCAG-Id) | Zuhause der Methode: `Rendering` oder `Document`. liveaudit hält es bis zu einer id aus dem `Report` | #132 |
| 8 | `check_same_origin_iframes_with_page` (`iframe_rules.rs`) | 3.1.1 / `html-has-lang` am `<html>` des Frames | `lang` des gleich-ursprünglichen `contentDocument` | Braucht nichts Neues: `document/lang-missing` auf dem Frame als eigenes Dokument | Nein | am, la | besteht: `document/lang-missing` | Auswertung schon geteilt; offen ist reine Host-Arbeit in auditmysite (je gerendertem Frame ein `Document` bauen, wie liveaudit). liveaudit prüft auch versteckte und 1-px-Frames, auditmysite filtert sie | Host-Arbeit, kein barrierlab-Issue |
| 9 | `check_page_titled_with_page` (`page_titled.rs`) | 2.4.2 / `document-title` (nur generischer Titel; fehlend/leer sind schon `document/title-*`) | Nur `<title>`-Text, kein Seitenzugriff | n/a (Tier 1) | Nein | nur am | (`document/title-generic`, falls je) | Die Regel ist eine englische Wortliste (`untitled`, `home`, `index` …); `a11y-rules` dokumentiert Regeln als wortlautunabhängig | bleibt in auditmysite, Grundsatzfrage s. u. |
| 10 | `check_accessible_authentication_with_page` (`accessible_authentication.rs`) | 3.3.8 / `accessible-auth-paste-blocked` | Sichtbare `input[type=password]`, `[autocomplete~=one-time-code]`, `[autocomplete~=current-password]` (Frames, offene Shadow-Roots); synthetisches abbrechbares `paste` mit Probetext; Ergebnis von `dispatchEvent`, Probetext in `value`?; `value` wiederhergestellt; `alert`/`confirm`/`prompt` stumm | Auswahl: ja. Ergebnis: nein. Neu: `paste_probe(node) -> Option<PasteProbe { cancelled, inserted }>` | **Ja** (führt Seiten-Handler aus, schreibt `value`) | am, la, ag | `auth/paste-blocked` (FAIL, 3.3.8) | Ereignis ist synthetisch (`isTrusted` falsch); Handler, die nur auf vertrauenswürdige Ereignisse reagieren, bleiben unsichtbar. Technikkennung F109 vor Aufnahme in `Meta` gegen das Techniques-Dokument prüfen | #133 |
| 11 | `check_on_input_with_page` (`on_input.rs`) | 3.2.2 / `context/on-input` (FAIL bei Kontextwechsel in der aufgerufenen Funktion, REVIEW für die Namensheuristik) | `select`/Combobox/Listbox/Radio; `onchange`-Attribut; bei `name(...)`-Handlern der **Quelltext von `window[name]`** (`Function.prototype.toString`); Schaltflächennamen; Absende-Element | Attribute, Formularzugehörigkeit: ja. Neu: `handler_source(node) -> Option<String>` je Element | Nein (liest nur) | am, la möglich, ag | `context/on-input` (besteht, REVIEW) bekommt den FAIL-Zweig | Muster über den Quelltext (`location=`, `.submit(`, `window.open(` …) sind selbst Heuristik. Die Wortlisten der REVIEW-Hälfte (`language`, `go`, `absenden` …) sind eine eigene Grundsatzfrage. Ob liveaudit es braucht, ist offen. Neue Spalte in `a11y-wasm` | neues Issue nötig (Entwurf unten) |
| 12 | `check_modern_attributes_with_page` (`modern_attributes.rs`) | 4.1.2 / `modern-attribute-misuse` (REVIEW: offenes Menü/Popover ohne Namen; FAIL: offenes Popover mit `inert`) | `[popover]:popover-open` (Laufzeitzustand), `role=menu` sichtbar, Name aus `aria-labelledby`/`aria-label`/`title` | Sichtbarkeit und Name: ja. Neu: ein Bit `popover_open` | Nein | am; la könnte, kein Bedarf bekannt | (`popover/open-inert`, falls je) | Die Namensnotiz hat laut Kommentar der Regel keine Norm hinter sich; nur der `inert`-Fall ist belastbar. Kein Korpusfall gefunden. Wert gering | bleibt in auditmysite, bis ein zweiter Host fragt |
| 13 | `check_display_modes_with_page` (`display_modes.rs`) | `display/init-missing`, `display/text-media-visible`, `display/text-not-visible`, `display/text-hidden` (Best Practice, 2.2.2 / 1.1.1) | `checkVisibility(...)`, Rechtecke (Medien über 32 x 32 px), `hidden`/`aria-hidden`/`inert`-Vorfahren, `aria-describedby`; **`window.__amsDisplayObserved` / `__amsDisplayAtBody`, gesetzt von einem vor der Navigation registrierten Skript** | Größtenteils (`is_rendered`, `bounds`, Flags). Es fehlen `opacity` und `bounds` für nicht fokussierbare Elemente; `init-missing`-Zeitpunkt: nein | Ja (Textmodus herstellen) | nur am | (besteht statisch als `display/*`) | `init-missing` braucht einen Hook vor der Navigation, den ein In-Page-Host nie setzen kann. Die Konvention gehört einem Projekt; auditmysite übernimmt `display/init-missing` und `display/text-hidden` bewusst nicht. Wenn je verschoben: L | bleibt in auditmysite |
| 14 | `check_pointer_cancellation_with_page` (`pointer_cancellation.rs`) | 2.5.2 / `pointer-cancellation` (UNTESTED-Notiz) | Eine DOM-Abfrage: gibt es etwas, das ein einzelner Zeiger bedient | n/a | Nein | nur am | besteht: `manual/pointer-cancellation` | Nichts zu entscheiden; dieselbe Notiz steht im Checklisten-Punkt, mit engerer Anwendbarkeit (auditmysite zählt jedes `script`) | Host-Arbeit: in auditmysite löschen |
| 15 | `check_timing_with_page` (`timing_adjustable.rs`) | 2.2.1 / `meta-refresh`-id, UNTESTED, **unbedingt** | Nichts (`_page` ungenutzt) | n/a | Nein | nur am | besteht: `manual/timing`, `timing/meta-refresh` | `manual/timing` ist bedingt (Felder oder Auto-Refresh) | Host-Arbeit: in auditmysite löschen |
| 16 | `check_timeouts_with_page` (`timing_adjustable.rs`) | 2.2.6 (AAA) / `timeouts`, UNTESTED, unbedingt | Nichts | n/a | Nein | nur am | `manual/timeouts` | Kein Checklisten-Punkt für 2.2.6 vorhanden | gehört zu #114 |
| 17 | `check_visual_presentation_with_page` (`visual_presentation.rs`) | 1.4.8 (AAA) / `visual-presentation`, UNTESTED, unbedingt | Nichts | n/a | Nein | nur am | `manual/visual-presentation` | Blocksatz und Zeilenhöhe sind schon `text/justified`, `text/line-height-tight`; Farbwahl und Spaltenbreite bleiben manuell; kein Checklisten-Punkt für 1.4.8 | gehört zu #114 |
| 18 | `check_media_alternative_with_page` (`media_alternative.rs`) | 1.2.8 (AAA) / `media-alt`, immer eine UNTESTED-Notiz | Nur DOM: `video`, `iframe[src]`, Links mit Transkript-Stichwort (de/en) im Umkreis von 4 Vorfahren | n/a (Tier 1) | Nein | nur am | `manual/media-alternatives` bekommt 1.2.8 | Der Transkript-Hinweis ist eine Wortliste (Grundsatzfrage s. u.) | gehört zu #114 |

## Wo die Seite wirklich gebraucht wird

Nach dem, was der Host tun muss:

- **Seitenzustand ändern, dann messen:** Zeilen 1 (Viewport), 2 (Stylesheet), 13 (Anzeigemodus).
- **Auf die Seite einwirken und die Reaktion beobachten:** Zeilen 4 und 5 (Fokus, Scroll), 10 (synthetisches Einfügen).
- **Laufzeitzustand nur lesen:** Zeilen 3 (berechnetes `transform`), 7 und 8 (Frame-Erreichbarkeit), 11 (Funktionsquelle), 12 (Popover offen).
- **E/A oder Hook vor der Navigation:** Zeile 6 (HTTP-Probe), Zeile 13 (`init-missing`).
- **Nichts:** Zeilen 9, 14 bis 18. Sie stehen nur deshalb in einer Seitenfunktion, weil `PAGE_RULES` eine einzige Signatur hat.

Der Mutationsdurchgang (Spalte „Mutiert") ist eine Entscheidung des Hosts, nie der Regel. Die
Regel meldet bei fehlender Messung `NotRun` mit Grund, nie ein stilles Bestehen.

## Was die Annahme aus #127 einschränkt

1. **Zeilen 1, 2, 13 brauchen Varianten, keine Momentaufnahmen.** Der Snapshot entsteht, nachdem der Host die Seite verändert hat. Reflow braucht den Viewport als Teil der Daten; Textabstand ein Vorher/Nachher-Paar, das `Rendering` heute nicht ausdrücken kann.
2. **Zeilen 4 und 5: Das Urteil hängt vom Scrollzustand ab, den der Host selbst auslöst.** Statische Geometrie sieht den Fall nicht, um den es dem Kriterium geht (ein Element, das unter eine klebende Kopfzeile scrollt). Nur die Beobachtung nach dem Fokussieren ist eine gültige Eingabe.
3. **Zeile 13 `init-missing` kann ein In-Page-Host nie liefern**; nur CDP oder ein Content-Script bei `document_start`.
4. **Zeile 6 braucht E/A.** Eine Bibliothek tut das nicht; der Host müsste Status und Content-Type liefern, bisher tut das nur ein Host.
5. **liveaudit kann den Viewport nicht setzen.** Der zweite Host für Reflow ist das Agent-Plugin, nicht liveaudit.
6. **Bei mehreren Zeilen ist die Regel über die Daten ein einziges Prädikat** (3, 7, 10, die Differenz in 2). Geteilt werden id, `untested`/`NotRun`-Semantik und Datenvertrag, nicht Logik. Das ist der Zweck der Verschiebung, aber die Regelrümpfe bleiben klein.
7. **Drei Zeilen sind Wortlisten-Heuristiken** (9, Namenshinweis in 11, 18). `a11y-rules` dokumentiert seine Regeln als wortlautunabhängig; das widerspricht dem. Eine Maintainer-Entscheidung, hier nicht getroffen.
8. **Die Zahl 17 in #127/#130 überschätzt die Abhängigkeit:** sechs Einstiegspunkte (9, 14 bis 18) brauchen nichts von der Seite.

## Reihenfolge

Regel: Eine Regel wandert, wenn zwei Hosts sie brauchen (auditmysite plus liveaudit oder das
Agent-Plugin), wie in #13. „Zwei Hosts" heißt hier: auditmysite und ein Host, der die Daten
heute liefern kann. Billigstes und am meisten Geteiltes zuerst.

**Welle 0: keine neuen Daten, keine neue Regel in barrierlab.**
In auditmysite die unbedingten Notizen löschen, sobald es die geteilte Checkliste fährt
(Zeilen 14, 15). Zuerst Checklisten-Punkte ergänzen für Zeilen 16 (2.2.6), 17 (1.4.8) und 18
(1.2.8), wie die bestehenden AAA-Punkte 2.2.3, 2.2.4, 3.3.5: das ist #114. Zeile 8 ist
Host-Arbeit in auditmysite.

**Welle 1: Issues liegen vor, zwei Hosts liefern heute.**

1. `frames/cross-origin` (Zeile 7, #132): S. Beide Hosts kennen die Erreichbarkeit schon.
2. `auth/paste-blocked` (Zeile 10, #133): S. In-Page und CDP können dieselbe Probe auslösen.
3. `orientation/root-rotated` (Zeile 3, #134): S. Zwei Zeichenketten auf zwei Elementen.
4. `focus/obscured` und `focus/obscured-enhanced` (Zeilen 4 und 5, #135): M. Erweitert, was liveaudit schon misst.
5. `text-spacing/clipped` (Zeile 2, #136): M. Beide Hosts können beide Durchgänge in der Seite fahren.

**Welle 2: bedingt oder ohne Issue.**
Reflow (Zeile 1) erst, wenn es neben auditmysite einen Host mit Viewport-Steuerung gibt.
Funktionsquelle für `context/on-input` (Zeile 11), nach Welle 1. `popover_open` (Zeile 12),
gering im Wert, Namensnotiz streichen.

**Bleibt in auditmysite:** HTTP-Probe für Untertitel (6), Anzeigemodi (13), die Wortlisten
(9, Namenshinweis in 11, Transkript in 18) bis zur Grundsatzentscheidung, der Fokuslauf und
das Steuern des Viewports selbst (immer Sache des Hosts).

## Nicht ermittelt

- Ob der Scroll von Chrome bei `focus()` oder Tab dem `scrollIntoView({block:'nearest'})` aus auditmysites Lauf entspricht.
- Ob liveaudit unter strengem `style-src` das Textabstands-Stylesheet einspritzen kann (ein konstruierbares Stylesheet könnte es umgehen; nicht geprüft). Die Regel muss dann `NotRun` mit Grund melden.
- Der genaue ARIA-1.2-Wortlaut zu Namen für `menu`/Popover (Zeile 12 stützt sich auf den Kommentar in auditmysite).
- Ob `audit::frames` in auditmysite versteckte Frames genauso filtert wie der Scan in Zeile 8 (Datei nicht gelesen).
- Ob die Fixtures aus auditmysites `tests/fixtures/detection_corpus/` als „echter Korpus" im Sinn der Definition of Done (Punkt 3) gelten; sie sind der Beleg je Zeile.

## Neue Issues (Entwürfe, noch nicht angelegt)

**1. `a11y-rules: on-input source half (3.2.2) over a host-supplied handler source`** (Zeile 11)
Auditmysite liest für `onchange="name(...)"` den Quelltext von `window[name]` und meldet FAIL,
wenn die Funktion den Kontext wechselt; `context/on-input` in `a11y-rules` hat nur die
REVIEW-Hälfte. Der Host liefert je Element `handler_source() -> Option<String>` (nur lesen,
neue Spalte in `a11y-wasm`), die Regel prüft Muster darauf und meldet bei `None` auf allen
Kandidaten `NotRun`. Vorher klären, ob liveaudit es braucht (sonst bleibt es in auditmysite),
und ob die Muster über Funktionsquelltext mit der Wortlautunabhängigkeit vereinbar sind.

**2. `a11y-rules: reflow/overflow-x (1.4.10) over viewport-tagged snapshots`** (Zeile 1; zurückgestellt, anlegen, sobald das Agent-Plugin Viewports setzen kann)
Auditmysite setzt 320 x 256, vergleicht `scrollWidth` mit `innerWidth` und prüft die Ränder aller
Elemente gegen die Ausnahmen der Norm; `reflow/min-width` ist nur ein statischer Stellvertreter. Der
Host liefert Viewport und Dokument-Scrollbreite am Snapshot und `bounds` der Elemente jenseits des
Randes; die Regel meldet `NotRun` mit der gemessenen Breite, wenn der Snapshot nicht 320 CSS-px breit
ist. Offen: Höhenachse 256 px, Ausnahmen für zweidimensionalen Inhalt als Daten oder als Regel.

**3. `Decision: wording-dependent heuristics in a11y-rules (page title, handler name hints, transcript links)`** (Zeilen 9, 11 Namenshinweis, 18)
Drei auditmysite-Prüfungen sind Wortlisten (`untitled`/`home`/`index`, `language`/`go`/`absenden`,
Transkript-Stichworte); `a11y-rules` dokumentiert seine Regeln dagegen als wortlautunabhängig. Zu
entscheiden ist, ob solche Listen als klar gekennzeichnete REVIEW-Heuristiken mit Sprachprofil
aufgenommen werden oder in den Hosts bleiben. Bis dahin wandert keine der drei Prüfungen.

Zeilen 16, 17 und 18 (Checklisten-Punkte 2.2.6, 1.4.8, 1.2.8) gehören an #114 und brauchen kein
eigenes Issue.
