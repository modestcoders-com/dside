# AGENTS.md — DS·IDE

Diese Datei liest der Partner beim Start eines Turns, zusätzlich zu Persona und Gedächtnis.
Sie beschreibt, wie in diesem Repository gearbeitet wird. `.dside/rules.md` ist der stille
Fallback, falls diese Datei fehlt.

## Bauen und prüfen

- **pnpm 10** verwenden. Andere Majors haben inkompatible Store-Layouts; pnpm 12 bricht
  zusätzlich an `ERR_PNPM_IGNORED_BUILDS` (he-tree-vue).
- `pnpm build` — vue-tsc und Vite. Der Hauptchunk ist groß (Monaco + PDF + Office); die
  Chunk-Warnung ist bekannt. Der Build beginnt mit `scripts/check-i18n.mjs`: alle sechs
  Kataloge müssen dieselben Schlüssel haben, doppelte Schlüssel und deutsche Reste fallen auf.
- `cd src-tauri && cargo test --lib` — die Unit-Tests. Die `live_*`-Tests sind ignoriert und
  brauchen `DS_IDE_API_KEY`.
- Windows-Release: `cargo build --release --features tauri/custom-protocol`. Ohne das
  Feature lädt die Binary die Dev-URL `http://localhost:1420`.
- Entwicklung: `pnpm tauri dev` (cargo muss im PATH sein), Vite läuft auf Port 1420.
- **UI-Tests:** `DS_IDE_TEST_BRIDGE=1 pnpm tauri dev` öffnet in Debug-Builds eine
  localhost-Brücke (127.0.0.1:9333, Port über `DS_IDE_TEST_BRIDGE_PORT`);
  `node scripts/ui-test.mjs eval 'report({...})'` führt DOM-Schritte aus, `shot` macht
  ein Fensterfoto. Der Release-Build enthält den Server nicht.

## Konventionen

- **PROGRESS.md ist die verbindliche Chronik.** Jede Runde bekommt einen Abschnitt mit
  Ursache, Änderung, Prüfung und dem, was offen bleibt. README ist die kurze Außenseite.
- **Keine Secrets ins Repo.** Der API-Key liegt im OS-Schlüsselbund; `secrets.json` ist
  nur der 0600-Fallback und gitignored.
- **Schreibzugriffe sind nachvollziehbar.** Vor `write`/`edit` wird der alte Stand in
  `file_versions` gesichert; der Datei-Tab zeigt Verlauf, Diff und Wiederherstellen.
- **Oberfläche in sechs Sprachen** pflegen: de, en, es, fr, ru, zh-Hans.
- **Tests statt Behauptungen.** Neue Engine-Logik bekommt einen Unit-Test; Windows-/
  macOS-spezifische Pfade werden nicht nur auf Linux geprüft.
- Referenz-Checkouts (minimax-code-main, kilocode-main, monaco-editor-0.57.0, Ubuntu/)
  bleiben lokal und sind gitignored.
- Lizenzhinweise: MIT; Ubuntu-Font unter UFL 1.0. Kein Code aus fremden Projekten — die
  Anordnung der Fenster ist an Agenten-IDEs angelehnt, nichts ist kopiert.
