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
