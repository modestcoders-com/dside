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
