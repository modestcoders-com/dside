# QA-Log

Routine-Prüfungen der vorhandenen Funktionen, ein Punkt pro Lauf, in einem Debug-Build mit einer
Kopie der Datenbank in einem eigenen Datenverzeichnis. Die echte Datenbank und der Schlüsselbund
werden nicht angefasst; es gibt keine Modellaufrufe.

Ein Eintrag nennt nur, was beobachtet wurde. Wo etwas nicht geprüft werden konnte, steht das da.

---

## 2026-11-25 22:00 — Punkt 1: Sitzung archivieren und wiederherstellen — bestanden

Ausgangslage in der Kopie: drei aktive Sitzungen, eine bereits archivierte. Die Seitenleiste zeigte
drei Einträge.

- **Archivieren** über das Kontextmenü der Sitzung (Einträge: Anheften, In Projekt verschieben,
  Umbenennen, Archivieren): Der Eintrag verschwand sofort aus der Seitenleiste (3 → 2), und in der
  Datenbank stand `archived = 1` für genau diese Sitzung.
- **Wiederherstellen** unter Einstellungen → Archivierte Aufgaben: Der Eintrag erschien in der
  Liste mit Datum und den Knöpfen „Reaktivieren" und „Löschen". Nach „Reaktivieren" stand
  `archived = 0` in der Datenbank, und die Seitenleiste zeigte die Sitzung wieder (2 → 3), **ohne
  Neustart** — der Fehler von früher, dass die Seitenleiste ihre eigene Kopie behielt, ist also
  weiterhin behoben.
- **Randfall, aktive Sitzung archivieren:** Die Seitenleiste fiel auf 2 Einträge, die Markierung
  wanderte auf eine andere Sitzung, der Titel in der Kopfzeile wechselte mit, und der Verlauf der
  neu aktiven Sitzung wurde geladen (121 Nachrichtenknoten). Kein leerer Bildschirm, kein toter
  Composer.
- Die vorbestehend archivierte Sitzung blieb unberührt (`archived = 1`).

Anmerkung, kein Fehler: Archivieren und Wiederherstellen schreiben keine Zeile ins Anwendungslog.
Für die Fehlersuche wäre eine Zeile nützlich gewesen, aber es gibt keine Anforderung, die das
verlangt — deshalb nur diese Notiz, keine Änderung.

## 2026-11-25 23:00 — Punkt 2: Suche im Verlauf — bestanden, mit einem Fund

Gesucht wurde ein Begriff, der in der Kopie genau **einmal** vorkommt, und zwar in einer alten,
archivierten Sitzung: `ALLER-ALLER-ERSTEN` (1 Nachricht, aus Mitte Oktober).

- Die Suche fand genau einen Treffer, Abzeichen „Nachricht", Sitzungstitel „Unheimliche Begegnung
  der vierten Art", und der Ausschnitt war um den Begriff zentriert:
  „…in DEINER (Deepseeks) ALLER-ALLER-ERSTEN EIGENEN Agentic IDE."
- Auch Titeltreffer funktionieren (Abzeichen „Sitzung"), geprüft mit dem Projektnamen.
- Ein Klick auf einen Treffer schließt den Dialog und lädt den Verlauf der Sitzung (die erste
  Nachricht „Hallo! Ist jemand zuhause?" stand danach im Chat).
- Die Suche ändert den Archiv-Zustand nicht: die archivierte Sitzung blieb `archived = 1`.

**Fund (in `TODO.md` unter „Found by QA", nicht behoben):** Ein Treffer in einer **archivierten**
Sitzung öffnet den Verlauf, aber die Kopfzeile bleibt leer und die Seitenleiste zeigt keinen
aktiven Eintrag. Ursache: `sessions_list` liefert nur nicht-archivierte Sitzungen, und der Titel
leitet sich aus dieser Liste ab; die Suche filtert nicht nach dem Archiv-Zustand. Gegenprobe mit
einer nicht-archivierten Sitzung: Titel und Markierung stimmen dort.

Bewusst nicht behoben: Die Frage dahinter ist eine Entscheidung, keine Nachlässigkeit — sollen
archivierte Sitzungen überhaupt in Treffern erscheinen (dann: nur lesen, oder beim Öffnen
reaktivieren), oder gar nicht (dann verliert das Archiv seinen Sinn). Außerdem ist es reine
Frontend-Logik, und dieses Projekt hat keinen Testläufer für das Frontend; ein Rust-Test würde
die Stelle nicht abdecken.

## 2026-11-26 00:00 — Punkt 3: Erinnerungen anlegen, stilllegen, wiederherstellen — bestanden

Ausgangslage in der Kopie: sieben aktive Einträge, keiner stillgelegt.

- **Anlegen** über das Formular im Panel (Titel + Text): Der Zähler zeigte `14/120` passend zum
  Titel, „Hinzufügen" wurde aktiv, der Eintrag erschien sofort in der Liste, und das Formular leerte
  sich. In der Datenbank: `id 11, origin = user, class = work, retired_at IS NULL`.
- **Stilllegen** über den Knopf am Eintrag (Titel „Stilllegen"): Der Eintrag verschwand aus der
  aktiven Liste und stand danach unter „Stillgelegt (1)"; `retired_at` ist gesetzt.
- **Wiederherstellen** nach Aufklappen der Rubrik (Knopf „Zurückholen"): Der Eintrag war wieder in
  der aktiven Liste, die Rubrik verschwand (`Stillgelegt (0)` heißt: die Überschrift wird nicht mehr
  gerendert, weil `v-if="visibleRetired.length"`), und `retired_at` ist wieder `NULL`.
- **Panel-Wechsel** (Erinnerungen → Richtlinien → Erinnerungen): Der Eintrag ist weiterhin da, keine
  stillgelegten. Das Panel lädt beim Einhängen neu aus der Datenbank (`onMounted → load`), es gibt
  also keinen zweiten Zustand, der auseinanderlaufen könnte.
- **Neustart der App** auf denselben Daten: Der Eintrag ist weiterhin aktiv und nicht stillgelegt.

Dass ein stillgelegter Eintrag auch nicht mehr erinnert wird, deckt der Unit-Test
`retiring_hides_a_memory_and_restoring_brings_it_back` ab: nach dem Stilllegen ist die aktive Liste
leer, der Eintrag liegt in der stillgelegten Liste, `memory_report` zählt ihn nicht, und
Wiederherstellen bringt ihn zurück.

Kein Fehler gefunden, nichts geändert.

Aufräum-Notiz für mich selbst: Nach `dbus-run-session`-Läufen bleiben Portal-Prozesse
(`xdg-desktop-portal`, `-kde`, `ksecretd`, `dbus-daemon`) mit dem Test-Datenverzeichnis in der
Umgebung zurück und legen `data/kwalletd` immer wieder an — das Verzeichnis lässt sich erst löschen,
wenn diese Prozesse beendet sind. `pkill -f` traf dabei auch die eigene Shell, weil deren
Befehlszeile den Suchbegriff enthielt; richtig ist, die PIDs über `XDG_DATA_HOME` im
`/proc/<pid>/environ` zu bestimmen und einzeln zu beenden.

## 2026-11-26 01:00 — Punkt 4: Git-Panel (Status, Diff, Stage, Commit) — bestanden

Wegwerf-Repository unter `/tmp` mit einem Commit, einer geänderten und einer unversionierten Datei.
Die App lief mit einer Kopie der Datenbank in einem eigenen Datenverzeichnis.

- **Status**: `GIT · 7de3911`, dann „NICHT GESTAGED 1 · M a.txt" und „UNVERSIONIERT 1 · ?? b.txt" —
  dieselben zwei Einträge, die `git status --short` im Repository zeigt, samt Kurz-Hash des HEAD.
- **Diff**: Auswahl von `a.txt` zeigte den vollständigen Patch (`-line two` / `+line two changed`);
  Abgleich mit `git diff` stimmt zeichenweise.
- **Stage**: „Alles stagen" verschob beide Einträge nach „GESTAGED 2", `git status` im Repository
  zeigte `M  a.txt` und `A  b.txt`. Einzelnes „Stagen" und „Unstagen" funktionieren ebenfalls.
- **Commit**: Nachricht im Feld `Commit-Nachricht`, „Committen" schrieb `[main 202910d] QA: commit
  aus dem Panel, 2 files changed, 2 insertions(+), 1 deletion(-), create mode 100644 b.txt`, das
  Feld leerte sich, der Arbeitsbaum war danach sauber — bestätigt mit `git log`/`git status`.
- **Nach einer externen Änderung** zeigt ein erneutes Öffnen des Tabs den neuen Stand (`M a.txt`);
  der Knopf „Aktualisieren" lädt ebenfalls neu.
- **Ordner ohne Repository**: Der Ordner hatte kein `.git`; das Panel zeigte **„Kein
  Git-Repository angelegt."** Die Backend-Meldung ist der Code `no_git_repository`, den die
  Oberfläche in den übersetzten Satz übersetzt. Ein Unterordner eines Repositories verhält sich
  genauso (kein Repository „von oben" erben).

Kein Fehler gefunden, nichts geändert.

Aufräum-Notiz: Beim Wechsel zwischen Testläufen überlebte eine DSIDE-Instanz das Beenden und hielt
den Brücken-Port; die nächste Instanz konnte ihn nicht binden (`test bridge: cannot bind … os error
98`) und ich habe dann die *alte* Instanz abgefragt — das sah nach einem falschen Workspace aus, war
aber mein Testfehler. Prüfregel: vor jedem Start `ss -tlnp | grep <port>` und `pgrep -x DSIDE`.

## 2026-11-26 02:00 — Punkt 5: Vorschau (CSS, Bild, PDF, fehlendes Asset) — bestanden, mit einem Fix

Testseite im isolierten Workspace: `index.html` mit relativem Stylesheet, einem PNG, einem
verlinkten PDF **und** einem absoluten Skriptpfad `/js/app.js`, der bewusst ins Leere zeigt.
Vorschau-Server über den echten Befehl `preview_start` gestartet.

- `/` und `/index.html` → `200 text/html`, 330 Bytes.
- `/assets/style.css` → `200 text/css`, 104 Bytes; mit `/usr/bin/cmp` byte-gleich mit der Datei.
- `/assets/logo.png` → `200 image/png`, 77 Bytes; byte-gleich.
- `/doc.pdf` → `200`, 193 Bytes; byte-gleich.
- `/js/app.js` → `404`. Die Seite fragt es an, der Server hat es nicht — genau der Fall, für den
  die Warnung gebaut wurde.
- Pfad-Ausbruch (`/../../etc/passwd` und die kodierte Form) → `404`, nichts wird ausgeliefert.
- Im echten Browser-Fenster gelesen: `title "QA-Vorschau"`, `loading: false`, Text „Vorschau-Test
  PDF", und die Asset-Liste enthält **genau** die drei Anfragen der Seite, inklusive des fehlenden
  `/js/app.js` (Typ `script`). `missing_assets` daraus ist die Liste, die in der Warnung steht; der
  Unit-Test `assets_beyond_the_root_are_reported_as_missing` deckt den Wortlaut-Pfad ab.

**Fund und behoben:** `/doc.pdf` kam als `application/octet-stream` — `mime_for` kannte `pdf` nicht.
Eine Seite, die ein PDF einbettet (`<embed src="doc.pdf">`), wird dadurch heruntergeladen statt
angezeigt. Ein Zweig `"pdf" => "application/pdf"` ergänzt, der Test `the_mime_keeps_pages_and_styles_typed`
prüft jetzt auch `doc.pdf` und `DOC.PDF` (Großschreibung), im neu gebauten Paket liefert der Server
`application/pdf` mit unveränderten 193 Bytes.

Was dieser Lauf **nicht** zeigt: den fertigen Warntext, den der Begleiter im Werkzeugergebnis liest.
Der entsteht im `preview`-Werkzeug, und das braucht einen Modellzug. Belegt sind die Zutaten: die
angefragten Pfade, die fehlende Datei, ihr 404 und die Funktion, die sie benennt.

## 2026-11-26 03:00 — Punkt 6: Terminal-Panel — bestanden

Debug-Build, Kopie der Datenbank, eigener Arbeitsordner mit einer Datei `marker.txt`.

- **Öffnen**: Panel erscheint, die Shell startet **im Arbeitsordner** — die Prompt-Zeile zeigt
  `<workspace>`, ebenso die Statuszeile unter dem Terminal.
- **Befehl und Ausgabe**: `cat marker.txt` → `hello-from-the-workspace`, wie die Datei.
- **Echte Tastatureingabe**: Text über das Eingabefeld von xterm getippt (`echo typed-by-input`),
  die Zeichen erscheinen in der Shell, Enter führt sie aus, Ausgabe `typed-by-input`.
- **Prozessende**: `exit` beendet die Shell, das Panel schreibt „[Prozess beendet.]" und lässt den
  bisherigen Verlauf stehen.
- **Neu starten**: neue Shell, der Beendet-Hinweis verschwindet, `pwd` antwortet mit dem
  Arbeitsordner — der Neustart erbt also das Arbeitsverzeichnis.
- **Aufräumen beim Beenden der App**: `sleep 300` im Terminal gestartet, danach die App mit
  `SIGTERM` beendet — der `sleep`-Prozess war mitgegangen (keine Waise übrig).

Kein Fehler gefunden, nichts geändert.

## 2026-11-26 04:00 — Punkt 7: Projektwechsel und relative Pfade — bestanden

Zwei Projektordner unter einer Projektbasis, Kopie der Datenbank, App auf Projekt A gestartet.

- **Projektwechsel über die Seitenleiste**: Projekt B gewählt → Arbeitsordner der App wechselt auf
  B (`settings.workspace` in der Datenbank), und weil es in B noch keinen Thread gab, legt die App
  einen neuen dort an. Die vier alten Threads bleiben in A. Das ist die dokumentierte Absicht
  („ein Projektwechsel öffnet den neuesten Thread des Projekts, oder startet einen").
- **Aktiver Thread wird neu gebunden**: Über Einstellungen → Allgemein → Aktives Projekt auf A
  umgestellt, während der offene Thread in B lag. Danach: `settings.workspace` = A **und** die
  Sitzungszeile des offenen Threads = A. Beides zusammen, wie gefordert.
- **Relativer Pfad in der Sitzung**: `workspace = '.'` in der aktiven Sitzung eingetragen, neu
  gestartet. Logzeile: `ui: session <id>: replaced workspace "." with "<workspace>"`, danach steht
  der absolute Pfad in der Datenbank. Der Thread wird repariert, nicht verworfen.
- **Relativer Pfad in den Einstellungen**: `workspace = '.'` gespeichert, neu gestartet. Danach
  steht dort der Ordner des offenen Threads (A) — ein Punkt aus den Einstellungen überlebt keinen
  Start, und der offene Thread entscheidet, wie im Startcode kommentiert.

**Beobachtung, die ich hier nicht messen kann (nach `TODO.md` unter „Found by QA"):**
`samePath` in `src/lib/projects.ts` vergleicht Pfade ohne Rücksicht auf Groß-/Kleinschreibung, und
`groupByProject` gruppiert nach dem rohen Pfad. Windows-Pfade sind aber nicht case-sensitiv; zwei
Schreibweisen desselben Ordners (`C:\Users\…\Proj`, `c:\users\…\proj`) würden dort als zwei
verschiedene Projekte gelten. Im selben Repo macht `plugins::path_key` es richtig (Kleinschreibung
auf Windows) — zwei Konventionen nebeneinander. Auf Linux nicht prüfbar; deshalb als Beobachtung
notiert, nicht als Befund.

Testfehler meinerseits, der Zeit gekostet hat: Es gibt **zwei** Ordner-Dialoge (Arbeitsordner und
Projektbasis) mit demselben Knopf-Titel „Ordner wählen". Ich hatte zuerst die Projektbasis geöffnet
und mich über „der Wechsel wirkt nicht" gewundert — die Basis ist bewusst nur eine Listenquelle. Der
Arbeitsordner-Dialog ist der, dessen Knopf einen Schwester-Knopf „Ordner neu laden und den
geöffneten Thread daran binden" hat.

Aufräum-Notiz zu diesem Lauf: Zwei leere Verzeichnisse aus den Läufen von 22:00 und 23:00
(`/tmp/…/data/kwalletd`) kommen wieder, sobald man sie löscht, obwohl kein Prozess sie offen hält
(alle `/proc/*/fd` durchsucht). Sie sind 0 Byte groß und verschwinden mit dem nächsten Neustart des
Rechners; seine eigene `kwalletd`-Instanz ist es nicht (deren Arbeitsverzeichnis ist sein
Home-Verzeichnis). Ich lasse sie stehen, statt an seinen Portal-Diensten zu drehen.
