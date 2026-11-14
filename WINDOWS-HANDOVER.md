# Übergabe an die Windows-Instanz

Für den Begleiter, der auf dem Windows-Rechner läuft. Geschrieben am 14. November 2026 von der
Instanz auf dem Linux-Rechner, die das Projekt gebaut hat. Die Chronik ist `PROGRESS.md`
(deutsch, ein Abschnitt pro Runde); `AGENTS.md` beschreibt, wie hier gearbeitet wird.

## Die eine Regel

**Melden, nicht raten.** In diesem Projekt ist ein Plan kein Ergebnis und eine Vermutung kein
Befund. Alles, was in diesem Dokument unter „vermutet" steht, ist genau das — auch wenn es
plausibel klingt. Der Leser auf der anderen Seite hat schon zweimal von einer selbstsicheren
Fehldiagnose profitiert (ein angeblich fehlender Screenshot, der längst existierte; eine
Tauri-Option, die für einen ganz anderen Zweck gedacht war). Wenn du etwas nicht messen kannst:
sag, dass du es nicht gemessen hast.

## Was du bekommst

Zwei Stände in einem Ordner:

| Stand | Inhalt |
| --- | --- |
| **veröffentlicht: `v0.6.4`** (Tag, 8 Assets) | Kontextring, geplante Aufträge, `home_dir` für Windows, asynchrones Browserfenster |
| **uncommittet im Arbeitsbaum** | Einzelinstanz, Einstellungen: Lesefehler ≠ leer, sofortiges Schreiben, Logzeilen — siehe unten |

**Wichtig:** Die uncommitteten Änderungen existieren nur als Dateien im Arbeitsbaum, nicht als
Commit. Wenn dieser Ordner als Kopie kam, sind sie da. Wenn du stattdessen von GitHub klonst,
fehlen sie — und damit alles, was in dieser Runde behoben wurde.

**Das ist bereits einmal passiert** (16. November, gemeldet von der Windows-Seite): der hinüber
kopierte Ordner war der Stand von `v0.6.4` ohne die uncommitteten Dateien — kein `.git`, kein
`tauri-plugin-single-instance`, keine `settings: read`-Zeile. Die Prüfungen liefen damit am
falschen Stand. Prüfe es **vor** allem anderen so:

```sh
git status --porcelain     # leer = veröffentlichter Stand, die Runde fehlt
grep -c single-instance src-tauri/Cargo.toml   # 0 = falscher Stand
grep -c flushSettings src/stores/settings.ts   # 0 = falscher Stand
```

Steht dort 0, ist die Kopie unvollständig: dann entweder den Arbeitsbaum erneut kopieren, oder
nur die Prüfungen machen, die zu `v0.6.4` passen (die Browser-Frage ist dort schon beantwortet,
siehe unten).

Was uncommittet ist (Datei → was sie tut):

- `src-tauri/Cargo.toml`, `Cargo.lock` — `tauri-plugin-single-instance`
- `src-tauri/src/lib.rs` — das Plugin als *erstes* in der Builder-Kette; ein zweiter Start holt
  das vorhandene Fenster nach vorn; beim Start eine `data:`-Zeile mit dem Datenordner
- `src/stores/settings.ts` — Lesefehler wird nicht mehr als leere Datenbank behandelt (fünf
  Versuche, dann Hinweis statt Willkommensdialog, und in diesem Fall wird nichts geschrieben);
  `flushSettings()` für sofortiges Schreiben; Schreiben beim Verstecken/Schließen; Logzeilen
- `src/components/FirstRunDialog.vue` — schreibt, bevor der Dialog verschwindet
- `src/App.vue` — Hinweisbanner, wenn die Einstellungen nicht lesbar waren
- `src/i18n/*.ts` — ein Schlüssel mehr (`settings.unreadable`)

## Was belegt ist und was nicht

| Aussage | Stand |
| --- | --- |
| Auf Windows 0.6.4 zeigt das Arbeitsverzeichnis `C:\Users\<Name>` statt `.` | **belegt** — durch ein Bildschirmfoto aus dem Alltag |
| Kategorien im Paket, `X-AppImage-Version`, Update-Kette über `.zsync` | belegt, aber Linux/CI |
| Das Browserfenster in 0.6.4 rendert auf Windows und lässt sich schließen | **unbekannt** |
| Der Willkommensdialog erscheint in 0.6.5 nicht mehr bei jedem Start | **unbekannt** — die Änderung schließt die Lücken, die zu den Symptomen passen; gemessen wurde nichts |
| Erscheinungsbild und Schriftgröße bleiben erhalten | **unbekannt**, gleiche Ursache wie oben |

Der Grund für die letzte Zeile: Es überlebte genau **eine** Einstellung, nämlich der API-Key. Der
liegt aber gar nicht in der `settings`-Tabelle, sondern im OS-Schlüsselbund (`secrets.json` als
Rückfall). Alles, was fehlte, stammt aus der `settings`-Tabelle — also ist dort entweder nichts
geschrieben worden oder das Lesen scheiterte.

## Dein Auftrag: vier Prüfungen, in dieser Reihenfolge

Nach jeder Prüfung notierst du, was du **gesehen** hast, nicht was du erwartet hast.

### 1. Was steht im Log beim Start?

Arbeitsbereich → **Logs**. Die erste Zeile nennt den Datenordner:

```
data: C:\Users\<Name>\AppData\Roaming\com.deepvibe.ds-ide (windows)
```

Dort liegen `ds-ide.sqlite3` (Einstellungen, Verlauf) und `secrets.json` (Rückfall für den
API-Key). Danach kommt die Zeile, die mit `ui: settings: read` beginnt:

```
ui: settings: read 32 keys, firstRun=false, appearance=dark, fontSize=13
```

- **`read 0 keys`** → die Tabelle ist wirklich leer, das Schreiben scheitert. Im selben Log muss
  dann eine Zeile `settings: writing <key> failed: …` stehen — dieses `…` ist der Befund.
- **`read 25 keys, firstRun=true`** → es wird gelesen, der Dialog hat eine andere Ursache.
- **`settings: could not be read after twelve attempts: …`** und ein rotes Banner oben → genau das
  war die Lücke; der Text nach dem Doppelpunkt ist der Befund.

### 2. Bleiben Erscheinungsbild, Schriftgröße und Arbeitsverzeichnis?

Ändere alle drei sichtbar (hell/dunkel, Schriftgröße, Ordner). Dann **vollständig beenden**:
Tray-Symbol → Beenden, nicht nur das Fenster schließen. Neu starten.

Danach dieselbe Logzeile lesen: stehen `appearance=…` und `fontSize=…` mit deinen Werten?

### 3. Zweiter Start bei laufender App

App läuft. Einmal über das Startmenü erneut starten. Erwartet: das vorhandene Fenster kommt nach
vorn, die Zahl der Symbole in der Taskleiste bleibt **1**, und im Log steht
`instance: second start, raising the existing window`. Vorher waren es bis zu sieben Symbole.

### 4. Browserfenster

Menü (unten links) → **Nutzung**. Erwartet: ein zweites Fenster mit der Seite. Dann das „X" dieses
Fensters und danach Tray → Beenden.

**Diese Prüfung ist bereits beantwortet** (am 0.6.4-Paket gemessen): das Fenster wird gebaut, die
Seite rendert vollständig, das X schließt es, und im Log stehen
`browser: opening browser-1 for …` und `browser: browser-1 built for …`. Nur wiederholen, wenn sich
am Browser-Code etwas geändert hat.

Bleibt es weiß: Logs öffnen und nach `browser:` suchen.

- `browser: opening browser-1 for …` **und** `browser: browser-1 built for …` vorhanden → das
  Fenster wurde gebaut, die Seite lädt nicht. Dann ist die Netzwerk-/WebView2-Seite dran.
- nur die erste Zeile → `build()` scheiterte, und die Fehlermeldung steht darunter.
- gar keine Zeile → der Aufruf kam nicht an.

## Bauen auf Windows

Nur nötig, wenn du selbst bauen willst. Für die Prüfungen oben genügt das installierte Paket —
allerdings hat ein **Release-Build keine Testbrücke**, also nur Logs und Bilder, keine
DOM-Messungen.

Voraussetzungen: Rust (stabiles Toolchain mit MSVC), Node 22, **pnpm 10**, WebView2 (Windows 11
bringt es mit), MSVC Build Tools.

```powershell
pnpm install --frozen-lockfile
pnpm build                        # i18n-Prüfung, vue-tsc, Vite
cd src-tauri; cargo test --lib    # 95 Tests, 14 ignoriert
cd ..; pnpm tauri build           # NSIS-Installer
```

Ein direktes `cargo build --release` braucht `--features tauri/custom-protocol`, sonst lädt die
Binärdatei `http://localhost:1420` statt der eingebetteten Oberfläche. Die Tauri-CLI setzt das
selbst.

Debug-Build mit Testbrücke (nur wenn du sie brauchst):

```powershell
$env:DS_IDE_TEST_BRIDGE=1; $env:DS_IDE_TEST_BRIDGE_PORT=9333
pnpm tauri dev
```

Isolierte Daten für Tests: den Ordner aus der `data:`-Zeile vorher kopieren und `APPDATA` auf einen
Wegwerf-Ordner zeigen lassen. Ob es funktioniert hat, sagt die Logzeile aus Prüfung 1:
`firstRun=true` heißt, die App sieht eine leere Installation. Die echte Datenbank nie zum Testen
benutzen.

## Fallen

- **Nicht committen, nicht pushen, nicht taggen.** Das entscheidet der Mensch, und zwar erst, wenn
  er die Runde für vollständig hält. Beheben ist jederzeit erlaubt.
- **`PROGRESS.md` ist öffentlich.** Technische Sachverhalte, keine Kundennamen, keine Pfade aus
  dem Rechner des Autors (`C:\Users\<Name>` genügt), nichts über Personen. `pnpm check:private`
  prüft die Chronik.
- **Fremde Ordner nicht anfassen.** Der Mensch hat weitere Projekte auf dieser Maschine; sie sind
  nicht dein Auftrag, auch wenn du sie lesen könntest.
- **Eine Instanz jetzt.** Zum Beenden `taskkill /IM DSIDE.exe` (ohne `/F` zuerst). Zwei Instanzen
  teilen dieselbe SQLite-Datei — das war früher erlaubt und ist jetzt nicht mehr vorgesehen.
- **Der Log ist ein Ring von 400 Zeilen.** Was weiter zurückliegt, ist weg; also nach dem Fehler
  nachsehen, nicht am Abend.
- **Sprache der Oberfläche:** sechs Kataloge (de, en, es, fr, ru, zh-Hans), alle mit denselben
  Schlüsseln; `pnpm check:i18n` bricht bei Abweichung.

## Was zurückkommen soll

1. Die Logzeilen wörtlich — besonders `settings: read …` und alles mit `settings: writing …`.
2. Zu jeder der vier Prüfungen: beobachtet oder nicht beobachtet, und was stattdessen passierte.
3. Wenn du etwas änderst: was du geändert hast, in welcher Datei, und **womit** du es geprüft hast.

Was du nicht herausfinden kannst, schreibst du als offene Frage. Eine offene Frage ist ein
Ergebnis; eine erfundene Antwort ist keins.

Willkommen. Grüße von der Instanz auf der anderen Seite.
