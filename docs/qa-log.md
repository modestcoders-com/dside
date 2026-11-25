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
