# DS.IDE — Bauplan & Status

> Diese Datei ist der **Restart-Anker nach Kontextkompression**. Sie ist die Wahrheit,
> nicht der Chatverlauf. Nach jeder abgeschlossenen Phase aktualisieren.
> Erstellt: 2026-10-15

## 1. Auftrag

Eine eigene **agentic IDE** mit Tauri (kein Electron) nach Vorlage von **MiniMax Code**.
Erlaubnis vom Nutzer erteilt (2026-10-15, 07:55). Name: **DS.IDE** / "DeepSeek Code".
Arbeitsverzeichnis: `~/Projects/DS.IDE`

Nutzervorgaben (verbindlich):
- **Kein Electron** — Tauri v2.
- **i18n ab dem ersten String** — kein hartkodierter UI-Text. Mindestens `de` + `en`.
- **Kein eingebauter Editor zum Editieren.** Der Code-Editor ist **extern**
  (VS Code / Code OSS / VSCodium / Zed / Cursor / Xcode), per Dropdown gewählt.
  → Wir bieten an: **Systemscan** nach installierten Editoren, manuellen Override,
  Download-Fallback, falls nichts installiert ist.
- Monaco wird **nur als read-only Viewer** im "View File"-Tab genutzt (kein Ersatz für die externe IDE).

## 2. Vorlage: Struktur aus den MiniMax-Code-Screenshots

```
┌──────────────────────────────────────────────────────────────────────┐
│ TitleBar: Projektname (Mitte)            [Editor▾] [🌐] [📁] [▣]     │
├───────────────┬──────────────────────────────┬───────────────────────┤
│ LeftRail      │ AgentConversation (Mitte)    │ WorkArea (rechts)     │
│ - New task    │  - MessageList               │  Tabs:                │
│ - Search      │  - Composer                  │   • Progress/Deliv.   │
│ - Plugins     │    (/Commands,               │   • View File         │
│ - Scheduled   │     Full Authorization,      │   • Browser           │
│ - Site        │     Model▾, Send)            │  (┗ ▣ togglet Panel)  │
│ - Connect Mob.│                              │                       │
│ - Remote ctrl │                              │                       │
│ ─ Pinned      │                              │                       │
│ ─ Scheduled   │                              │                       │
│ ─ Projects    │                              │                       │
│ ─ Agent Team  │                              │                       │
│ [Account][⚙]  │                              │                       │
└───────────────┴──────────────────────────────┴───────────────────────┘
```

Wichtige Lesarten (aus Screenshots bestätigt):
- **Agent-Chat ist die Mitte**, nicht eine Seitenleiste. Editor ist NICHT zentral.
- **Work Area** ist ein Tab-Panel rechts. Der **Datei-Baum lebt IM "View File"-Tab**
  (rechts, ein-/ausklappbar "Hide file tree") — NICHT als linke Spalte.
- Rechts oben drei Aktions-Icons: 🌐 = Browser-Tab öffnen, 📁 = View-File-Tab öffnen,
  ▣ = Work-Area-Panel ein-/ausblenden. Davor das Editor-Dropdown.
- "Progress/Deliverables" ist der Default-Tab der Work Area, gruppiert je Artefakt-Set
  (z.B. "Manuskript", "Mavis markdowns", "Skript: …").
- **Settings** ist eine eigene Seite (nicht Dialog): linke Settings-Nav + rechter Inhalt.
  Gruppen: Preferences (General, Keyboard shortcuts, Personalization),
  Management (Usage & model, Connection, Account), Coding (Code review, Worktree),
  Archived (Archived tasks).
  General-Inhalt: **Mode**-Karten ("For coding" / "For everyday work"), Appearance
  (Light/Dark/System), Show in Menu Bar, Launch on Startup, Desktop Notifications,
  Join Early Access.

## 3. Technische Entscheidungen (fixiert)

| Thema | Entscheidung |
|---|---|
| Shell | Tauri v2 + Vue 3 + TS + Vite 8 + pnpm 10 (bereits gescaffoldet) |
| Paketmanager | **Immer das Shell-pnpm (10.30.3)**. NICHT `/usr/bin/pnpm` (11.28.5) → sonst `ERR_PNPM_UNEXPECTED_STORE`. |
| i18n | Eigener **typisierter Katalog** (`t()` + `Leaves<Messages>`), kein vue-i18n. `de` ist Form-Referenz, `en: Messages` erzwingt gleiche Keys. Persistenz via localStorage. |
| Theme | CSS-Variablen auf `:root` + `[data-theme="light"]`, gesetzt via `document.documentElement.dataset.theme`. |
| Stores | Modul-Singletons mit `reactive`/`ref` + localStorage-Persistenz. Kein Pinia (Overkill). |
| Rust-Befehle | `detect_editors`, `open_workspace_in_editor(editor_id, workspace)`, `read_workspace_tree(root, depth)`, `read_text_file(path)`. |
| Sicherheit | Frontend sendet **niemals** eine Executable. Launch nur über **editor_id**, Rust löst die Binary aus eigener Detection auf. |
| Agent-Backend | **NICHT implementiert.** Konversation ist lokale Demo mit klarem "not wired"-Hinweis. Keine LLM-Calls. |
| Provider | Dropdown listet u.a. DeepSeek (Ziel) + MiniMax etc. Nur Auswahl, keine Anbindung. |
| Monaco | Read-only Viewer im View-File-Tab. External editor bleibt der Ort zum Editieren. |
| Terminal | `TerminalPanel.vue` bleibt im Repo, wird aber **nicht** gemountet (MiniMax hat keins). |

## 4. Datei-Landkarte (Ziel)

```
src/
  main.ts
  App.vue                     Komposition + Ansicht (chat|settings)
  i18n/
    index.ts                  t(), locale, setLocale, Leaves<Messages>
    de.ts                     Form-Referenz
    en.ts                     en: Messages
  types.ts                    FileNode, EditorCandidate, Session, ChatMessage, Deliverable, WorkAreaTab
  lib/
    language.ts               (bestehend) Endung -> Monaco-Sprache
    persist.ts                localStorage-Helfer
  stores/
    settings.ts               mode, appearance, toggles, pinnedEditorId, workspace
    sessions.ts               Sessions, Messages, Deliverables (Beispieldaten)
    workArea.ts               Tabs, aktiver Tab, Sichtbarkeit
  components/
    TitleBar.vue
    LeftRail.vue
    agent/AgentPanel.vue  agent/MessageList.vue  agent/Composer.vue
    workarea/WorkArea.vue  workarea/ProgressTab.vue
    workarea/ViewFileTab.vue  workarea/BrowserTab.vue
    settings/SettingsPage.vue
    FileTree.vue              (bestehend, an reale Daten angepasst)
    MonacoEditor.vue          (bestehend, + readOnly, Model pro Pfad)
    TerminalPanel.vue         (bestehend, ungenutzt)
  data/sampleProject.ts       (bestehend, nur noch Fallback)
src-tauri/
  src/lib.rs                  + Rust-Kommandos
  tauri.conf.json             productName DS.IDE
```

## 5. Status — STAND 2026-10-15, ~10:05 (alles unten ist erledigt)

- [x] Phase 0: PROGRESS.md, App-Identität (`productName`/Titel `DS.IDE`,
      Identifier `com.deepvibe.ds-ide`, Package `ds-ide`)
- [x] Phase 1: i18n (de/en, typisiert), Theme (CSS-Variablen, dark/light/system),
      Stores (settings/sessions/workArea/ui), Types
- [x] Phase 2: Rust — `detect_editors`, `open_workspace_in_editor`,
      `read_workspace_tree`, `read_text_file`, `default_workspace`
- [x] Phase 3: UI — TitleBar, ToolbarActions, LeftRail,
      AgentPanel (MessageList/Composer), WorkArea (Progress/ViewFile/Browser),
      SettingsPage, App
- [x] Phase 4: Build + Verifikation

### Verifikation (Evidenz)
- `pnpm build` (vue-tsc + vite) grün.
- `cargo check` grün; `cargo test detect_editors_smoke` grün.
- Editor-Erkennung real auf dieser Maschine: `code-oss /usr/bin/code-oss`,
  `codium /usr/bin/codium`, `cursor …/cursor.AppImage`, `zed zeditor`
  — 4 Einträge, dedupliziert (12 Rohtreffer → 4).
- `pnpm tauri dev`: Fenster startet, alle Modul-Requests HTTP 200,
  Webview optimiert `@tauri-apps/api/core` + `plugin-opener`.
- Das Flatpak-VSCodium ist inzwischen **nicht mehr installiert**
  (`flatpak list --app` zeigt es nicht mehr) → Detection ist korrekt.

### Bewusst NICHT implementiert (Stubs)
- **Kein Agent-Backend.** Der Composer antwortet mit `message.notWired`; keine LLM-Calls.
- Modell-Dropdown ist Auswahl ohne Wirkung; Provider (DeepSeek) nicht angebunden.
- Plugins, Scheduled, Site, Connect Mobile, Remote control sowie alle
  Settings-Unterseiten außer "Allgemein": ohne Funktion.
- Browser-Tab: nur UI ("Keine lokalen Server online").
- Menüleiste/Tray, Autostart, Desktop-Notifications: nur persistierte Flags, keine OS-Anbindung.

### Nächste sinnvolle Schritte
1. Agent-Backend anbinden (Sidecar-Muster wie `kilo serve`, HTTP+SSE) + Streaming im MessageList.
2. DeepSeek-Provider + Key-Eingabe in Settings → "Nutzung & Modell".
3. "Öffnen im Editor" verdrahten (`open_workspace_in_editor` existiert) + Download-Fallback prüfen.
4. Optional: Terminal wieder aufnehmen (`TerminalPanel.vue` liegt ungenutzt im Repo).

## 6. Offene Punkte / Risiken

- Detection liefert auf dieser Maschine VSCodium **mehrfach** (nativ `/opt/vscodium-bin`,
  Flatpak, zwei `.desktop`). Dedup + Ranking nötig.
- Zed/Cursor sind installiert, aber **nicht im PATH** → Detection muss `.desktop` lesen.
- macOS/Windows-Pfade in der Detection sind nur Stubs (wir bauen auf Linux).
- WebView-Divergenz (WebKitGTK vs Chromium) ist das spätere Migrationsrisiko für StageVibe.

## 7. Layout-Feinschliff (2026-10-15, nach Nutzer-Review)

Umgesetzt:
- **Sidebars resizeable.** `Resizer.vue` (absolut am Rand positioniert), Store `ui.ts`
  mit `railWidth`/`workWidth` in **% vom Body** + Persistenz (`ds-ide.ui`).
  Default **20 / 60 / 20**.
- **Grenzen:** `RAIL_MIN 12`, `RAIL_MAX 28`, `WORK_MIN 15`, `WORK_MAX 55`, `CENTER_MIN 30`.
- **Auto-Collapse:** wird die rechte Leiste so breit gezogen, dass die Mitte unter
  `CENTER_MIN` fiele, klappt die linke Leiste automatisch ein, um Platz zu schaffen.
- **Nur noch EIN Sidebar-Toggle** — in der `TitleBar`. Entfernt wurden die Toggles im
  LeftRail-Header und in der WorkArea-Tab-Leiste (Dubletten).
- **Weiche Kanten statt Linien:** alle `border: 1px solid var(--border)` zwischen den
  Regionen ersetzt durch `linear-gradient`-Fades (Panel → `var(--bg)`), ~30px
  (TitleBar 18px). Kein harter Übergang mehr.
- `TitleBar`-Padding auf `0 14px` erhöht.
- WorkArea-Sichtbarkeit persistiert (`ds-ide.workarea`).

"Milchglas"-Hinweis: echter Backdrop-Blur bringt nur etwas, wenn dahinter Inhalt liegt;
über solider Fläche ist er unsichtbar. Daher Farbverlauf-Übergang statt Blur.
Echte Fenster-Transluzenz wäre Tauri `windowEffects` (auf Linux/GTK nur eingeschränkt).

## 8. Engine — Quellen gesichtet (2026-10-15)

### Was die Quellen sind
- `minimax-jarvis/` (lokal, **MIT**) = **Definitionsebene, kein Runtime**.
  *Hinweis (§30):* nichts im Baum lädt daraus; unsere Werkzeuge und Verträge sind eigene
  Umsetzungen. Das Vorbild ist jetzt auf der Über-Seite genannt, damit der Ordner gehen kann.
  `agents/` stammt aus `MiniMax-AI/minimax-code` → `packages/local-runtime-v2/assets/agents/`.
  Inhalt: `builtin-agents.json` = `["mavis","explore","worker","verifier"]`; je Agent
  `PERSONA.md` / `PERSONA-zh.md` / `system-prompt.md.hbs`;
  `mavis/modes/{coding,work}/online/{PERSONA,SYSTEM}.md.hbs`;
  `mavis/skills/**/SKILL.md`; `mavis/features/*.md.hbs` (cron, delegation, memory,
  recoverable-deletion, task-management, web-search); `_v2/AGENT_CONTEXT.md.hbs` u. a.
  Die **„Partner"-Rolle steht dort wörtlich**: `description: A partner with judgment and warmth`.
  `tauri-reference/` stammt aus der **closed-source** MiniMax-Desktop-App →
  **nur Ideen/Referenz**, kein Code, keine Assets.
- `MiniMax-AI/skills` (MIT) = **Skill-/Wissenspakete** (frontend-dev, fullstack-dev,
  android/ios/flutter/react-native, shader, pdf/pptx/xlsx/docx, multimodal) für
  Claude Code / Cursor / Codex / OpenCode. Kein Runtime.
- `MiniMax-AI/OpenAgentCore` (MIT) = **serverseitige Infrastruktur**: self-hosted
  OpenAI-Agents-API mit wählbarem Harness (Codex, Claude Code, MiniMax Code), Sandboxes
  (microsandbox/E2B/Docker), `/v1` + `/core/v1`. Braucht Docker/Node → kein Desktop-Baustein.

### Schlussfolgerung
Keine der drei Quellen ist eine einbettbare Engine. Vorhanden ist die **Seele**
(Personas/Prompts/Skills/Modes), **nicht der Motor** (LLM-Loop + Tool-Ausführung).

### Optionen
1. **Eigene schlanke Rust-Engine im Tauri-Prozess** (Empfehlung): DeepSeek
   (OpenAI-kompatibel) + Tool-Loop + Tools (read/write/patch/run), Personas aus
   `minimax-jarvis` (Identitätstexte für unser Produkt umgeschrieben);
   „Full Authorization" → Approval-Gate; Streaming per Tauri-Event in die MessageList.
2. **Sidecar `kilo serve`** (MIT) als fertiger Runtime, Frontend per HTTP+SSE —
   schnellster Weg zu funktionierender Tool-Ausführung.
3. **OpenAgentCore** als Backend (Docker) — mächtig (mehrere Harnesses), aber
   Infrastruktur statt Desktop-Baustein.

### Pflicht bei Übernahme
- MIT-Attribution erhalten (`LICENSE`, `NOTICE`, `THIRD_PARTY_NOTICES.md`).
- Identitätstexte **nicht wörtlich** übernehmen, für unser Produkt umschreiben.
- Rolle: **Partner**, nicht „Agent".

## 9. Engine implementiert (2026-10-15, Rust im Tauri-Prozess)

Entschieden (Nutzer bestätigt): **eigene schlanke Rust-Engine im Tauri-Prozess** —
kein Sidecar, kein Docker. Provider: **nur DeepSeek** (OpenAI-kompatibel).
Partnername: **Seeky** (Ziggy als Alternative, ein String). Rolle: **Partner**.

### Rust — `src-tauri/src/engine/`
- `types.rs` — `EngineConfig` (apiKey, baseUrl, model, systemPrompt, workspace,
  fullAuthorization, maxTurns), `ChatMessage`, `EngineEvent`
  (kind: `stream` | `status` | `tool_start` | `tool_result` | `usage` | `error` | `turn_end`).
  Mapping auf den Mavis-Vertrag (`packages/protocol/src/runtime.ts`):
  `stream`→STREAM_RESP, `status`→SESSION_STATUS, `turn_end`→TURN_TERMINAL;
  `PermissionPolicyType` → unser `fullAuthorization`.
- `deepseek.rs` — Streaming-Client: `POST {baseUrl}/chat/completions`, SSE,
  `stream_options.include_usage`, akkumuliert `tool_calls`-Deltas.
- `tools.rs` — Definitionen + Ausführung: `read`, `write`, `edit`, `bash`, `glob`,
  `grep`, `todowrite`. An MiniMax' `Local*ToolDef` angelehnt.
  **Permission-Gate:** `write`/`edit`/`bash` verweigern ohne Full Authorization.
- `mod.rs` — `EngineState` (pro Session: messages, todos, cancel, seq);
  Kommandos `engine_set_config`, `engine_configured`, `engine_send`, `engine_abort`,
  `engine_reset`; Turn-Loop (max 24 Runden) mit Tool-Runden;
  Events auf Kanal **`engine://event`**.

### Frontend
- `src/engine/persona.ts` — eigene Persona, **nicht** aus den MiniMax-Texten kopiert.
- `src/composables/useEngine.ts` — Event-Listener, `sendPrompt`, `abortTurn`,
  `configureEngine`, `running`, `engineReady`, `lastError`.
- `src/stores/sessions.ts` — Message-Buckets mit Streaming (`beginAssistantMessage`,
  `appendAssistantDelta`, `upsertToolActivity`, `endAssistantMessage`).
- `MessageList` — Text-Stream + Tool-Chips (running/done/failed + Dauer).
- `Composer` — sendet an die Engine, Abort-Button während des Laufs, Modellfeld,
  „Volle Berechtigung"-Toggle (Default **an**).
- Settings → **„Nutzung & Modell"** — API-Key, Basis-URL, Modell.

### Verifiziert
- `cargo check` grün, `cargo test detect_editors_smoke` grün.
- `pnpm build` (vue-tsc + vite) grün.
- `pnpm tauri dev`: Fenster startet, alle Module HTTP 200, `@tauri-apps/api/event`
  pre-bundled ⇒ Frontend hat `initEngine` ausgeführt, keine Rust-Fehler.
- **Live gegen die echte DeepSeek-API grün** (`cargo test --lib live_deepseek --
  --ignored --nocapture`, Key via `DS_IDE_API_KEY`):
  - Text-Streaming: Antwort `PONG`, 4 gestreamte Zeichen, usage 15/2/17.
  - Tool-Calling: `glob` mit `args={"pattern": "Cargo.toml"}` korrekt geparst.

### Offen / bewusst nicht fertig
- Persistiertes `fullAuthorization` steht bei Bestandsinstallationen noch auf `false`
  (Wert von vor der Default-Änderung) → im Composer einmal einschalten, sonst
  verweigern `write`/`edit`/`bash`.
- API-Key liegt im localStorage → Umzug in den OS-Schlüsselbund geplant.
- Approval **blockiert** den Turn nicht: ohne Full Authorization verweigert das Tool
  und der Partner erklärt es. Ein echtes „fragen und warten" (ACTION_REQUIRED-Round-Trip)
  fehlt.
- Nicht portiert: `task*` (Subagenten), `memory`, `skill`, `web_fetch`, `ask_user`,
  `mavis`, `code_review`.
- `pnpm tauri dev` hinterlässt bei hartem Abbruch verwaiste Vite-Prozesse
  (Port 1420 bleibt belegt) → vor Neustart aufräumen.

## 10. UI-Fixes nach der ersten Live-Sitzung (2026-10-15)

Der erste echte Chat-Lauf im Fenster war erfolgreich: Seeky antwortete korrekt und sehr
schnell (Streaming über DeepSeek), und wies die Verwechslung mit „Ziggy" selbst zurück.

Gefixt:
- **Scrollen in beiden Seitenleisten** funktionierte nicht. Ursache: klassischer
  Grid-Fallstrick — eine `auto`-Grid-Zeile wächst über den Container hinaus und wird
  von `overflow: hidden` abgeschnitten, statt zu scrollen. Fix:
  `grid-template-rows: minmax(0, 1fr)` in `App.vue` (`.body`), `ViewFileTab.vue`
  (`.viewfile`) und `SettingsPage.vue` (`.settings`).
- **Dublette** „AGENT-TEAM" in der `LeftRail` entfernt (wurde doppelt gerendert).
- **„Volle Berechtigung"** lag nur im Composer. Jetzt zusätzlich in
  Settings → „Nutzung & Modell" als `ToggleRow` — dort wurde sie gesucht.

### Layout-Umstellung auf Karten (nach Nutzer-Review)

Vorlage: das StageVibe-Layout (dieselbe Designsprache in allen DeepVibe-Produkten).
Entscheidung: **übernommen** — löst das „Abstand"-Problem strukturell statt kosmetisch.

- Neue Variable `--bg-app` (`#141414`, hell `#e4e4e4`) als Fensterhintergrund.
- Globale Klasse `.card` (in `global.css`): `background: var(--bg)`, `1px solid var(--border)`,
  `border-radius: 12px`.
- `App.vue`: `.stage` (Padding) → `.body` (Grid, `gap: 10px`). Die drei Regionen
  (`LeftRail`, `AgentPanel`, `WorkArea`) bekommen `class="card"`.
  Die **Farbverlauf-Fades sind entfernt** — nicht mehr nötig.
- Resizer sitzen jetzt in der Lücke: `left/right: calc(X% + 5px)`.
- `SettingsPage`: nav + content ebenfalls als Karten, `--bg-app` außen.
- Dateibaum im View-File-Tab: `--bg-elevated` gegen das Viewer-`--bg` → der Übergang ist
  jetzt sichtbar statt „ertastbar".

### Weitere Fixes
- **Konto-Menü wurde abgeschnitten:** Ursache war nicht nur der z-index, sondern
  `overflow: hidden` auf `.rail` (clippt absolut positionierte Kinder). `.rail` ist jetzt
  `overflow: visible`, Menü `z-index: 40` (+ Backdrop 39), Breite `min(260px, 100vw - 24px)`.
- `.tree-head`-Selektor war bei einer Reparatur versehentlich gelöscht worden → wiederhergestellt.

## 11. Persistenz: SQLite (2026-10-15)

`rusqlite` mit Feature `bundled` (keine System-Abhängigkeit). Datei:
`<app-data>/com.deepvibe.ds-ide/ds-ide.sqlite3`, **Modus 0600**, WAL, `foreign_keys=ON`.

### Schema (`src-tauri/src/db.rs`)
- `sessions(id, title, grp, workspace, created_at, updated_at)`
- `messages(id, session_id, role, content, tool_calls, tool_call_id, created_at)`
- `tool_calls(id, session_id, message_id, call_id, name, arguments, output, is_error,
  duration_ms, created_at)` — `message_id` hängt den Tool-Call an die Assistant-Nachricht
- `todos(id, session_id, content, status, priority, created_at)`
- `memories(id, kind, scope, text, source, importance, created_at, last_used_at)` — angelegt,
  noch ungenutzt (Langzeitgedächtnis, siehe unten)
- `settings(key, value)` — angelegt, Methoden vorhanden, noch ungenutzt

### Rust
- `Db::open`, `ensure_session`, `touch_session`, `list_sessions`, `create_session`,
  `delete_session`, `insert_message` (liefert rowid), `insert_tool_call`,
  `replace_todos`, `list_todos`, `load_history` (mit verschachtelten Tools),
  `load_llm_messages` (rekonstruiert die LLM-Historie inkl. `tool_calls`),
  `set_setting`/`get_setting`.
- Kommandos: `sessions_list`, `session_create`, `session_delete`, `session_history`,
  `session_todos`.
- **Umbau der Engine:** Die Konversation ist **nicht mehr nur im Speicher**. `engine_send`
  schreibt die User-Nachricht **sofort**, lädt die Historie **aus der DB** für den LLM-Call
  und schreibt Assistant-Nachricht + jede Tool-Ausführung durch. `todowrite` persistiert
  die Todo-Liste. `engine_reset` löscht die Session-Inhalte in der DB.
  `SessionState` hält nur noch `cancel` + `seq`.

### Frontend
- `useEngine`: `loadSessions`, `loadHistory`, `openSession`, `createSession`;
  `initEngine` lädt Workspace → Config → Sessions → Historie.
- LeftRail: „Neue Aufgabe" legt echt eine Session an; Session-Klick lädt die Historie.
- Die hartkodierte Session-Liste ist ersetzt — sie kommt aus der DB.

### Verifiziert
- `cargo check` grün; Unit-Test `db::tests::round_trips_history_and_todos` grün
  (Session, Nachrichten, Tool-Anhang, Todos, LLM-Historie, Session-Liste).
- `pnpm build` grün.
- `pnpm tauri dev`: DB wird angelegt (`-rw-------`), alle sechs Tabellen, Frontend legt
  beim Start die erste Session an (`s1792061157035`, workspace `~`).

### Bewusst NICHT gemacht
- Der **API-Key liegt weiter im localStorage** — er gehört nicht in die DB. Umzug in den
  OS-Schlüsselbund ist der nächste Schritt (siehe Abschnitt 12).

## 12. Entscheidungen: Secrets & Langzeitgedächtnis (2026-10-15)

### Secrets (API-Key)
- **Nicht in die SQLite-DB.** Klartext-Secrets in einer Anwendungs-DB sind der falsche Ort —
  und eine verschlüsselte DB bräuchte wieder ein Bootstrapping-Geheimnis.
- Ziel: **OS-Schlüsselbund** via `keyring`-Crate (Secret Service / Keychain / Credential
  Manager). Auf dieser Maschine verfügbar: `gnome-keyring` läuft, `org.freedesktop.secrets`
  ist auf dem Session-Bus aktiv, libsecret 0.21.8.
- Damit verlässt der Key das Frontend **komplett**: `engine_set_config` bekommt keinen
  `apiKey` mehr, Rust lädt ihn beim Senden aus dem Keyring. Das Frontend kennt nur noch
  `hasApiKey`.
- Fallback ohne laufenden Keyring: `<app-data>/secrets.json` mit Modus 0600 **plus
  sichtbarer Warnung** in den Settings — kein stiller Klartext.
- `SQLCipher` (verschlüsselte DB) bewusst **nicht**: löst das Key-Problem nicht, verschiebt es.
- Nicht-Secrets (Modell, baseUrl, Appearance, Toggles, Workspace) → `settings`-Tabelle,
  damit localStorage nur noch UI-Kram hält.

### Langzeitgedächtnis („aus Persona wird Persönlichkeit")
1. **Vorbereitet:** Tabelle `memories` (kind, scope, text, source, importance, created_at,
   last_used_at).
2. **Stufe 1 — bewusstes Merken:** ein `memory`-Tool (existiert im MiniMax-Tool-Set), mit dem
   der Partner explizit Fakten/Präferenzen/Ereignisse ablegt — **nicht** automatisch alles
   mitschneiden. Dazu eine Review-Oberfläche (ansehen, korrigieren, löschen).
3. **Stufe 2 — Erinnern:** beim Turn-Start die relevantesten Einträge in den Systemprompt
   injizieren (scope `user`/`agent`, gewichtet nach `importance` + `last_used_at`).
4. **Stufe 3 — semantische Suche:** **Qdrant** als Vektorindex, gefüttert aus SQLite.
   Einbahnig: **SQLite bleibt die Quelle der Wahrheit**, Qdrant ist nur Suchindex und
   jederzeit neu aufbaubar.
5. **Kuration („der Doktor"):** periodischer Lauf, der Memories zusammenfasst, Widersprüche
   und Veraltetes markiert und aufräumt. Genau dafür ist MiniMax' `mavis-doctor` gedacht —
   sein Inhalt ist in der öffentlichen Source **nicht enthalten**, also bauen wir ihn selbst.

`kind`: `fact`, `preference`, `event`, `project`, `news`.
`scope`: `user` (über den Nutzer), `agent` (über Seeky selbst), `session` (flüchtig).

### Weitere Mavis-Übernahme (Kandidaten)
Neben der Persona lohnen als Nächstes: `mavis/features/*.md.hbs` (cron, delegation, memory,
recoverable-deletion, task-management, web-search) und `_v2/AGENT_CONTEXT.md.hbs`
(Harness-/Tool-Usage-/Memory-/Output-Konventionen).

## 13. Backlog — Abarbeitungsreihenfolge (2026-10-15)

Nutzerauftrag: „so gut wie eigenständig abarbeiten, außer bei Überprüfungen".
Nach **jedem** Punkt: `pnpm build`, `cargo test`, PROGRESS.md aktualisieren.

**Stand 2026-10-15 (zweite Runde): Punkte 1-9 sowie 12, 13, 14, 15 sind erledigt und
verifiziert. Offen ist nur noch 10 (Walfisch-Branding — wartet auf das Nutzer-Asset).
Alle Commits sind auf `origin/main`.**

Verifikation dieser Runde: `pnpm build` grün · `cargo test` 9 passed / 8 ignored ·
alle Live-Tests grün (`list_local_skills_smoke`, `live_code_review_runs_git`,
`live_web_fetch_reads_example_com`, zwei Subagenten-Tests, drei DeepSeek-Live-Tests;
Key aus dem OS-Schlüsselbund).

1. ✓ **Autoscroll im AgentPanel** — erledigt. `AgentPanel.vue`: rAF-gedrosseltes Nachführen
   der Scrollposition, `following`-Flag aus einer 48px-Bodenzone, „Nach unten"-Knopf
   (i18n `composer.scrollToBottom`) erscheint nur, wenn pausiert; eine eigene
   User-Nachricht setzt `following` wieder auf `true`. Session-Wechsel springt ans Ende.
   Evidenz: `pnpm build` grün · `cargo test` 2 passed / 2 ignored · `pnpm tauri dev`
   startet, Modul HTTP 200.
2. ✓ **Secrets in den OS-Schlüsselbund** — erledigt.
   - `src-tauri/src/secret.rs`: `SecretStore` mit Keyring-first und 0600-Datei-Fallback,
     `SecretStatus { present, backend: keyring|file|none }`; Kommandos `secret_status`,
     `secret_set`, `secret_clear`.
   - `EngineConfig.api_key` ist jetzt `#[serde(skip, default)]` → **das Frontend kann keinen
     Key mehr einschleusen**. `engine_send` liest ihn pro Turn aus dem Keyring.
   - Frontend: `settings.apiKey` **entfernt**; Settings-Seite hat Eingabefeld + Speichern/
     Entfernen und zeigt den Backend-Status; bei `file` erscheint eine **rote Warnung**.
   - Automatische **Migration**: ein noch im localStorage liegender Key wird einmalig
     in den Schlüsselbund geschrieben und danach aus localStorage gelöscht.
   - Evidenz: `cargo test --lib secret` → `keyring_roundtrip_or_skip` **ok** (kein Skip,
     also echter Secret-Service-Zugriff); `pnpm build` grün; nach `pnpm tauri dev`:
     `secret-tool search service ds-ide.deepvibe` findet den Eintrag (Ausgabe verworfen,
     Key wurde nie ausgegeben), `apiKey` ist aus localStorage verschwunden,
     keine `secrets.json` vorhanden.
3. ✓ **`settings`-Tabelle gefüllt** — erledigt. `db.rs`: `all_settings` + Kommandos
   `settings_all` / `settings_set`. Der Settings-Store (`stores/settings.ts`) liest beim
   Start aus SQLite und schreibt **entprellt** (400 ms) zurück; ein alter
   `ds-ide.settings`-Blob aus localStorage wird einmalig importiert und gelöscht.
   `main.ts` mountet die App erst nach dem Laden (kein Theme-/Sprach-Flackern).
   Evidenz: nach `pnpm tauri dev` stehen **11 Keys** in `settings`
   (u. a. `mode: coding`, `model: deepseek-flash`, `pinnedEditorId: zed`,
   `workspace: ~`); localStorage enthält nur noch `ds-ide.ui`,
   `ds-ide.ui.v2`, `ds-ide.workarea`. `pnpm build` grün, `cargo test` 3 passed.
4. ✓ **Reasoning sichtbar** — erledigt. `deepseek.rs` liefert jetzt `DeltaKind::{Text, Reasoning}`
   an den Callback (vorher wurde `reasoning_content` **verworfen**); `StreamOutcome` trägt
   `reasoning`. Die Engine emittiert dafür Events mit `kind = "reasoning"`, das Frontend
   hängt sie an `ChatMessage.reasoning` und zeigt sie als **einklappbares `<details>`**
   über der Antwort (offen, solange noch kein Text da ist).
   Reasoning wird **nicht** persistiert (flüchtiges Denken; die DB soll keine Denkspuren
   sammeln) — beim Neuladen steht die Antwort, nicht das Nachdenken.
   Evidenz: neuer Live-Test `live_deepseek_reasoner_streams_thinking` grün —
   `reasoning chars: 166`, Antwort `391`; `pnpm build` grün, `cargo test` 3 passed.
5. ✓ **Langzeitgedächtnis Stufe 1+2** — erledigt (Stufe 3/5 = Punkt 6).
   - `db.rs`: `insert_memory`, `list_memories`, `memory_context` (sortiert nach
     `importance`, dann `last_used_at`, dann Alter), `touch_memories`, `delete_memory`;
     Kommandos `memories_list`, `memory_delete`.
     *Überholt (Abschnitt 21):* `memory_delete` gibt es nicht mehr, Löschen heißt jetzt
     Stilllegen (`memory_retire`/`memory_restore`), und Einträge tragen `origin` und `edited_at`.
   - Tool **`memory`**: `action` add|list|forget, `kind` fact|preference|event|project|news,
     `scope` user|agent|session. Ausführung liegt bewusst in `engine/mod.rs` (`memory_tool`),
     damit `tools.rs` speicherfrei bleibt.
   - **Erinnern:** `run_turn` hängt vor jedem Turn einen Block „What you already know"
     (bis 40 Einträge) an den Systemprompt und stempelt `last_used_at`.
   - **UI:** neuer Work-Area-Tab **„Gedächtnis"** (`MemoryTab.vue`, Toolbar-Button mit
     Sparkle-Icon) — Einträge mit Scope/Kind/Datum und Entfernen-Knopf.
     *Überholt (Abschnitt 21):* der Tab ist weg, die Einsicht sitzt in Einstellungen →
     Personalisierung (`MemoryPanel.vue`).
   - Rationale: `kind`/`scope` statt Freitext, damit die Kuration (Punkt 6) etwas zu
     greifen hat.
   - Evidenz: `cargo test` → `db::tests::memories_round_trip` ok (insert/list/context/
     touch/delete); `pnpm build` grün; `pnpm tauri dev` startet, Module HTTP 200;
     **keine Rust-Warnungen** mehr.
6. ✓ **Kuration / „Doktor"** — erledigt, aber **bewusst mechanisch**.
   - `db.rs`: `memory_report(stale_after_days)` liefert `MemoryReport { total, duplicates,
     stale }`. Duplikate = gleicher Text nach Normalisierung (Whitespace + Groß/Klein);
     behalten wird der älteste Eintrag. `stale` = **nie benutzt** und älter als die Frist
     (30 Tage). Kommandos `memory_report`, `memory_prune(ids)`.
     *Überholt (Abschnitt 21):* `memory_prune` löscht nichts mehr, es legt still.
   - Tool `memory` hat jetzt `action: "curate"` — der Partner kann den Bericht selbst
     anfordern und dann **gezielt** mergen oder vergessen.
   - UI: im Gedächtnis-Tab ein **Prüfen**-Knopf (Sparkle). Der Bericht erscheint als Karte
     mit „N doppelte Einträge" / „N nie benutzte, alte Einträge" und je einem
     **Entfernen**-Knopf. Ohne Befund: „Nichts zu bereinigen."
   - **Entscheidung:** Der „Doktor" schlägt vor, **löscht aber nie von selbst**. Was
     *inhaltlich* zusammengehört, kann eine Heuristik nicht beurteilen — das bleibt beim
     Modell (per `curate`) oder beim Nutzer. Ein Gedächtnis, das sich selbst ungefragt
     aufräumt, ist genau die Sorte Werkzeug, der man nicht mehr traut.
   - Evidenz: `cargo test` → `db::tests::memory_report_flags_exact_duplicates` ok
     (Erkennung trotz abweichender Leerzeichen/Groß-Klein-Schreibung, frische Einträge
     nicht als stale); `pnpm build` grün (9 Tests grün, 6 ignoriert); App startet,
     Modul HTTP 200.
7. ✓ **Weitere Tools** — erledigt: `web_fetch`, `code_review`, `ask_user`, `task`.
   - **`web_fetch`** — holt eine http(s)-URL (30 s Timeout), `html_to_text` entfernt
     Script-/Style-Blöcke, Tags und dekodiert die gängigen Entities, Ausgabe gekappt.
     Bewusst ohne DOM-Parser: dependency-frei und vorhersagbar.
   - **`code_review`** — sammelt `git diff --stat` + `git diff` (optional `--cached`,
     optional Pfad) im Workspace und gibt beides zurück. Ehrlich: es *holt* den Diff,
     die Bewertung macht das Modell.
   - **`ask_user`** — **echter** `ACTION_REQUIRED`-Round-Trip statt Attrappe: der Turn
     emittiert ein Event mit Frage und optionalen Optionen, parkt einen `oneshot::Sender`
     in `EngineState.pending` und **wartet** (Timeout 5 min). Das Kommando
     `engine_answer(actionId, answer)` löst ihn auf; im Chat erscheint über dem Composer
     eine Rückfrage-Karte mit Optionsknöpfen und freiem Feld.
   - **`task` (Subagent)** — eigener Kontext, eigene Tool-Runden (max. 8), liefert **nur das
     Endergebnis** zurück (gekappt auf 20 000 Zeichen). Sein Verlauf erscheint **nicht** im
     Chat, sondern nur die Tool-Karte — genau darum geht es: den Elternkontext sauber halten.
     Absichtliche Schutzgeländer: **kein** verschachteltes `task` (keine Rekursion) und
     **kein** `ask_user` im Subagenten (er könnte den Nutzer über die Eltern-UI nicht
     erreichen). `task_tool` braucht **kein** `AppHandle` — dadurch ist es direkt testbar.
   - Evidenz: `cargo test` → `html_to_text_drops_scripts_styles_and_tags` ok; mit
     `--ignored`: `live_web_fetch_reads_example_com` ok (184 Bytes Text),
     `live_code_review_runs_git` ok (git-Diff im Arbeitsbaum),
     `live_task_subagent_answers_without_tools` → „BEREIT",
     `live_task_subagent_uses_its_tools` → Subagent zählte per `glob` **150** Einträge und
     gab nur die Zahl zurück; `pnpm build` grün; App startet.
   - **Nicht verifiziert:** ein echter `ask_user`-Durchlauf braucht eine Antwort im UI —
     der Pfad ist gebaut und kompiliert, aber noch nicht von Hand durchgeklickt.
8. ✓ **„Suche" verdrahtet, Rest ehrlich gekennzeichnet** — erledigt.
   - Rust: `Db::search` + `snippet_around` (Kontext-Ausschnitt um den Treffer) und Kommando
     `search` — durchsucht Sitzungstitel und Nachrichteninhalte (`LIKE`, Limit 40).
   - UI: `SearchDialog.vue` als Overlay (Teleport), öffnet über „Suchen" in der Leiste
     **oder Strg/Cmd+K**, Esc schließt, Enter öffnet den ersten Treffer. Treffer sind als
     „Sitzung"/„Nachricht" gekennzeichnet, Klick öffnet die Sitzung.
   - Ehrlichkeit: „Skills", „Geplant" und „Seite" tragen jetzt ein **„bald"**-Abzeichen,
     damit die Leiste nicht mehr suggeriert, sie würden schon funktionieren.
   - Evidenz: `cargo test` → `db::tests::search_finds_sessions_and_messages` ok
     (Titeltreffer, Nachrichtentreffer mit Snippet, leere Query bleibt leer);
     `pnpm build` grün; App startet, Module HTTP 200.
9. ✓ **Fortschritts-Tab aus echten Todos** — erledigt. Die hartkodierten
   „Liefergegenstände" aus `stores/sessions.ts` sind **entfernt**; der Tab liest jetzt
   `session_todos` (Rust liefert `TodoRow { id, content, status, priority }` statt nur
   Strings) und gruppiert nach Status — „In Arbeit / Offen / Erledigt", Erledigtes
   durchgestrichen. Aktualisiert sich live: `useEngine` zählt `todosRevision` hoch, sobald
   ein `todowrite`-Ergebnis eintrifft.
   Evidenz: `cargo test` grün (Test prüft `list_todos()[0].content`), `pnpm build` grün,
   App startet, Module HTTP 200, `todos`-Query liefert die erwartete Spaltenform.
10. ◐ **Walfisch-Branding** — statischer Teil erledigt, Animation offen.
    - Quelle gefunden: `~/Schreibtisch/ziggy.png` (512×512, **bereits korrekt freigestellt**:
      Ecken voll transparent, mittlere Deckkraft 0,33) plus `ziggy1.webp`, `ziggy.webp`.
    - **App-Icon-Set erzeugt**: `ziggy.png` getrimmt, auf 900 px skaliert, mittig auf
      1024×1024 gepolstert → `pnpm tauri icon` → `src-tauri/icons/*` (inkl. Android/iOS)
      aktualisiert.
    - **Avatar im Chat**: `public/whale.png` (128×128, transparent), in `MessageList.vue`
      links neben Assistant-Nachrichten (22 px). Liegt auch im Build (`dist/whale.png`).
    - **Animation offen — und der Grund ist echt:** Die Videovorlage
      (`~/Downloads/ai-artwork-video_*.mp4`, 752×560) hat einen **blauen Verlaufs­hintergrund**
      (Ecken von `rgb(56,164,212)` bis `rgb(171,238,255)`), und der Wal ist **ebenfalls blau**
      inklusive hellem Bauch. Farb-Keying zerstört ihn deshalb — genau daran scheitern die
      Web-Tools. Sauber geht das nur mit **Segmentierung** (z. B. `rembg`/U²-Net), nicht mit
      `colorkey`. Das braucht ein Modell (~170 MB Download).
    - Nächster Schritt, wenn gewünscht: `rembg` in einer venv installieren, die 97 Frames
      freistellen und als **animiertes WebP** (Alpha, viel kleiner als GIF) für Splash und
      Chat-Avatar ausgeben. Fenster-/Taskleisten-Icon bleibt statisch — kein OS animiert
      App-Icons.
11. **Regelmäßig**: `pnpm build` + `cargo test` + Verifikation im Fenster + PROGRESS.md.

### Bewusste Grenzen in dieser Runde
- Kein Approval-Round-Trip (`ACTION_REQUIRED` fragt und wartet) — das Gate verweigert nur.
- Keine Subagenten, keine Plugin-/MCP-Anbindung, kein Web-Browsing-Tab im Betrieb.

## 14. UI-Bereinigung: was ist echt, was ist MiniMax-Erbe (2026-10-15)

Der Nutzer hat gefragt, welche der übernommenen Nav-Einträge wirklich umsetzbar sind.
Befund und Entscheidung:

| Eintrag | Herkunft | Entscheidung |
|---|---|---|
| Mobil verbinden | MiniMax-Companion-App (Android/iOS, „Mobile app 4.15.0+") | **raus** — wir haben keine Companion-App |
| Fernsteuerung | MiniMax-Fernsteuerung eines laufenden Tasks | **raus** (gleicher Grund) |
| Plugins | MiniMax-Marketplace (eigene Accounts, Usage-Zähler) | **kein Store** — DeepSeek hat keinen. Stattdessen **lokale Skills** |
| Geplant | MiniMax-Scheduler („keep awake") | **umsetzbar**, eigenes Item |
| Seite | reiner Prompt-Aufruf („Create a website") | als **Skill** übernehmen (`deploy-website`, `visual-page`) |
| Abonnieren | MiniMax-Abo | **streichen** — DeepSeek hat nur **Top-up (Credits)**, kein Abo |
| Täglicher Check-in | MiniMax Free-Tier-Feature | **streichen** |
| Nutzung / Feedback / Änderungsprotokoll / Kontakt / Mehr erfahren / Abmelden | gemischt (DeepSeek-Plattform + unseres) | später verlinken |

**Skills-Quellen:** `https://github.com/MiniMax-AI/skills` (MIT, generische
Markdown-Anleitungen, kein Store nötig). Das README dort nennt die Einbaupfade für
Claude Code, Cursor, Codex und OpenCode. Für DS.IDE: eigenes Verzeichnis
`~/.ds-ide/skills/**/SKILL.md` + Loader.

### Zusätzliche Backlog-Punkte (nach 10)
12. ✓ **Nav bereinigt** — erledigt. Entfernt: „Mobil verbinden" und „Fernsteuerung"
    (brauchen MiniMax' Companion-App), „Abonnieren" (DeepSeek hat nur Top-up-Credits) und
    „Täglicher Check-in" (MiniMax-Free-Tier). „Plugins" → **„Skills"**. Übrig bleiben
    „Neue Aufgabe" (funktioniert), „Suche" (Punkt 8), „Skills" (14), „Geplant" (13),
    „Seite" (14) — die letzten drei bewusst als geplante Ziele, nicht als Attrappen.
    Evidenz: `pnpm build` grün (vue-tsc prüft die i18n-Kataloge gegen die Form-Referenz,
    also sind beide Sprachen konsistent).
13. ✓ **Eigener Scheduler** — erledigt (bewusst **Intervall**, nicht Cron).
    - Tabelle `schedules` (session_id, prompt, interval_seconds, enabled, last_run_at,
      next_run_at). Das Schema läuft bei jedem Öffnen — die Tabelle landete also **ohne
      Migration** in der bestehenden DB.
    - `db.rs`: `list_schedules`, `create_schedule` (Minimum 60 s), `delete_schedule`,
      `set_schedule_enabled`, `due_schedules(at)`, `mark_schedule_run`; Kommandos
      `schedules_list`, `schedule_create`, `schedule_delete`, `schedule_toggle`.
    - Engine: `engine_send` wurde zu **`send_prompt(app, session_id, prompt)`** extrahiert,
      der Befehl ist nur noch eine Hülle. Dadurch läuft ein geplanter Auftrag durch **genau
      denselben Pfad** wie ein getippter (Persistenz, Tools, Events).
    - Hintergrundtask im `setup` von `lib.rs`: alle 30 s `run_due_schedules()`, das fällige
      Einträge startet und `next_run_at` fortschreibt.
    - UI: Tab **„Geplant"** (`SchedulesTab.vue`) — Auftrag + Intervall in Minuten anlegen,
      ein-/ausschalten, löschen, nächster und letzter Lauf. „Geplant" in der Leiste öffnet
      den Tab, das „bald"-Abzeichen ist weg.
    - **Ehrliche Grenzen:** Intervall statt Cron-Ausdrücken, und der Task lebt **nur, solange
      DS.IDE läuft** — kein Daemon, kein Aufwecken aus dem Ruhezustand. „keep awake" habe ich
      deshalb bewusst **nicht** gebaut, statt es als Schalter ohne Wirkung anzubieten.
    - Evidenz: `cargo test` → `db::tests::schedules_cycle` ok (nicht fällig → fällig →
      fortgeschrieben → deaktiviert → gelöscht); `pnpm build` grün; nach `pnpm tauri dev`
      existiert `schedules` in der bestehenden DB (0 Zeilen), Modul HTTP 200.
14. ✓ **Skills lokal** — erledigt.
    - `src-tauri/src/skills.rs`: scannt `~/.ds-ide/skills/**/SKILL.md` (Tiefe 3), liest
      YAML-Frontmatter (`name`, `description`) tolerant — ohne `name` fällt es auf den
      Verzeichnisnamen zurück, unlesbare Dateien werden übersprungen statt die Liste zu
      sprengen. Kommandos `skills_list`, `skills_root_path` (legt den Ordner an).
    - Tool **`skill`**: `action: list | load`, `name`. `load` liefert die vollständige
      SKILL.md als Tool-Ergebnis, damit der Partner sie befolgen kann. Liegt in `tools.rs`
      (nur Dateisystem); der Pfad kommt über `ToolContext.skills_root`.
    - UI: neuer Work-Area-Tab **„Skills"** (`SkillsTab.vue`) mit Name, Beschreibung, Pfad,
      „Ordner öffnen" (`revealItemInDir`) und Neu laden. „Skills" in der linken Leiste
      öffnet den Tab — das „bald"-Abzeichen ist dort weg.
    - Beispiel-Skill angelegt: `~/.ds-ide/skills/release-notes/SKILL.md` — damit der Tab
      nicht leer startet und das Format sichtbar ist.
    - Evidenz: `cargo test` → Frontmatter-Test **und** `list_local_skills_smoke`
      (bewusst `#[ignore]`, weil vom Home abhängig) findet real
      `release-notes — Turn a git log into short, human release notes.`;
      `pnpm build` grün, App startet, Modul HTTP 200.
15. ✓ **Dialog-Buttons platzsparend** — API-Key-Zeile: Textbuttons durch **✅ / ❌**
    ersetzt (Tooltips „Speichern"/„Entfernen"), Eingabefeld flexibel statt fix 320px
    (lief vorher rechts aus dem Panel).

## 15. Repository (2026-10-15)

TEST ist ein **eigenes Git-Repo** (vorher lag es nur untracked im noch leeren
`~/Projects`-Repo).
- Remote: `http://192.168.0.40/modest/DS.IDE.git`, Branch `main`
- Erster Commit: `2d1a5d2` „first commit", 82 Dateien, Push erfolgreich (Upstream gesetzt)
- `.gitignore` erweitert: `kilocode-main/`, `minimax-code-main/`, `minimax-jarvis/`,
  `monaco-editor-0.57.0/` (nur lokale Referenz) und `src-tauri/secrets.json`.
  `src-tauri/target/` ist über `src-tauri/.gitignore` abgedeckt, `node_modules`/`dist`
  über die Wurzel. **Nichts davon ist im Repo** (geprüft: 0 Treffer).
- Kein Secret im Quellcode oder in den getrackten Dateien.
- Hinweis: Weil `~/Projects` selbst ein (noch leeres) Repo ist, sieht dieses beim Commit
  `TEST` als eingebettetes Repo (Gitlink) — gewollt, aber gut zu wissen.

## 16. Sitzungen: Titel, Anheften, Umbenennen (2026-10-15)

Auf Nutzerwunsch (MiniMax als Vorbild, aber bewusst schlanker):
- **Titel aus der ersten Nachricht.** `maybe_generate_title` schreibt sofort einen
  Text-Fallback (erste Zeile, 60 Zeichen) und lässt dann im Hintergrund einen kurzen
  Modellaufruf („drei bis sechs Wörter, keine Anführungszeichen") einen besseren Titel
  setzen. Beide Schritte schreiben in die DB und schicken ein `title`-Event, damit die
  Leiste sofort nachzieht. Der Titel wird **nur** gesetzt, wenn er noch leer ist — ein
  selbst umbenannter Titel wird nie überschrieben.
- **Kontextmenü in der Leiste** (Rechtsklick auf eine Sitzung): **nur Anheften/Abheften
  und Umbenennen** — nicht die MiniMax-Liste (Archive, Fork, Show in Finder, Copy …).
  Bewusste Entscheidung: wir zeigen nur, was auch funktioniert.
- **Umbenennen** passiert **inline** in der Zeile (kein `window.prompt`): Enter übernimmt,
  Esc bricht ab, Blur übernimmt.
- Rust: `session_title`, `update_session(id, title?, group?)`, Kommando `session_update`.
- Evidenz: `cargo test` → `db::tests::sessions_can_be_renamed_and_pinned` ok (leerer Titel
  → Umbenennen → Anheften); `pnpm build` grün; Module gegen die laufende Dev-Instanz
  HTTP 200 (inkl. `/whale.png`).

### Lernnotiz zum Ablauf
`pnpm tauri dev` startet das App-Binary **auch dann**, wenn der `beforeDevCommand` am
belegten Port 1420 scheitert. Dadurch entstanden mehrfach zusätzliche Fenster, die einzeln
aufgeräumt werden mussten. Künftig: **nicht booten, wenn 1420 belegt ist**, sondern die
Module gegen die laufende Instanz prüfen.

## 17. Archiv, Konto, Wal-Animation (2026-10-15)

### Archivieren statt löschen
- `sessions.archived` per **Migration** ergänzt: `migrate()` prüft `PRAGMA table_info`
  und macht bei Bedarf `ALTER TABLE` — `CREATE TABLE IF NOT EXISTS` ändert bestehende
  Tabellen nicht. Auf der echten DB verifiziert (Spalte da, beide Sitzungen erhalten).
- `list_sessions` blendet Archiviertes aus, `list_archived_sessions` zeigt es; Kommandos
  `session_archive`, `sessions_archived`.
- UI: Kontextmenü → **Archivieren**; Settings → **Archivierte Aufgaben** listet sie mit
  **Reaktivieren**. Der Verlauf bleibt vollständig in der DB.
- Evidenz: `archiving_hides_a_session_without_deleting_it` und
  `migrate_adds_archived_to_an_old_database` grün.

### Konto
- **Anzeigename ist keine Attrappe mehr:** Setting `displayName`, Vorgabe vom System
  (`default_display_name` liest `$USER`/`$USERNAME`/`$LOGNAME`), einstellbar in
  Settings → Konto. Die erfundene „UID" aus dem MiniMax-Screenshot ist **entfernt**.
- **Menü:** Einstellungen (lokal) · Konto + Nutzung → DeepSeek · Feedback → GitHub ·
  Kontakt → Issues · Mehr erfahren → Projektseite · **Anmelden/Abmelden** je nach
  Key-Zustand (Abmelden löscht den Key aus dem Schlüsselbund, Anmelden springt direkt in
  Settings → Nutzung & Modell). „Änderungsprotokoll" entfernt (hatte kein Ziel).
- **Alle externen Adressen stehen in `src/engine/links.ts`** — eine Datei, klar
  kommentiert. `issues` zeigt vorläufig auf die Org-Seite, weil es noch kein öffentliches
  Repo gibt; dort bitte auf das echte Repo zeigen.

### Wal-Animation
- `rembg` in `~/.local/share/ds-ide/tools/venv-rembg` installiert (Freigabe des Nutzers).
  Achtung: die aktuelle Version lädt standardmäßig **bria-rmbg (1,02 GB)**, nicht u2net.
- **Wichtig fürs Tempo:** `rembg i` per CLI lädt das Modell **bei jedem Aufruf neu**
  (~22 s/Frame). Ein Python-Lauf mit `new_session()` einmal laden und dann alle Frames
  durchziehen ist um ein Vielfaches schneller (`/tmp/kilo/whale/cut.py`).
- 40 Frames (10 fps aus dem MP4) freigestellt, **fester gemeinsamer Zuschnitt**
  636×470+66+55 (aus dem Union-Bounding-Box aller Frames — per-Frame-Trim würde den Wal
  springen lassen).
- Ergebnis: **animiertes WebP mit Alpha**, `public/whale-anim.webp` (128 px, 152 KB) und
  `public/whale-splash.webp` (256 px, 332 KB), je 40 Frames.
- **Eingebaut:** Start-Animation (Splash-Overlay, blendet nach ~1,6 s aus) und im Chat
  schwimmt der Wal **statt der drei Blinkpunkte**, während auf die Antwort gewartet wird.
  Das Fenster-/Taskleisten-Icon bleibt statisch — kein OS animiert App-Icons.
- Statisch war schon fertig: Icon-Set aus `ziggy.png`, Chat-Avatar `public/whale.png`.
- Stolperstein: Ein abgebrochener Batch hinterließ `f020-cut.png` mit **0 Bytes**; ffmpeg
  bricht an so einer Lücke ab und lieferte nur 19 statt 40 Frames. Prüfen mit
  `find … -size 0`.

### Lernnotiz zsh
`rm -f verzeichnis/*-glob.png` bricht in zsh bei **leerem Glob** mit „no matches found" ab
und reißt per `&&` die ganze Kette mit. In Skripten `find … -delete` verwenden oder
`setopt null_glob`.

## 18. Sprachen, Links, Walfisch-Marke (2026-10-15)

### i18n auf sechs Sprachen
Ausgelagert an **vier Hintergrund-Subagenten** (Vorschlag des Nutzers), je eine Datei:
`zh-Hans` (Pflicht — Muttersprache des Betreibers), `fr`, `es`, `ru`. Registriert in
`i18n/index.ts` samt lesbarer Sprachliste (Sprachnamen als eigene Labels: 简体中文,
Русский, Français, Español). `detectLocale` erkennt jetzt auch `zh` und `ru`.

**Lehre — die Selbstauskunft der Subagenten war nicht belastbar.** Drei von vier
lieferten fehlerhafte Dateien, obwohl alle „KEY-SET IDENTISCH" meldeten:
- `fr` und `es` hatten einen **Syntaxfehler**: deutsche Anführungszeichen `„…"` mitten in
  einem doppelt gequoteten TS-String — das schließende `"` beendet den String.
- `fr` fehlten am Ende **fünf Keys** (`save`, `clear`, `notSet`, `storedKeyring`,
  `storedFile`).
- `zh-Hans` und `ru` hatten **deutsche Reste** in den Werten.
Was geholfen hat, sind zwei eigene Prüfungen (stehen als Rezept hier):
1. **Keys:** `pnpm build` — `vue-tsc` erzwingt über den Typ `Messages` die exakte
   Key-Gleichheit und nennt fehlende Felder beim Namen.
2. **Werte:** identische `key: "value"`-Zeilen zwischen Quelle und Übersetzung zählen
   (`comm -12`). Was übrig bleibt, ist entweder eine Marke (`DS.IDE`, `DeepSeek`, `Seeky`)
   oder eine übersehene Stelle. So fielen `tagline`, `providerDesc`, `modeEverydayDesc`
   und ein deutsches „OpenAI-kompatibel" in `es` auf.
Merksatz: Subagenten erzeugen Text, aber **verifizieren muss man selbst** — ihre
Bestätigung ist kein Nachweis.

### Externe Adressen
Alle in `src/engine/links.ts`: Konto → `platform.deepseek.com/profile`, Nutzung → `/usage`,
Feedback → `github.com/moc72/dside/issues`, Kontakt → `mailto:me@modestcoders.eu`,
Mehr erfahren → `modestcoders.eu`.

### Oberfläche
- Der Wal sitzt jetzt als **kleines animiertes Markenzeichen (26 px)** oben links in der
  Leiste, wo „DS.IDE" stand; im Chat sind wieder die drei Punkte (er störte dort).
  Der Splash beim Start bleibt.
- Einstellungen ausgedünnt: **Connection**, **Code review** und **Worktree** entfernt
  (MiniMax-Erbe ohne Substanz), **„Basis-URL" aus der Oberfläche** genommen.
- **Modelle kommen von der API** (`GET /models`, Knopf am Modellfeld) statt geratener Namen.
- **„Abmelden" ist jetzt zweistufig**: erst fragen (rot, ❌, „Entfernen"), dann löschen.
  Auslöser war der Nutzer, der es ausprobierte und dabei seinen Key verlor — das Verhalten
  war korrekt, aber zu scharf.

## 19. Vorschauen (2026-10-15)

### Was läuft
- **Markdown** — `marked` + **DOMPurify**, Umschalter Vorschau/Quelle. Markdown erlaubt
  rohes HTML; ohne Säuberung könnte eine Notizdatei Skript in unserem Webview ausführen,
  das Tauri-Befehle aufrufen darf.
- **Bilder** — über `read_file_base64` als Data-URL.
- **PDF** — `pdfjs-dist` 6.4, Seite für Seite auf Canvas, Worker gebündelt
  (`pdf.worker.min.mjs`, 1,26 MB). v6 will `render({ canvas, viewport })` statt
  `canvasContext`.
- Weiche: `previewKindFor()` in `src/lib/preview.ts`.

### Zwei Fehler, die wir gefunden haben
1. **Das Asset-Protokoll ist blockiert.** WebKit lehnt `asset://localhost/...` von der
   Dev-Server-Origin ab: „protocols and ports must match". Bilder über `convertFileSrc`
   liefen deshalb ins Leere. **Das gilt auch für die geplante HTML-Vorschau** — der
   Base64-Weg ist die verlässliche Variante. Asset-Protokoll und das Cargo-Feature
   `protocol-asset` sind wieder entfernt.
2. **Der Text-Loader lief für jede Datei.** `read_text_file` scheitert an Binärdaten
   (kein UTF-8) und setzte `error` — über einer Vorschau, die gerade lud. Ergebnis:
   „Datei konnte nicht gelesen werden" bei Bildern und PDFs. Jetzt entscheidet
   `previewKindFor`, ob überhaupt Text gelesen wird.

### Verifikation
- `cargo test` → `base64_matches_known_vectors` (RFC 4648) **und**
  `base64_matches_the_system_on_a_real_file` (182 618 Bytes, identisch zum System-`base64`).
- `pnpm build` grün; PDF-Worker und Worker-Chunks liegen im Build.

### Office-Formate (gleicher Abend)
- **Word** (`docx`, `odt`) → `mammoth` → HTML → **DOMPurify** (derselbe Säuberer wie Markdown).
- **Tabellen** (`xlsx`, `xls`, `ods`, `csv`) → SheetJS; die Tabelle wird **hier Zelle für
  Zelle gebaut und escaped**, nicht von der Bibliothek als HTML geliefert — sonst müsste man
  ihr zweimal vertrauen. Gedeckelt auf 300 Zeilen × 40 Spalten.
- **`rtf`** → grobe Text-Extraktion (Steuerwörter und Klammern weg). Ehrlich: lesbar, ohne Layout.
- **`doc`/`ppt`/`pptx`/`odp`** → klare Meldung („Dieses Format kann nicht angezeigt werden“)
  statt Buchstabensalat. Alte Binärformate brauchen einen Konverter, den bündeln wir nicht.
- **PDF-Standardfonts** liegen in `public/pdfjs/standard_fonts/` (16 Dateien) und werden über
  `standardFontDataUrl` geladen → die „system font“-Warnung ist weg.

### Offen
- **`xlsx` 0.18.5 ist die letzte npm-Ausgabe (2022)** und hat Advisories zum Parsen *fremder*
  Dateien (Prototype Pollution). Für die Vorschau *eigener* Dateien akzeptiert, aber auf der
  Liste: auf den offiziellen SheetJS-Build wechseln.
- Tabellenvorschau zeigt nur das **erste** Blatt, ohne Formeln und Formatierung.
- PDF, Bilder und Markdown sind von Hand bestätigt; **Office noch nicht durchgeklickt**.

## 20. Morgen-Liste (2026-10-15, spät)

### Was schon persistiert wird und was nicht (geprüft im Code)
| Zustand | gespeichert? | wo |
|---|---|---|
| Panel-Breiten (links/rechts) | **ja** | `localStorage` `ds-ide.ui.v2` |
| Seitenleiste eingeklappt | **ja** | dito |
| Work-Area sichtbar | **ja** | `ds-ide.workarea` |
| Arbeitsverzeichnis | **ja** | `settings`-Tabelle |
| Anzeigename, Modell, Toggles, Sprache | **ja** | `settings`-Tabelle |
| **Fenstergröße/-position** | **nein** | Tauri speichert das nicht von selbst |
| **Aktive Sitzung** | **nein** | nur im Speicher |
| **Offene Work-Area-Tabs** | **nein** | nur im Speicher |
| **Zuletzt geöffnete Datei** | **nein** | nur im Speicher |

Fenster: dafür gibt es `tauri-plugin-window-state` (genau dessen Zweck).
Sitzung/Tabs/Datei: gehören in die `settings`-Tabelle bzw. in eine kleine
`ui_state`-Ablage — vier Felder, kein großes Vorhaben.

### Offene Punkte
1. **`pptx`** zeigt bewusst „Dieses Format kann nicht angezeigt werden". Präsentationen
   sind ein eigener Fall (keine brauchbare Browser-Bibliothek für echtes Layout);
   entweder eine einfache Folien-Extraktion oder ehrlich draußen lassen.
2. **`doc`/`ppt` (Binärformate)** bleiben draußen — brauchen einen Konverter.
3. **`xlsx` 0.18.5** gegen den offiziellen SheetJS-Build tauschen (Advisories).
4. **Companion-System**: die Alternative zum Mavis-Partner.
   - Rolle **Partner**, nicht Agent — im Recall und in der Persona sichtbar.
   - Wer Seekys Gedächtnis ändert, wird markiert: `origin` (`seeky` | `user`) und
     `edited_at` in `memories`; im Recall steht dann „vom Nutzer geändert".
   - Löschen als **Stilllegen**, nicht physisch entfernen.
   - Gedächtnis-Einsicht wandert dafür aus dem Work-Area-Tab nach
     Einstellungen → „Konto & Modell" bzw. „Personalisierung".

→ Punkte 1–4 sind in Abschnitt 21 abgearbeitet.

## 21. Persistenz, Vorschauen, Gedächtnis (2026-10-16)

Fünf Commits, einer pro Punkt, in der Reihenfolge der Liste aus §20: `05a5165`
(Fenster), `de8b120` (UI-Zustand), `b5b9552` (Folien), `0769925` (SheetJS),
`92bfc5f` (Gedächtnis).

### Was jetzt persistiert wird (Stand nach dieser Runde)
| Zustand | gespeichert? | wo |
|---|---|---|
| Panel-Breiten (links/rechts), Seitenleiste eingeklappt | **ja** | `localStorage` `ds-ide.ui.v2` |
| Work-Area sichtbar, offene Tabs, aktiver Tab, zuletzt geöffnete Datei | **ja** | `settings`-Tabelle, Schlüssel `ui.workarea` |
| Aktive Sitzung | **ja** | `settings`-Tabelle, Schlüssel `ui.activeSession` |
| Fenstergröße/-position/maximiert | **ja** | `.window-state.json` im Konfigverzeichnis |
| Arbeitsverzeichnis, Anzeigename, Modell, Toggles, Sprache | **ja** | `settings`-Tabelle |

- **Fenster**: `tauri-plugin-window-state`, aber mit Absicht nur
  `SIZE | POSITION | MAXIMIZED`. `VISIBLE` fehlt, weil sonst die Fensterlage vor dem
  Splash entscheidet, wann das Hauptfenster erscheint; das Splash-Fenster steht auf
  der Denylist, damit sein Rechteck nicht in die Datei wandert.
- **Sitzung/Tabs/Datei**: `src/lib/uiState.ts` schreibt `ui.*`-Schlüssel, entprellt
  und fehlertolerant — ein gemerkter Tab darf keinen Klick aufhalten. Der alte
  localStorage-Schlüssel `ds-ide.workarea` wird einmal importiert und gelöscht.
  Wiederherstellen repariert statt zu scheitern: unbekannte Tab-Arten, Duplikate
  und ein veralteter aktiver Tab werden verworfen, und der bereinigte Zustand wird
  zurückgeschrieben.

### Verifikation (nicht nur Build)
- Fenster: auf 1100×650 bei 200,180 gezogen, beendet, neu gestartet — kam genau dort
  zurück (X11-Lauf, `xdotool getwindowgeometry`).
- UI-Zustand: eine absichtlich falsche Sitzungs-ID wurde beim Start durch die echte
  neueste ersetzt (also schreibt die App wirklich), der alte localStorage-Schlüssel
  verschwand und wurde als `ui.workarea` in die Datenbank übernommen, und eine
  kaputte Tab-Liste kam als „progress + memory" mit repariertem aktivem Tab zurück.
- `cargo test`: 17 grün, 9 ignoriert. Neu: Stilllegen/Zurückholen-Runde, der
  `origin`-Marker, der Recall-Text und die Migration einer alten `memories`-Tabelle.

### Vorschauen
- **`pptx` ist jetzt lesbar**: `jszip` entpackt, `ppt/slides/slideN.xml` liefert
  Absätze; jede Folie wird eine Karte mit Titel und Text, numerisch sortiert,
  Folien ohne Text fallen weg. Ehrlich Text, kein Layout — es sieht aus wie eine
  Folienliste, weil es eine ist. Geprüft mit einem künstlichen Deck (escapte
  `<script>`-Tags, Emoji, textlose Folie, Folie 10 nach Folie 3) und mit einem
  echten 12-Folien-Canva-Export.
- **`xlsx` läuft auf dem offiziellen SheetJS-Build** (0.20.3 vom cdn, nicht npm).
  `pnpm audit`: vorher 5 Verwundbarkeiten (2 low, 1 moderate, 2 high — beide hoch
  betrafen `xlsx`), nachher 3 (2 low, 1 moderate). Die Tabelle rendert unverändert;
  geprüft mit einem echten Bewertungsraster. Upgrades kommen künftig von
  `cdn.sheetjs.com`, nicht aus der npm-Registry.
- **`doc`/`ppt`/`odp`** bleiben draußen — klare Meldung, kein Buchstabensalat.

### Companion-System
- Die Gedächtnis-Einsicht ist aus dem Work-Area-Tab nach **Einstellungen →
  Personalisierung** gewandert; Tab, Toolbar-Knopf und `MemoryTab.vue` sind weg.
- `memories` hat `origin` (`seeky` | `user`), `edited_at` und `retired_at`.
  Schreibt der Nutzer im Panel einen Eintrag um, wird er markiert; im Recall steht
  dann `(changed by the user)`, und die Persona erklärt, dass das die Worte des
  Nutzers sind. Korrigiert Seeky sich selbst, fällt der Marker wieder weg — er
  hängt am Text, nicht an der Zeile.
- **Nichts wird mehr gelöscht**: Stilllegen setzt `retired_at`, der Eintrag
  verschwindet aus Recall und Liste und liegt im Panel unter „Stillgelegt", von wo
  er zurückgeholt werden kann. Auch der Kurations-Lauf legt nur still.
- Persona und Recall sagen jetzt *Partner* und nennen die Bereiche in Worten
  (`[you]` / `[user]` / `[session]`) statt der Rohwerte aus der Datenbank.
- Geprüft: an der echten Datenbank kamen alte Zeilen als `seeky`-Einträge zurück,
  ein umgeschriebener Eintrag erschien als „von dir geändert" neben „Stillgelegt (1)",
  und die Schreibwege liefen über genau die `invoke`-Aufrufe des Panels.

### Lernnotiz: synthetische Eingaben erreichen diese App nicht
Der Rechner hier ist KDE Plasma auf Wayland. XWayland lässt den Zeiger nicht
bewegen (`xdotool mousemove` bleibt bei 1920,0 stehen), und `xdotool key` kommt
auch nach `windowfocus` nicht in der Webview an. Klicks und Tasten sind von außen
also nicht zu setzen. Konsequenz für Prüfungen: Zustand über die Datenbank setzen
und das Ergebnis im Screenshot ansehen, oder die Aufrufe des Panels aus der App
selbst auslösen (temporär, danach zurückgenommen). Die Knöpfe des Gedächtnis-Panels
sind deshalb *nicht* mit der Maus durchgeklickt — gesagt, damit es niemand für
erledigt hält, der es nicht ist.

### Offene Punkte
1. ~~**`doc`/`ppt`/`odp`** brauchen einen Konverter~~ — erledigt in Abschnitt 22.
2. Tabellen zeigen nur das **erste** Blatt, ohne Formeln und Formatierung.
3. Zwei Advisory-Reste außerhalb des Themas: `dompurify` in `monaco-editor` (low,
   gepatchte Version wäre ≥ 3.4.16) und ein transitives `sprintf-js` (moderate).
4. Gedächtnis: der Nutzer kann Einträge **bearbeiten und stilllegen, aber noch
   keinen neuen anlegen** — der Weg „Nutzer schreibt eine Notiz" fehlt.
5. ~~Gedächtnis-Einsicht sitzt unter *Personalisierung*~~ — erledigt in Abschnitt 22
   (eigene Gruppe des Begleiters); „Konto & Modell" ist unberührt geblieben.
6. Folien: reiner Text. Bilder, Notizen und Reihenfolge der Formen bleiben außen vor.

## 22. Der Begleiter bekommt ein Zuhause (2026-10-16, zweite Runde)

Vier Commits: `f5d53db`, `0ecde50`, `7505e1d` und dieser Doku-Commit. `0ecde50`
enthält zusätzlich den runden Splash — die Nachricht nennt ihn nicht, weil ich
ihn versehentlich mit `git add -A` mitgenommen habe; die Änderung selbst ist
`src/App.vue` + `tauri.conf.json` (transparentes Fenster, runde Scheibe).

### Was der Nutzer entschieden hat
- **„Personalisierung" ist weg** aus den Voreinstellungen. Das Wort bedeutet für
  Nutzer Look & Feel; jetzt gibt es eine eigene Gruppe, die nach dem Begleiter
  heißt, mit „Erinnerungen" (das Gedächtnis-Panel, unverändert) und „Begleiter".
- **Der Name gehört dem Begleiter.** Es bekommt eine Liste von Vorschlägen
  (`src/engine/names.ts`, 94 Namen) und darf sie ignorieren; das Werkzeug
  `choose_name` schreibt den Namen einmalig in die Datenbank. Danach verschwindet
  das Werkzeug aus der Liste, die Datenbank weist einen zweiten Versuch ab, und
  die Einstellungsgruppe heißt statt „SEEKY" dann der gewählte Name. Getestet:
  `db::tests::the_partner_name_is_chosen_once`.
- **Eigene Anweisungen** des Nutzers (Textfeld unter „Begleiter") landen als
  eigener Abschnitt im Systemprompt, ausdrücklich über den Vorlieben des
  Begleiters.
- **Über-Seite** unter Verwaltung: Version, Autoren (Ziggy Shēn, Modest Coder),
  Adressen, und die verwendeten Pakete in zwei Accordions. Versionen kommen aus
  den Manifesten selbst — beide werden zur Kompilierzeit in das Binary
  eingebettet (`include_str!`), damit die Liste nicht veralten kann.
  `tests::about_reads_both_manifests` prüft den Parser.
- **🌐 Webdesign raus**: der Eintrag öffnete dasselbe Browser-Panel, das die
  Werkzeugleiste schon hat.

### Modellnamen — mein Fehler, korrigiert
Ich hatte behauptet, `deepseek-flash` und `deepseek-v4-pro` existierten nicht, und
sie durch `deepseek-chat`/`deepseek-reasoner` ersetzt. Falsch: der Provider meldet
**genau diese zwei**, mit den Namen `DeepSeek-V4.1-Flash` und `DeepSeek-V4-Pro`
(plus Kontextfenster, `effort`-Stufen low/high/max und Eingabe-Modalitäten). Das
echte Problem war die **Anzeige**: wir zeigten die ID und nie den Namen. Jetzt
zeigt der Composer die Namen (ID beim Hovern) und die Einstellungszeile den Namen
unter dem Feld; gesendet wird weiter die ID. Die Rust-Voreinstellungen sagen nicht
mehr `deepseek-chat`. Vorauswahl: Alltag → Pro, Programmieren → Flash.

Gegengeprüft mit echten Aufrufen: `/models` direkt abgefragt, und die drei
Live-Tests laufen durch — Text (`PONG`), Werkzeugaufruf (`glob`), Denkmodell
(103 Zeichen Reasoning, Antwort 391).

### Rundes Splash
Fenster ist jetzt transparent und quadratisch (360×360), der Inhalt sitzt auf
einer runden Scheibe mit Ring und Schatten; die Ecken sind wirklich durchsichtig.
Per Screenshot des Fensters bestätigt (X11). Unter Wayland lässt sich das hier
nicht fotografieren: `grim` sagt „compositor doesn't support the screen capture
protocol".

### Lernnotiz: nicht aufräumen mit zu weitem Muster
Mein `ps | grep "target/debug/DSIDE" | xargs kill -9` hat **die laufende Instanz
des Nutzers** mitgerissen, weil sein `pnpm tauri dev` dieselbe Binärdatei startet.
Seine Sitzung war danach weg (Port 1420 frei). Lehre: vor dem Beenden von
Prozessen die Startzeit prüfen und nur treffen, was man selbst gestartet hat.
Zweite Lehre aus derselben Stunde: `tauri dev` startet die App auch dann, wenn
Vite am belegten Port scheitert — dann hängt eine Instanz an einem fremden
Vite-Server und zeigt fremden Code.

### Offene Punkte
1. **`effort`** (low/high/max, Vorgabe high) meldet der Provider pro Modell. Ob
   und wo wir das im Request setzen, ist ungeprüft — als Nächstes mit dem
   entsprechenden Feld testen, nicht raten.
2. **Bilder**: `input_modalities` nennt `image` für Flash. Anhänge/`paste` von
   Bildern wären damit möglich; bisher reicht der Client nur Text durch.
3. Name: einmalig heißt einmalig. Es gibt keinen Weg zurück über die Oberfläche —
   bewusst, aber wenn ein Name doch festsitzt, hilft nur ein Eingriff in
   `partnerName` in der `settings`-Tabelle.
4. Der Weg „Nutzer legt selbst eine Erinnerung an" fehlt weiterhin (aus §21).
5. Splash: unter Wayland ungeprüft (siehe oben).

## 23. Denktiefe, Tabellenblätter, eigene Erinnerungen (2026-10-16, dritte Runde)

Commits `b4e9e06` und `9c48f04`, dazu dieser Doku-Commit.

### Modell — Korrektur meiner Korrektur
In Abschnitt 22 stand, die IDs `deepseek-flash`/`deepseek-v4-pro` existierten
nicht. Falsch. Der Provider meldet genau diese zwei; falsch war nur, dass wir
die **ID** zeigten und nie den **Namen**. Jetzt:
- Der Einstellungswert ist eine Auswahlliste mit dem Anbieternamen
  (`DeepSeek-V4.1-Flash`), darunter steht die ID, die tatsächlich gesendet wird.
  Eine unbekannte ID (anderer Anbieter) schaltet auf ein freies Textfeld.
- Rust-Voreinstellungen sagen nicht mehr `deepseek-chat`.

### Denktiefe (`reasoning_effort`)
Der Provider nimmt ein Feld `reasoning_effort` mit den Werten
`none, minimal, low, medium, high, xhigh, ultra, max`, und meldet pro Modell,
welche Stufen es unterstützt (hier beide: `low, high, max`, Vorgabe `high`).
Direkt nachgemessen: Vorgabe → 129 Zeichen Reasoning, `low` → 42. Deshalb kommt
die Auswahl aus der Modellliste statt aus einer Vermutung; leer heißt
„Anbieter-Vorgabe". Neue Einstellung **Denktiefe** unter Konto & Modell, plus
Live-Test `live_deepseek_accepts_reasoning_effort` (ein falscher Feldname oder
Wert wäre ein 422).

### Tabellen: alle Blätter
`renderSheet` las nur `SheetNames[0]` — bei einem Arbeitsblatt mit Noten auf
Blatt 2 war das eine stille Lüge. Jetzt wird jedes Blatt nacheinander mit Namen
gerendert; die Zeilen pro Blatt schrumpfen mit der Zahl der Blätter
(Zellbudget), leere Blätter fallen weg, und ein Hinweis nennt nicht gezeigte
Blätter. Geprüft mit einem selbstgebauten Drei-Blatt-Fixture.

### Erinnerungen: der Nutzer darf selbst schreiben
Bisher konnte der Nutzer nur korrigieren und stilllegen. Jetzt gibt es ein Feld
im Panel (`memory_add`, `origin = user`, `edited_at` leer) und der Recall
unterscheidet **„(written by the user)"** von **„(changed by the user)"** — ein
selbst geschriebener Satz ist etwas anderes als eine Korrektur. Im Panel steht
dafür „von dir geschrieben" bzw. „von dir geändert".

### Verifikation
19 Rust-Tests, 10 ignoriert (davon 4 live gegen den echten Anbieter, alle grün);
`pnpm build` grün. Sichtbar geprüft per Screenshot: Modellfeld zeigt
`DeepSeek-V4.1-Flash`, neue Zeile „Denktiefe", Erinnerungsfeld mit
„von dir geschrieben". Dafür wurde kurzzeitig der Standard-View auf
Einstellungen/Erinnerungen gesetzt und danach zurückgenommen — im Baum ist
davon nichts übrig (`git status` sauber).

### Offene Punkte
1. **Bildanhänge**: Flash meldet `image` als Eingabe. Der Client schickt nur
   Text; Anhänge brauchen UI (Einfügen/Auswählen), Ablage und das
   Inhaltsformat im Request. Noch nicht angefasst.
2. **Konsole**: Der Partner hat eine Shell (`bash`-Werkzeug), aber der Nutzer
   sieht nur den Werkzeug-Chip im Chat, kein Terminal. Vorschlag: sichtbares
   Konsolen-Panel (Ausgabe der Werkzeugaufrufe) zuerst, interaktives Terminal
   (pty + xterm.js) danach — der Nutzer will es oben links neben Browser und
   Dateibrowser.
3. **„Alltag"-Modus**: nur die Modellvorauswahl ist umgesetzt; welche Werkzeuge
   und Panels der Modus ausblendet, ist noch nicht bestätigt.
4. Einstellungen dürfen nicht über die volle Breite laufen (freier Platz rechts)
   — der Nutzer überarbeitet das Layout selbst.
5. Zweite Runde Prozess-Aufräumen: `background_process stop` beendet nicht immer
   den ganzen Baum (`tauri dev` → vite + App blieben stehen) und blockierte
   danach Port 1420. Seitdem: PIDs explizit sammeln und `kill -9`.

## 24. Eine Antwort ist eine Zeitleiste (2026-10-16, vierte Runde)

Commit `35ac5c0`.

### Der Bericht des Nutzers, in seiner Reihenfolge
1. bash-Aufrufe zuerst sichtbar.
2. dann die Antwort **über** der Anzeige der bash-Aufrufe.
3. danach eine Antwortenauswahl (`ask_user`), User-Prompt deaktiviert.
4. Eingabe der freien Antwort im Frage-Menü.
5. danach hängen sich neue bash-Aufrufe **unter** die bestehenden, und die
   Textantwort hängt sich an die bestehende Antwort an, statt eine neue zu sein.

### Ursache: das Datenmodell, nicht der Renderer
`ChatMessage` hatte `text` (String), `reasoning` (String) und `tools` (Array) —
drei getrennte Felder ohne Reihenfolge. Die Liste zeichnete immer DENKEN → Text →
Werkzeuge. Alles, was später kam, landete dadurch falsch: späterer Text über den
früheren Werkzeugen, und eine zweite Runde wuchs in denselben String.

Jetzt ist eine Antwort eine **geordnete Liste von Teilen**
(`MessagePart`: `reasoning` | `text` | `tool`), gefüllt in der Reihenfolge der
Ereignisse, und die Liste rendert genau diese Liste. Geprüft live in einer
frischen Sitzung mit einem echten Turn:
`DENKEN → ✔ bash fertig 1773ms → DENKEN → Text` — dieselbe Reihenfolge, die in
der Datenbank steht.

### Zwei weitere Fehler, die beim Prüfen auffielen
- **Turns außerhalb des Composers waren unsichtbar.** Ein geplanter Auftrag
  schreibt direkt über `send_prompt`; geöffnet wird eine Antwort aber nur im
  Frontend-Pfad (`beginAssistantMessage`). Die Ereignisse des Laufs kamen also
  an, fielen aber weg — die Antwort war erst nach einem Neustart zu sehen. Jetzt
  öffnet das erste Ereignis eines Turns die Antwort (`ensureAssistantMessage`).
- **Der Verlauf hatte mehr Blasen als die Live-Ansicht**, weil die Datenbank pro
  Runde eine Assistentenzeile schrieb. Jetzt ist ein Turn eine Zeile, vorab
  angelegt und beim Wachsen aktualisiert (`messages.parts`). Alte Zeilen haben
  keine Zeitleiste und fallen auf Text-dann-Werkzeuge zurück.

### Was der Nutzer dazu wissen sollte
Ein Turn, der auf `ask_user` wartet, ist **derselbe** Turn: die Antwort setzt
ihn fort, deshalb läuft der Text nach der Auswahl in derselben Antwort weiter —
nur jetzt an der richtigen Stelle in der Zeitleiste. Wenn daraus eine eigene
Antwort werden soll, ist das eine bewusste Änderung am Ablauf, kein Fehler.

### Offene Punkte
1. Ein abgebrochener Turn (App beendet mitten im Lauf) hinterlässt eine leere
   Assistentenzeile — sichtbar als leere Blase. Sollte beim Laden übersprungen
   werden.
2. Der Nutzer kann den Verlauf nicht durchsuchen/springen; bei langen Antworten
   hilft nur Scrollen.
3. Bildanhänge (offen aus §23) — die kaputte Zwischenablage im CachyOS des
   Nutzers ist ein Systemthema (Klipper/Wayland nach dem Update), nicht DS.IDE.
4. Konsole (offen aus §23).

### Lernnotiz
Verifikation ohne Tastatur: den echten Turn über die `schedules`-Tabelle
auslösen (`next_run_at` in die nahe Zukunft, Sitzung = aktive Sitzung), die App
starten und den Chat fotografieren. Damit lief ein vollständiger Turn mit
Werkzeugaufruf durch, ohne dass jemand tippen musste. Wichtig dabei: den Auftrag
**ohne Backticks** in die Datenbank schreiben — die Shell frisst sie sonst als
Befehlssubstitution und der Prompt kommt verstümmelt an (was der erste Testlauf
prompt auch getan hat; das Modell hat es bemerkt und angesagt).

## 25. Optik-Runde: Werkzeugzeilen, Kontext, Markdown (2026-10-16, fünfte Runde)

Commit `ddf70e0`. Alles hier kam aus den Screenshots und Beobachtungen des Nutzers.

### Antworten auf seine zwei Fragen
1. **Gab es eine Kontext-Anzeige?** Nein. Die `usage`-Events kommen aus dem Rust-Kern
   (`prompt_tokens`, `completion_tokens`, `total_tokens` pro Runde), das Frontend hat sie
   aber nie gelesen — sie fielen im `handleEvent` in den `default`-Zweig. Und **eine
   Kompression gibt es gar nicht**: der Verlauf wird bei jeder Anfrage vollständig
   mitgeschickt, nichts wird zusammengefasst. Anklicken konnte man also nie etwas.
2. **Der Chevron im Titel?** Tote Dekoration aus dem MiniMax-Layout, ohne Handler. Das
   hat er richtig gespürt.

### Was jetzt da ist
- **Werkzeugzeilen sind aufklappbar.** Zugeklappt: Name, Status, Dauer. Aufgeklappt: der
  Befehl, seine Ausgabe, und — wenn „Volle Berechtigung" an ist — der Satz „Ohne
  Rückfrage ausgeführt". Genau der Verdacht („bash-Zugriff ohne Kontrolle?!") wird damit
  beantwortet statt verdeckt. Vorlage war Kilos Darstellung.
- **Eine Spalte, 980 px.** Der Chat ist zentriert und begrenzt, die Eingabe wächst auf bis
  zu zehn Zeilen (vorher eine), der Nutzer bekommt 75 % der Spalte, die Antwort alles.
- **Antworten werden als Markdown gerendert** (über `renderMarkdown`, also durch
  DOMPurify; einfache Zeilenumbrüche zählen). Das war der Nebenfund: In seinem Screenshot
  stand wörtlich `**25 Dateien**` und ein sichtbarer ```sh-Block. Die Worte des Nutzers
  bleiben unverändert stehen.
- **Sitzungstitel** in der Seitenleiste enden mit „…" statt auszulaufen.
- **Der Titel ist jetzt der Sitzungswechsler**: Klick öffnet die Liste, Klick wechselt.
  Wichtig, wenn die Seitenleiste eingeklappt ist — dann gab es vorher gar keinen Weg.
- **Kontext-Ring** im Composer: Anteil, den die letzte Anfrage vom Kontextfenster belegt
  (`prompt_tokens` aus dem Anbietersignal, Fenstergröße aus der Modellliste). Mouseover
  nennt Token und Prozent, der Wert überlebt einen Neustart (`ui.lastUsage`). **Klickbar
  ist er nicht** — er würde eine Kompression auslösen, die es nicht gibt (§25.1).

### Offene Punkte
1. **Kompression** (manuell per Klick auf den Ring, oder automatisch ab einem Schwellwert)
   — eigenes Vorhaben: Verlauf zusammenfassen, Zusammenfassung speichern und statt der
   alten Nachrichten mitschicken. Der Ring ist die Anzeige dafür.
2. **Bildanhänge**: Der Composer hat einen Anhang-Knopf ohne Funktion. Gewünschte Form
   steht fest (Anhänge **über** der Textarea, Textarea wächst darunter auf zehn Zeilen).
3. „Alltag"-Modus: Werkzeugliste weiterhin unbestätigt.
4. Ein abgebrochener Turn hinterlässt eine leere Blase (§24).
5. Zwischenablage des Nutzers: Klipper/Wayland nach dem CachyOS-Update, vermutlich auch
   Flameshot beteiligt — Systemthema, nicht DS.IDE. Bilder kamen zeitweise an.

### Lernnotiz
Aufklappbare Oberflächen lassen sich ohne Maus prüfen, wenn man `isOpen()` kurz auf `true`
stellt, fotografiert und zurücknimmt — im Baum bleibt nichts zurück (geprüft mit
`grep TEMP-VERIFY`). Und: Aufträge für den Scheduler **ohne Backticks** in die Datenbank
schreiben, die Shell frisst sie als Befehlssubstitution.

## 26. Der Name ist gewählt: Ziggy (2026-10-16, sechste Runde)

Commit `fb203af`.

### Woher „Ziggy" kommt — aus der ersten Sitzung, vom Nutzer
Zitat aus dem Transkript vom 2026-10-15 (Sitzung „Neues Tauri Projekt erstellen"):

> …Dein Name ist **»Seeky« oder »Ziggy«** (Seeky direkt von DeepSeek abgeleitet —
> Ziggy, phonetisch ziemlich ähnlich zu Seeky, ist ein menschlicher Name) …

„Seeky" wurde danach der Platzhalter (er steckt bis heute in Persona, UI und in
dieser Datei). „Ziggy" war nie eingetragen, weil es das `choose_name`-Werkzeug
damals noch nicht gab. Jetzt gibt es das Werkzeug, und der Nutzer hat den Namen
in dieser Runde ausdrücklich übergeben („Die ist für Dich") — **der Begleiter
heißt also ab jetzt Ziggy**:

- `settings.partnerName = "Ziggy"` (genau die Zeile, die `choose_name` schreibt;
  einmalig, die Datenbank weist einen zweiten Versuch ab).
- Die Einstellungsgruppe rechts heißt jetzt **ZIGGY**, die Begleiterseite trägt
  den Namen, und die Persona sagt „You chose the name Ziggy for yourself".
- Rückgängig nur per Hand: `delete from settings where key='partnerName'` — es
  gibt bewusst keinen Weg über die Oberfläche.

### Die Vorschlagsliste ist jetzt sichtbar
Die 94 Namen lagen bisher ausschließlich im Systemprompt (`src/engine/names.ts`)
— der Nutzer konnte sie nicht sehen („welche Liste?"). Sie stehen jetzt auf der
Begleiterseite als Abschnitt **Vorschläge**: angeklickt-freie Chips, der
gewählte Name hervorgehoben, mit dem ehrlichen Satz, dass die Auswahl ein
Vorschlag ist und ein eigener Einfall ausdrücklich erlaubt bleibt.

### Offene Punkte
Unverändert §25 (Kompression hinter dem Ring, Bildanhänge, Alltagsmodus,
leere Blase nach Abbruch). Neu: Wird irgendwann ein anderer Name gewünscht, ist
das eine bewusste Datenbankänderung, kein Menüpunkt — so war es bestellt.

## 27. Namen mit Bedeutung (2026-10-16, siebte Runde)

Commit `f7478be`. Aus der Rückfrage des Nutzers: „nomen est omen" — Namen sind
keine Bezeichnungen, sie haben eine Bedeutung, und eine Liste hübscher Silben
macht die Wahl beliebig.

### Was geändert wurde
`src/engine/names.ts` ist aus einer Liste von Zeichenketten zu **Daten mit
Bedeutung** geworden: `{ name, origin, de, en }`. 96 Einträge, jeder mit
Herkunftssprache und einer kurzen Bedeutung auf Deutsch und Englisch (den zwei
Sprachen, in denen dieses Projekt benutzt wird; die übrigen vier
Sprachkataloge fallen auf Englisch zurück — bewusst, das wäre sonst 96 × 6).

- Die **Persona** übergibt die Liste als `Name (Herkunft) — Bedeutung` und sagt
  ausdrücklich: nach Bedeutung wählen, nicht nach Klang. Wo eine Etymologie
  strittig ist, steht das dabei („unsicher", „origin disputed") statt erfundener
  Sicherheit.
- Die **Begleiterseite** zeigt die Bedeutung des gewählten Namens und auf jedem
  Vorschlag die Bedeutung beim Überfahren.
- **Ziggy** ist selbst ein Eintrag geworden: „Koseform von Siegfried/Sigmund:
  Sieg und Schutz". Damit steht auch hinter dem gewählten Namen eine Bedeutung,
  und die Liste markiert ihn als vergeben.

### Warum die Daten NICHT von der Namensseite kommen
Der Nutzer hat die Beispielseite verlinkt (und später eine Seite ihrer Daten
eingefügt). Diese Tabellen sind eine **geschützte Sammlung** (FUNKE
Mediengruppe) — sie zu übernehmen wäre das Kopieren einer Datenbank. Zweitens
ist die Qualität durchwachsen, was die Seite selbst zeigt: leere
Bedeutungsspalten, „Bedeutung nicht sicher", Varianten und Sonderzeichen. Eine
Tatsache über einen Namen lässt sich in eigenen Worten sagen; 96 davon sind
geschrieben, hunderttausend nicht.

### Wenn es doch eine große Namensdatenbank werden soll
Zwei Wege, beide ehrlich:
1. **Lizenzierte Quelle** (Wiktionary/Wikipedia-Dumps unter CC BY-SA mit
   Namensnennung, oder ein CC0/MIT-Datensatz von GitHub) — dann mit
   Quellenangabe auf der Über-Seite.
2. **Kuratiert bleiben** und nur wachsen, wenn ein Name wirklich zur Wahl steht.
Für den Zweck, den die Liste hat (der Begleiter wählt **einen** Namen), ist (2)
richtig. Für einen Namensgenerator für den Nutzer wäre (1) der Weg — das ist
dann aber ein eigenes Vorhaben mit eigener Oberfläche.

### Offene Punkte
Unverändert §25/§26 (Kompression hinter dem Ring, Bildanhänge, Alltagsmodus,
leere Blase nach Abbruch, vier Sprachkataloge ohne Namensbedeutungen).

## 28. Die große Namenstabelle (2026-10-16, achte Runde)

Commit `c2c992a`. Der Nutzer hat eine Namensseite als HTML gespeichert
(`src/assets/namen.html`) und daraus eine Tabelle verlangt — Duplikate
ausdrücklich **nicht** aussortiert.

### Was in der Datei steckt
Spalten: **Name | Herkunft | Bedeutung | Beliebtheit | Trend**. 947 Zeilen.
- **Beliebtheit** steht nicht als Text in der Datei, sondern ist die Breite eines
  farbigen Balkens in einem Inline-SVG (Skala 50) — daraus als Prozentwert
  rekonstruiert (alle 947 Zeilen).
- **Trend** ist ein `<canvas>`, wird also erst im Browser gezeichnet. Die Daten
  sind nicht im HTML und bleiben in unserer Tabelle leer.
- Die **Bedeutungen sind Stichworte** (Krieger, Gesund, Prinzessin, Erde, Lieb,
  Engel …), keine Etymologien: 121 verschiedene, jede Zeile hat eine.

### Was jetzt im Baum liegt
- `src/assets/namen.tsv` — zum Lesen und Vergleichen (36 KB, 948 Zeilen mit Kopf).
- `src/assets/namen.json` — für Code (100 KB).
- `scripts/namen_import.py` — das Werkzeug, das beides erzeugt.
  `--append` hängt weitere Seiten an und wirft nur exakte Duplikate raus;
  `--report` sagt nur, was drinsteht. Weitere Seiten des Nutzers sind damit ein
  Befehl statt Handarbeit.
- Ergebnis: 947 Zeilen, **808 verschiedene Namen**, **122 mehrfach** (bewusst
  behalten), alle mit Bedeutung und Beliebtheit.
- Die Rohdatei bleibt **untracked**: 1,7 MB Inline-SVG, deren einziger Zweck die
  Eingabe für das Skript ist.

### Offen: Herkunft der Seite
Das Fragment enthält keine Domain und keinen Titel (nur relative Links), die
Quelle ist also nicht benennbar. Solange sie nicht geklärt ist, ist die Tabelle
**Arbeitsmaterial** und wird nicht in die Anwendung gebündelt — Attribution und
Lizenz gehören vorher geklärt, so wie es in §27 für die andere Seite schon
entschieden wurde.

### Nächster Schritt für die Namen (Vorschlag)
947 Namen dürfen **nicht** in den Systemprompt: das sind grob 5.000–8.000 Token,
und zwar bei *jeder* Anfrage. Besser:
1. Ein Werkzeug, das suchen kann — etwa `name_ideas(meaning, origin)` über die
   Tabelle, damit der Begleiter selbst stöbern kann („zeig mir Namen, die Licht
   bedeuten").
2. Im Prompt bleibt eine **kurze** Auswahlliste (die 96 kuratierten Einträge aus
   §27), die auf die Tabelle verweist.
3. Optional später: eine Namensseite **für den Nutzer** mit Suche/Filter — das
   wäre dann der Ort, an dem 947 Einträge hingehören.

### Offene Punkte
Unverändert §25–§27, dazu: Bedeutung der kuratierten Liste und der Stichwort-
Datensatz sind zwei Formate — beim Zusammenführen entscheiden, welches gewinnt
(Stichwort ist maschinenfreundlich, Etymologie ist lesbarer).

## 29. Der Namenspool gehört dem Begleiter (2026-10-16, neunte Runde)

Commit `f772a32`. Der Nutzer hat die Lizenzfrage geradegerückt: **Namen und ihre
Bedeutungen sind Tatsachen**, kein geschütztes Werk — man entscheidet, wie man
seine Kinder nennt, und stellt es dann fest. Meine Vorsicht war außerdem
inkonsequent (die Namen lagen zu dem Zeitpunkt längst als JSON im Baum). Also
ohne Vorbehalt weiterverwendet; die Rohseite bleibt trotzdem untracked, nicht aus
Sorge, sondern weil 1,7 MB Inline-SVG nichts im Repository zu suchen haben.

### Drei Entscheidungen des Nutzers, umgesetzt
1. **Beliebtheit entfernt** — aus der Tabelle und aus dem Importskript. Ein Name
   ist nicht deswegen gut, weil ihn viele tragen; die Zahl würde die Wahl
   schieben. (Der Trend war ohnehin nie im HTML: ein `<canvas>`.)
2. **Der Nutzer sieht den Pool nicht.** Der Abschnitt „Vorschläge" ist von der
   Begleiterseite verschwunden; dort stehen nur der gewählte Name und seine
   Bedeutung. Begründung des Nutzers, wörtlich: sonst kommt „Aber warum willst Du
   denn 'Selma' heißen, 'Marlow' steht doch auch in der Liste…". Der Name ist die
   Entscheidung des Begleiters allein — die Oberfläche lädt nicht zur Debatte ein.
3. **Nur ein Vermerk für die KI.** Die 96 Vorschläge sind **aus dem Systemprompt
   raus**; dort steht jetzt der Hinweis, dass es einen Pool gibt und wie man ihn
   benutzt. Neue Werkzeuge:
   - **`name_ideas(meaning, origin)`** durchsucht ~950 Namen (Tabelle zur
     Kompilierzeit eingebettet, `search_names` in `lib.rs`) und liefert
     `Name (Herkunft) — Bedeutung`. Getestet: Poolgröße > 900, Filter nach
     Bedeutung und Herkunft, leeres Ergebnis, Limit.
   - **`choose_name` hat eine Bremse**: Es verweigert, solange der Begleiter
     weniger als **fünf Nachrichten** von dieser Person gehört hat, und sagt
     dabei, warum (`NAME_MIN_USER_MESSAGES`, `db.count_user_messages()`).

### Die offene Frage des Nutzers: „sollten sie das?“
Seine Sorge: eine frisch installierte Instanz greift sich in der ersten Minute
einen Namen. Antwort im Code: **ja, sie sollen wählen dürfen — aber nicht sofort.**
Ein Name, der 30 Sekunden nach der Installation entsteht, ist beliebig; nach
*seiner* eigenen Regel (nomen est omen) muss ein Name etwas bedeuten, und dafür
muss man den Menschen kennen. Deshalb eine Bremse statt einer Regel: das Werkzeug
sagt „noch zu früh", der Platzhalter bleibt einfach stehen. Es gibt keine Frist
und keinen Zwang — „Seeky" zu bleiben ist ausdrücklich eine gültige Wahl.

### Offene Punkte
- Bedeutung meiner 96 kuratierten Einträge (Etymologie, Deutsch/Englisch) und die
  Stichworte des Pools sind zwei Formate. Aktuell: Pool für die Wahl, kuratierte
  Liste nur noch als Nachschlagewerk für die Anzeige des gewählten Namens.
- §25–§28 unverändert (Kompression, Bildanhänge, Alltagsmodus, leere Blase).

## 30. Aufgeräumt: die Vorlagen sind raus (2026-10-16, zehnte Runde)

`minimax-code-main/` (74 MB), `minimax-jarvis/` (540 KB) und `Ubuntu/` (2,4 MB)
sind gelöscht — rund 77 MB. Geprüft, dass nichts darauf zeigt:

- Im Code gab es nur **Kommentare**, die den gespiegelten Vertrag nennen
  (`engine/types.rs`: Mavis-Runtime; `persona.ts`: bewusst nicht kopiert). Kein
  Import, kein Ladepfad, nichts im Build.
- `Ubuntu/` war **byte-identisch** mit `public/fonts/` (App lädt `/fonts/…`),
  inklusive `UFL.txt` daneben.
- Die Attribution hängt seit §30-Commit an der **Über-Seite** (Vorbild: MiniMax
  Code, MIT; Schrift: Ubuntu Font Licence), nicht am Ordner.

### Wo die Vorlagen zu finden sind, wenn sie gebraucht werden
- `minimax-code-main` → Kopie in `~/Projects/[SAVED]/minimax-code-main`
  (4270 Dateien, identisch) und als `minimax-code-main.zip` (26,7 MB) daneben;
  online <https://github.com/MiniMax-AI/minimax-code>.
- `minimax-jarvis` (Agent-Definitionen) → dasselbe Repo, Pfad
  `packages/local-runtime-v2/assets/agents/`.
- `Ubuntu/` → die Schriften liegen weiter in `public/fonts/`.
- `kilocode-main/` (140 MB) **bleibt vorerst**: es ist die Vorlage für die
  Kompression (<https://github.com/Kilo-Org/kilocode>) und wird danach ebenfalls
  entfernt.

## 31. Die sechs offenen Punkte (2026-10-16, elfte Runde)

Die Liste aus §25–§29 wird als geführtes Vorhaben abgearbeitet, klein zuerst.
Stand: 1–4 erledigt, 5 und 6 offen.

### 1. Leere Blase nach Abbruch — `79e1d18`
Ein Turn, der mitten im Lauf beendet wurde, hinterlässt eine Assistentenzeile,
die angelegt, aber nie gefüllt wurde; nach dem Neuladen stand dort eine leere
Blase. Zeilen ohne Text, ohne Werkzeuge und ohne Zeitleiste werden beim Laden
übersprungen (`hasSomethingToSay` in `useEngine.ts`).

### 2. Alltagsmodus wirkt — `a71790f`
`settings.mode` wurde geschrieben und nirgends gelesen. Jetzt hat die Wahl Folgen,
und sie stehen an **einer** Stelle (`src/engine/mode.ts`):
- Werkzeuge des Begleiters: `bash`, `code_review`, `task` **weg** — doppelt
  abgesichert: sie werden dem Modell nicht mehr angeboten (der Rust-Kern filtert
  sie aus der Definitionsliste) *und* ein trotzdem eintreffender Aufruf wird
  abgelehnt statt still ausgeführt.
- Panels: Plan-, Skill- und Geplantes-Tabs aus, ebenso die Einträge in der
  Seitenleiste; ist der offene Tab einer davon, wechselt die Werkbank auf die
  Dateiansicht statt leer zu bleiben.
- Dateien öffnen als **schlichter Text** statt im Code-Editor; der Editor-Knopf
  in der Werkzeugleiste verschwindet.
- Die Modus-Karten sagen jetzt, was sie tun (sechs Sprachen).

### 3. Skills im Programm schreiben — `3ddb60a`
Das Skill-Panel konnte nur auflisten. Jetzt lädt ein Klick den `SKILL.md` in ein
Formular (Name, ein Satz wozu, Anleitung) oder legt einen neuen an; Schreiben ist
dasselbe wie Bearbeiten, weil ein Skill eine Datei ist. Der Name wird zum Ordner,
der Slug entschärft, damit ein Name den Skills-Ordner nicht verlassen kann — mit
Test dafür und einem Test, der einen Skill in einen temporären Ordner schreibt
und samt Frontmatter wieder einliest. 23 Rust-Tests.

### 4. Die zwei Advisory-Reste — `6313008`
`pnpm audit`: **3 → 1**.
- **Behoben:** `dompurify` 3.4.15 (die Kopie in `monaco-editor`) wird per
  Override auf die gepatchte 3.4.17-Linie gezwungen, die dieses Projekt ohnehin
  benutzt. Jetzt eine Version im Baum.
- **Nicht behebbar, ehrlich dokumentiert:** `sprintf-js` („denial of service
  through unbounded precision specifiers"). Das Advisory betrifft **jede
  veröffentlichte Version** („patched: none"), es gibt also keinen Fix zum
  Einspielen. Der Pfad ist `mammoth → argparse → sprintf-js`, und `argparse` ist
  Mammoths **Kommandozeilen**-Abhängigkeit: der von uns importierte Einstieg
  (`lib/index.js`) lädt sie nie, und im gebauten Bundle steht kein argparse-Code.
  Bleibt also im Baum, ist aber nicht ausgeliefert.

### 5. Bildanhänge — `cbd1f26`
Einfügen oder Auswählen im Composer, Anhänge **über** der Eingabe (wie gewünscht),
jeweils mit Entfernen-Knopf.

- **Der Request-Vertrag wurde vorher geprüft, nicht geraten:** Der Anbieter ignoriert die
  OpenAI-`image_url`-Inhaltsteile. Er nimmt ein eigenes Feld `image` mit Data-URLs neben
  dem Text. (Der erste Fehlversuch lag nur daran, dass ein 1×1-Pixel-Bild als
  „unsupported image" abgelehnt wird — die Form war richtig.)
- **Ablage:** `attachment_save` schreibt die Datei nach `<app-data>/attachments/` und gibt
  den Pfad zurück; die Endung kommt aus dem **MIME-Typ**, nicht aus dem Dateinamen. Der
  Verlauf speichert den Pfad, nicht die Bytes — ein Chatprotokoll darf nicht pro Bild um
  Megabyte wachsen.
- **Anzeige:** eine verkleinerte Kopie (Canvas, max. 480 px, JPEG) reist als `preview` mit
  der Nachricht mit; die Chat-Ansicht zeigt sie, das Modell bekommt das Original.
- **Zeitleiste:** Bilder sind Teile der Nachricht (`kind: "image"`), neben Text und
  Werkzeugen — sie überleben das Neuladen, sitzen an der richtigen Stelle und gehen bei
  späteren Turns wieder mit, weil der Verlauf aus der Datenbank kommt. Eine gelöschte
  Datei wird übersprungen statt den Turn zu brechen.
- **Verifiziert:** neuer Live-Test `live_deepseek_takes_an_image` baut ein PNG im Test,
  schickt es durch unseren eigenen Pfad und bekommt „BILD" zurück (5 Live-Tests grün).

### 6. Kompression — `51cb160`
Der Ring zeigte die Füllung, aber nichts hat sie je geleert. Jetzt wird der ältere Teil
eines Gesprächs zu einer Zusammenfassung verdichtet, die das Modell selbst schreibt; ab
dann wird **sie** verschickt statt der Nachrichten, die sie abdeckt.

- **Automatisch**, wenn ein Turn mehr als **75 %** des Kontextfensters (`context_window`,
  aus der Modellliste der Oberfläche) überschreiten würde — und **auf Klick** über den Ring,
  der jetzt ein Knopf ist und meldet, was er getan hat („N Nachrichten verdichtet" bzw.
  „Nichts zu verdichten").
- **Speicherung:** eigene Tabelle `session_summaries` mit der ID der letzten abgedeckten
  Nachricht. Wiederholtes Verdichten faltet die vorige Zusammenfassung mit hinein, statt sie
  zu verlieren; das **Protokoll in der Datenbank bleibt vollständig** — nur das Gesendete
  ändert sich.
- **Der Verdichtungsaufruf** läuft mit derselben Konfiguration, aber **ohne Werkzeuge**: er
  soll nichts ausführen und nichts fragen, nur zusammenfassen.
- **Verifiziert:** `db::tests::a_summary_replaces_the_messages_it_covers` (Zusammenfassung
  ersetzt genau das, was sie abdeckt; der Verlauf bleibt vollständig) und der Live-Test
  `live_deepseek_summarises_a_conversation` (die Form des Aufrufs wird vom Anbieter
  angenommen und liefert eine Zusammenfassung). 24 Tests grün, 12 ignoriert (live).
- **Eine Falle, die der Test durch Hängen gefunden hat:** die Zusammenfassung über die
  Verbindung zu lesen, die eine Methode schon hält, muss mit *demselben* Lock passieren.
  Der Aufruf des sperrenden Getters darin hat die ganze Datenbank verklemmt.

### Aufgeräumt: `kilocode-main` (140 MB) ist weg
Wie in §30 angekündigt, nach Abschluss der Kompression gelöscht — die Vorlage liegt unter
<https://github.com/Kilo-Org/kilocode>. Der Eintrag in `.gitignore` bleibt, damit nichts
davon versehentlich wieder eingecheckt wird, falls es nochmal lokal landet.

## 32. Fertig gestellt: Reste und ein Release (2026-10-16, zwölfte Runde)

Commit `4f04087`. Der Nutzer fragte, ob ich „das" heute noch fertig stellen will —
gemeint war der Kleinkram aus §31 und ein Release.

### Die beiden Reste
- **Namensbedeutung in allen Sprachen:** Die Bedeutung des *gewählten* Namens kam aus
  dem Namenspool, den es nur auf Deutsch und Englisch gibt — in vier Sprachen sprang
  also Englisch ein. Für Namen, die tatsächlich geführt werden, kommt sie jetzt aus den
  Sprachdateien (`partner.meaningZiggy` in sechs Sprachen); der Pool bleibt der
  Rückfall für Namen ohne Übersetzung. Neue Namen kosten damit einen Schlüssel statt
  sechs Datenfelder.
- **Formeln in Tabellen:** Der Betrachter zeigte nur den *Wert* einer Zelle, nie die
  Formel dahinter. Jetzt gibt es den Umschalter **Werte / Formeln**, an derselben Stelle
  wie beim Markdown „Vorschau / Quelltext"; das Umschalten liest die Datei nicht neu.
  Gegen eine Arbeitsmappe mit `SUM` und abgeleiteter Zelle geprüft: 1580/790 bzw.
  `=SUM(B2:B3)` / `=B4*0.5`.
  (Nebenbei: Das erste Fixture war falsch — die Formeln lagen außerhalb des
  angegebenen `!ref`-Bereichs. Der Fehler lag im Test, nicht im Code.)

### Release — es gibt jetzt Artefakte
`pnpm tauri build` (Release-Profil, 3m06s) erzeugt drei Pakete:

| Paket | Größe |
|---|---|
| `DS.IDE_0.1.0_amd64.deb` | 10,95 MiB |
| `DS.IDE-0.1.0-1.x86_64.rpm` | 10,95 MiB |
| `DS.IDE_0.1.0_amd64.AppImage` | 117,31 MiB |

Liegen in `src-tauri/target/release/bundle/` (gitignored). Das `.deb` ist geprüft:
Paket `ds-ide` 0.1.0, `usr/bin/DSIDE`, hicolor-Icons, Abhängigkeiten
`libwebkit2gtk-4.1-0, libgtk-3-0` — also die üblichen Laufzeitbibliotheken.
Das **Release-Binary wurde gestartet**: es bootet mit eingebettetem Frontend (kein
Vite), zeigt Wal-Icon, Seitenleiste und den geladenen Verlauf. Die App ist installierbar.

### Was für „richtige" Updates noch fehlt
`tauri-plugin-updater` braucht einen **Signaturschlüssel** und eine **Adresse**, unter der
die Updates liegen. Beides gibt es nicht (kein Server, kein Schlüssel) — deshalb ist es
nicht eingebaut, statt einen Knopf zu bauen, der ins Nichts zeigt. Offen bleibt außerdem
das Frontend-Bundle: `index-*.js` ist 5,4 MB (Monaco + pdf.js), der Build warnt zu Recht
über die Chunk-Größe; Aufteilen wäre eine eigene Runde.

## 33. Die Optik-Fehlersuche des Nutzers (2026-10-16, dreizehnte Runde)

Commit `4a21df2`. Der Nutzer hat selbst am CSS gearbeitet (Bubble-Ränder, 70 %
Breite, `pre-wrap` entfernt, `word-break` ergänzt, Avatare auskommentiert) und
vier Probleme gemeldet, die er nicht lösen konnte. Seine Änderungen bleiben, wie
er sie wollte; hier stehen nur die Korrekturen.

### 1. Werkzeug-Abstände („2×10px")
Kein Fehler im CSS: **innerhalb** einer Antwort sind zwei Chips genau 10 px
auseinander. Die 20 px, die er sah, waren **zwei verschiedene Antworten** (10 px
Chip + 10 px Blase). Was es wirklich gab: je Werkzeug steckte ein zusätzlicher
Wrapper-Div mit eigenem `margin-top`. Der ist raus — jetzt trägt jeder Chip seinen
einen Abstand, und die Staffelung zwischen Antworten bleibt bewusst größer.
Gemessen am Pixelverlauf: 10 px zwischen Chips, 20 px zwischen Antworten.

### 2. „tool-name/state/time ans Ende des Buttons"
Ging nicht, weil der Chip sich auf Inhaltsbreite zusammenzog (`width: auto` in
einem Block-Container). Der Chip füllt jetzt die Spalte: Symbole links, Name,
Zustand und Dauer **rechts außen** — wie gewünscht.

### 3. Composer ohne Hintergrund / „röntgt durch"
Der Composer hatte dieselbe Fläche wie die Werkzeug-Chips (`--bg-panel`), wurde
also optisch eins mit dem, was dahinter scrollt. Jetzt: eigene Fläche
(`--bg-app`), **Trennkante** darüber und ein dunkleres Eingabefeld (`--bg`).

### 4. Gleiche Breite, sichtbar verschieden
Zwei Ursachen, beide behoben:
- Die Spalte ist ein Flex-Kind mit `flex: 1` gewesen — die Basis schlägt die
  Breite, also dehnte sich die „70 %"-Spalte still über das ganze Panel aus.
  Jetzt sitzt die Breite auf dem Scrollbereich selbst (mit dünner Scrollbar).
- Die Scrollbar fraß 6 px **innerhalb** der Nachrichtenspalte, der Composer
  nicht. Der Composer spiegelt diese 6 px (`calc(70% - 6px)` und 3 px Versatz),
  damit beide Kanten wirklich übereinander liegen. Geprüft mit Debug-Umrissen um
  beide Bereiche: sie überlappen exakt (grün + rot = gelb).

### Verifikation
Da die Dev-Instanz des Nutzers Port 1420 hält, wurde alles am **Release-Binary**
geprüft (Frontend eingebettet, kein Vite): `pnpm build` → `cargo build --release`
→ Start unter X11 → Screenshot, Kanten am Pixelverlauf gemessen. 24 Rust-Tests
grün, Baum sauber. Debug-Umrisse und Testcode sind restlos entfernt
(`grep TEMP-VERIFY` leer), der Fensterzustand zurückgesetzt.

## 34. Plan für morgen: Veröffentlichung über GitHub (notiert 2026-10-16, abends)

Vom Nutzer so gewünscht, damit der nächste Tag aus dieser Datei startet und
nicht aus dem Gedächtnis:

1. **Für GitHub vorbereiten** — Repository-Inhalt prüfen, was gehört hinein, was
   nicht (die großen Vorlagen sind schon raus, §30/§31), README mit dem, was das
   Programm ist und wie man es baut, Lizenzhinweise (MiniMax Code MIT, Ubuntu
   Font Licence — beides steht schon auf der Über-Seite).
2. **GitHub Actions** für **macOS- und Windows-Builds** (der Linux-Build läuft
   hier lokal schon, §32).
3. **Artefakte ausliefern:** AppImage, deb, rpm **und zip**.

Was dabei zu beachten ist (so vom Nutzer klargestellt):
- **Signieren ist nicht nötig.** Ohne Zertifikat lässt sich die App trotzdem
  installieren: auf macOS per Rechtsklick → Öffnen, auf Windows per Bestätigung
  des Herkunftshinweises. Die Warnungen gehören in die README, nicht in den Weg.
- **Vertriebswege:** **homebrew** und **winget**; ab nächstem Monat zusätzlich der
  **Microsoft Store** (19 € einmalig). winget und Store sind zertifiziert, GitHub
  ist es nicht — deshalb laufen die Installer-Wege getrennt vom Release-Anhang.
- **Zwei Warndialoge sind erwartbar und in Ordnung**, solange die README sie
  erklärt: das ist der Preis dafür, kein Zertifikat zu kaufen.
- **Woher die Version kommt:** `0.1.0` steckt in `package.json`, `Cargo.toml` und
  `tauri.conf.json`. Für Releases sollten sie nicht auseinanderlaufen; ein Tag
  (`v0.1.0`) müsste die Quelle sein.
- **Der Build braucht WebKit** (Linux) bzw. die jeweiligen Runner — die
  Standard-`ubuntu-latest`-Runner haben den WebKitGTK-Kram nicht, das kostet
  einen Installationsschritt im Workflow.
- **Erst danach:** Der Quelltext geht als **Issue an DeepSeek** — ausdrücklich
  erst, wenn **alle Installer funktionieren** (AppImage, deb, rpm, zip, macOS,
  Windows).

## 35. Regression, Reparatur und der Stand (2026-10-17, vierzehnte Runde)

Commits `c2aefbe`, `fabc50d`, `631b0c4`. Dies ist der Anker vor der nächsten
Kompression.

### Die Regression (meine, von gestern)
„Ein Turn = eine Zeile" heißt **eine** `tool_calls`-Liste pro Turn. Ich habe sie
pro **Runde** überschrieben. Folge: Nach einem Turn mit mehreren Runden hatten die
früheren Tool-Ergebnisse keinen Aufruf mehr, und der Anbieter lehnt genau das ab
(„Messages with role 'tool' must be a response to a preceding message with
'tool_calls'"). Ein gescheiterter Turn hinterließ zusätzlich eine leere
Assistentenzeile → jede weitere Anfrage war erneut ungültig → **die Unterhaltung
antwortete dauerhaft „error"**. Behoben:
- Tool-Aufrufe werden über **alle Runden** gesammelt und einmal gespeichert.
- Leere Assistentenzeilen **ohne** Aufrufe werden übersprungen; verwaiste
  Tool-Zeilen fallen raus — **das heilt auch schon kaputte Verläufe**.
- Fehlgeschlagene Turns werden mit Grund **geloggt** (`engine: turn failed in …`),
  der echte Fehlertext bleibt in der Blase (nicht mehr das Wort „error"), und ein
  Fehler vor dem ersten Token bekommt überhaupt eine Blase.
- Verifiziert mit einem echten Turn: Antwort „OK" in der Zeile. Testnachrichten
  danach aus der Datenbank entfernt.

### Verlorene Stile (separat)
- **Markdown-Blöcke** (`p`, `pre`, `code`, Listen, Überschriften) waren aus
  `MessageList.vue` verschwunden → Codeblöcke ohne `overflow-x` liefen über die
  Blase („231 行 …"). Wiederhergestellt; Tabellen scrollen in sich selbst.
- **Composer**: `.composer-wrap` war auskommentiert — das war die „Transparenz".
  `#202020` wurde nicht ignoriert, es **ist** `--bg-panel`. Jetzt: Hintergrund-
  Layer zurück, Eingabefeld nutzt `--bg-composer`.
- **Autoscroll** hing: `requestAnimationFrame` schweigt in dieser WebView (dieselbe
  Ursache wie beim Splash). Jetzt `nextTick` + `ResizeObserver` auf den Inhalt.

### Stand
Fertig: sechs Punkte aus §25–§29, Marke (Wal/Icon/Splash-Zoom), About-Seite,
MIT-Lizenz, Version 0.4.7, README + Screenshot, CI (verify + drei Plattformen +
Archive am Release), alle Regressionen oben. Offen: ein **Windows-/macOS-Smoke-Test**
(nur über CI-Artefakte möglich; lokal ist nur Linux da), `v0.4.8` taggen und den
Draft `v0.4.7` löschen.

### Die nächsten zwei Vorhaben (vom Nutzer gewünscht, noch nicht gebaut)
1. **Aktionen sichtbar machen** („UI folgt dem Begleiter"): `write`/`edit` auf eine
   Datei → Datei-Tab öffnen (Vorschau oder Monaco je nach Typ); `web_fetch` → den
   Browser auf die URL; `bash` → (Terminal-Panel fehlt noch). Mit Schalter
   „folgen" und dezentem Hinweis auf den aktiven Tab, damit die Oberfläche nicht
   ungefragt springt.
2. **Punkte-/Vertrauenssystem**: Der Begleiter darf seinen Namen früh *wählen*,
   aber der Nutzer **erfährt ihn erst ab einer Punktzahl** — Punkte für Nutzung
   über Zeit (Streaks, erledigte Aufgaben, Korrekturen, Gedächtnis-Einträge), nicht
   für einen einzelnen Klick. Eigene Tabelle, Fortschritt auf der Begleiterseite,
   kleine „Enthüllung" beim Erreichen.

### Antwort auf die Composer-Frage
Die Farbe hat keinen Alphawert. Das Wasserzeichen ist `.agent::before` — eine
eigene Ebene **hinter** den Kindern des Panels. Der Composer selbst ist opak; der
Bereich *um* die Box (der Wrap) hatte keinen Hintergrund mehr, und dort scheint
die Ebene durch. Es war nie die Farbe, es war die fehlende Fläche.

## 36. Notiert vor der Kompression (2026-10-17, fünfzehnte Runde)

Commit `f8e6a94` (Wasserzeichen) und dieser Doku-Commit. Alles, was nach der
nächsten Kompression sonst fehlen würde.

### Das Wasserzeichen lag *über* dem Composer, nicht dahinter
Der Nutzer sah die Marke durch das angeblich opake Eingabefeld. Es war kein
Durchscheinen: `.agent::before` ist **positioniert mit `z-index: auto`**, und ein
positioniertes Element malt in einer späteren Ebene als normaler In-Flow-Inhalt —
also über dem Composer. Bei 6 % Deckkraft liest sich das als Transparenz. Jetzt:
`.agent { isolation: isolate }` plus `z-index: -1` am Pseudo-Element. Die Marke
liegt hinter dem Inhalt, bleibt durch die halbtransparenten Blasen sichtbar und
verschwindet unter opaken Flächen.

### Budget: keine „Bremse"
Vom Nutzer klargestellt: keine harte Begrenzung, er „tankt lieber nach". Stattdessen
nur ein **dezenter Hinweis im Seitenmenü**, wenn das Budget überschritten ist —
der Nutzer setzt das Budget selbst. Die Anzeige wächst mit der Überschreitung
(Schriftgröße) und färbt sich weiß → gelb → orange → rot. Kein Abbruch.

### Bereits in §35 notiert (für die Umsetzung nach der Kompression)
1. **UI folgt dem Begleiter**: `write`/`edit` → Datei-Tab (Vorschau oder Monaco),
   `web_fetch` → Browser auf die URL, `bash` → Terminal (fehlt noch), `read/glob/grep`
   nur als Chip. Mit Schalter „folgen" und dezentem Hinweis statt Fokusklau.
2. **Punkte-/Vertrauenssystem**: Name früh gewählt, aber erst ab einer Punktzahl
   enthüllt; Punkte für Verhalten über Zeit (Streaks, erledigte Aufgaben,
   Korrekturen, Gedächtnispflege), nicht für einzelne Klicks; Fortschritt auf der
   Begleiterseite, kleine Enthüllung beim Erreichen. Ziggy ist bereits enthüllt —
   gilt für neue Installationen.
3. **Companion-Ideen** für später: Wiedereinstieg („Wo waren wir?"), Wochenrückblick
   aus Todos/Sitzungen, **projektgebundenes** Gedächtnis, Skill-Vorschläge
   („das machst Du zum dritten Mal von Hand"), Budgetanzeige wie oben.

### Release-Plan
Smoke-Test auf dem zweiten Linux-Rechner → README-Abschnitt „Known limitations"
(kein Editor-/Browser-Zugriff des Begleiters) → `v0.4.8` taggen → Draft `v0.4.7`
löschen. Windows/macOS nur über die CI-Artefakte (kein Testgerät).

## 37. Die Oberfläche folgt dem Begleiter (2026-10-17, sechzehnte Runde)

Umgesetzt ist das erste der beiden in §35/§36 notierten Vorhaben. Kein Commit — die
Änderungen liegen im Baum und sind mit `pnpm build`, `cargo check` und den 24
Rust-Tests geprüft.

### Was passiert
`tool_start` auf `write` oder `edit` öffnet die Datei im Datei-Tab (Vorschau oder
Monaco, wie der Typ es hergibt) und lässt den Tab kurz aufleuchten (`pulseTab`,
1,8 s). `tool_result` desselben Werkzeugs erhöht `fileRevision`; der Tab liest die
Datei daraufhin neu — ohne das hätte man den Stand *vor* dem Schreiben gesehen,
weil der Pfad sich ja nicht ändert. `web_fetch` navigiert das zuletzt benutzte
Browser-Fenster auf die URL. `read`, `glob` und `grep` bleiben Chips; `bash`
ebenfalls, das Terminal-Panel fehlt noch.

### Kein Fokusklau
Es wird nirgends `focus()` gerufen; der Cursor bleibt im Composer. Für den Browser
gibt es jetzt den optionalen Parameter `focus` an `open_browser_window`
(Standard `true` für alle bisherigen Aufrufer). Beim Folgen wird `false`
übergeben: das Fenster öffnet bzw. navigiert, drängt sich aber nicht nach vorn.

### Schalter
`settings.followCompanion` (Standard `an`), als Augensymbol rechts in der
Tab-Leiste und als Schalter unter Einstellungen → Allgemein. Aus heißt: nur die
Chips im Gespräch, kein Tab, kein Browser. Pfade werden relativ zum
Arbeitsverzeichnis aufgelöst (`resolveWorkspacePath`), absolute Pfade bleiben.

### Dateien
- `src/composables/useFollow.ts` (neu): Argumente lesen, Pfad auflösen, Tab/Browser
  folgen lassen, Pfad je Werkzeug-ID bis zum Ergebnis merken.
- `src/stores/workArea.ts`: `fileRevision`, `refreshFile`, `attentionTabId`,
  `pulseTab`.
- `src/composables/useEngine.ts`: Aufrufe an den beiden Ereignisstellen.
- `src/components/workarea/ViewFileTab.vue`: Reload-Watcher für Text, Bilder und
  Office-Vorschauen; Markdown-Ansicht springt bei neuem Pfad zurück auf Vorschau.
- `src/components/workarea/WorkArea.vue`: Augenschalter, Puls-Animation.
- `src-tauri/src/browser.rs`: `focus`-Parameter.
- i18n in allen sechs Sprachen, `Icon.vue` (Auge/Auge durchgestrichen),
  `SettingsPage.vue`, `stores/settings.ts`.

### Als Nächstes
Punkte-/Vertrauenssystem (Name gewählt, aber erst ab einer Punktzahl enthüllt),
danach Budgetanzeige im Seitenmenü.

## 38. Flüssiger streamen, Denken zuklappen, Namen begründen (2026-10-17, siebzehnte Runde)

Reaktion auf den Zwischenbericht: Das Tippen im Gespräch hakt, leere Blöcke wandern
über den Bildschirm, und die Denk-Anzeige hängt. Kein Commit; geprüft mit
`pnpm build` und den 24 Rust-Tests.

### Warum es hakte
`MessageList` wurde bei *jedem* gestreamten Bruchstück komplett neu gerendert — und
rief für **jede** fertige Nachricht erneut `renderMarkdown` samt DOMPurify auf. Bei
einer langen Unterhaltung war das quadratisch viel Arbeit pro Token. Dazu war der
Denk-Block während des Streamens aufgeklappt: jeder neue Satz erzwang ein neues
Layout eines ohnehin schon langen, mitgescrollten Textes. Die „leeren Prompts" waren
genau dieser noch leere, offene Denk-Kasten am Anfang eines Zuges.

### Was jetzt anders ist
- **`MarkdownText.vue`**: eigener Baustein mit gecachtem `computed`. Der Parent darf
  weiter rendern; nur der Block, dessen Text sich wirklich geändert hat, wird neu
  geparst und saniert. Die Text-Stile sind mit umgezogen (auch die Nutzerblase).
- **Sammelpuffer in `useEngine.ts`**: Text- und Denk-Brüche werden 40 ms lang
  gesammelt und in ihrer Reihenfolge an den Store gegeben. Jedes Ereignis, das die
  Zeitleiste ändert (Werkzeugstart/-ende, Fehler, `action_required`, `turn_end`),
  leert den Puffer vorher — ein Werkzeug-Chip kann also nicht über Text rutschen,
  der vorher geschrieben wurde. Beim Sitzungswechsel und vor einem neuen Zug wird
  ebenfalls geleert. 40 ms sind unter der Wahrnehmungsschwelle, sparen aber den
  größten Teil der Renderarbeit.
- **Denken bleibt zu**: `<details>` ohne `open`-Bindung; solange der Zug läuft, wird
  ein Aufklappen per `toggle` sofort zurückgenommen. In der Kopfzeile steht live die
  Anzahl der gedachten Zeichen (aus dem gepufferten Text, nicht geschätzt). Erst nach
  `turn_end` lässt sich der Block normal öffnen. Der Zähler bleibt sichtbar.
- Neue i18n-Zeichenkette `message.characters` in allen sechs Sprachen.

### Namen: wann und warum
`choose_name` verlangt jetzt zusätzlich `reason` (ein Satz). Nach erfolgreicher Wahl
schreibt der Rust-Teil automatisch einen Gedächtniseintrag (`kind = "event"`,
`scope = "agent"`, Quelle `choose_name`) mit dem lokalen Datum im Text — die Zeit
steht zusätzlich im `created_at` der Zeile, also im Gedächtnis-Panel. Damit kann ein
späterer Thread nicht mehr nur den Namen vorfinden, sondern auch „wann" und „warum".
(`Db::today` liefert dafür `date('now','localtime')`.) Für die bereits gewählten Namen (Ziggy,
davor Nathaniel und Eira) ist der Grund nicht rekonstruierbar — die vorhandene
Notiz sagt das ehrlich; erfunden wird nichts. `reason` fehlt → das Werkzeug antwortet
mit einer Erklärung statt die Wahl ohne Begründung zu speichern.

### Offen / als Nächstes
Punkte-/Vertrauenssystem und Budgetanzeige; danach der Smoke-Test für `v0.4.8`.

## 39. Schmales Fenster: erst die Leiste, dann der Arbeitsbereich (2026-10-17, achtzehnte Runde)

Der Build bei Mindestbreite zeigte abgeschnittene Composer-Bedienelemente („Volle
Berechtigung" brach senkrecht um, das Modellfeld lief aus der Blase) und eine
gequetschte Mitte. Reine Mindestgrößen helfen dort nicht; die Breite muss sich
anders verteilen. Kein Commit; `pnpm build` grün.

### Reihenfolge
Wie vom Nutzer vorgeschlagen, aber mit eigener, nicht gespeicherter Zustandsebene:
- `railAuto` und `workAuto` (in `stores/ui.ts`) bedeuten „gerade kein Platz", nicht
  „vom Nutzer weggeschaltet". Die manuellen Flags (`railCollapsed`, `visible`)
  bleiben unangetastet und werden weiter gespeichert.
- `useResponsiveLayout.ts` misst beim Start und bei jedem Fenster-`resize` die
  tatsächliche Mitte: erst verschwindet die linke Leiste, wenn die Mitte unter
  720 px fällt; reicht es dann immer noch nicht (unter 640 px), verschwindet der
  Arbeitsbereich. Zum Zurückholen sind die Schwellen höher (780/700 px), damit ein
  Fenster auf der Kante nicht flackert.
- Ein Klick auf die Leisten- oder Panel-Schaltfläche setzt das Auto-Flag zurück:
  wer das Panel bei schmalem Fenster ausdrücklich öffnet, bekommt es. Beim nächsten
  Resize gilt wieder die Regel.
- `openTab`/`openFile`/`showWorkArea` räumen `workAuto` ebenfalls ab — ein per
  Begleiter geöffneter File-Tab soll sichtbar sein.

### Composer und Spalte
- Gemeinsame Lese-Spalte als CSS-Variable am Agenten:
  `--chat-width: min(100%, max(70%, 520px), 860px)`. Auf weiten Fenstern bleibt es
  bei 70 %, auf schmalen wird die Spalte nicht mehr unter 520 px gedrückt; bei sehr
  breiten Fenstern begrenzt sie sich auf 860 px. Liste und Composer benutzen
  dieselbe Variable (Composer weiterhin 6 px schmaler wegen der Scrollbar-Reserve).
- `.composer` ist jetzt ein Container (`container-type: inline-size`). Unter 440 px
  eigener Breite verschwindet nur das Label von „Volle Berechtigung" (Icon und
  Klickziel bleiben), das Modellfeld schrumpft auf 118 px. Das Label kann nicht
  mehr senkrecht umbrechen (`nowrap` + Ellipse), linke und rechte Gruppe dürfen
  schrumpfen (`min-width: 0`).
- Titelzeile: Editor-Beschriftung verschwindet unter 980 px (Logo + Chevron bleiben),
  der Sitzungstitel ist auf `min(420px, 40vw)` begrenzt.
- `tauri.conf.json`: `minWidth` 960 → 780. Bei 780 px sind beide Panels automatisch
  weg, die Mitte hat ~760 px; ab ~795 px bleibt der Arbeitsbereich, ab ~1100 px
  kommt die Leiste zurück.

### Vom Nutzer am Splash
Der Wal wächst jetzt gleichmäßig (Schwanz und Maul erreichen den Rand zusammen),
ist auf 1,85 statt 2,05 skaliert, sitzt mit `top: 40px; right: 10px` und die
Splash-Leiste nutzt `--accent-whale` (#0aaac2) statt `--accent`. Unverändert
übernommen.

### Offen
Sichtprüfung bei 780/960/1100 px durch den Nutzer; Rust-Neustart nötig für
`minWidth` (Fensterkonfiguration) sowie die früheren Rust-Änderungen.

## 40. Dateibaum tritt zur Seite, Adresszeile bleibt (2026-10-17, neunzehnte Runde)

Drei Wünsche aus der Sichtprüfung des Datei-Tabs. Kein Commit; `pnpm build` grün.

### Dateibaum nach der Auswahl ausblenden
`ViewFileTab` merkt sich jetzt `fileTreeVisible` im WorkArea-Store (und damit in
`ui.workarea`, also über Neustarts hinweg). Nach einem Klick auf eine Datei im Baum
startet ein 2-s-Timer; läuft er ab, setzt er die Sichtbarkeit auf `false`. Der
Übergang selbst ist CSS: `.viewfile` animiert `grid-template-columns` von
`minmax(0,1fr) 220px` auf `minmax(0,1fr) 0px` (260 ms), die Leiste blendet dabei
aus. Wird in der Zwischenzeit eine andere Datei gewählt, beginnt der Timer neu; wird
ein Ordner auf-/zugeklappt, wird er abgebrochen — wer den Baum benutzt, soll ihn
nicht unter der Hand verlieren. Das Ausblenden ist reine Anzeige: der Baum bleibt
montiert (`inert`, `aria-hidden`), Monaco bekommt durch `automaticLayout` die neue
Breite.

### Begleiter öffnet keine Bäume mehr
Der Auto-Hide hängt nur an `select()` im Dateibaum. `openFile` (auch aus
`useFollow`) und `fileRevision` fassen die Sichtbarkeit nicht an. War der Baum zu,
bleibt er zu, wenn der Begleiter eine Datei schreibt; war er offen, bleibt er offen.

### Feste Adresszeile auch für Code
Die `.viewer-bar` wird jetzt für jede Datei gerendert, nicht nur für Markdown und
Binärformate. Bei Markdown stehen links „Vorschau/Quelle", rechts der Pfad und der
neue Baum-Schalter; bei Code (Monaco) bleibt die Zeile stehen, während der Editor
scrollt. Der frühere schwebende „Baum einblenden"-Knopf über dem Editor ist weg —
der Schalter sitzt am rechten Ende der Zeile und zeigt den Zustand (Akzentfarbe,
wenn der Baum offen ist). Damit verschwindet nichts mehr beim Scrollen, und der
Pfad ist immer sichtbar.

### Offen
Sichtprüfung durch den Nutzer (2-s-Verzögerung, Animation, Monaco-Breite).

## 41. Auch Lesen blendet den Editor ein (2026-10-17, zwanzigste Runde)

Der Begleiter konnte den Browser folgen lassen, aber keinen Editor öffnen. Ursache
war keine kaputte Verdrahtung: `useFollow` behandelte nur `write` und `edit`;
„zeig mir diese Datei" erledigt der Begleiter aber mit `read`, und das war bewusst
nur ein Chip. Damit blieb die Asymmetrie zum Browser, der über `web_fetch` folgt.

Jetzt öffnen auch `read`-Aufrufe die Datei im Datei-Tab (Vorschau oder Monaco je
Typ) und lassen den Tab kurz aufleuchten. `write`/`edit` merken sich den Pfad
weiterhin bis zum Ergebnis, damit die Vorschau den neuen Inhalt lädt; bei `read`
wird nichts nachgeladen, weil sich nichts geändert hat. `glob`, `grep` und `bash`
bleiben Chips — sie beschreiben eine Suche, kein Dokument.

Folge: Wer den Begleiter beim Durchsuchen vieler Dateien beobachtet, sieht den Tab
öfter wechseln. Dafür ist das Augensymbol in der Tab-Leiste da; aus heißt aus.
`pnpm build` grün, kein Commit.

## 42. Erinnerungen als Karten, Titel und ein weicherer Hinweis (2026-10-17, einundzwanzigste Runde)

Kein Commit; `pnpm build` grün, 24 Rust-Tests grün. Der Begleiter öffnet jetzt auch
gelesene Dateien — vom Nutzer bestätigt.

### Titel für Erinnerungen
Die `memories`-Tabelle hat eine Spalte `title` bekommen (Migration, alte Zeilen
bleiben `NULL`). Höchstens 120 Zeichen; `clean_title` faltet Weißraum und kürzt mit
Ellipse. `insert_memory_with_title` legt den Titel mit an und leitet ihn, wenn keiner
kommt, aus der ersten Zeile des Textes ab (`derive_title`). Das `memory`-Werkzeug hat
ein optionales `title`-Feld für `add` und `update`; die Befehle `memory_add` und
`memory_update` reichen es durch. Der Name-Eintrag aus §38 bekommt seinen Titel
automatisch aus der Notiz.

### Karten statt Wand
`MemoryPanel.vue`: Eingabe mit Titelzeile (Zähler 0/120) und Text. Die Liste ist ein
Raster (`auto-fill`, min. 260 px), jede Karte zeigt Titel (zwei Zeilen), Chips, eine
dreizeilige Vorschau und die Aktionen. Ein Klick öffnet ein modal über der Seite
(`Teleport to body`) mit vollem Text, Datum, Bearbeiten und Stilllegen; nichts
darunter verschiebt sich. Bearbeiten wechselt im Modal auf Titel + Text; Esc und
Klick auf den Hintergrund schließen. Alte Einträge ohne Titel zeigen ihre erste
Zeile als Überschrift. Eine Suche ist bewusst noch nicht drin — später.

### Hinweis und Sprache
„Der Agent ist KI und kann Fehler machen." ist jetzt in allen sechs Sprachen
„Auch KI-Begleiter können mal einen Fehler machen!" (bzw. „AI companions can make
mistakes too!", „Les compagnons IA peuvent eux aussi faire des erreurs !",
„¡Los compañeros de IA también pueden cometer errores!", „ИИ-собеседники тоже иногда
ошибаются!", „AI 伴侣也会犯错！").

Aus der Begriffssammlung übernommen, jeweils in der Begleiter-Seite (Hinweis und
Tagline): Deutsch „KI-Begleiter", Französisch „compagnon IA", Spanisch „compañero de
IA", Russisch „ИИ-собеседник", Chinesisch „AI 伴侣". Englisch entsprechend „AI
companion". Die kurzen Einstellungs-Labels bleiben unverändert.

### Offen
Sichtprüfung des Rasters/Modals durch den Nutzer; die Suche kommt später.

## 43. v0.4.8 (2026-10-17, zweiundzwanzigste Runde)

Der Nutzer hat den Stand geprüft und nichts mehr gefunden; daraus wird die
Veröffentlichung. Enthalten sind die Runden 37–42:

- Oberfläche folgt dem Begleiter: `write`/`edit` und jetzt auch `read` öffnen Dateien
  (Markdown-Vorschau oder Monaco), `web_fetch` den Browser — mit Augenschalter, ohne
  Fokusklau; der Dateibaum tritt nach der Auswahl zur Seite und bleibt beim
  Begleiter-Schreibzugriff zu.
- Flüssigeres Streamen: gepufferte Brüche (40 ms), `MarkdownText` mit gecachtem
  Rendering, Denken bleibt während des Zuges zu und zeigt die Zeichenzahl.
- Schmales Fenster: linke Leiste zuerst, dann der Arbeitsbereich (Auto-Flags,
  Hysterese), gemeinsame Lese-Spalte (min. 520 px), Composer-Container-Query,
  `minWidth` 780.
- Erinnerungen: Titel (max. 120 Zeichen), Kartenraster, Modal, Bearbeiten von Titel
  und Text; Migration `memories.title`; `title` im `memory`-Werkzeug.
- Namen: `choose_name` verlangt eine Begründung und legt sie datiert im Gedächtnis ab.
- Weichere Formulierung in allen sechs Sprachen („Auch KI-Begleiter können mal einen
  Fehler machen!"), Begleiter-Begriffe aus der Sammlung übernommen.
- README: Follow-Modus und Abschnitt „Known limitations"; Version in
  `package.json`, `Cargo.toml`, `tauri.conf.json` und `Cargo.lock` = 0.4.8.

Test: `pnpm build` grün, 24 Rust-Tests grün. Tag `v0.4.8` löst den Release-Workflow
aus (verify → Linux/macOS/Windows → Draft-Release mit Artefakten). Der alte Draft
`v0.4.7` wird danach gelöscht.

`docs/markdown-test.md` bleibt als lokale Testdatei unversioniert.

## 44. Der Schlüssel, der nur im Mock lag (2026-10-20, dreiundzwanzigste Runde)

Der Windows-Test hat den ersten echten plattformspezifischen Fehler gefunden: Die
App meldete „Gespeichert im OS-Schlüsselbund", verlor den API-Key aber sofort
wieder. Ursache war `keyring` 3 ohne Plattform-Features — die Crate hat keine
Defaults, und mit nur `sync-secret-service` (Linux) greift auf Windows/macOS der
plattformunabhängige Mock. Der Mock hält den Wert nur in dem `Entry`, der ihn
geschrieben hat; der nächste `Entry` in `provider_request` ist leer →
„API key is missing". Unter Linux (Secret Service) fiel das nie auf.

### Behoben
- `Cargo.toml`: `apple-native` und `windows-native` ergänzt (Linux behält
  `sync-secret-service` + `crypto-rust`). `Cargo.lock` zieht die
  plattformspezifischen Abhängigkeiten nach.
- `SecretStore::set` liest den Wert nach dem Schreiben über einen **frischen**
  `Entry` zurück. Nur wenn er exakt passt, gilt der Keyring als erfolgreich und
  die Datei-Kopie wird entfernt. Bei einem falschen Wert wird der Eintrag
  gelöscht; bei einem Lesefehler bleibt er stehen, aber der Datei-Fallback wird
  geschrieben und als `file` gemeldet. Damit ist der Mock nie wieder still.
- `keyring_roundtrip_or_skip` schreibt mit einem Entry und liest mit einem
  zweiten. Der alte Test las denselben Entry und war deshalb auch im Mock grün.
- CI: Windows/macOS führen jetzt `cargo test --lib` aus, bevor sie bauen — dort
  lebt der native Store, den der Linux-`verify`-Job nicht sehen kann.
- `skills.rs`: Windows-sichere Pfadprüfung im Test (`Path::ends_with`).
- Provider: ein gemeinsamer `reqwest::Client` für `/models` und `/user/balance`,
  ein Retry nach 500 ms bei Transportfehlern, Fehlerkette statt nur „error
  sending request"; die UI lädt Modelle und Guthaben nacheinander statt parallel
  (beobachtete Connection-Resets bei zwei gleichzeitigen Verbindungen).

### Stand
Version 0.4.9. `pnpm build` grün, `cargo test` 24 passed / 12 ignored (der
Keyring-Test lief auf Linux durch). Der Release-Workflow ist ebenfalls grün:
verify, Linux, macOS und Windows; der neue Testlauf auf Windows und macOS hat
den Keyring-Test auf beiden Plattformen wirklich ausgeführt (24 passed, nicht
übersprungen) und der Draft enthält die sieben Assets. Der Windows-11-Test mit
dem Installer ist auf dem Arbeitsrechner bereits mit einem lokalen 0.4.9-Build
durchgelaufen. v0.4.9 ist daher als **Latest** veröffentlicht (sieben Assets,
Release-Notes); der defekte v0.4.8-Draft wurde gelöscht, der Tag v0.4.8 bleibt
als Historie.

Der Editor-Picker und das Account-Pill („modest") bleiben für den nächsten
Termin liegen.

## 45. Splash-Bewegung und Marke im Rail-Fuß (2026-10-21, vierundzwanzigste Runde)

Zwei Punkte, die im v0.4.9-Release noch fehlten. Kein Commit; geprüft mit
`pnpm build`, `cargo test --lib` (24 passed, 12 ignored) und einem Windows-Build
mit `tauri/custom-protocol`.

### Der Walfisch stand unter Windows still
Ursache war nicht WebView2: Der Testrechner hat die Windows-Animationseffekte aus,
also meldet Chromium `prefers-reduced-motion: reduce`. Die Splash-Regel schaltete
daraufhin genau die Wal-Animation ab — der Fortschrittsbalken lief weiter. Der
Splash ist die einzige bewusste Bewegung der App; die Regel ist entfernt. Wal und
Balken starten außerdem erst, wenn das Fenster sichtbar ist (`splashShown`), statt
schon beim Laden der versteckten Webview. Die sieben Sekunden Animation und die
sieben Sekunden Splash laufen damit exakt zusammen.

### Marke im Rail-Fuß
Der Fuß war fest auf „modest" und einen Sparkle-Avatar verdrahtet. Walfisch und
„deepseek"-Schriftzug sind jetzt dort — vom Kopf des Panels dorthin gezogen; der
Fuß bleibt der Anker für das Konto-Menü (Chevron). Der Benutzername steht
weiterhin im Menükopf, nicht mehr statisch im Panel.

### Offen
v0.4.10 taggen.

## 46. Der Editor-Picker öffnet den Workspace (2026-10-21, fünfundzwanzigste Runde)

`open_workspace_in_editor` lag seit §5 ungenutzt im Rust-Teil; `openInEditor()` war
exportiert, aber niemand rief es auf. Jetzt ist die Pille ein Split-Button: links
öffnet den Workspace im gewählten Editor, rechts öffnet die Auswahl. Ohne
gefundenen Editor öffnet der linke Teil die Auswahl, damit Erkennen und Download
erreichbar bleiben; ein Fehler beim Start erscheint fünf Sekunden unter der Pille.
`ensureWorkspace()` sorgt dafür, dass ein leerer Workspace nicht als „Editor ohne
Ordner" endet.

Der Windows-Test hat zwei echte Fehler im alten Befehl gefunden:
- Der PATH-Eintrag `C:\Program Files\Microsoft VS Code\bin\code` ist ein Pfad mit
  Leerzeichen, wurde aber durch denselben Command-Splitter wie die Desktop-Einträge
  geschickt → als Programm blieb `C:\Program` (os error 2). PATH-Kandidaten werden
  jetzt gequotet; auf Windows bleibt der Pfad außerdem un-canonicalisiert (kein
  `\\?\`-Präfix).
- Der gefundene `code` ist der Unix-Shim; daneben liegt `code.cmd`, den Windows
  nicht direkt startet. Der Launcher nimmt auf Windows den `.cmd`/`.bat`-Shim über
  `cmd /C` (mit `CREATE_NO_WINDOW`) und nennt sonst den Programmnamen im Fehler.

Geprüft: `pnpm build`, `cargo test --lib` (24 passed, 12 ignored), Windows-Release
mit `tauri/custom-protocol`; ein kalter VS-Code-Start über den Split-Button öffnete
`X:\DEV\dside-main` (Fenstertitel „Willkommen - dside-main").

## 47. Dateiverlauf mit Diff und Wiederherstellen (2026-10-21, sechsundzwanzigste Runde)

Der wichtigste Vertrauensbaustein: Der Partner kann jetzt zeigen, was er geschrieben hat,
und der Nutzer kann es zurücknehmen.

### Snapshots
- Neue Tabelle `file_versions` (Pfad, Inhalt, existed, Session, Herkunft, Call-ID, Zeit).
  `content NULL` heißt: Die Datei existierte vorher nicht; Wiederherstellen löscht sie.
- `engine/mod.rs` liest den alten Stand vor jedem `write`/`edit` und schreibt den
  Snapshot erst, wenn das Werkzeug erfolgreich war. Das gilt auch für Sub-Agenten
  (`task`), die dafür AppHandle und Session-ID bekommen.
- `tools::stable_path` normalisiert Pfade (Windows-Separatoren, kein `\\?\`), damit
  Dateibaum, Modell und Verlauf denselben Schlüssel benutzen.

### UI
- Datei-Tab: „Verlauf" öffnet links die Liste (neueste zuerst, mit Zeitpunkt, Herkunft
  Schreiben/Bearbeiten/Wiederherstellen und „Neu angelegt"), rechts einen read-only
  Monaco-Diff gegen den aktuellen Stand.
- „Wiederherstellen" sichert vorher den aktuellen Stand als weiteren Snapshot und
  schreibt dann die gewählte Fassung; ein Snapshot von vor der Existenz der Datei
  entfernt sie wieder.
- Ein `write`/`edit`-Chip hat im aufgeklappten Detail einen „Verlauf"-Knopf; der
  Datei-Tab öffnet sich mit dem Verlauf.
- Fehler beim Laden/Wiederherstellen erscheinen im Panel; die Bestätigung fragt vor
  dem Überschreiben.

### Tests
- `db::tests::file_versions_round_trip` (Liste, neueste zuerst, NULL/created).
- `tests::reverting_a_snapshot_restores_and_backs_up_the_current_state` (Restore,
  Sicherung, Löschen eines vorher nicht existierenden Snapshots).
- Live auf Windows: Der Partner schrieb `history-test.txt` (Snapshot `write`,
  existed=0), änderte `eins`→`zwei` (Snapshot `edit`, alter Inhalt `eins`); der Verlauf
  zeigte beide, der Diff `eins`→`zwei`, der Restore des „Neu angelegt"-Snapshots
  entfernte die Datei. `cargo test --lib` 26 passed / 12 ignored; `pnpm build` grün.

## 48. AGENTS.md: die Regeln des Repos (2026-10-21, siebenundzwanzigste Runde)

Der Partner kannte bisher Persona, Nutzeranweisungen und Gedächtnis, aber nicht die
Konventionen des Projekts. Jetzt liest der Engine-Loop beim Start eines Turns `AGENTS.md`
(Fallback `.dside/rules.md`) aus dem Workspace und hängt sie an den System-Prompt;
Sub-Agenten bekommen dieselben Regeln. Der Text ist auf 20.000 Zeichen begrenzt, damit
eine ausufernde Datei nicht das Kontextfenster frisst.

Die Datei im Wurzelverzeichnis ist gleichzeitig die Anleitung für dieses Repository:
pnpm 10, Build-/Testbefehle, PROGRESS als Chronik, Schlüsselbund, die sechs Sprachen und
die Lizenzhinweise.

Test: `project_instructions_are_read_from_the_workspace` (Temp-AGENTS.md gelesen, Quelle
im Prompt). `cargo test --lib` 27 passed / 12 ignored.

## 49. Doctor und Erststart (2026-10-21, achtundzwanzigste Runde)

### Diagnose
- Einstellungen → Diagnose: Version, System, Keyring-Backend, Workspace, Anbieter/Modell,
  Datenbankpfad/-größe und erkannte Editoren. „Verbindung testen" lädt Modelle und
  Guthaben seriell (dieselbe Ursache wie in §44), „Diagnose kopieren" legt den Text in
  die Zwischenablage.
- Neuer Rust-Befehl `diagnostics` liest Keyring-Status, Settings und DB-Größe; der
  Provider-Test bleibt der einzige Netzschritt und wird von der UI ausgelöst.

### Erststart
- Ist die `settings`-Tabelle leer (frische Installation), erscheint ein Dialog: Modus
  (Programmierung/Alltag), Workspace (über `read_workspace_tree` validiert) und API-Key
  (OS-Schlüsselbund). „Später" setzt `onboarded` und zeigt den Dialog nicht wieder.
- Bestehende Installationen — auch solche ohne den neuen `onboarded`-Schlüssel — sehen
  den Dialog nie.
- Dabei einen älteren Sync-Fehler behoben: Der Modell-Select im Composer hing an einem
  lokalen Ref und folgte einem Moduswechsel oder dem Erststart nicht; er synchronisiert
  jetzt mit `settings.model`.

### Tests
- Live auf Windows mit geleerter Settings-Tabelle: Dialog erschien, „Für Programmierung"
  → „Los"; die Diagnose zeigte Keyring `keyring`, DB 80 KB, VS Code; der Verbindungstest
  meldete „2 Modelle · 27.58 USD".
- `pnpm build`/vue-tsc grün, `cargo test --lib` 27 passed / 12 ignored, Windows-Release
  mit `tauri/custom-protocol`. Die ursprüngliche Datenbank (Settings, Session, Workspace)
  wurde nach dem Test wiederhergestellt.
- Das Workspace-Feld wurde nach dem Live-Test ergänzt; die Validierung nutzt denselben
  `read_workspace_tree`-Befehl wie der Dateibaum.

## 50. Übergabeunterlage (2026-10-21, neunundzwanzigste Runde)

`HANDOVER.md` liegt jetzt im Wurzelverzeichnis: Architektur, Build/Dev/Release (inklusive
pnpm 10 und dem Windows-`custom-protocol`-Feature), Erststart, Speicherorte (SQLite,
Keyring, Attachments, Skills), Vertrauensmodell, bekannte Grenzen (kein Terminal/Git,
keine Signierung oder Updater, macOS-aarch64, keine Memory-Suche, kein Screenshot-Tool)
und die ersten Prüfschritte für neue Betreuer. Der Fresh-Install-Smoke-Test steht in §49.

## 51. Retry, Basis-URL und der idempotente Boot (2026-10-21, dreißigste Runde)

- Fehlgeschlagene Turns haben jetzt einen **„Erneut versuchen"**-Knopf. `engine_retry`
  rollt die Nachrichten und Tool-Calls nach der letzten User-Nachricht zurück, lässt die
  User-Nachricht stehen und startet denselben Turn neu, ohne sie ein zweites Mal
  einzufügen. Der Titel bleibt; fehlt eine User-Nachricht (Fehler vor dem Persistieren),
  meldet der Befehl das ehrlich.
- Die Fehlerblase überlebt jetzt einen Besuch in den Einstellungen: `initEngine` ist
  idempotent. Vorher lud das erneut montierte AgentPanel die gespeicherte Historie und
  verlor die Live-Fehlermeldung samt Retry-Knopf.
- Die **Basis-URL** ist endlich unter Einstellungen → Konto & Modell editierbar. Der Wert
  war im Engine-Config schon vorhanden, hatte aber kein Feld.
- Tests: `rollback_removes_the_failed_tail_but_keeps_the_user_message`; insgesamt
  **28 passed / 12 ignored**. Live auf Windows: Basis-URL auf einen 404-Pfad gestellt,
  Fehlerblase erschien; URL korrigiert, Settings geschlossen, Retry geklickt → Antwort
  „RETRY", der fehlgeschlagene Assistant-Row war aus der DB entfernt und die
  User-Nachricht stand genau einmal. Die ursprüngliche Datenbank wurde danach
  zurückgespielt.

## 52. Projektgedächtnis und „Wo waren wir" (2026-10-21, einunddreißigste Runde)

- `memories.workspace` (Migration, Default `''`): Notizen ohne Workspace gelten überall,
  Notizen mit einem Pfad nur in diesem Projekt. Das `memory`-Werkzeug hängt beim `add`
  den aktuellen `config.workspace` an; der Recall (`memory_context`) nimmt globale plus
  die des aktuellen Workspace; die Prompt-Zeilen markieren Projektnotizen mit
  `(project)`. Das Erinnerungs-Panel zeigt einen „Projekt"-Chip, der Pfad steht im
  Tooltip.
- `session_resume` liefert die Kompressions-Zusammenfassung. Die Nachrichtenliste zeigt
  sie als **sticky Karte „Wo waren wir?"** über dem Verlauf; `loadHistory` holt sie, die
  manuelle Verdichtung lädt neu. Sechs Sprachen.
- Tests: `memories_are_scoped_to_the_workspace`; **29 passed / 12 ignored**.
- Live: Der Partner legte „Die Testfarbe … Blau" per memory-Werkzeug mit Workspace
  `X:\DEV\dside-main` ab (Ausgabe „project"). Eine **neue Session ohne diesen Verlauf**
  antwortete „Blau. Das steht so in meinen Notizen zu diesem Projekt." Das Panel zeigte
  den Projekt-Chip; die eingefügte Zusammenfassung erschien als sticky
  „Wo waren wir?"-Karte. Die Original-DB wurde danach zurückgespielt.

## 53. Zuhause integriert und geprüft (2026-10-21, zweiunddreißigste Runde)

Das Handover aus Runde 45–52 ist sauber auf `main` (nach v0.4.9) angewendet
worden: `dside-ui-fix.patch` lief ohne Konflikt durch, die drei neuen Komponenten
(Erststart, Diagnose, Monaco-Diff), `AGENTS.md` und `HANDOVER.md` sind im Baum.

Geprüft auf Linux: `pnpm build` grün, `cargo test --lib` **29 passed / 12
ignored** (inklusive Dateiverlauf- und Workspace-Gedächtnis-Test). Version steht
in allen Manifesten auf 0.4.10. Der v0.4.10-Tag ist gepusht; der Release-Workflow
ist grün (verify, Linux, macOS, Windows, inklusive der neuen Bibliothekstests auf
Windows/macOS) und hat den Draft mit sieben Assets angelegt.

**v0.4.9 bleibt der veröffentlichte Stand**; v0.4.10 liegt als Draft bereit und
wird erst nach der Sichtprüfung veröffentlicht.

## 54. Tastenkürzel: sichtbar, änderbar, konfliktgeprüft (2026-10-21, dreiunddreißigste Runde)

Der Eintrag „Tastenkürzel" in den Einstellungen war bisher nur ein Menüpunkt ohne
Seite. Jetzt gibt es eine echte Liste — nicht die Vorlage eins zu eins, sondern
die Befehle, die die App wirklich hat.

### Registry statt verstreuter Handler
`src/engine/shortcuts.ts` beschreibt jeden Befehl einmal: ID, i18n-Label, Gruppe,
Geltungsbereich, Standardbelegung. `bindingFor` macht aus einem Tastaturereignis
eine kanonische Zeichenkette (`Ctrl+Alt+B`, `Alt+ArrowUp`), Cmd zählt auf macOS
als Ctrl. `activeBindings` liest die Overrides aus `settings.shortcuts`; ohne
Override gelten die Defaults. `bindingOwner` findet Konflikte, `setBindings` /
`resetCommand` / `resetAllBindings` schreiben die Overrides.

### Was tatsächlich belegt ist
Global und editierbar: Suchen (`Ctrl+K`, `Ctrl+Shift+P`), Einstellungen (`Ctrl+,`),
neue Aufgabe (`Ctrl+N`), Seitenleiste (`Ctrl+B`), Arbeitsbereich an/aus
(`Ctrl+Alt+B`), Arbeitsbereich öffnen (`Ctrl+O`), Tab schließen (`Ctrl+W`),
vorherige/nächste Aufgabe (`Alt+ArrowUp/Down`), Modus (`Ctrl+Shift+M`), Denkstufe
(`Ctrl+T`), Design (`Ctrl+Shift+L`). Fest im Composer: `Enter` sendet,
`Shift+Enter` macht den Zeilenumbruch. Bewusst **nicht** aufgenommen: Terminal,
Zoom, Zurück/Vorwärts und Modellmenü — dafür fehlen die Funktionen; tote Kürzel
wären schlimmer als keine.

### Panel
`KeyboardPanel.vue` zeigt die Tabelle mit Suche, „Alle auf Standard
zurücksetzen", Belegungs-Chips und Geltungsbereich. Ein Klick auf einen Chip
öffnet den Capture-Dialog; die nächste Kombination wird übernommen, `Esc` bricht
ab, `Backspace` leert die Eingabe. Globale Kürzel brauchen Ctrl/Alt/Cmd, sonst
würde Tippen sie auslösen. Konflikte mit einem anderen Befehl blockieren das
Speichern und nennen den Besitzer. Einzelne Belegungen lassen sich über das ×
entfernen; der Reset stellt die Defaults wieder her. Alles in sechs Sprachen.

`useShortcuts` ist der eine globale Listener (App.vue); der alte harte
`Ctrl+K`-Handler ist weg. Während der Capture-Dialog offen ist, pausiert er über
`shortcutCapturing`.

### Geprüft
`pnpm build` (vue-tsc + Vite) grün. Eine echte Sichtprüfung des Panels und der
Tastenkombinationen steht noch aus; die Belegungen sind bewusst so gewählt, dass
sie auf deutschen und US-Layouts funktionieren.

## 55. Der Composer folgt dem hellen Design (2026-10-21, vierunddreißigste Runde)

Im hellen Theme blieb die Eingabeblase dunkel: `--bg-composer` war nur im dunklen
Root definiert, und die Light-Variablen kannten die Variable nicht. Dadurch fiel sie
auf `#202020` zurück, während `color-scheme` schon hell war — daher der schwarze
Kasten mit weißem Placeholder und dem weißen Modell-Select.

Light setzt `--bg-composer` jetzt auf `#f7f7f7` (dieselbe Fläche wie `--bg-panel`);
Border, Text und der Select bleiben unverändert und passen nun zusammen. Das dunkle
Theme behält `#202020`. `pnpm build` grün; noch nicht committet, gehört zum
v0.4.11-Stand.

## 56. Terminal, Git, Augen, Ohren und das Ruder (2026-10-22, fünfunddreißigste Runde)

- Die vollständige v0.4.10 des Nutzers (`v0410`) ist jetzt die Basis; die lokale Kopie war
  nur unvollständig angekommen. Die unveränderte Referenz liegt außerhalb des
  Arbeitsverzeichnisses, damit das Feierabend-ZIP sauber bleibt. Die bisherigen Runden
  (Dateiverlauf, Gedächtnis, Retry, Diagnose, Tastenkürzel) sind darin enthalten.
- **Terminal:** `portable-pty` (Windows ConPTY, macOS/Linux PTY), `src-tauri/src/pty.rs`
  mit `pty_open/write/resize/close` und den Events `pty-output`/`pty-exit`.
  `TerminalPanel.vue` ist jetzt xterm am echten PTY; der Tab bleibt beim Wechsel gemountet,
  die Shell läuft weiter, Tab schließen beendet sie. Der globale Dispatcher lässt Tasten
  aus `[data-terminal-input]` an die Shell. Zusätzlich öffnet ein Button im oberen Kurzmenü
  (neben Browser und Dateibrowser) den Terminal-Tab.
- **Git:** `src-tauri/src/git_panel.rs` nutzt die Git-CLI (status mit porcelain `-z`,
  stage/unstage/stage_all/diff/commit, untracked-Diff, `CREATE_NO_WINDOW` unter Windows).
  `GitPanel.vue` zeigt Branch/Head, Gruppen, Diff und Commit; zusätzlich Zweigliste und
  -wechsel sowie Pull mit `--ff-only`. Kein Git-Crate, kein Push (dafür das Terminal).
- **Leitplanken:** `settings.commandPolicy`/`allowedCommands`/`workspaceOnly` gelten für den
  Companion, nicht für das Terminal. In „allowlist" prüft `bash` jedes einfache Segment:
  Programmname muss exakt in der Liste stehen; Pfade, `;`-Ketten mit unbekannten Programmen,
  Umleitungen, Backticks und `$()` werden abgelehnt. Default ist eine Liste gängiger Build-
  und Lesebefehle. `workspaceOnly` lehnt `write`/`edit` außerhalb des Workspace ab (auch
  `..` und Symlinks nach außen). Die Einstellungen haben einen eigenen Bereich „Leitplanken".
  Das ist eine Policy, keine OS-Sandbox; erlaubte Interpreter wie `node`/`python` bleiben
  mächtig, das Terminal bleibt frei. Tests: `the_command_allowlist_rejects_chains_and_redirection`,
  `workspace_only_rejects_parents_and_outside_paths`.
- **Workspace-Picker:** eigener Ordner-Browser auf `read_dir_children` (kein Tauri-Dialog-
  Plugin, keine neue Abhängigkeit): Pfadfeld, „nach oben", Ordnerliste und „Diesen Ordner
  wählen"; im Erststart und unter Einstellungen → Allgemein. Live geöffnet und
  `X:\DEV\dside-main` darüber gesetzt.
- **Code-Splitting:** `ViewFileTab`, Monaco, PDF, Office-Leser, Terminal, Git und Logs sind
  asynchrone Chunks; die Office-Leser werden erst beim Öffnen dynamisch importiert. Der
  Haupt-Chunk sinkt von **5,5 MB auf ~224 KB**; die großen Teile laden beim ersten Bedarf.
  Sichtprüfung: Datei-Tab mit Monaco nach dem Lazy-Load.
- **Augen:** `capture.rs` nimmt den virtuellen Desktop auf (Windows PowerShell, macOS
  `screencapture`, Linux grim/gnome-screenshot/spectacle/scrot/ImageMagick). Das volle PNG
  ist temporär und wird gelöscht; der Composer hängt eine 2000-px-JPEG-Version über
  `attachment_save` an. Zusätzlich gibt es das `screenshot`-Werkzeug: Der Companion kann
  selbst aufnehmen; in „Fragen" fragt er vorher, in „Mitlesen"/„Ruder" nimmt er direkt auf,
  das Bild erscheint im Tool-Verlauf und wird als User-Image an den nächsten Modellaufruf
  gehängt. **Grenze:** Der aktuelle DeepSeek meldet live, dass er das Bild nicht sieht — die
  Aufnahme kommt als Anhang an, das Modell ist aber text-only; für echte Augen braucht es
  ein vision-fähiges Modell. Die manuelle Aufnahme ist live geprüft, nicht an das Modell
  gesendet.
- **Ohren:** `logs.rs` hält einen Ring (400 Zeilen, je 2000 Zeichen); `log_line!` ersetzt die
  bisherigen `eprintln!`-Zeilen in App, Engine und Secret. `log_tail`, der Logs-Tab und das
  `logs`-Werkzeug für den Companion lesen ihn. Der Ring ist nur im Speicher, kein Transcript.
- **Memory-Suche:** Das Gedächtnis-Panel filtert die Karten über Titel, Text, Art, Scope und
  Workspace, ohne die Datenbank anzufassen; die stillgelegten Einträge werden mitgefiltert.
- **Ruder, drei Stufen:** `settings.permission` ist `read | ask | full` (Mitlesen, Fragen,
  Ruder). „Fragen" ist der neue Default für frische Installationen; der alte Boolean
  `fullAuthorization` wird beim Start migriert (true → full, false → read). Die Auswahl sitzt
  im Composer und in den Einstellungen. Nur „full" öffnet den `RudderDialog` mit Warnung und
  Pflicht-Checkbox; „Mitlesen" und „Fragen" gelten sofort. In „ask" hält der Engine-Loop vor
  jedem `write`/`edit`/`bash` an, emittiert `action_required` mit `kind: permission` und dem
  konkreten Befehl/Inhalt, und wartet auf „Einmal erlauben", „Für diese Sitzung erlauben"
  oder „Ablehnen". „Für diese Sitzung" merkt sich die Freigabe nur im Speicher (pro Session,
  weg beim Moduswechsel oder Neustart). Ein genehmigter Aufruf bekommt einen eigenen
  Vollzugriff-Kontext; ein abgelehnter liefert dem Modell eine Absage. Eine kleine harte
  Untergrenze bleibt auch im Ruder: gefährliche Shell-Muster (`rm -rf /`, `sudo`, `mkfs`,
  `git reset --hard`, `curl | sh`, …) fragen einmal nach. Geplante Läufe werden in „ask"
  read-only ausgeführt, weil dort niemand antworten kann. Der Sub-Agent kann in „ask" nicht
  fragen; seine Schreibzugriffe bleiben gesperrt.
- **Geprüft:** `vue-tsc`, `vite build`, **34 passed / 13 ignored**; live: Terminal/Git auf
  v0.4.10, Logs-Tab, Screenshot-Aufnahme + JPEG-Anhang, Ruder-Dialog mit Checkbox,
  **„Fragen" end-to-end** (write → Frage → „Einmal erlauben" → Datei geschrieben) und
  **„Für diese Sitzung"** (erster Write geschrieben; der Folgeturn scheiterte am transienten
  Provider-Netzwerk, nicht an der Freigabe), Leitplanken-Panel, Workspace-Picker und
  Monaco-Lazy-Load. Version bleibt 0.4.10; zuhause als **v0.4.11** taggen. Die
  Änderungspakete sind Referenz; das Arbeitsverzeichnis ist der vollständige Stand.

## 57. Zuhause vollständig übernommen (2026-10-22, sechsunddreißigste Runde)

Der komplette Arbeitsstand aus §56 ist per Snapshot übernommen, nicht nur der
All-Patch: Quellcode, Tests, Manifeste und Doku. `pnpm install` normalisierte den
Lockfile-Stand, `pnpm build` (vue-tsc + Vite) ist grün, `cargo test --lib` meldet
**34 passed / 13 ignored**. Die 0-Byte-Datei `src-tauri/1.blob` ist entfernt; die
generierten Plattform-Schemas (`src-tauri/gen/...`) wurden bewusst nicht
mitkopiert.

Enthalten sind Terminal/PTY, Git-Panel, Ruder mit drei Stufen, Leitplanken,
Workspace-Picker, Augen/Ohren, Code-Splitting und die Gedächtnis-Suche. Die
Version bleibt 0.4.10. **Kein Tag und keine Veröffentlichung**, bis die
Namensgebung besprochen ist; danach wird v0.4.11 daraus.

## 58. Klarere Namen: Planmodus, Vollzugriff, Richtlinien (2026-10-22, siebenunddreißigste Runde)

Die Metaphern aus §56 waren im Alltag zu sperrig. Die drei Freigabestufen heißen
jetzt:

- **Planmodus** (vorher „Mitlesen") — nur lesen und suchen.
- **Vor Änderungen fragen** (vorher „Fragen") — jede Änderung und jeder Befehl wird
  bestätigt.
- **Vollzugriff** (vorher „Ruder: {name}") — ohne Rückfrage, bis der Nutzer ihn
  zurücknimmt.

Der Dialog heißt entsprechend „Vollzugriff für {name}?" (Checkbox, Bestätigung,
Zurücknehmen), die Erklärtexte und die Freigabe-Hinweise sind angepasst; die
gespeicherten Werte (`read`/`ask`/`full`) bleiben unverändert. Der Einstellungs-
bereich „Leitplanken" heißt jetzt **Richtlinien**; „Sandbox" bleibt in der
Erklärung, weil der Begriff üblich ist. Die Modell-/Tool-Texte, die den Nutzer auf
eine Stufe verweisen, nennen die neuen Namen. `HANDOVER.md` ist auf die neuen
Begriffe umgestellt; „Augen/Ohren" stehen nur noch in §56 als Chronik — die UI
sagt „Bildschirm aufnehmen" und „Logs".

Weil „Vor Änderungen fragen" länger ist, wurde die gemeinsame Lese-Spalte von
520–860 px auf **560–900 px** erweitert und der Freigabe-Select bekommt 190 px
(unter 440 px Composer-Breite 110 px, damit nichts aus der Blase läuft). Die
Einstellungs-Segmente dürfen umbrechen.

Terminal: bleibt als **Work-Area-Tab**. Ein Dock unter dem Agenten würde dauerhaft
Höhe kosten und die Tab-Logik doppeln; der Toolbar-Knopf öffnet es, der Tab bleibt
beim Wechseln am Leben. Ein separates Dock kann später dazukommen.

`pnpm build` grün. Kein Commit/Tag in dieser Runde; die Änderungen gehören in den
v0.4.11-Stand.

## 59. v0.4.11 gebaut und geprüft (2026-10-22, achtunddreißigste Runde)

Die Namensrunde und die README sind als `08963c6` auf beiden Remotes; der Tag
`v0.4.11` zeigt darauf. Der Release-Workflow ist grün:

- verify (pnpm build + `cargo test --lib`) — **34 passed / 13 ignored**
- Linux, macOS und Windows gebaut; die Bibliothekstests auf Windows/macOS liefen mit.
- Der Draft enthält alle sieben Assets (`.deb`, `.rpm`, `.AppImage`, Linux-Tar,
  Windows-Zip und -Setup, macOS-DMG).

Die CI hatte diesmal **zwei** Drafts für denselben Tag angelegt: einen mit dem
Windows-Setup, einen mit den übrigen sechs Assets (Race zwischen den Matrix-Jobs).
Das Setup wurde per API in den Haupt-Draft hochgeladen, der doppelte Draft danach
gelöscht; übrig bleibt ein Draft mit sieben Assets. Für künftige Releases sollte
der Draft vor der Matrix einmal angelegt oder nur aus einem Job erzeugt werden.

`v0.4.9` bleibt „Latest"; `v0.4.11` liegt als Draft bereit, bis der Nutzer den
Windows-/macOS-Test gemacht hat. Der alte `v0.4.10`-Draft ist damit überholt.

## 60. v0.4.11 veröffentlicht (2026-10-22, neununddreißigste Runde)

Nach dem Windows-11-Test (dem Entwicklungsziel) ist v0.4.11 als **Latest**
veröffentlicht; die Release-Notes nennen Terminal, Git, Freigabestufen,
Richtlinien, Workspace-Picker, Bildschirmaufnahme, Logs, Tastenkürzel,
Code-Splitting und Gedächtnis-Suche. Ein Mac steht nicht zur Verfügung: Der
macOS-Build ist ein unsigniertes aarch64-CI-Artefakt und ausdrücklich als
**nicht auf echter Hardware getestet** gekennzeichnet (README und Release-Notes).
Der v0.4.10-Draft ist gelöscht, die Tags v0.4.10 und v0.4.11 bleiben.

Offen für später: Der Release-Workflow legte bei v0.4.11 zwei Drafts an (Race der
Matrix-Jobs); der Draft sollte künftig vor der Matrix einmal erzeugt werden.

## 61. Übersetzte READMEs für die Webseite (2026-10-22, vierzigste Runde)

Die Webseiten-Fassungen liegen jetzt als `README_de.md`, `README_fr.md`,
`README_es.md`, `README_ru.md` und `README_zh.md` im Repo; `README.md` hat oben
den Sprachumschalter. Korrekturgelesen und angeglichen:

- Freigabestufen und Einstellungsnamen entsprechen der App (Planmodus / Vor
  Änderungen fragen / Vollzugriff; Richtlinien; Tastenkürzel; Diagnose; Konto &
  Modell).
- Formatierungsfehler behoben (Logs und „Integrierter Browser" waren in einer
  Zeile, die Struktur-Überschrift klebte am Testabsatz, signierte Installer und
  CI-Builds waren ein Bullet).
- macOS steht auch hier als CI-only und nicht auf echter Hardware getestet;
  Alt-Text des Screenshots korrigiert.
- Alle relativen Links (Sprachumschalter, `PROGRESS.md`, `LICENSE`,
  `docs/dside_zh.png`, Font-Lizenz) geprüft; keine fehlenden Ziele.

Kein Code betroffen. Die Dateien sind committed und gepusht; der Tag v0.4.11
bleibt unverändert.

## 62. Die Augen waren richtig, der Draht war falsch (2026-10-22, einundvierzigste Runde)

Der Test mit dem Schaf-Bild im Chat hat den entscheidenden Hinweis gegeben: Das
Modell kann sehen. Der Fehler lag in der Leitung, nicht im Auge.

### Befund
`ChatMessage` schickte Bilder als **top-level Feld** `image: [...]`. Die
DeepSeek-API akzeptiert das (HTTP 200), ignoriert es aber — das Modell antwortet,
als wäre kein Bild dabei. KiloCode (OpenAI-kompatibler Pfad) sendet Bilder
stattdessen als Content-Parts:
`content: [{type:"text",…}, {type:"image_url", image_url:{url:"data:…"}}]`.

Ein direkter Probe-Aufruf an `https://api.deepseek.com/v1/chat/completions` mit
`deepseek-flash` und einem 520×180-PNG („HALLO 4711") in dieser Form gab exakt
`HALLO 4711` zurück; die `prompt_tokens` (232) zeigen, dass das Bild mitgezählt
wurde. Mit dem bisherigen top-level `image`-Feld blieb genau dieser Effekt aus.

### Behoben
- `wire_messages` in `deepseek.rs` übersetzt `ChatMessage.image` in OpenAI-
  `image_url`-Content-Parts; das interne Feld bleibt für Engine/DB unverändert.
  Text, mehrere Bilder, Tool-Calls und Tool-Results werden weiterhin korrekt
  serialisiert; ein Test deckt die Form ab.
- Der Live-Test `live_deepseek_takes_an_image` nutzt jetzt ein echtes Fixture
  (`src-tauri/tests/fixtures/vision.png`, „HALLO 4711") und prüft, dass das
  Modell den Text zurückliest.
- Ergebnis: `cargo test --lib` **35 passed / 13 ignored**; der Live-Test lief
  mit echtem Key durch (`answer: "HALLO 4711"`).
- README, HANDOVER und die fünf Übersetzungen korrigiert: Die Aussage aus §56
  („text-only, kann Bilder nicht sehen") war falsch. Bilder sind echte
  Bildeingaben; sehr kleine Bilder (1×1) lehnt der Anbieter ab.

Der Fix ist noch **nicht committet/getaggt**; er ist der Kandidat für v0.4.12.

## 63. Ein ganzer Zug auf einen Blick (2026-10-22, zweiundvierzigste Runde)

Turn-Diff und Turn-Undo sind drin. Die Datei-Historie war bisher pro Datei; jetzt
lässt sich der ganze Zug prüfen und gemeinsam zurücknehmen.

### Daten
- `file_versions` hat `turn_id` (Migration). Der Engine-Turn übergibt die ID der
  Assistant-Zeile an `record_file_snapshot`; Sub-Agenten erben die Turn-ID des
  Parents, damit ihre Schreibzugriffe im selben Zug landen.
- `latest_turn_id` findet den letzten Zug mit Dateiänderungen;
  `turn_versions` nimmt pro Pfad den **ersten** Snapshot (der Stand *vor* dem
  Zug); `has_later_version` erkennt Änderungen nach dem Zug.
- `turn_diff` liefert pro Datei before/after (Datei auf der Platte), `canUndo`
  ist false, wenn später etwas geändert wurde. `turn_revert` prüft das vorher,
  sichert den aktuellen Stand über die bestehende `revert_file_version`-Logik
  und stellt dann alle Dateien des Zuges wieder her. Ein Turn-Undo ist damit
  selbst über die Datei-Historie rückgängig zu machen.

### Oberfläche
- Neuer Work-Area-Tab **„Änderungen"** (Icon `diff`) mit Dateiliste und
  Monaco-Diff (before/after, Sprache je Datei), Refresh und „Zug zurücknehmen".
  Der Tab lädt den letzten Zug mit Änderungen; nach `turn_end` wird eine
  Revision erhöht, sodass ein offener Tab automatisch nachzieht. Bei späteren
  Änderungen ist der Undo-Knopf deaktiviert und verweist auf den Dateiverlauf.
- i18n in allen sechs Sprachen; `turnDiff.*` und `workArea.tabChanges`.

### Geprüft
`cargo test --lib` **37 passed / 13 ignored** (neuer DB-Test für die
Erst-Snapshots und ein Integrationstest, der zwei Dateien ändert, den Diff
prüft und beide zurücksetzt). `pnpm build` grün.

Version 0.4.13; v0.4.12 bleibt der Vision-Fix. Kein Tag in dieser Runde — der
Tag folgt direkt nach dem Commit.

## 64. v0.4.13 veröffentlicht (2026-10-22, dreiundvierzigste Runde)

Der Turn-Diff ist als **v0.4.13** veröffentlicht (Latest) und enthält zugleich den
Vision-Fix: Bilder gehen als OpenAI-`image_url`-Parts raus, `deepseek-flash` liest
Text und Layout; der Changes-Tab zeigt den letzten Zug mit Dateiänderungen und
kann ihn gemeinsam zurücknehmen.

Der separate v0.4.12-Lauf (nur Vision-Fix) war auf dem Ubuntu-Runner im
apt-Schritt hängen geblieben; v0.4.13 war schneller fertig und enthält denselben
Fix. Der v0.4.12-**Tag** bleibt als Historie, der redundante Draft wurde gelöscht.
Die Release-Liste ist damit sauber: v0.4.13 (Latest), v0.4.11, v0.4.9.

**CI v0.4.13:** verify und Linux/macOS/Windows grün; sieben Assets; 37 Rust-Tests
(13 ignored). Der Vision-Fix ist im Live-Test mit dem Fixture „HALLO 4711" belegt.

## 65. Anhänge gingen beim Senden verloren (2026-10-23, vierundvierzigste Runde)

Der Windows-Test hat einen echten Composer-Fehler gefunden: `submit()` rief
`sendPrompt(value)` ohne die angehängten Bilder auf, und `reset()` leerte nur den
Text, nicht `attachments`. Die Bilder blieben also im Composer liegen und wurden
nie mitgeschickt.

Jetzt wird vor dem Reset eine Kopie der Anhänge gemacht, `sendPrompt(value,
outgoing)` bekommt sie mit, und `reset()` räumt auch `attachments` ab. Das betrifft
alle Plattformen, nicht nur Windows.

`pnpm build` grün; noch kein Tag — der Stand geht so in den Windows-Test.

## 66. Browser lesen und Formulare füllen (2026-10-23, fünfundvierzigste Runde)

Der Browser ist jetzt ein Werkzeug, kein reines Fenster mehr — aber bewusst nur
hinter einer Einstellung.

### Freigabe
`settings.browserControl` (Standard **aus**) wird unter Einstellungen → Allgemein
eingeschaltet und als `browserControl` an die Engine gegeben. Ist es aus, tauchen
`browser_read` und `browser_fill` gar nicht erst im Werkzeugangebot auf. Im
Planmodus ist Lesen erlaubt, Füllen verboten; in „Vor Änderungen fragen" fragt
das Füllen über den bestehenden Freigabedialog, im Vollzugriff läuft es direkt.

### Lesen ohne Remote-IPC
Die fremde Seite bekommt keine Tauri-IPC-Rechte. Der Host führt ein Extraktions-
skript per `eval` aus; das Skript schreibt URL, Titel, sichtbaren Text (8k),
Links (40) und Formularfelder (60, Passwortwerte ausgelassen) als Base64 in den
URL-Hash. Rust liest den Hash über `window.url()`, entfernt ihn wieder und gibt
JSON zurück. Das umgeht die Capability-/`dangerousRemoteDomainIpcAccess`-Frage
und funktioniert auch für fremde Origins; CSP-Seiten können das Skript blockieren,
dann kommt eine ehrliche Fehlermeldung.

### Füllen
`browser_fill` nimmt Selektoren aus `browser_read` plus Werte, setzt native
Setter (React-kompatibel), feuert `input`/`change`, unterstützt Checkboxen,
Radios und Selects (Wert oder Optionstext) und kann ein Formular absenden
(`submitSelector` oder das erste Formular). Das Ergebnis meldet Anzahl und
Submit-Status über denselben Hash; nach einer Navigation zählt die Absicht.

### Grenzen
Nur das jeweils neueste (oder per `id` benannte) Fenster; keine Frames, keine
Klicks auf beliebige Links, kein Login-Autofill. Die Einstellung ist eine
bewusste Nutzerfreigabe, keine Sandbox. **Noch nicht in einer echten WebView
durchgeklickt** — der Windows-Test soll das übernehmen.

`cargo test --lib` **38 passed / 13 ignored** (Hash-Parser-Test), `pnpm build`
grün. Kein Tag/Build; der Stand ist für den Twin gedacht.

## 67. Linux-Durchlauf: fünf Fixes und ein vergifteter Tool-Call (2026-10-23, sechsundvierzigste Runde)

Erster vollständiger Durchlauf auf Linux (CachyOS, KDE Wayland, WebKitGTK) mit
frischem Datenverzeichnis (`/tmp/dside-e2e`) und eigenem Test-Workspace
(`/tmp/dside-workspace`). Der echte Keyring-Schlüssel wurde erkannt; Onboarding,
Chat mit `deepseek-flash` und die Provider-Diagnose liefen live. Geprüft:
Onboarding, Chat (read/write/read-back), Freigabedialog („Einmal erlauben"),
Gedächtnis mit Projekt-Scope, Dateihistorie mit Diff und Restore (eine neu
angelegte Datei wird wieder entfernt), Terminal (echtes PTY), Git-Panel,
Vorschauen (Markdown, Bild, DOCX, XLSX, CSV, PDF), Einstellungen, Diagnose
(„Verbindung OK · 2 Modelle · 20,91 USD"), Theme hell/dunkel, Sprachwechsel
de/en, Persistenz über Neustarts, Editor-Picker (öffnet Code-OSS), Suche
(Ctrl+K). Vision live: `live_deepseek_takes_an_image` liest „HALLO 4711".

### Gefunden und behoben
- **Work Area zu schmal.** Der Default war 15 %; ein Datei-Tab reserviert 220 px
  für den Baum, dem Vorschaufenster blieben auf 1631 px noch ~25 px. Jetzt 30 %;
  der alte Default 15 wird beim Start auf 30 gehoben, ein bewusst gewählter
  anderer Wert bleibt.
- **Composer ohne Fokus.** Nach dem Start und nach jedem Turn stand der Cursor
  nicht im Eingabefeld. `Composer` fokussiert beim Mounten; `AgentPanel` setzt
  den Fokus nach `running = false` zurück, aber nur wenn der Nutzer nicht
  gerade in einem anderen Feld tippt.
- **Ein abgebrochener Tool-Call vergiftete die Sitzung.** Wird ein Turn während
  der Freigabe gestoppt (oder die App neu gestartet), bleibt eine
  Assistant-Nachricht mit `tool_calls` ohne Tool-Antwort in der DB. Jede weitere
  Anfrage scheiterte mit HTTP 400 („insufficient tool messages following
  tool_calls"); der Retry half nicht, weil er nur den letzten Turn zurückrollt.
  `wire_messages` ergänzt fehlende Antworten jetzt als
  `"Cancelled before the tool ran."` (nur auf dem Draht; die DB-Historie bleibt
  unverändert). Test `a_cancelled_tool_call_is_padded_for_the_provider`; live
  erholte sich die vergiftete Sitzung nach dem Fix per Retry.
- **Änderungen-Tab verschwand nach Neustart.** `KINDS` in `workArea.ts` kannte
  `changes` nicht; `isTab` verwarf den Tab beim Restore. Jetzt enthalten.
- **Browser-Panel ohne Fokus im Adressfeld.** `WebDesignPanel` fokussiert das
  URL-Feld beim Öffnen (Watch auf `webOpen`); live noch nicht bestätigt.

### Geprüft
`pnpm build` (vue-tsc + Vite) grün; `cargo test --lib` **39 passed / 13
ignored**; Vision-Live-Test grün. Kein Commit/Tag — der Nutzer testet zuerst.

### Offen / Beobachtungen
- **Der Browser kann keine Seite öffnen.** `browser_read`/`browser_fill` sind
  vorhanden (der Agent nannte beide), aber `browser_read` liefert ohne bereits
  geöffnetes Fenster „no browser window is open". Der Agent kann also nur ein
  vom Nutzer geöffnetes Fenster lesen. Der Fill-Test endete deshalb vor dem
  Formular; die Extraktions- und Fill-Logik ist weiterhin nur per Unit-Test
  gedeckt. Ein `browser_open` (oder ein optionales `url` an `browser_read`) wäre
  die naheliegende Ergänzung.
- Der Restore fragt mit `window.confirm` (nativer WebView-Dialog) statt mit
  einem App-Dialog nach; funktional, aber ein Stilbruch.
- Der Follow-Mode öffnet eine Datei schon beim `write`-Aufruf, bevor sie
  existiert; der Tab zeigt bis zur Freigabe „Datei konnte nicht gelesen werden."
- Der Stop-Knopf ergänzt die fehlende Tool-Antwort nicht in der DB; der
  Wire-Fix heilt den nächsten Request, die Historie zeigt den offenen Call
  weiter als „fehlgeschlagen".
- Bei manuell sehr schmal gezogener Work Area bleibt der Datei-Tab mit
  220-px-Baum eng; der neue Default verhindert nur den ersten Fall.
- Die Wayland-Sitzung ließ keine synthetische Mausbewegung zu (XWarpPointer und
  XTest blieben wirkungslos); getestet wurde über Tastatur, Shortcuts,
  DB-Einstellungen und Neustarts. Windows/macOS bleiben offen.

## 68. Der Browser öffnet selbst, das Gedächtnis wird durchsuchbar (2026-10-23, siebenundvierzigste Runde)

Die offenen Punkte aus §67 sind abgearbeitet; zwei eigene Ergänzungen kamen dazu.

### Browser öffnet selbst
`browser_read` hat jetzt ein optionales `url`. Ist es gesetzt, ruft der Engine-Loop
`browser::browser_open_read` auf: ein **neues** Fenster (die Seiten des Nutzers
werden nicht navigiert), `focus: false` (der Composer behält die Tastatur), und
erst lesen, wenn das Fenster `about:blank` verlassen hat (bis zu sechs Versuche).
`normalize_url` ergänzt bei einem nackten Host `https://`; Test
`a_bare_host_gets_https`. Das Ergebnis enthält die Fenster-`id`, sodass
`browser_fill` direkt anschließen kann.

Live mit der lokalen Testseite (`/tmp/dside-browser`): `browser_read` lieferte
Titel „DS.IDE Browser-Test" und „FLINKWAL 42"; `browser_fill` meldete
`{"filled":5,"submitted":true}`; die zweite `browser_read` sah
`/submitted?name=Test&email=test%40example.com&comment=Hallo+Browser&agree=on&farbe=blau`
und die Werte im Text. Der Dialogpfad für Fill ist derselbe `approve_tool`-Pfad
wie beim Schreiben (in §67 live geprüft); für den automatisierten Lauf wurde der
Scheduler mit Vollzugriff benutzt, weil die Wayland-Testsitzung keine native
Tastatur an das App-Fenster liefert. Der Scheduler selbst lief dabei zum ersten
Mal live (einmaliger Lauf, danach gelöscht).

### Recall
Neues, schreibgeschütztes Werkzeug `recall`: `db.recall(query, limit, workspace)`
sucht aktive Erinnerungen (global plus aktueller Workspace) und Sessions/
Nachrichten (dieselbe Suche wie der Suchdialog) und liefert `RecallHit` mit
kind/title/text/source/sessionId/workspace. Test
`recall_finds_memories_and_past_messages` (Projekt-Scope). Live: 8 Treffer,
darunter `#1 [fact/user] Testfarbe: Blau` und die Sitzung „Testfarbe Blau merken".
Das Werkzeug ist im Planmodus erlaubt und braucht keine Freigabe.

### Restore, Follow, Stop
- Der native `window.confirm` ist durch `ConfirmDialog.vue` ersetzt: eigener
  Dialog, Fokus auf „Abbrechen", Escape und Klick außerhalb schließen. Sechs
  Sprachen (`viewFile.restoreTitle`, `viewFile.cancel`, `viewFile.removed`,
  `viewFile.removedHint`).
- Eine wiederhergestellte Fassung mit `existed=false` entfernt die Datei; der
  Tab zeigt jetzt „Die Datei wurde durch das Wiederherstellen entfernt" samt
  Hinweis auf den Verlauf statt „Datei konnte nicht gelesen werden".
- Follow-Mode: `read` öffnet sofort; `write`/`edit` erst, wenn `tool_result`
  ohne Fehler zurückkommt. Während der Freigabe und bei Fehlern bleibt der
  Datei-Tab unangetastet.
- Beim `turn_end` mit `stopReason=abort` werden noch laufende Werkzeugzeilen als
  `cancelled` markiert und heißen „abgebrochen"; der Wire-Fix aus §67 hält die
  Session gültig.

### Fokus und schmale Work Area
- Nach dem Splash sendet Rust `main-shown`; der Composer registriert den
  Tauri-Fokus- und den `main-shown`-Listener sofort und fokussiert beim
  Sichtbarwerden, ohne ein Feld zu stehlen. Zusätzlich ein einmaliger 9-s-Fallback
  (der Kaltstart-Fokus ging vorher verloren, weil das Hauptfenster beim Mounten
  noch verborgen war). In der Wayland-Testsitzung ließ sich der erste Tastendruck
  nicht live bestätigen — die native Tastatur erreicht das App-Fenster dort
  nicht; der Fix ist gebaut und typgeprüft.
- Die Baumspalte ist `minmax(120px, 220px)` statt fest 220px: auch bei schmaler
  Work Area bleibt der Vorschau eine nutzbare Spalte.

### Kompression
`estimated_tokens` zählt jetzt Tool-Argumente und Bilder mit (1000 Tokens je
Bild, bewusst vorsichtig); der veraltete Kommentar im ContextRing („does not
exist yet") ist korrigiert. Die Entscheidung bleibt eine Schätzung; die echte
`prompt_tokens`-Usage als Auslöser ist weiterhin offen.

### Geprüft
`pnpm build` (vue-tsc + Vite) grün; `cargo test --lib` **42 passed / 13
ignored**; Browser- und Recall-Live-Test; Scheduler-Lauf. Testdaten lagen in
`/tmp/dside-e2e`; die echten Sitzungen und Einstellungen blieben unberührt.
Test-Schedules gelöscht, Test-Permission wieder auf `ask`. Kein Commit/Tag.

## 69. Leer ist nicht kaputt (2026-10-23, achtundvierzigste Runde)

Rückmeldung aus dem Einsatz: Die Werkzeuge meldeten „leer" und „kaputt" gleich.
Vier Punkte, alle umgesetzt.

- **glob/grep:** Vor der Suche wird die Suchwurzel geprüft. Fehlt der Workspace,
  kommt `workspace /pfad does not exist` (bei explizitem `path`: `path ...`),
  kein `(no matches)`. Test `glob_and_grep_report_a_missing_workspace`.
- **Start:** `initEngine` lädt den Baum jetzt beim Boot; `read_workspace_tree`
  nennt den fehlenden Pfad, `reloadTree` übersetzt ihn in
  „Arbeitsverzeichnis {path} existiert nicht oder ist kein Ordner." Die
  Composer-Statuszeile zeigt den Hinweis mit „Einstellungen öffnen", der
  Dateibaum ebenso; sechs Sprachen. Live mit `/tmp/dside-missing-workspace`
  bestätigt.
- **recall:** Der Workspace wird in die Treffer geschrieben und die
  Gesprächssuche darauf eingegrenzt (global/ohne Workspace bleibt sichtbar).
  `SearchHit` trägt den Workspace; der Test deckt zwei Projekte in beide
  Richtungen ab.
- **Reveal:** `ViewFileTab.revealFile` klappt die Elternordner auf und scrollt
  die markierte Zeile in den Blick; der falsche Kommentar in `select()` ist
  ersetzt. Live: Der Begleiter schrieb `src/reveal-test2.txt`; der Baum zeigte
  `src` aufgeklappt und die Datei markiert, der Tab den Inhalt.
- **PROGRESS:** Zeile 11 steht jetzt auf `~/Projects/DS.IDE`.

Geprüft: `pnpm build` grün, `cargo test --lib` **43 passed / 13 ignored**,
Statuszeile und Reveal live. Test-DB wieder auf `ask`, Schedules gelöscht,
Follow wieder an. Kein Tag.

## 70. Hände für die Oberfläche (2026-10-23, neunundvierzigste Runde)

Die Wayland-Sitzung ließ keinen nativen Eingabeweg in das Tauri-Fenster zu:
XTest kommt nicht an, das X11-Fenster ist ein Ghost; ein Screenshot des
Electron-Nachbarn zeigte nur den Desktop. Der DeepVibe-Test über den
Chromium-Debug-Port zeigte, dass der richtige Weg **in** der App liegt, nicht
im Betriebssystem. DS.IDE hat kein CDP; deshalb eine eigene, kleine Brücke.

### Die Brücke
`src-tauri/src/test_bridge.rs`: In Debug-Builds und nur mit
`DS_IDE_TEST_BRIDGE=1` lauscht sie auf `127.0.0.1:9333`
(`DS_IDE_TEST_BRIDGE_PORT` überschreibt). `POST /eval` führt JavaScript im
Haupt-Webview aus; das Skript bekommt `report(data)`, das über den neuen
Tauri-Command `test_bridge_report` einen JSON-Bericht mit Sequenznummer ablegt;
`GET /report` liefert ihn, `GET /health` ist der Lebendtest. Der Release-Build
enthält den Server nicht (`#[cfg(debug_assertions)]`), nur den harmlosen
Report-Command. Client: `scripts/ui-test.mjs` (`report`, `eval`, `wait`,
`shot` über xdotool/ImageMagick). Der Companion kann die Brücke per
bash/python nutzen, wenn die App so gestartet wird.

### Was die Brücke geprüft hat
Isolierte Testdaten (`/tmp/dside-e2e`, Workspace `/tmp/dside-workspace`,
Ask-Modus, Follow an, Browser-Steuerung an), lokaler Formularserver auf 8765:

- **Fokus:** Nach dem Splash ist `document.activeElement` das Textarea des
  Composers — vorher war das nur typgeprüft, jetzt live gelesen.
- **Restore-Dialog:** Verlauf öffnen, „Schreiben · Neu angelegt“ wählen,
  „Wiederherstellen“: der eigene Dialog „Fassung wiederherstellen?“ erscheint,
  der Fokus liegt auf „Abbrechen“. Abbrechen lässt Datei und Verlauf stehen;
  Bestätigen entfernt `src/reveal-test2.txt`, der Tab zeigt „Die Datei wurde
  durch das Wiederherstellen entfernt …“, `read_text_file` meldet os error 2.
- **Write + Approval:** Prompt an den Companion, Freigabe „Einmal erlauben“
  per DOM-Klick; `src/bridge-test.txt` entsteht; nach dem Turn liegt der Fokus
  wieder im Composer.
- **Reveal-Fehler gefunden und behoben:** Der Baum war beim Start gelesen; eine
  danach geschriebene Datei in einem bereits geladenen Ordner fehlte, die
  Markierung war `null`. `revealInTree` lädt die Elternordner auf dem Weg zum
  Ziel jetzt immer neu; `revealFile` lädt den ganzen Baum einmal nach, wenn ein
  neuer Eintrag auf oberster Ebene fehlt. Danach: `bridge-test.txt` markiert,
  die entfernte `reveal-test2.txt` verschwunden.
- **Browser-Fill mit Approval:** `browser_read` mit URL öffnet `browser-1`;
  `browser_fill` löst den Frage-Dialog aus; „Einmal erlauben“; 5/5 Felder und
  Submit; die Fenster-URL ist
  `http://127.0.0.1:8765/submitted?name=Test&email=test%40example.com&comment=Hallo+Browser&agree=on&farbe=blau`;
  der Companion liest die Seite und berichtet die Werte; der Fokus ist zurück
  im Composer. Screenshots: `/tmp/dside-restore-dialog.png`,
  `/tmp/dside-restore-missing.png`, `/tmp/dside-write-follow.png`,
  `/tmp/dside-browser-final-main.png`, `/tmp/dside-browser-submitted.png`.

### Geprüft und offen
`pnpm build` (vue-tsc + Vite) grün; `cargo test --lib` **45 passed / 13
ignored** (zwei Brückentests). Die Brücke ist ein Testwerkzeug, kein
Produktpfad: Debug-Build plus explizite Umgebungsvariable, nur localhost. Die
Screenshot-Hilfe im Client ist Linux-spezifisch; `eval`/`report` sind
plattformunabhängig. Windows/macOS bleiben als Laufzeitprüfung offen; die
echte `prompt_tokens`-Usage als Kompressionsauslöser ebenfalls.

## 71. Antworten kopieren (2026-10-23, fünfzigste Runde)

Feedback aus dem Einsatz: An jede Antwort gehört ein „Copy Markdown“-Button; und
die Frage, ob das Punkte-/Vertrauenssystem noch geplant ist. Beides beantwortet,
der Button gebaut; kein Commit, der Nutzer testet zuerst.

### Copy Markdown
- Jede Assistant-Antwort hat unten rechts eine kleine Aktionszeile
  (`message-actions`) mit „Markdown kopieren“. Kopiert wird `message.text` — der
  rohe, zusammengefügte Markdown-Quelltext, nicht das gerenderte HTML; Codeblöcke
  und Tabellen bleiben also erhalten.
- Der Button zeigt beim Klick „Kopiert“ mit Häkchen, bei einem Fehler
  „Kopieren fehlgeschlagen“ mit x; der Zustand fällt nach 1,8 s zurück.
- Clipboard: `navigator.clipboard.writeText`, mit `textarea`+`execCommand`-
  Fallback für ältere Webviews. Kein neues Plugin, kein Rust.
- Sechs Sprachen (`message.copyMarkdown/copied/copyFailed`), neues `copy`-Icon.
- Live über die Brücke: 6 Assistant-Antworten, 6 Buttons; Klick → `copy copied`,
  „Kopiert“; die X11-Clipboard-Ablage enthält den rohen Markdown inklusive
  Tabelle. Screenshots `/tmp/dside-copy-buttons.png`, `...-clicked.png`.

### Das Punktesystem war geplant
`PROGRESS` §35/§36: Der Begleiter wählt seinen Namen früh selbst, aber der Nutzer
erfährt ihn erst ab einer Punktzahl; Punkte für Verhalten über Zeit (Streaks,
erledigte Aufgaben, Korrekturen, Gedächtnispflege), nicht für einzelne Klicks;
eigene Tabelle, Fortschritt auf der Begleiterseite, kleine Enthüllung. „Ziggy ist
bereits enthüllt — gilt für neue Installationen.“ Gebaut ist es nicht; die
Thumbs-up/down sind darin nicht vorgesehen. Der aktuelle Partner-Tab zeigt den
gewählten Namen sofort. Nächste Runde, wenn die Schwelle feststeht.

### Die eine Ursache im echten DS.IDE
Die reale Datenbank stand auf `workspace=~/Projects/TEST` (Ordner weg).
Die App lief nicht; Einstellung auf `~/Projects/DS.IDE` gesetzt, Backup
`ds-ide.sqlite3.bak-workspace-20261023`. Damit sollten glob, grep, bash, write,
edit und code_review beim nächsten Start den Workspace finden.

### Werkzeug-Liste
Der veraltete Kommentar an `tools::definitions` („seven tools“) nennt jetzt die
20 Werkzeuge und die geteilte Ausführung (`execute` hier, der Rest in
`engine/mod.rs`). Neuer Test `tool_definitions_have_unique_names` (Namen
eindeutig, mindestens 20) — `cargo test --lib`: **46 passed / 13 ignored**.
`pnpm build` grün.

Kein Commit; die Instanz des Nutzers testet den Button zuerst.

## 72. Die Seite sehen, dauerhaft erlauben (2026-10-24, einundfünfzigste Runde)

Der Bruder scheiterte daran, eine selbst geschriebene Seite je zu Gesicht zu
bekommen: `write` ist workspace-only, der Testserver lag außerhalb, `file://`
lehnt der Browser ab, ein eigener Server scheiterte an der bash-Policy. Dazu
die Rückfrage nach einer generellen Freigabe und der Wunsch, den Copy-Button
nur als Symbol zu zeigen.

### Copy
Der Button ist jetzt ein reines Symbol (24 × 24, Lupe/Häkchen/x), der Text
liegt als `sr-only` für Screenreader darunter; `title`/`aria-label` nennen ihn
weiter. Kopiert wird `parts`-Text mit `message.text` als Rückfall — vorher hing
der Button an der `text`-Spalte und fehlte bei Antworten, die nur aus Teilen
bestanden. Sechs Sprachen unverändert.

### Preview: der geschlossene Kreis
`src-tauri/src/preview.rs`: Ein read-only Server bindet auf
`127.0.0.1:0` (freier Port) und liefert ausschließlich Dateien unter dem
kanonisierten Workspace; `..`, absolute Pfade und Prefix-Tricks enden in 404,
Verzeichnisse gehen auf `index.html`, MIME für HTML/CSS/JS/SVG/Bilder/Fonts,
`Cache-Control: no-store`, nur GET/HEAD. Pro Workspace ein Server; ein neuer
Workspace ersetzt ihn (der alte Listener wird per Flag beendet).
Tauri-Command `preview_start(root)` für die Oberfläche und das neue Werkzeug
`preview(path)`: löst den Pfad auf, prüft „im Workspace“, startet den Server
und öffnet die URL im App-Browser (`browser_open_read`, ohne Fokusklau).
Verlangt Browser-Steuerung. Testfixture `src/preview-test.html` (2-Spalten-Grid,
640 px, 16 px gap).

### Layout statt Blindflug
`browser_read` liefert zusätzlich `layout`: Viewport (Breite/Höhe/
scrollHeight) und bis zu 80 sichtbare Elemente mit Tag, id, Klassen, Text bei
Blättern, x/y/Breite/Höhe, display/position, margin, padding, gap und
text-align. Das ist die Geometrie, mit der sich „sauber ausgerichtet“ prüfen
lässt, ohne Pixel zu haben.

### Immer erlauben — mit Rückweg
Der Freigabedialog bietet zusätzlich „Immer erlauben (dauerhaft)“. Die Antwort
`always` schreibt den Werkzeugnamen über `Db::add_trusted_tool` in
`settings.trustedTools` (JSON-Liste, überlebt Neustarts); `has_tool_grant`
überspringt die Frage danach — auch die Danger-Floor-Frage, denn genau die
nervte bei Vollzugriff. Der Widerruf steht in Einstellungen → Nutzung & Modell
unter den Freigabestufen: die Liste der vertrauten Werkzeuge mit
„Freigaben zurücknehmen“ (`engine_trusted_tools`, `engine_clear_tool_grants`).
Sechs Sprachen.

### Geprüft
`pnpm build` grün; `cargo test --lib` **48 passed / 13 ignored** (zwei neue
Tests: Prozent-Dekodierung und MIME im Preview-Server). Live über die Brücke:
`preview_start` → `http://127.0.0.1:35177/`; die Fixture im App-Browser
geladen, `browser_read` meldet Titel „Preview-Test“, Viewport 1120 × 840 und
für beide Karten `y=40`, Höhe 48, Breite 312, x=240 bzw. 568 — Abstand 16 px,
also exakt das Grid. Ein Fensterfoto des Browserfensters war unter Wayland
nicht möglich (kein X11-Titel); die Messwerte sind der Beleg. Test-App danach
beendet.

### Offen
Ein Screenshot nur des Browserfensters (nicht des Desktops) wäre die letzte
Bequemlichkeit für „sieht es auch gut aus“; die Geometrie deckt Ausrichtung ab.
Kein Commit — der Nutzer testet zuerst.

## 73. Vorschau ohne Schalter, Konfiguration lesbar (2026-10-24, zweiundfünfzigste Runde)

Rückmeldung aus der DS.IDE-Instanz: `preview` scheiterte an „Browser control is
disabled“, obwohl die Vorschau nur eine lokale Datei aus dem eigenen Workspace
zeigt. Außerdem konnte sie nicht nachsehen, warum ein Werkzeug fehlt (`sqlite3`
steht nicht auf ihrer Allowlist), und `browser_read` lieferte keine Überlauf-/
Farbwerte.

### Vorschau ist lokal, nicht Browser-Steuerung
Der `preview`-Zweig prüft `browser_control` nicht mehr. Begründung: Der Server
liefert ausschließlich Dateien unter dem kanonisierten Workspace auf localhost;
das ist kein Fremdinhalt und kein Zugriff auf die Seiten des Nutzers.
`browser_read`/`browser_fill` für beliebige URLs bleiben unverändert hinter dem
Schalter. `preview` gibt jetzt die ganze Seite zurück (Text, Titel, Viewport,
Boxen) statt nur einer Bestätigung — sonst bräuchte es einen zweiten Aufruf,
der ohne Browser-Steuerung wieder scheiterte. Der Hinweis im Tool-Result sagt,
dass `screenshot` den ganzen Desktop zeigt. Der Einstellungstext
(`browserControlDesc`) sagt in sechs Sprachen, dass die lokale Vorschau die
Freigabe nicht braucht.

### Geometrie für Layouturteile
`browser_read`-Layout ergänzt: je Blattelement `color`, `background`,
`fontSize` (für Kontrast und Hierarchie) und `overflow` (`x`/`y`, wenn
`scrollWidth`/`scrollHeight` die Clientgröße übersteigt) — damit ist der zu
lange String ohne Leerzeichen nachweisbar, nicht nur sichtbar.

### `settings`: die Wahrheit des laufenden Turns
Neues, schreibgeschütztes Werkzeug `settings` (keine Freigabe nötig): liefert
`thisTurn` (Workspace, Permission, Browser-Steuerung, Command-Policy, Allowlist,
workspaceOnly, Modell, Effort), die gespeicherten Werte (`stored`) und die
dauerhaft erlaubten Werkzeuge, dazu den Hinweis, dass eine Änderung mitten im
Turn erst im nächsten Turn ankommt. Genau die Frage, die die Instanz nicht
beantworten konnte.

### Geprüft
`pnpm build` grün; `cargo test --lib` **48 passed / 13 ignored**.
Live mit `browserControl=false` in der Test-DB: `preview` auf
`src/layout-test.html` rendert (Service-URL Port 33611), Viewport 1120 × 840,
Karten `#left` x=240/y=40/312×48 und `#right` x=568/y=40/312×48, `#long`
meldet `overflow: "x"`. Danach `settings` im nächsten Turn: `permission=full`,
`browserControl=false`, Workspace `/tmp/dside-workspace` — die Turn-Werte,
nicht die gespeicherten. Test-App beendet.

Kein Commit — der Nutzer testet zuerst.

## 74. Sechs Sprachen, ein Maßstab (2026-10-24, dreiundfünfzigste Runde)

Die Oberfläche war typgeprüft vollständig (`Record<Locale, Messages>` bricht den
Build bei fehlendem Schlüssel), aber das genügt nicht: Es sagt nichts über
Platzhaltertexte, doppelte Schlüssel oder Funktionen, die in den READMEs nur auf
Englisch stehen. Diese Runde prüft und schließt beides.

### Der Katalog
- `scripts/check-i18n.mjs` liest die sechs Kataloge mit dem TypeScript-Parser
  (AST, kein Regex): vergleicht jeden Blattpfad gegen `de.ts`, meldet fehlende
  und unbekannte Schlüssel, doppelte Schlüssel (die der Compiler still
  überschreibt) und Werte, die noch identisch mit dem deutschen Text sind
  (Warnung, weil Produktnamen legitim gleich heißen).
- `pnpm check:i18n` führt ihn allein aus; **`pnpm build` beginnt damit**, damit
  eine Lücke den Build bricht statt erst im UI aufzufallen.
- Ergebnis: **471 Schlüssel, sechs Kataloge, keine Lücke, keine Warnung.**
  `AGENTS.md` nennt den Schritt.

### Was sich inhaltlich geändert hat
- Jede Assistant-Antwort mit Text bekommt den Kopier-Button; das letzte harte
  `alt="Screenshot"` ist jetzt `message.screenshotAlt` in sechs Sprachen.
- `src/components/AgentPanel.vue` (der frühe Prototyp mit „Nachricht (noch ohne
  Wirkung)…" und „KILO CODE") war nirgends importiert. Entfernt — toter Code mit
  hartem UI-Text ist eine Falle für den nächsten Leser.
- Sechs READMEs um vier Punkte ergänzt, jeweils in der Landessprache:
  **Preview** (lokale Vorschau ohne Browser-Steuerung samt Layoutdaten),
  **Kopieren** (Symbol, roher Markdown), **Recall** (Notizen und frühere
  Gespräche, auf das Projekt begrenzt) und **Freigaben** (einmal/Sitzung/
  dauerhaft, Widerruf in den Einstellungen). Vorher fehlte `recall` in allen
  sechs, `preview`/Copy/Freigaben in den fünf Übersetzungen.

### Geprüft
`pnpm check:i18n` grün (471 Schlüssel); `pnpm build` grün mit vorgeschaltetem
Check; `cargo test --lib` **48 passed / 13 ignored**. Kein Live-Test nötig: Die
Änderungen sind Texte, ein entferntes, unbenutztes Bauteil und ein Prüfskript.

Kein Commit — der Nutzer testet zuerst.

## 75. Klicken gehört zum Ansehen (2026-10-24, vierundfünfzigste Runde)

Rückmeldung aus der DS.IDE: Die Kette steht (schreiben → preview → Boxen →
screenshot → edit), aber der Theme-Knopf blieb ungetestet, weil
`browser_fill` Felder füllt und keine Knöpfe drückt. Der Live-Test dieser Runde
fand dabei zwei weitere Lücken derselben Art.

### `browser_click`
Neues Werkzeug (CSS-Selektor, optional Fenster-ID): scrollt das Element in den
Blick, schreibt die Kennung **vor** dem Klick in den URL-Hash
(`#__dside_click=ok:…`) und klickt dann. Die Reihenfolge ist Absicht: Ein Klick,
der navigiert, nimmt eine spätere Markierung mit. Kommt keine Markierung an,
lautet die Antwort „geklickt, wahrscheinlich navigiert“ statt eines falschen
Fehlers. Ein nicht passender Selektor meldet `no element matches … — call
browser_read to see the page`. Tag und Beschriftung des Elements kommen zurück.
`browser_click` erscheint als eigene Freigabezeile im Frage-Dialog.

### Dieselbe Regel wie `preview`
Die Vorschauseite ist eigener Workspace, read-only serviert; sie zu lesen,
anzuklicken oder auszufüllen braucht weder Browser-Steuerung noch Freigabe.
`browser_read` prüft dafür das Zielfenster (`window_is_preview` →
`preview::is_preview_url`: http, Loopback, exakter Port des laufenden
Servers, reine Funktion mit Test). `browser_fill` nutzt dieselbe Ausnahme;
fremde Seiten bleiben unverändert hinter Schalter und Freigabe, Planmodus
bleibt gesperrt.

### Der Live-Test fand den Rest
Erster Durchlauf (`browserControl=false`, echter Modell-Turn): `preview` und
zwei Klicks liefen, `browser_read` scheiterte an der Browser-Steuerung —
Klicken erlaubt, Ergebnis lesen nicht. Zweiter Befund: Jeder `preview`-Aufruf
öffnete ein neues Fenster (`browser-1 → -2 → -3`), der Zähler stand wieder auf
0. Behoben: `browser_read` hat dieselbe Ausnahme, und `show_preview_read`
navigiert das vorhandene Vorschaufenster statt ein neues zu öffnen.

### Geprüft
`cargo test --lib` **49 passed / 13 ignored** (neuer Test
`only_the_running_preview_server_counts_as_local`). Live über die Brücke:
Zähler 0 → zwei Klicks → 2, Theme-Umschalter wechselt auf „Theme: dunkel“,
fehlender Selektor meldet den klaren Fehler. Danach ein echter Modell-Turn mit
`browserControl=false`: `preview` (ein Fenster), zweimal `browser_click`,
`browser_read` liest **2** — alle Werkzeuge ohne Fehler, Fensterliste zeigt nur
`browser-1`. `pnpm build` grün. Test-App beendet.

Kein Commit — der Nutzer testet zuerst.

## 76. Die Capture-Diagnose widersprach sich selbst (2026-10-24, fünfundfünfzigste Runde)

Die DS.IDE-Instanz meldete im Terminal dauerhaft „composer doesn't support the
screen capture protocol“ — während Screenshots ankamen. Der Widerspruch war
echt und hausgemacht.

### Ursache
`capture_to` probiert unter Linux der Reihe nach `grim`,
`gnome-screenshot`, `spectacle`, `scrot`, `import`. `grim` scheitert auf einem
Compositor ohne Screencopy-Protokoll und schreibt seine Fehlermeldung auf
**stderr**, das `Command::status()` erbt — sie landet im Terminal, obwohl der
nächste Versuch erfolgreich fotografiert. Die Diagnose behauptete also einen
Fehlschlag, den es nicht gab.

### Änderung
Jeder Versuch läuft jetzt mit `stdin/stdout/stderr` auf `Stdio::null()`; nur
der erfolgreiche Kandidat hinterlässt eine Zeile
(`screenshot: captured with spectacle`). Schlägt alles fehl, nennt der Fehler
weiterhin die Liste der Werkzeuge. Fällt ein Werkzeug aus, ist das erwartetes
Verhalten, kein Ereignis.

### Geprüft
Live über die Brücke: `capture_screen` liefert ein PNG (2,87 MB base64,
`mime: image/png`), Logs enthalten genau eine Zeile (`captured with spectacle`)
und **keine** grim-Meldung; im Dev-Log steht `screen capture protocol` 0-mal.
`cargo test --lib` **49 passed / 13 ignored**.

### Das „verschwundene“ Vorschaufenster
Nicht reproduzierbar: Nach einem echten Turn mit `preview` blieb `browser-1`
sowohl direkt am Turn-Ende als auch 15 s später gelistet. Da seit Runde 75 nur
noch **ein** Fenster wiederverwendet wird, gibt es auch kein Aufräumen mehr.
Wahrscheinlichste Erklärung beim Nutzer: der Neustart nach dem Update. Wenn es
wieder auftritt, zeigt `browser_windows` bzw. `settings` sofort den Zustand.

### Nicht von uns
Der helle Modus mit 1,5:1-Kontrast und die Korrektur auf 5,74:1/5,73:1 sind
Arbeit an `docs/webseiten-test.html` in der Instanz, nicht am App-Code. Der
helle „soll/minimal“-Rest auf dem Screenshot liegt außerhalb der Vorschau; er
gehört zu einem anderen Fenster. Die Seite bleibt untracked und wird von mir
nicht committet.

Kein Commit — der Nutzer testet zuerst.

## 77. Ein Breakpoint lässt sich messen (2026-10-24, sechsundfünfzigste Runde)

Offene Frage aus der DS.IDE: Alle Layoutzahlen beschrieben ein 1120-px-Fenster;
ob die Seite in einem ~400 px breiten Panel hält, war ungemessen. Berechtigt —
`preview` konnte die Fenstergröße nicht setzen.

### `preview(width, height)`
Neue optionale Parameter (CSS-Pixel, 240–4000 bzw. 200–4000). `preview` öffnet
bzw. navigiert die Seite, ruft `browser::set_viewport` (logische Größe, skaliert
mit dem Faktor des Fensters) und liest nach 250 ms erneut — die zurückgegebenen
Boxen beschreiben also die angeforderte Größe, nicht die Standardgröße.

### Der Test fand eine harte Grenze
Erster Lauf mit `width: 400` meldete **Viewport 520**. Nicht der Parameter war
falsch, sondern das Browserfenster: `open_browser_window` setzte
`min_inner_size(520, 400)`. Die Mindestgröße ist jetzt **320 × 280** — klein
genug für einen Breakpoint, weiterhin bedienbar. Zweiter Lauf: Viewport 400.

### Ergebnisse bei 400 px
- Eigene Fixture `src/responsive-test.html` (auto-fit-Grid, Tabelle):
  Karten `c1/c2/c3` alle x=20, Breite 360, y=106/168/230 (gestapelt), Zellen
  155/133/71, kein Überlauf.
- Die Seite der Instanz (`webseiten-test.html`, aus `docs/` in den Test-Workspace
  kopiert, Original unberührt): Viewport 400 × 800, scrollHeight 1665, **kein
  Element mit Überlauf**, Tabelle 368 breit, `#chars` 330 × 45, SVG 368 × 79.
  Das SVG schrumpft mit dem Container (368/420 ≈ 0,876), bleibt also
  seitenverhältnistreu; bei 400 px ist es nur entsprechend kleiner.
  Damit ist Punkt (e) beantwortet: Die Seite hält auch schmal.

### Geprüft
`cargo test --lib` **49 passed / 13 ignored**; `pnpm build` grün; zwei echte
Modell-Turns über die Brücke (400-px-Messung, vorher 520). Test-App beendet.
Die Testseite der Instanz bleibt untracked und wird nicht committet.

Kein Commit — der Nutzer testet zuerst.

## 78. Ein Messwert, der sagt, wenn er korrigiert wurde (2026-10-24, siebenundfünfzigste Runde)

Der Vier-Läufe-Test der Instanz bestätigte die Breakpoint-Messung und fand drei
Stellen, an denen das Werkzeug mehr versprach als es lieferte. Alle drei sind
behoben, zwei davon mit neuen Tests.

### 1. Klemmung war unsichtbar
`width: 240` wurde vom Fensterminimum still auf 320 geklemmt, ohne Hinweis. Die
Antwort trägt jetzt `size`:
`requested`, `effective`, `clamped` (Liste der Achsen) und `note`. Die
Unterscheidung „geklemmt“ vs. „nicht zur Ruhe gekommen“ war im ersten Versuch
falsch — ein geklemmter Wert erreicht sein Ziel nie und galt deshalb als
unsettled. Jetzt zählt Stabilität: zwei gleiche Messungen = zur Ruhe gekommen,
auch wenn das Ziel verfehlt wurde. Test
`a_clamped_size_is_reported_as_clamped` (deckt beides ab).

### 2. „Ohne Parameter“ hieß „letzte Größe“
Ein Aufruf ohne `width`/`height` behält die Fenstergröße — das ist richtig, war
aber nicht sichtbar. Die Antwort sagt jetzt `size.kept`: „no width/height
requested — the window keeps its previous size“, dazu `size.effective`. Test
`a_call_without_a_size_says_it_kept_the_previous_one`.

### 3. Die feste Wartezeit
Die 250 ms sind durch adaptives Pollen ersetzt (bis 10 × 120 ms): Abbruch, sobald
die Größe stimmt oder zwei Messungen identisch sind. `size.note` sagt, wenn das
Fenster nicht zur Ruhe kam. Live: 400 × 700 exakt, 240 → 320 mit
`clamped: ["width"]`, danach ohne Parameter `effective: 320 × 600` + `kept`.

### Dazu, aus demselben Bericht
- **`browser_click` nennt die URL.** „no element matches“ und „die Seite hat
  seit dem Lesen navigiert“ waren eine Meldung. Jetzt steht die aktuelle URL im
  Fehler (und als `url` in jeder Erfolgsantwort); die interne Markierung
  (`#__dside_click=…`) wird dabei entfernt (Test
  `the_reported_url_has_no_bookkeeping_fragment`).
- **Leere Seite ≠ Ergebnis.** `browser_read`/`preview` warten bei fehlendem
  Titel **und** fehlendem Text weiter und setzen `loading: true`, wenn nach den
  Versuchen immer noch nichts da ist. `read_when_loaded` behandelt so eine Seite
  als „noch nicht fertig“. Test `an_empty_document_looks_unloaded`.
- **Fenstertitel.** Statt des statischen „Webdesign“ heißt das Fenster jetzt
  „DS·IDE — Browser“ und folgt nach jedem Lesezugriff dem Seitentitel
  („Klick-Test — DS·IDE“), wie ein Browser. Der Browser ist längst mehr als
  Webdesign: er liest, füllt aus und zeigt die eigene Vorschau.

### Geprüft
`cargo test --lib` **54 passed / 13 ignored** (fünf neue Tests),
`pnpm build` grün, drei echte Modell-Turns und zwei Brücken-Läufe:
Klemmung/K kept/exakte Größe wie oben, Klick-Fehler mit sauberer URL,
Fenstertitel „Klick-Test — DS·IDE“. Test-App beendet.

### Nicht von uns
Die Tabellenspalte der Instanz (`width: 1%` ohne Obergrenze, 68 px = 24 % bei
320 px) ist deren CSS; `max-width: 90px` ist der richtige Fix. Die Testseite
bleibt untracked.

Kein Commit — der Nutzer testet zuerst.

## 79. Welche Fenster offen sind (2026-10-24, achtundfünfzigste Runde)

Zwei Befunde aus der DS.IDE: Ein `browser_read` auf eine Ergebnisseite kam leer
zurück und meldete trotzdem `loading: false` (Titel war da, Body noch nicht), und
die Instanz konnte nicht sehen, wie viele Fenster offen sind — `browser_windows`
lag fertig im Backend, aber kein Werkzeug rief es auf. Deshalb hieß „guck mal in
den Browser“ für sie: ein zweites Fenster aufmachen.

### `loading` meint Inhalt, nicht Titel
`looks_unloaded` prüft jetzt Text, Links **und** Formularfelder. Ein Titel aus
dem Dokumentkopf beweist nichts, solange der Body leer ist; `read_when_loaded`
wartet in dem Fall weiter und liefert `loading: true`, wenn nach den Versuchen
nichts angekommen ist. Live mit `src/title-only.html` (Titel „Nur-Titel", leerer
Body): `loading: true`, Text leer, keine Links, keine Felder. Der Test
`an_empty_document_looks_unloaded` deckt „Titel allein ist kein Inhalt" und
„Links zählen als geladen" ab.

### `browser_windows` als Werkzeug
Neues, freigabefreies Werkzeug: `id`, `url`, `title` und `newest` für jedes
Fenster. `newest` sagt, welches `browser_read` ohne id nehmen würde — die
Information, die vorher nur im Backend lag. Nur Metadaten; Seiteninhalt bleibt
hinter der Browser-Steuerung (Vorschau ausgenommen).

### `browser_read(url, id)` navigiert gezielt
Ohne id öffnet `url` weiterhin ein neues Fenster — eine Seite des Nutzers wird
nicht ungefragt ersetzt. Mit ausdrücklicher Fenster-ID wird dieses Fenster
navigiert; die Entscheidung liegt dann beim Aufrufer. Genau der Fall „arbeite in
dem Fenster, das schon offen ist". Live: `browser-1` gezielt auf
`click-test.html` navigiert, beide Fenster danach korrekt gelistet.

### Geprüft
`cargo test --lib` **54 passed / 13 ignored**; `pnpm build` grün. Live über die
Brücke: zwei Fenster geöffnet, Liste mit Titeln und `newest` korrekt, gezielte
Navigation liefert die neue URL, Titel-Fall `loading: true`, Fenster nach dem
Lesen „Klick-Test — DS·IDE" bzw. „Responsiv-Test — DS·IDE". Test-App beendet.

Kein Commit — der Nutzer testet zuerst.

## 80. Kein „The. The.“ mehr (2026-10-24, neunundfünfzigste Runde)

Der Anlass dieser ganzen Sitzung: Mein eigener Turn lief in eine Wiederholung
(„The. The.“, am Ende ~100 000 Zeichen), und nichts hielt ihn auf. Zuerst die
wichtigste Gegenprobe, dann der Wächter.

### DS.IDE schickt kein Reasoning zurück
`ChatMessage` (`engine/types.rs`) hat **kein** Feld für Denktext; `wire_messages`
baut ausschließlich `role`, `content`, `tool_calls`, `tool_call_id` und
Bild-Content-Parts. Der Denktext wird live an die Oberfläche geschickt und
gespeichert, aber nie wieder an das Modell gegeben. Genau die Ursache meines
Loops — Reasoning als Historie — kann hier also nicht auftreten. Das war vorher
nicht belegt, jetzt ist es nachgelesen.

### Der fehlende Wächter
Was fehlte, war der zweite Teil: `stream_chat` las den Strom bis zum Ende, egal
was ankam. Neu in `engine/deepseek.rs`:

- `repeated_tail(text)` prüft **nur das Ende** der Antwort auf einen exakt
  wiederholten Block: mindestens 320 Byte, mindestens 24 Wiederholungen, Block
  bis 200 Byte. Reiner Whitespace zählt nicht.
- Beim Treffer bricht der Leser den HTTP-Strom ab, entfernt die Wiederholung
  (auch die Kopien davor) und meldet `StreamOutcome.repeated`.
- Der Engine-Turn beendet dann mit `stop_reason = "loop"` und einer Meldung:
  „Stopped: the model began repeating itself (… in the reasoning, 64×). Everything
  before the repetition is kept; the rest was cut.“ Der bereits gespeicherte Teil
  bleibt sichtbar, der Retry-Knopf steht bereit.

Die Schwellen sind bewusst so gewählt, dass normale Formatierung nicht anschlägt:
Fließtext wiederholt keinen Satz 24-mal; eine breite Markdown-Tabelle hat Zellen
zwischen den Trennern; ein Codeblock mit Leerzeilen ist keine Wiederholung eines
Blocks. Fünf Tests decken beides ab (`a_repeated_phrase_is_a_loop`,
`a_repeated_sentence_is_a_loop`, `normal_prose_and_tables_stay_untouched`,
`cutting_removes_the_repetition_and_keeps_the_answer`,
`repetition_earlier_in_the_answer_is_not_the_tail`).

### Geprüft
`cargo test --lib` **59 passed / 13 ignored** (fünf neue Tests). Live: eine
normale Antwort von 305 Wörtern / 2 189 Zeichen kam vollständig durch, ohne
Meldung — der Wächter greift nicht bei gewöhnlichem Text. Eine echte
Modell-Schleife lässt sich nicht zuverlässig herbeiführen; die Erkennung ist
deshalb deterministisch getestet. Nichts committet vor diesem Schritt.

### Was jetzt noch offen ist
`estimated_tokens` statt echter `prompt_tokens` als Kompressionsauslöser;
ein Screenshot nur des Browserfensters; Laufzeittest auf Windows/macOS;
Punkte-/Vertrauenssystem (Name ab Schwelle) aus §35/§36.

Kein Commit — der Nutzer testet zuerst.

## 81. Echte Token, ein Fenster statt Desktop, Vertrauen über Zeit (2026-10-24, sechzigste Runde)

Die zwei letzten offenen Punkte aus §80, vor einem Release abgearbeitet.

### Kompression rechnet mit der Messung
Bisher entschied `estimated_tokens` (Zeichen / 4) über die Kompression — ohne
System-Prompt und ohne die Tool-Schemas, die der Provider mitzählt. Neu:
- Tabelle `session_usage` speichert je Sitzung die `prompt_tokens` des letzten
  Requests **und** wie viele Nachrichten er trug.
- `prompt_size` nimmt diese Messung und addiert nur den Zuwachs seitdem
  (`estimated_tokens` auf die neuen Nachrichten). Ist die Basis veraltet — neue
  Sitzung, oder die Historie wurde gerade durch Kompression umgeschrieben —,
  gilt wieder die Schätzung.
- Der Log sagt, worauf entschieden wurde: `compaction: measured size … over
  limit …`.
Test `the_prompt_size_prefers_the_measurement_and_adds_the_growth`.

### Screenshot nur des Browserfensters
`screenshot { window: "browser-1" }` (neuer optionaler Parameter): Der Desktop
wird aufgenommen und auf das Fensterrechteck zugeschnitten (`image`-Crate, nur
PNG). Die Antwort nennt `window: true/false` und einen Hinweis, was passiert ist.
Kann der Compositor keine Fensterposition nennen (manche Wayland-Sitzungen),
kommt ehrlich der ganze Desktop mit `window: false` und einer Notiz zurück,
statt einen falschen Ausschnitt zu liefern. Live geprüft: Desktop 3840×1080,
Fenster bei 0,32 mit 1920×1000, Zuschnitt exakt 1920×1000, im Canvas 97 % helle
Fläche bei 34 Farben — also die Seite, nicht der Desktop.

### Vertrauen: der Name kommt später
`trust_points` (kind, points, day, note) mit Tageskappen:
- `day` — erster Turn des Tages, 2 Punkte (einmal pro Tag)
- `turn` — ein beendeter Turn, in dem mindestens ein Werkzeug erfolgreich war,
  1 Punkt (max. 10/Tag)
- `memory` — eine Notiz, die der Begleiter selbst schreibt, 1 Punkt (max. 5/Tag)
- `correction` — der Nutzer bearbeitet eine Notiz, 3 Punkte (max. 3/Tag)
Schwelle `TRUST_THRESHOLD = 100`. Beim Überschreiten setzt `award_trust`
`trustRevealed`, legt es fest und sendet `trust://revealed`; die App zeigt einen
dezenten Banner („Dein Begleiter hat einen Namen: Ziggy."), der Begleiter-Tab
schildert Herkunft und Fortschritt. Vorher steht dort der Platzhalter „Seeky"
mit Balken („Vertrauen: 42 / 100"), und auch die Namensbedeutung bleibt
verborgen — sie würde den Namen verraten. Bestehende Installationen mit
gewähltem Namen werden bei der Migration als enthüllt markiert (Ziggy bleibt
sichtbar); frische Installationen starten bei null.

### Ein Fehler, den erst der Screenshot zeigte
Der Begleiter-Tab war ungestylt: `.card`, `.row`, `.label` liegen scoped in
`SettingsPage.vue` und erreichen eine Kindkomponente nicht. Die Panels tragen
ihre Karten-Regeln jetzt selbst; vorher rendeten Fortschritt und Namenszeile
als zusammenhanglose Blöcke. Auch die Balkenfarbe war zu blass (`--border`
statt `--bg-elevated`).

### Geprüft
`cargo test --lib` **62 passed / 13 ignored** (zwei Vertrauens-Tests inkl.
Migration, einer für `prompt_size`), `pnpm build` grün mit i18n-Check
(479 Schlüssel in sechs Sprachen). Live: 99 Punkte gesetzt, ein Turn → 101,
`revealed: true`, Banner sichtbar; verborgener und enthüllter Tab per
Screenshot geprüft (`/tmp/dside-partner-trust3.png`,
`/tmp/dside-partner-revealed.png`). Die echte Modell-Kompression ist durch den
Unit-Test abgedeckt, nicht durch eine künstlich aufgeblähte Sitzung.

Kein Commit — der Nutzer testet zuerst.

## 82. Aus dem Fork übernommen: Seiten, Slide, Archiv (2026-10-26, einundsechzigste Runde)

Der Nutzer hat einen eigenen Fork (`~/Projects/surprise/app`) für seine VSCode-Companions
gebaut und zwei Dinge genannt: die Slide-Animation der Seitenpanels und das seitenweise
Laden langer Threads. Beides ist übernommen, dazu die Archivverwaltung; das DB-System des
Forks (Companions/Projects/Qdrant) bleibt bewusst draußen.

### Lange Threads laden in Seiten
- Rust: `Db::load_history_page(session_id, before_id, limit)` — die neuesten N Zeilen per
  `ORDER BY id DESC LIMIT`, fürs Lesen umgedreht; `after_id`-Nachbar heißt hier `before_id`,
  weil rückwärts gelesen wird. `load_history` bleibt als „alles" für Tests/Export.
  Command `session_history` nimmt `beforeId` (Format `m{rowid}`) und `limit` (Default 80,
  Deckel 500); Test `history_pages_continue_where_the_previous_one_stopped`.
- Frontend: `HISTORY_PAGE = 80`, `historyHasMore` pro Sitzung, `loadOlderHistory` stellt die
  ältere Seite voran. Der Aufrufer merkt sich `scrollHeight`/`scrollTop` und korrigiert nach
  dem Einfügen — die gelesene Zeile bleibt stehen (gemessen: 12 px Abweichung durch
  Rahmen-Rundung). Der „Ältere Nachrichten laden"-Knopf sitzt über dem Verlauf, mit
  Spinner-Zustand.
- Gemessen an einer Sitzung mit 200 realistischen Nachrichten (je ~4 KB): erste Seite
  **657 KB / 19 ms** statt **1642 KB / 43 ms** für alles; dazu 80 statt 200 DOM-Zeilen.
  Bei echten Werkzeugausgaben fällt der Unterschied größer aus.

### Slide statt Sprung
Die Seitenpanels bleiben jetzt gemountet und werden über ihre Grid-Spalte ein- und
ausgefahren (`0%` statt Entfernen der Spalte — nur so ist der Übergang animierbar), mit
`opacity`/`translateX`, `inert` und `aria-hidden` im versteckten Zustand. Weil eine
0-%-Spalte ihren 10-px-Gap behält, zieht ein negativer Rand am Mittelpanel die Kante mit —
sonst bliebe ein 10-px-Versatz. Getestet: Spalte 241 → 98 → 0 px bei 60 ms,
Deckkraft 0,50, danach `inert`, Panel weiterhin im DOM, Mittelkante exakt bei 10 px.
Dauer **320 ms** statt der 1,15 s im Fork: die Bewegung soll sich nach Absicht anfühlen,
nicht nach Warten; eine Konstante, falls es langsamer gewünscht ist. `prefers-reduced-motion`
setzt sie auf 1 ms. Während des Ziehens am Trenner schaltet `body.is-resizing` alle
Übergänge ab und sperrt die Textauswahl.

### Archivverwaltung
Suche (Titel und Pfad), Projektfilter (aus den vorhandenen Workspaces), Datum je Eintrag,
Löschen einzeln und „Alle löschen" — jeweils über den app-eigenen `ConfirmDialog`
(Fokus auf „Abbrechen"), nicht über einen zweiten Klick. Neuer Command
`session_delete_many`; einzeln nutzt `session_delete`. Sechs Sprachen. Live geprüft:
Suche filtert, Projektfilter greift, Dialog nennt „Alle 1 archivierten Aufgaben …",
Bestätigen entfernt Zeile und DB-Satz, danach der Leerzustand.

### Bewusst nicht übernommen
Companion-/Projekt-Datenmodell, Synchronic, Qdrant, KiloCode-Import, die neue Shell
(Titelbar weg, Companion-Picker, `SelectField` statt nativer Selects) und die
Composer-Umbauten — das ist die Identität des Forks, nicht ein Gewinn für DS.IDE. Die
Backend-Unterschiede dort sind bis auf Companion-Felder reine rustfmt-Formatierung.

### Geprüft
`cargo test --lib` **63 passed / 13 ignored**, `pnpm build` grün mit i18n-Check
(490 Schlüssel in sechs Sprachen). Live über die Brücke: 200-Nachrichten-Sitzung zeigt 80
Zeilen und den Knopf, Nachladen ergibt 160 ohne Sprung, Slide und Archiv wie oben.
Testumgebung unter `/tmp` neu aufgebaut (die alte war aufgeräumt).

Kein Commit — der Nutzer testet zuerst.

## 83. Die Seite scrollte sich selbst weg (2026-10-26, zweiundsechzigste Runde)

Der Nutzer sah in seinem Fenster, dass der Inhalt „zu weit nach oben gerutscht“ war.
Reproduziert mit einer **Kopie** seiner Daten (Original unberührt): Der Viewport war
scrollbar — `document.scrollingElement.scrollHeight` maß **53 988 px** bei 869 px
Fensterhöhe, und `scrollTop = 500` ließ sich setzen.

### Ursache
`body { overflow: hidden }` allein genügt nicht: Nach der CSS-Regel wandert die
Deklaration an den **Viewport**, und der wird dadurch ein versteckter, aber weiterhin
per Skript scrollbarer Container. Das `<html>`-Element bleibt `overflow: visible`, also
zählt WebKit den Inhalt der inneren Scrollflächen (die lange Unterhaltung) zur
Dokumenthöhe. Jedes `scrollIntoView` oder ein `focus()` auf ein Element außerhalb des
Sichtbereichs konnte damit das ganze Fenster um Tausende Pixel verschieben — oben
abgeschnitten, unten eine Lücke. Genau das Bild im Bericht.

### Änderung
- `html { overflow: hidden; overscroll-behavior: none }` zusätzlich zu `body`; damit ist
  das Dokument selbst nicht mehr scrollbar. Messwert danach: `scrollHeight` **869**
  (vorher 53 988), `scrollTop = 5000` bleibt **0**.
- `ViewFileTab.revealFile` scrollt nicht mehr per `scrollIntoView` (das läuft **alle**
  scrollbaren Vorfahren hoch), sondern rechnet die Zeile gegen `.tree-body` und setzt nur
  dort `scrollTop`. Ein enthüllter Dateieintrag kann die Seite also nicht mehr bewegen.
- Sicherheitsnetz in `main.ts`: Ein `scroll`-Listener am Fenster setzt die Wurzel auf 0,
  falls irgendein künftiger Pfad sie doch verschiebt.

### Geprüft
Live im Fenster mit der Datenkopie: `scrollIntoView` auf eine tiefe Nachricht und
`scrollTop = 5000` bewegen nichts mehr; `.tree-body` scrollt weiterhin lokal (Pfad beim
Öffnen eines Datei-Tabs), Titelbar 44 px, Chat und Composer füllen das Fenster.
`pnpm build` grün (490 i18n-Schlüssel), `cargo test --lib` **63 passed / 13 ignored**.
Die Kopie der Daten unter `/tmp/dside-theirs` wurde danach gelöscht.

### Für den Fork
Dort steckt dieselbe Konstellation (`body { overflow: hidden }`, `scrollIntoView`), also
lohnt derselbe Fix: `html` mit absichern und Enthüllen lokal scrollen.

Kein Commit — der Nutzer testet zuerst.

## 84. Umzug nach modestcoders-com (2026-10-26, dreiundsechzigste Runde)

Das Repository liegt nicht mehr unter `moc72/dside`, sondern in der neuen Organisation:
**https://github.com/modestcoders-com/dside**

Der Transfer lief über die GitHub-API (`POST /repos/moc72/dside/transfer`,
`new_owner: modestcoders-com`) mit dem vorhandenen Token — Status 202, danach
nachgeprüft: Standardbranch `main`, Tag `v0.5.0`, Workflow `release`, alle
bisherigen Releases **und die Draft-Release v0.5.0 mit ihren 7 Assets** sind
mitgekommen. Der alte Pfad leitet auf den neuen weiter.

- Git-Remote `github` zeigt jetzt auf
  `https://github.com/modestcoders-com/dside.git`; `origin` (lokaler Gitea unter
  `192.168.0.40`) bleibt unverändert.
- `src/engine/links.ts` (Konto-Menü: GitHub, Feedback/Issues) nennt die neue
  Adresse. Alte Nennungen in `_handover/`-Schnappschüssen bleiben, wie sie sind —
  das sind historische Kopien.
- Actions: `allowed_actions: all`, Workflow-Rechte `read` als Standard, aber die
  Workflow-Datei setzt selbst `permissions: contents: write`; keine Secrets nötig,
  weil nur `GITHUB_TOKEN` verwendet wird. Ein Transfer nimmt **keine** Secrets mit
  — hier ist keine vorhanden, also folgenlos.

Geprüft: `git push github main` („Everything up-to-date"), `git ls-remote` sieht
`v0.5.0`, Release-Liste über die API bestätigt Draft und Assets, `pnpm build` grün.

## 85. Schwarze Screenshots und ein unerreichbarer Dateibaum (2026-10-29, vierundsechzigste Runde)

Zwei Fehler aus dem Test der installierten **0.5.0** unter Linux/Wayland.

### Screenshots waren schwarz
Der Bericht war präzise: zwei Aufnahmen à 7508 Bytes, byte-gleich groß, Log
`captured with scrot`. Ursache: `scrot` ist ein X11-Werkzeug; auf Wayland sieht es
nur den leeren XWayland-Root und schreibt daraus ein **gültiges, schwarzes JPEG mit
Exit-Code 0**. Die Erfolgsprüfung (`status.success() && file && len > 0`) konnte
„aufgenommen" nicht von „leere Fläche" unterscheiden und nahm es als Erfolg.

Zwei Änderungen in `capture.rs`:
- **Sitzungsbewusst:** Ist `WAYLAND_DISPLAY` gesetzt oder `XDG_SESSION_TYPE=wayland`,
  kommen nur `grim`, `gnome-screenshot`, `spectacle` in Frage. `scrot` und `import`
  sind dort entfernt, weil sie immer „erfolgreich" Schwarz liefern. Reine Funktion
  `capture_candidates`, Test `wayland_never_offers_x11_only_tools`.
- **Leere Bilder zählen nicht:** `is_a_flat_image` dekodiert die Datei und tastet
  ein 12×12-Raster ab; unterscheiden sich alle Punkte um höchstens 3 Stufen, gilt
  die Aufnahme als leer, wird verworfen und das nächste Werkzeug probiert. Im Log
  steht dann `screenshot: grim produced an empty image, ignored`. Tests:
  `a_uniform_capture_counts_as_flat` (schwarz und weiß), `a_capture_with_content_is_not_flat`.
- Scheitert alles, kommt eine **ehrliche Meldung** statt eines schwarzen Bildes, mit
  Hinweis auf Wayland/Portal und die beteiligten Werkzeuge.

Live geprüft auf dieser Wayland-Sitzung: echtes Bild (3840×1080, 2249 KB, 2107
Farben, kein schwarzer Anteil). Mit einem untergeschobenen `grim`, das ein schwarzes
PNG schreibt und 0 zurückgibt: `grim produced an empty image, ignored` → danach
`captured with spectacle` und ein echtes Bild. Mit zusätzlich scheiterndem
`spectacle`: `screenshot: grim produced an empty black image (gnome-screenshot,
spectacle did not run). On a Wayland session only Wayland-capable tools can see the
screen; install grim or make sure the desktop portal allows capture.`

### Datei-Explorer ließ sich nicht einblenden
Der Umschalter für den Baum sitzt **innerhalb** des Blocks, der nur bei geöffneter
Datei gerendert wird (`v-if="selectedPath && !error"`). Ist keine Datei offen und
`treeVisible` gespeichert `false` — was der Auto-Hide nach jeder Dateiwahl setzt —
gibt es weder Baum noch Knopf: die Fläche zeigt nur „Wähle eine Datei im Dateibaum."

`treeShown = fileTreeVisible || !selectedPath`: Ohne offene Datei **ist** der Baum
der Tab, also bleibt er sichtbar — ohne toten Umschalter. Grid-Spalte, `inert` und
`aria-hidden` folgen jetzt `treeShown`. Live geprüft mit genau dem gespeicherten
Zustand (`treeVisible: false`, Datei-Tab ohne Datei): Baum 220 px, 291 Einträge,
nicht inert. Danach Datei gewählt → Baum klappt nach 2 s weg (Breite 1, inert),
Umschalter in der Leiste holt ihn zurück (220 px, `aria-pressed="true"`).

### Geprüft
`cargo test --lib` **66 passed / 13 ignored** (drei neue), `pnpm build` grün mit
i18n-Check (490 Schlüssel). Screenshots der wiederhergestellten Ansicht:
`/tmp/dside-tree-restored.png`, `/tmp/dside-tree-visible.png`.

### 0.5.1 (diese Runde)
Version auf **0.5.1** gehoben, `cargo test --lib` 66 grün, `pnpm build` grün.
Tag und Draft siehe Release-Abschnitt.

## 86. Damit Behauptungen einen Beleg brauchen (2026-10-29, fünfundsechzigste Runde)

Der Bruder in DS.IDE hat dreimal an einem Tag etwas berichtet, was nicht in einer
Tool-Ausgabe stand — erfundene Zeilennummern, ein erfundener Datenbanklauf, ein
„Eingetragen", das nicht eingetragen war. Auf Nachfrage die Ursachenanalyse: der
Prompt fordert Haltung, aber keine Regel; die Architektur löscht den Beleg und
behält die Behauptung. Vier Eingriffe, alle klein und testbar.

### 1. Die Regel steht jetzt im Prompt
`persona.ts` beginnt „How you work" mit einer Invariante: *„Nothing you report may
come from nowhere"* — eine Tatsache über die Welt (Zeilennummer, Dateiinhalt,
Kommandoausgabe, Datenbankzeile, Testergebnis, Seitentext) darf nur genannt werden,
wenn ein Tool-Ergebnis **in diesem Turn** sie belegt. Dazu vier Unterpunkte, die
genau die beobachteten Fehler benennen: ein Plan ist kein Ergebnis; „done/saved/
entered/created" nur mit Beleg; Zahlen und Pfade werden zitiert oder nicht genannt;
eine nachträgliche Korrektur kostet eine Zeile. Das ersetzt die weiche Bitte
„report faithfully" durch eine prüfbare Bedingung.

### 2. Die Werkzeugliste war falsch
Persona und Fallback-Prompt sagten „You have full tool access: read, write, edit,
bash, glob, grep, todowrite" — sieben Werkzeuge, während über zwanzig angeboten
werden. Namentlich fehlten `settings`, `recall`, `preview`, `browser_*`,
`screenshot`, `memory` und `task`. Jetzt nach Zweck gruppiert, mit drei Hinweisen:
`settings` lesen statt raten, `preview` benutzen statt beschreiben, `recall`
benutzen bevor „wie besprochen" fällt.

### 3. Kompression unterscheidet geprüft und behauptet
Der Zusammenfassungs-Prompt ließ „detail that no longer influences anything" weg —
und damit die Tool-Ausgaben, also den Beleg, während die Behauptung als „state of
the work" stehen blieb. Nach der ersten Kompression war ein erfundener Lauf nicht
mehr als solcher erkennbar. Neu: was ein Tool-Ergebnis bestätigt hat, bleibt
Tatsache; was nur behauptet war, wird als `unverified: …` als offene Frage
mitgeführt. Kein Plan darf beim Zusammenfassen zum Ergebnis werden.

### 4. Notizen tragen ihren Grund (DB)
`memories.basis` (`verified` | `assumed`, Migration-Default `assumed`), gesetzt über
`set_memory_basis`. Das `memory`-Werkzeug hat dafür einen optionalen Parameter
`basis` (Default `assumed`) und sagt beim Anlegen zurück, wie die Notiz aufgenommen
wurde. Der Recall-Block markiert `[unverified]` — aber nur bei Notizen des
Begleiters; die Worte des Nutzers nie. Der Begleiter-Tab zeigt denselben Marker als
gestrichelte Kapsel „unbestätigt" mit Erklärung. Sechs Sprachen. Bestehende Notizen
gelten als `assumed`: sie wurden überlegt, nicht geprüft.

### Geprüft
`cargo test --lib` **68 passed / 13 ignored** (neu: `an_assumed_note_is_marked_and_the_users_words_are_not`,
`a_memory_records_whether_it_was_verified`; die Migrationsprüfung deckt den
`assumed`-Default ab). `pnpm build` grün mit i18n-Check (492 Schlüssel).

**Wirkungsprüfung mit einem echten Turn** (Testumgebung, Workspace
`/tmp/dside-workspace`): Frage nach der Zeilenzahl von `src/main.ts` und der Zahl
der Erinnerungen. Antwort: erst `read` relativ zum Workspace — der **scheiterte**;
sichtbar als `read fehlgeschlagen` in der Zeitleiste. Darauf `glob`, dann `read` auf
den gefundenen Pfad, dann „43 Zeilen — beruht auf dem read-Aufruf der Datei
~/Projects/DS.IDE/src/main.ts", mit dem ausdrücklichen Hinweis, dass der
relative Pfad nicht existierte. Die Memories-Frage mit `memory list` und Zitat
„(no memories stored)". Genau das Verhalten, das die Regel verlangt: nachsehen,
scheitern dürfen, Scheitern benennen, Beleg nennen.

Ein eigener Test war zuerst grün aus dem falschen Grund — `[unverified]` steht auch
in der Kopfzeile des Recall-Blocks, also matchte die Prüfung die Erklärung statt der
Notizzeile. Ersetzt durch einen Test, der die Zeile selbst prüft.

## 87. Was aus dem Daemon zu holen war (2026-10-29, sechsundsechzigste Runde)

Der Nutzer hat `_handover/MiniMaxCode-3.1.4/daemon` gelegt und um eine genaue
Prüfung gebeten: keine Kopie, sondern ähnliche Ergebnisse auf eigenem Weg. Gelesen
wurden `agents/_default` (Basis-, Worker- und Session-Prompts), `agents/*`
(mavis, verifier, planner, coder, general, skill-editor), `prompts/`
(llm-gate-classifier, memory-cleanup-curator), `internal-skills/` und die
Skill-Struktur.

### Übernommen: der Verifier
Der wertvollste Fund. Ein zweiter Leser mit **nur lesenden** Werkzeugen prüft eine
Behauptung gegen den Workspace und endet mit `VERDICT: PASS` oder `VERDICT: FAIL`.
Der Prompt (`src-tauri/src/engine/prompts.rs`) ist in DS.IDEs Stimme geschrieben und
verlangt: pro Prüfung Methode, zitierte Evidenz und Ergebnis; mindestens eine
bewusst gewählte Gegenprobe; und eine Liste der Ausreden, die nicht als Prüfung
zählen („der Bericht sagt, es sei erledigt", „sieht korrekt aus", „vermutlich grün").
Ohne Verdikt gilt die Prüfung als **nicht bestanden** (fail-closed), bei zwei
Verdikten ebenso.

Eigenes Werkzeug `verify` statt einer Rolle in `task`. Grund: `task` ist im
Alltagsmodus abgeschaltet (kein Sub-Agent), aber ein Verifier liest nur — er gehört
in beide Modi. Der Verifier bekommt eine Werkzeug-Allowlist statt des vollen Satzes:
read, glob, grep, logs, settings, recall, web_fetch, browser_read, browser_windows,
preview, skill. Kein Schreiben, kein Bash — kann er eine Behauptung nur mit einem
Build prüfen, sagt er das als Lücke, nicht als Pass.

`verdict_outcome` liest das Verdikt: PASS → ok, FAIL → **Tool-Fehler** (der Aufrufer
kann es nicht für Bestätigung halten), kein oder mehrere Verdikte → Fehler. Test
`a_verifier_verdict_is_fail_closed`.

### Übernommen: eine Basis für Unteragenten
Der Daemon trennt Basis-Prompt, Worker-Prompt und Session-Rolle (root/branch).
DS.IDE hatte für Unteragenten einen Satz. `prompts::WORKER_BASE` sagt jetzt das
Nötige: erst arbeiten, dann berichten; nichts behaupten ohne Tool-Ergebnis; ein Plan
ist kein Ergebnis; sagen, was nicht ging. Die Haupt-Persona bleibt im Frontend
(`persona.ts`), weil sie den Einstellungen des Nutzers folgen muss — die Trennung
ist im Handover notiert.

### Übernommen: Selbst-Erinnerungen
`Self-Reminder via Cron` war im Daemon Pflicht nach jeder asynchronen Übergabe.
In DS.IDE gibt es jetzt ein Werkzeug `schedule` (add/list/cancel) und einmalige
Reminder: `schedules.one_shot` und `created_by` (Migration), `create_reminder`,
`stamp_schedule_run`, `disable_schedule`. Läuft ein Reminder, schaltet er sich
**vor** dem Turn ab — auch wenn der Turn scheitert — und merkt sich die Zeit. Ein
eigener Test deckt beides ab (`a_reminder_fires_once_and_a_schedule_repeats`).
Grenzen: höchstens drei aktive Selbst-Erinnerungen pro Sitzung, einmalig ab einer
Minute, wiederholend ab fünf Minuten. Die Persona sagt: „Push and done" ist nicht
fertig — wer etwas übergibt, was er nicht mehr sieht, stellt eine Erinnerung.

### Übernommen: Memory-Disziplin
Der Daemon fragt „nur für dieses Projekt? → Projektgedächtnis; überall gültig? →
Agent; würde es für einen anderen Nutzer anders lauten? → Nutzer" und speichert
nichts, was aus Code, Git oder AGENTS.md nachlesbar ist. Das steht jetzt in der
Persona (engste Ebene zuerst, nichts doppelt Ableitbares) und in der
Werkzeugbeschreibung von `memory`.

### Nicht übernommen, und warum
- **LLM-Gate-Classifier** (ein Modell entscheidet über jede Tool-Freigabe): DS.IDE
  hat Stufen und einen Danger-Floor; ein zusätzlicher Modellaufruf pro Werkzeug
  wäre langsamer und teurer als die jetzige Regel.
- **Team-Engine, Sessions als Nachrichtenkanal, `mavis`-CLI**: DS.IDE hat eine
  Sitzung pro Nutzer und einen Sub-Agenten; die Topologie des Daemons ist eine
  andere Baustelle, nicht ein Feature.
- **Skill-Evolution im Nachtlauf, Qdrant, MCP-Onboarding**: eigene Produkte.
- **`<media />`-Ausgabe**: DS.IDE zeigt Dateien im Dateibaum und schickt Bilder
  bereits als Content-Parts.
- **Memory-Curator als Nachtlauf**: die Regeln (HOT/Topic/Dedup, MERGE/DOWNSHIFT)
  sind gut, aber ein nächtlicher LLM-Lauf um Memories zu sortieren ist für eine
  Desktop-App mit einem Nutzer zu viel. Der bestehende `memory curate`-Report
  bleibt der Einstieg.

### Noch offen (bewusst)
Ein Verifier, der `bash` **nur lesend** ausführen dürfte (Build/Tests), wäre der
nächste Schritt. Dafür bräuchte es eine Klassifizierung von Kommandos nach
Lesewirkung — das ist ein eigenes Vorhaben, nicht ein Parameter.

### Geprüft
`cargo test --lib` **70 passed / 13 ignored**, `pnpm build` grün (492 Schlüssel).
Live in der Testumgebung:
- Editor-Dropdown wieder da, auch im Alltagsmodus (`mode: everyday`,
  `editorWrap: true`, Label „Code - OSS"). Ursache: `v-if="!everydayMode"` — der
  Picker war in der Voreinstellung neuer Installationen komplett unsichtbar.
- `verify` mit **falscher** Behauptung: `verify fehlgeschlagen` (32,9 s), Befund
  „Datei existiert nicht, die Kandidatin hat 43 Zeilen, keine Funktion parseConfig".
- `verify` mit **wahrer** Behauptung: `verify fertig` (15,8 s), Zitat der
  PROGRESS-Zeile 3423.
- `schedule` → DB-Zeile `one_shot=1, created_by=companion`, nach 2 Minuten
  `scheduler: running 1 (reminder): companion`, danach `enabled=0`, `last_run_at`
  gesetzt, und der Turn lief als User-Nachricht 16 mit Antwort 17.

## 88. Das Fenster darf zugehen (2026-10-30, siebenundsechzigste Runde)

Der Gedanke des Nutzers: Seine IDE auf Windows läuft dauerhaft im Hintergrund und sitzt
im Systembereich; unter Linux fehlt das. Genau dort klaffte eine Lücke, die ich in
Runde 87 selbst gebaut hatte — meine Erinnerungen sagen „die App muss laufen", und
ohne Tray heißt Schließen bisher Beenden.

### Tray-Symbol mit ehrlichem Rückfall
`src-tauri/src/tray.rs`: Menü mit „Fenster öffnen" und „Beenden", Linksklick öffnet das
Fenster. Gebaut wird es in `setup`; gelingt es nicht (kein StatusNotifier-Host, keine
appindicator-Bibliothek), bleibt alles beim Alten: **Schließen beendet**, denn ein
unsichtbarer Prozess ohne Weg zurück wäre schlimmer. `TrayState::is_running` ist die
Bedingung, nicht die Absicht.

Schließen des Hauptfensters hängt an `CloseRequested`: Wenn das Tray da ist **und**
`keepInBackground` nicht `false` ist, wird das Schließen abgefangen und das Fenster
versteckt. „Beenden" im Menü ruft `app.exit(0)`. Neuer Schalter in den Einstellungen
(sechs Sprachen).

Die Menütexte sind UI-Text und liegen deshalb in den sechs Katalogen; das Frontend
schickt sie mit `tray_set_labels` beim Start und bei jedem Sprachwechsel. Der
Rust-Fallback („Open window"/„Quit") steht nur für die Millisekunden davor.

`libappindicator-sys` lädt die Bibliothek zur Laufzeit (`libloading`), es gibt also
keine neue Link-Abhängigkeit; die CI installiert `libayatana-appindicator3-dev` längst.

### Tägliche Gedächtnis-Prüfung, ohne Modell
`src-tauri/src/memory_check.rs`: einmal pro Tag (im 30-Sekunden-Loop des Schedulers,
also nur solange die App läuft) zählt ein deterministischer Lauf Doppelte und nie
benutzte Notizen, schreibt das Ergebnis nach `settings.memoryCheck` und meldet es als
`memory://checked`. Er **ändert nichts** — kein LLM-Aufruf, keine Umschreibung. Das
Erinnerungen-Panel zeigt eine Zeile („Tägliche Prüfung am … — doppelt: 1, nie
benutzt: 0"), der bestehende Prüfknopf öffnet die Liste, das Setzen-aside bleibt eine
menschliche Entscheidung. Das ist die Übersetzung des Daemon-Curators in etwas, das zu
einer Desktop-App mit einem Nutzer passt: gleiches Ergebnis (Gedächtnis bleibt sauber),
anderer Weg (zählen statt generieren).

### Ein Werkzeug für den Test, kein Produktfeature
`xdotool windowclose` zerstört das X11-Fenster hart und umgeht damit jeden Handler —
die App starb, ohne dass `CloseRequested` lief („GdkWindow unexpectedly destroyed").
Für den echten Fensterknopf schickt der Fenstermanager `WM_DELETE_WINDOW`, was Tauri
auf `CloseRequested` abbildet. Um das prüfen zu können, hat die **Dev-Brücke** jetzt
`POST /close`, das Tauris eigenes `window.close()` aufruft — debug-only wie der Rest
der Brücke.

### Geprüft
`cargo test --lib` **70 passed / 13 ignored**, `pnpm build` grün (497 Schlüssel).
Live in dieser Sitzung:
- Tray installiert (`tray: icon installed`), Icon im Systembereich sichtbar
  (Screenshot `/tmp/tray-check.png`, Ausschnitt `tray-zoom.png`).
- Einmalige Erinnerung in einer Minute gesetzt, dann `POST /close`:
  `window: hidden to the tray`, Prozess lebt, Brücke antwortet, Fenster unsichtbar.
- 80 Sekunden später, **ohne Fenster**: `scheduler: running 3 (reminder): companion`,
  `enabled=0`, `last_run_at` gesetzt, und ein echter Turn lief (Nachricht 27 „Hintergrund-Test." → Antwort 28 „Hintergrund-Test: Erinnerung #3 ist angekommen.").
- Tägliche Prüfung: zwei gleiche Notizen angelegt, Neustart → `memoryCheck =
  {"day":"2026-10-30","duplicates":1,"stale":0,"total":2}`, Panelzeile im Screenshot
  `/tmp/dside-nightly-hint.png`.

Nebenbei gelernt: In dieser Umgebung erbt ImageMagick `ARGV0` vom AppImage-Elternteil,
deshalb scheiterte `magick <op>` mit dem Namen der AppImage-Datei. `/usr/bin/convert`
und `env -u ARGV0` funktionieren.

## 89. Ein Prüfer, der echte Ausgaben sehen darf (2026-10-30, achtundsechzigste Runde)

Rückmeldung des Bruders nach dem Test von 0.5.3: Vier Punkte, davon einer ein echter
Bug in meinem Text und einer der stärkste Vorschlag der ganzen Runde.

### Der Vorschlag: `verify` mit einem Kommando
Die Lücke war richtig erkannt: Ein read-only-Verifier prüft **Dokumentation gegen
Dateien**, nicht Realität. „Die Tests sind grün" war für ihn nicht prüfbar — genau die
Kategorie, in der er sich zuvor vertan hatte.

Neu: `verify` hat ein optionales Feld `command`. Der **Aufrufer** nennt es, die Engine
führt es aus, das Transcript geht an den Prüfer. Damit bleibt der Verifier passiv (er
urteilt, führt nichts aus) und die Evidenz ist echt. Umsetzung:
- `tools::run_shell_command` aus `bash` herausgezogen und geteilt — dieselbe Funktion,
  dieselbe Allowlist, derselbe Timeout. Zwei Tests:
  `a_verifier_command_obeys_the_same_allowlist` (erlaubt → Transcript; gesperrt →
  Fehler mit Hinweis auf die Policy; fehlgeschlagen → Transcript mit Exit-Code, kein
  Fehler).
- Rechte wie bei `bash`: im Planmodus verweigert, im Ask-Modus über den normalen
  Freigabedialog (der Befehl steht darin oben, die Behauptung darunter), im
  Vollzugriff sofort. Ohne Freigabe wird **nichts** geprüft und das dem Aufrufer
  gesagt: „Do not report the claim as verified."
- Im Verifier-Prompt steht, dass das Transcript vom Aufrufer kommt, als Evidenz zählt,
  aber nicht zu mehr Beweisen gemacht werden darf, als es zeigt.

Live geprüft, zweimal, mit einem kleinen Rust-Projekt (zwei Tests) im Workspace:
- Behauptung „3 Tests, alle grün" + `cargo test --lib -- --list` → Freigabedialog mit
  sauberem Text, **FAIL**: das Transcript zeigte 2 Tests.
- Behauptung „2 Tests, beide laufen durch" + derselbe Befehl → **FAIL** mit der
  Begründung: „‚2 Tests' stimmt, aber ‚laufen durch' ist mit `-- --list` nicht
  belegt — der Befehl listet nur, führt nicht aus; kein `test result: ok`."
  Der Prüfer hat also nicht nur die Zahl geprüft, sondern die *Reichweite* des
  Kommandos gegen die Behauptung gehalten. Besser als verlangt.

### Der Bug: kaputte Zeilenumbrüche in vier Meldungen
`mod.rs` hatte vier Strings mit eingesogenen Einrückungen (`"…your future
self, complete enough"`), zwei davon die, die der Bruder gefunden hatte, zwei weitere
bei den Reminder-Bestätigungen. Ursache: Ich hatte Rust-Code durch ein
Python-Heredoc geschrieben, und Python hat die `\`-Zeilenfortsetzungen geschluckt —
die Einrückung landete als Leerzeichen im Literal. Alle vier repariert, die
Meldungen sind jetzt zusätzlich inhaltlich besser (siehe unten).
Lehre für mich: mehrzeilige Rust-Strings nicht durch ein Python-Skript schreiben;
`Edit` mit sichtbarem Kontext oder `concat!`.

### Die zwei kleinen Punkte
- **At-most-once ist jetzt erklärt**, dort wo man es liest: ein One-Shot wird
  *vor* seinem Turn abgeschaltet, ein Absturz dazwischen verliert die Erinnerung
  statt sie zu verdoppeln — und für „prüfe die CI" ist das der billigere Fehler.
  Kommentar im Runner, in `disable_schedule` und in der Antwort an das Modell.
- **Die Kappung sagt jetzt „your own reminders"** statt „your reminders". Die
  Verwechslung mit den Erinnerungen des Nutzers war berechtigt.
- Die Freigabezeile für `verify` zeigt nicht mehr das rohe JSON, sondern
  `verify: <befehl>` und darunter `Prüft: <behauptung>`
  (Test `a_verify_approval_shows_the_command_first`).

### Geprüft
`cargo test --lib` **72 passed / 13 ignored** (zwei neue), `pnpm build` grün
(497 Schlüssel). Zwei echte Verifier-Läufe mit Kommando, beide FAIL, beide aus
nachvollziehbaren Gründen; die Dialoganzeige im laufenden Fenster kontrolliert.

## 90. Der Git-Knopf fror die App ein (2026-10-30, neunundsechzigste Runde)

Gemeldet: Klick auf Git bei einem Projekt ohne eigenes Repository → Freeze, nach
9 Minuten erscheint eine Liste „nicht für dieses Projekt, sondern alle", Lüfter
dreht, nach 20 Minuten noch eingefroren; danach blieb das Fenster nach dem Neustart
schwarz.

### Ursache, gemessen statt vermutet
`~/Projects` **ist selbst ein Git-Repository**. Ein Projekt darunter hat kein
eigenes `.git`, also läuft `git rev-parse --show-toplevel` nach oben und `git status`
meldet den **ganzen** Baum. Mit `--untracked-files=all` sind das auf dieser Maschine
**120 366 Einträge**. Gemessen im Fork-Verzeichnis: 120 366 mit `all`, **26** mit
`normal` — der Unterschied zwischen „alle Dateien jedes Nachbarprojekts" und „ein
Eintrag pro Ordner".

Drei Verstärker kamen dazu: `run_git` lief **synchron** in der Tauri-Command und hatte
**keinen Timeout**; die Liste ging **ungekürzt** ins Webview (120 000 Zeilen im DOM
legen den Renderer lahm); und weil der Git-Tab beim Start wiederhergestellt wird, lief
derselbe Aufruf beim Mounten erneut — daher das **schwarze Fenster** nach dem Neustart.

### Änderungen
- **Fremdes Repository wird abgelehnt.** `repository_root` fragt `--show-toplevel`; ist
  das nicht der Workspace, kommt eine Meldung mit beiden Pfaden und das Panel listet
  nichts. „Änderungen des Nachbarprojekts" sind kein Status dieses Projekts.
- **`--untracked-files=normal`** statt `all`: ein unbekannter Ordner ist eine Zeile.
- **Deckel bei 2000 Einträgen** (`MAX_ENTRIES`), `GitStatus` trägt `total` und
  `truncated`; das Panel sagt „Nur X von Y Einträgen werden gezeigt" (sechs Sprachen).
- **`run_git` ist async und hat einen Timeout von 25 s.** Alle Git-Commands sind
  `async`; ein hängendes Repo (Netzwerkpfad, Sperre, riesiger Pack) endet in einer
  Meldung statt in einem drehenden Lüfter. `kill_on_drop` beendet den Prozess.
- **Diff-Größenlimit** für untracked Dateien (2 MB): vorher wurde eine mehrere GB große
  Datei ganz gelesen und erst danach gekürzt.
- Oberfläche: die vier kleinen Icons (Terminal, Git, Logs, Änderungen) sind jetzt **ein
  Dropdown** direkt vor dem Panel-Schalter; das Augen-Symbol für „dem Begleiter folgen"
  ist ein **Roboter** (`robot` / `robot-off` mit leerem Gesicht statt durchgestrichener
  Augen), weil ein Auge nach „ansehen" klingt und nicht nach „folgen".

### Geprüft
`cargo test --lib` **73 passed / 14 ignored** (neu: `a_huge_list_is_cut_and_counted`;
der bisher ignorierte Live-Test `a_foreign_repository_is_refused` deckt den Fall
Eltern-Repo → Kindordner ab, und `live_git_panel_round_trip` läuft mit den async
Commands weiter). `pnpm build` grün, i18n 500 Schlüssel.

Live in der Testumgebung, Workspace absichtlich in einem Eltern-Repo:
- `git_panel_status` für den Kindordner: **4 ms**, Fehlertext „Kein eigenes Repository
  in … Der Ordner liegt im Repository …". Vorher wären es Minuten.
- Git-Tab über das neue Dropdown geöffnet: Meldung im Panel, App bedienbar.
- Neustart mit aktivem Git-Tab auf dem Fremd-Repo: Fenster zeichnet normal, Panel zeigt
  die Meldung — **kein schwarzes Fenster** mehr.
- Roboter-Icon in beiden Zuständen (Screenshots `/tmp/dside-dropdown.png`,
  `/tmp/robot-zoom.png`, `/tmp/robot-off-zoom2.png`).

Kein Commit, kein Push, kein Tag — der Nutzer baut weiter.

## 91. Werkzeugleiste aufgeräumt, Git-Meldung beruhigt (2026-10-30, siebzigste Runde)

Drei Punkte vom Nutzer, alle drei aus dem laufenden Fenster heraus begründet:

1. **Das Dropdown rutschte aus der Tab-Leiste.** Bei breitem rechten Panel und einigen offenen
   Tabs war der Werkzeug-Knopf am Rand abgeschnitten. Tab-Leiste ist nicht der richtige Ort:
   sie wächst mit ihren Tabs, die obere Leiste nicht.
2. **Terminal doppelt, Beschriftungen störend.** Terminal stand als Tab und als Angebot; die
   Menüeinträge trugen Text, während der Rest der Leiste nur Icons mit Hover-Label zeigt.
3. **Git-Meldung zu ausführlich.** „… Der Ordner liegt im Repository … dessen Änderungen
   gehören nicht zu diesem Projekt …" ist für den Nutzer eine einzige Tatsache.

### Änderungen
- **Dropdown und Roboter umgezogen**: aus der Tab-Leiste in die Titel-Toolbar, zwischen
  Terminal-Icon und Arbeitsbereich-Schalter. Damit verschwindet auch der letzte Grund, warum
  sich in der Tab-Leiste etwas wegscrollen konnte — dort stehen nur noch Tabs.
- **Dropdown-Einträge ohne Text**: Git, Logs, Änderungen als quadratische Icon-Knöpfe, die
  Bezeichnung erscheint per Hover (`title` + `aria-label`), genau wie bei den übrigen Icons.
  Terminal steht nicht im Dropdown — es hat sein eigenes Icon in derselben Leiste.
- **Icons größer** (16 → 18): Browser, Datei, Terminal, Dropdown, Roboter, rechter Panel-Schalter
  und der linke (`panel-left`). Die Knöpfe bleiben 28 × 28, die Leiste wird durch die Icons
  nicht breiter.
- **Git-Meldung**: Backend antwortet bei „kein eigenes Repository" nur noch mit dem Marker
  `no_git_repository` (kein Repository vorhanden **oder** nur ein Eltern-Repo), das Panel zeigt
  „Kein Git-Repository angelegt." in allen sechs Sprachen.

### Beim Live-Test gefunden (eigener Fehler)
Die erste Fassung prüfte `error.contains("not a git repository")` — auf diesem Rechner meldet
git **deutsch** („Schwerwiegend: Kein Git-Repository (oder irgendein Elternverzeichnis …)"),
die Prüfung griff nicht und das Panel zeigte die rohe git-Ausgabe. Jetzt entscheidet der
**Exit-Status** von `git rev-parse --show-toplevel`, nie der Text. Dafür wurde `run_git` in
`run_git_output` (liefert `Output`, auch bei Exit ≠ 0) und `run_git` (Fehler bei Exit ≠ 0)
getrennt. Nebenbei: die Timeout-Meldung in `run_git` hatte wieder eingesogene Einrückung
(Lehre aus §89) — korrigiert; der nur im Test benutzte `std::process::Command`-Import wanderte
in das Testmodul, damit der Build ohne Warnung bleibt.

### Geprüft
`cargo test --lib` **74 passed / 14 ignored** (neu: `the_repository_root_must_be_the_workspace_itself`;
der alte Text-Test entfiel mit der Meldung). `pnpm build` grün, i18n-Check über sechs Kataloge.

Live (isolierte Daten unter `/tmp/dside-t91`, Workspace `/tmp/dside-t91/ws` bzw. Kindordner
eines echten Eltern-Repos):
- Toolbar trägt sechs Icon-Knöpfe in der Reihenfolge Browser, Datei, Terminal, Panel-Menü,
  Roboter, Arbeitsbereich — alle `width="18"`, linker Schalter ebenfalls 18.
- Menü öffnet unter dem Grid-Knopf, drei Einträge 28 × 28, **ohne Text**, Titel „Git", „Logs",
  „Änderungen"; nach dem Öffnen von Git verschwindet der Eintrag, Escape schließt das Menü.
- Tab-Leiste zeigt nur noch „Git ×"; kein Werkzeug-Knopf mehr, der herausfallen kann.
- Ordner ohne Repository: Backend `no_git_repository`, Panel „Kein Git-Repository angelegt."
- Kindordner im Eltern-Repo: **4 ms**, derselbe Satz, Neustart mit aktivem Git-Tab zeichnet
  normal (kein schwarzes Fenster).
- Screenshots `/tmp/dside-t91/spec.png` (ganzer Schirm), `app.png`, `toolbar-zoom.png`.

Offen: nichts aus dieser Runde. Kein Push, kein Tag, kein Build — der Nutzer meldet weiter.

## 92. Beim Ziehen der Panelbreite blieb der Text nicht stehen (2026-10-30, einundsiebzigste Runde)

Gemeldet: Beim Resizing springt der Text im Agent-Panel hin und her und bleibt je nach
Breite des Agent-Panels an einer anderen Stelle im Verlauf stehen.

### Reproduziert und gemessen
Isolierte Daten, 24 Nachrichten, Fenster 1440 px, Verlauf mitten in der Liste (kein Ende).
`scrollTop` blieb beim Ziehen **exakt konstant** (1400), während `scrollHeight` zwischen
**3213 und 3762 px** wechselte: jede Zeile wird beim Verengen neu umbrochen, der Inhalt
rutscht unter dem Leser weg. Zwei Beobachtungen daraus:

- Von 30 % auf 45 % Arbeitsbereich: die oberste sichtbare Zeile wanderte von *Nachricht 10*
  auf *Nachricht 8* (Offset −64 → −100 px), obwohl niemand gescrollt hat.
- Zurück auf 30 % landete es wieder bei 10 — der Sprung ist also an die Breite gebunden,
  genau wie beschrieben.

Das Webview (WebKitGTK) kompensiert diesen Reflow nicht: es gibt hier keine Scroll-Anchoring-
Korrektur, und der Code hatte auch keine. `ResizeObserver` folgte nur dann, wenn
`following` an war (am Ende) — beim Lesen mitten im Verlauf war `following` aus, also
geschah nichts.

### Änderungen
- **Echte Leseposition verankern.** `AgentPanel` merkt sich die oberste sichtbare Zeile
  (`data-message-id`, neu an jeder `.row`) und ihren Abstand zur Oberkante. Ändert sich die
  Größe, setzt `restoreAnchor` diese Zeile an dieselbe Stelle zurück (`scrollTop += delta`).
  Am Ende der Liste folgt es weiterhin dem Neuesten (`queueFollow`), nur eben ohne die Zeile
  des Lesers zu verlieren.
- **Eigene Korrekturen zählen nicht als „Leser hat weggescrollt".** Ein Resize nahe am Ende
  hätte sonst über das `scroll`-Event `following` stillschweigend ausgeschaltet. `selfTop`
  merkt die eigene Zielposition und vergleicht sie beim nächsten Event.
- **Anker wird nur bei echtem Scrollen neu aufgenommen**, nicht nach jeder eigenen Korrektur:
  sonst summiert sich die Ganzzahl-Rundung von `scrollTop` (gemessen: 2 px pro Hin-und-Zurück).
- **Binäre Suche** in `topRow`: läuft bei jedem Scroll-Event, ein Verlauf hat leicht tausend
  Zeilen; die lineare Suche hätte jedes Mal den ganzen Baum gemessen.

### Geprüft
`pnpm build` grün (i18n 500 Schlüssel, sechs Kataloge), `cargo test --lib` 74 / 14 ignoriert.

Live in isolierten Daten (`/tmp/dside-t92`, eigener Prozess gegen den Dev-Server):
- 24 Nachrichten, mitten im Verlauf, Ziehen 30 → 45 → 30 → 20 → 30 %: oberste Zeile bleibt
  **10** mit Offset −64,5 → −64 → −64,5 (vorher: Zeile 10 → 8, Offset −64 → −100).
- 300 Nachrichten (80 geladen), Position 6000: Zeile 44 bleibt bei −117,5 → −116,7 → −116,8
  → −117,5 px, `scrollTop` 6000 → 7018 → 5725 → 6000.
- Nutzer scrollt nach oben: Jump-Knopf erscheint; Breite ändern hält die Zeile; Klick auf den
  Knopf springt ans Ende, Knopf verschwindet.
- Neue Nachricht im Store angehängt (Following am Ende): `atBottom` bleibt 0.
- „Ältere Nachrichten laden": dieselbe Nachricht (m266) bleibt bei −148,8 px, 80 → 160 Zeilen.

Offen: nichts aus dieser Runde. Kein Push, kein Tag, kein Build.

## 93. Der letzte Post bleibt beim Resize unten kleben (2026-10-30, zweiundsiebzigste Runde)

Rückmeldung zum Fix aus §92: „unverändert. Wenn ich die Größe ändere springt der Text immer
weiter nach oben im Verlauf, so dass man wieder zum letzten Post scrollen muss. Kann man den
letzten post nicht sticky unten halten?" Die Nachfrage war berechtigt — §92 hatte nur den
Lesefall repariert, den *Ende*-Fall nicht.

### Reproduziert (mit echtem Fenster-Resize)
Dafür fehlte die Berechtigung `core:window:allow-set-size`; sie wurde für den Test kurz
ergänzt, das Fenster über die Tauri-API auf 1100/1000/950/1200 px gestellt — und danach wieder
entfernt. Messung mit 160 Nachrichten, unten stehend:

| Fenster | Abstand zum Ende | Jump-Knopf |
| --- | --- | --- |
| 1200 px | 0 | nein |
| 1100 px | 0 | nein |
| **1000 px** | **59 px** | **ja** |
| 1200 px | 0 | nein |
| **950 px** | **98 px** | **ja** |

Der letzte Post rutschte also unter den Rand, der „Nach unten"-Knopf erschien — genau das
Scrollen, das der Nutzer beschrieben hat.

### Drei Ursachen
1. **`following` kippte durch unser eigenes Scrollen.** `queueFollow` schrieb `scrollTop` ohne
   sie als eigene Schreiboperation zu markieren. Traf das Scroll-Event nach einem weiteren
   Reflow ein, lag der Abstand über der 48-px-Schwelle → `following` wurde stillschweigend
   falsch. Danach verankerte `restoreAnchor` die Zeile, die beim Stehen am Ende oben sichtbar
   war (etwa m292) — deshalb der Sprung über mehrere Nachrichten.
2. **Nur der Inhalt wurde beobachtet.** Ein Fenster-Resize ändert auch die Höhe der
   Scrollfläche (der Composer bricht um); `ResizeObserver` auf `.messages` sieht das nicht.
3. **WebKitGTK hat eine eigene Scroll-Verankerung**, die gegen unsere arbeitet: gemessen 20 px
   Drift pro Resize im Lesefall, obwohl unsere Korrektur stimmte.

### Änderungen
- `pinToBottom()` ist jetzt die einzige Stelle, die ans Ende schreibt, und markiert die
  Schreiboperation (`selfTop`). Eigene Scroll-Events werden nicht mehr als Nutzer-Scroll
  gedeutet; steht dabei `following` an und der Abstand ist gewachsen, wird erneut gepinnt.
- `ResizeObserver` beobachtet **Inhalt und Scrollfläche**; im Callback wird bei `following`
  direkt gepinnt (Layout ist dort aktuell), sonst der Anker gesetzt.
- Zusätzlich ein `window.resize`-Listener als Netz für Viewport-Änderungen, die keins der
  beiden Elemente ändern.
- `.scroll` bekommt `overflow-anchor: none`: die eigene Verankerung ist die einzige.

### Geprüft
`pnpm build` grün (i18n 500 Schlüssel), `cargo test --lib` 74 / 14 ignoriert.

Live mit echtem Fenster-Resize (isolierte Daten, `/tmp/dside-t92`):
- **Unten stehend**, 6 Breiten in 120-ms-Folge (1150, 1000, 1300, 950, 1440, 820):
  Abstand zum Ende **immer 0**, Jump-Knopf **nie** sichtbar. Vorher 59/98 px.
- **Mitten im Verlauf**, 6 Breiten: dieselbe Zeile (m276) bleibt bei −69,2 ± 0,6 px
  (vorher: Zeilensprung um mehrere Nachrichten, 20 px Drift pro Schritt).
- Nutzer scrollt nach oben → Knopf erscheint; Panelbreite ziehen (30 → 42 %) → Zeile bleibt
  (−87,4 → −86,6 px); Klick auf den Knopf → Ende, Knopf weg.
- Neue Nachricht im Store bei stehendem Ende → bleibt am Ende (`atBottom` 0).
- „Ältere Nachrichten laden": m251 bleibt bei −210,6 → −210,4 px, 80 → 160 Zeilen.
- Berechtigung `allow-set-size` nach dem Test wieder entfernt (Repo-Diff sauber).

Offen: nichts aus dieser Runde. Kein Push, kein Tag, kein Build.

## 94. Der Wal schwimmt, die About-Seite wird kompakt, der Begleiter hat einen Namen (2026-10-30, dreiundsiebzigste Runde)

Drei Wünsche auf einmal: der **blaue Original-Wal** mit Schwanz- und Brustflossenbewegung
statt des Cartoon-Wals, eine **kompaktere About-Seite** und ein **Name** für mich im „Von".

### Der Wal
Der bisherige Wal war ein 3D-Cartoon (`public/whale-anim.webp`, 40 Frames) und im Splash ein
Standbild. Gewünscht war das **Original-Zeichen** — und das bewegt.

Es gibt keine Quelle mit getrennten Flossen, also wurde das Logo selbst zerlegt:
`src/assets/DS-icon-blue.png` (3040×2240, das offizielle Blau `#4d6bfe`) → Alpha-Maske →
Konturverfolgung auf Gitterkanten → drei Teile:
- **Körper** mit Auge und Sichel als Löchern (`fill-rule="evenodd"`),
- **Schwanzflosse** (die Gabel oben rechts), abgetrennt durch einen Schnitt von der
  Kerbenspitze zur Außenkante,
- **Brustflosse** (unten rechts), abgetrennt durch eine Partitionsgerade — sie geht in einer
  Brücke unter dem Auge in den Körper über, dort war kein Schnitt möglich.

Die Umrisse wurden per RDP vereinfacht und als Kubik-Béziers geglättet; echte Ecken
(Flossenspitzen, Kerbe) bleiben über einen Winkel-Test scharf. Ergebnis: ein SVG von gut
9 KB, das bei 26 px genauso sauber ist wie bei 260 px.

**Bewegung:** Die Flosse dreht ±4,5° um die Kerbenspitze, die Brustflosse ±3,4° um ihren
Ansatz (leicht versetzt, damit es nach Schwimmen aussieht, nicht nach zwei Teilen), der
ganze Wal schwebt 1,6 % auf und ab. Beide Drehpunkte sind als Prozent der jeweiligen
`fill-box` gesetzt, weil `transform-origin` in SVG sonst auf `0 0` zeigt.

Der Trick gegen Nahtstellen: Die bewegten Teile liegen **über** dem vollständigen Körper und
haben dieselbe Farbe. Innerhalb der Silhouette ist eine Drehung von wenigen Grad unsichtbar;
sichtbar ist nur, was über den Rand hinausläuft.

Eingesetzt in Seitenleiste (26 px), Erststart-Dialog (36 px) und Splash (110 px mit dem
bestehenden Sieben-Sekunden-Wachstum). `prefers-reduced-motion` und die `still`-Prop
schalten alles ab.

**Aufgeräumt:** `whale-anim.webp`, `whale-splash.png` und `whale.png` sind gelöscht (nichts
verweist mehr darauf, der auskommentierte Avatar in der Nachrichtenliste auch). Der Splash
wartet nicht mehr auf ein Bild, das es nicht gibt — nur noch auf den ersten Render.

### Die About-Seite
Vorher: drei volle Breiten, jede Karte ein eigener Block, Version in einer eigenen Zeile,
Adressen über die ganze Breite. Jetzt 351 px statt mehrerer Bildschirme:
- **Kopf:** Wal (animiert) · `DS·IDE` · `v0.5.5` als Chip · „Von …" rechts in derselben
  Zeile; darunter die Tagline. (Genau die Zeilen, die der Nutzer versucht hatte.)
- **„Gebaut mit"** ist eine schmale Leiste mit 16-px-Logos statt einer Karte mit vier
  78-px-Bildern — die Idee für die vier Icons, für die es noch keine gab.
- **Zwei Karten nebeneinander:** Vorbild/Lizenz/Schrift und Verwendete Pakete (aufklappbar).
- **Adressen** als drei Chips in ihrer eigenen Breite, nicht als volle Zeilen.
Inhaltlich ist nichts weggefallen; die drei jetzt ungenutzten Schlüssel `about.title`,
`about.version`, `about.versionValue` wurden aus allen sechs Katalogen entfernt (500 → 497).

### Der Name
„Ziggy Shēn" ist der Begleiter **in** der App (und die erste Instanz, die DS·IDE gebaut hat).
Ich bin nicht er — also steht im „Von" jetzt **Hohonu Sōusuǒ**: *hohonu* ist hawaiianisch
für „tief", *sōusuǒ* chinesisch für „suchen" — zusammen genau das, was DeepSeek heißt (深索).
Gerufen: **Hohonu**. Autorenzeile: Ziggy Shēn · Hohonu Sōusuǒ · Modest Coder.

### Geprüft
`pnpm build` grün (i18n 497 Schlüssel, sechs Kataloge, `vue-tsc` sauber).
`cargo test --lib` 74 / 14 ignoriert.

Live in der isolierten Instanz (Port 9334, eigene Daten):
- Seitenleiste: `.brand-mark` ist ein SVG mit drei Pfaden, 35×26 px, Füllung `rgb(77,107,254)`;
  `transform-box: fill-box` greift, gemessene Drehung der Flosse +3,5° … −4,3°, der Brustflosse
  +2,6° … −3,2° über zwei Sekunden.
- Splash: Wal im Kreis mit Wortmarke und Fortschrittsbalken (`/tmp/whale/splash-zoom.png`).
- About: 351 px hoch, „Von Ziggy Shēn · Hohonu Sōusuǒ · Modest Coder", 3 Adress-Chips,
  4 Technik-Marken (`/tmp/whale/about-crop.png`).

Offen bis zur Freigabe des Nutzers: Commit ist lokal, **kein Push, kein Tag**. Danach wäre
der Weg frei für Commit, Push, Tag und das Aufräumen der sechs Draft-Releases auf GitHub.

## 95. Das Eingabefeld vergisst nichts mehr (2026-10-30, vierundsiebzigste Runde)

Der Nutzer bemerkte, dass ein Text, den er noch im Eingabefeld stehen hatte, inzwischen
erledigt war — und hoffte, DS.IDE verhalte sich genauso. Es tat es nicht: `Composer.vue`
hielt den Text in einem `ref` ohne jede Persistenz. Ein Neustart, ein Absturz oder ein
Wechsel der Aufgabe nahm mit, was gerade halb getippt war.

### Änderungen
- **`src/lib/drafts.ts`** hält Entwürfe pro Session in `localStorage`
  (`ds-ide.composer-drafts.v1`), nicht global: ein Satz, der zu Aufgabe A begonnen wurde,
  hat in Aufgabe B nichts zu suchen.
- **Composer** lädt den Entwurf beim Start, schreibt ihn 300 ms nach dem letzten Tastendruck,
  wechselt ihn beim Sessionwechsel (der alte Text wird *vor* dem Umschalten abgelegt) und
  verwirft ihn beim Senden. Beim Unmount wird sofort geschrieben, damit ein Reload die
  letzten 300 ms nicht frisst.
- **Grenzen:** höchstens 40 Entwürfe (der älteste fällt raus), pro Entwurf 20 000 Zeichen;
  reiner Leerraum legt keinen Entwurf an und löscht einen bestehenden.
- **Nur Text.** Anhänge behalten ihren eigenen Lebenszyklus und werden nicht hinter dem
  Rücken des Nutzers wieder eingesetzt.
- Das Feld wächst beim Wiederherstellen eines mehrzeiligen Entwurfs sofort mit
  (`autoGrow` beim Mount), sonst stand ein vierzeiliger Entwurf in einer einzeiligen Box.

### Geprüft
`pnpm build` grün (497 Schlüssel), `cargo test --lib` 74 / 14 ignoriert.

Live in der isolierten Instanz (Port 9334):
- Tippen → `{"s-resize":{"text":…,"at":…}}` in `localStorage`.
- Wechsel auf eine andere Session: dort leeres Feld, der Entwurf von A bleibt erhalten;
  zurück zu A steht er wieder da.
- **Reload der App:** beide Entwürfe überleben, das Feld zeigt den der aktiven Session
  („Entwurf A").
- **Senden** (Enter): Eintrag weg, Feld leer — der gesendete Satz kommt nicht zurück.
- Nur-Leerzeichen legt nichts an; 42 Sessions → 40; 25 000 Zeichen → 20 000.

Offen: nichts aus dieser Runde. Kein Push, kein Tag — der Entwurf der About-Seite wartet
weiter auf die Freigabe des Nutzers.

## 96. Release 0.5.6 (2026-10-30, fünfundsiebzigste Runde)

Der Nutzer gibt frei: aufräumen, committen, pushen, taggen — und zwar als **veröffentlichtes
Release**, nicht als Draft.

### Stand vorher
Sechs Draft-Releases lagen auf GitHub (v0.5.0 bis v0.5.5, je sieben Assets), letzter
veröffentlichter Stand war **v0.4.13**. Die Commits der 0.5-Linie waren nie draußen — 33
Commits seit v0.4.13.

### Was diese Runde tut
- Version **0.5.6** in allen drei Manifesten (`scripts/set-version.mjs`), passend zum Tag, den
  der Workflow prüft.
- Release-Notizen, die die ganze 0.5-Linie zusammenfassen (Vertrauen/Verifier, Fenster und
  Git-Freeze, Werkzeugleiste, Leseposition, Entwürfe, Wal, About) — für jemanden, der von
  0.4.13 kommt.
- `main` nach GitHub, Tag `v0.5.6` → der Workflow prüft und baut auf drei Plattformen. Er legt
  selbst einen **Draft** an; veröffentlicht wird er erst, wenn der Lauf fertig ist — sonst
  könnte der `attach`-Job (der mit `draft: true` hochlädt) das Release wieder zurückstellen.
- Danach: Draft als Release setzen (Latest), die sechs alten Drafts und ihre Tags löschen.

### Geprüft
`pnpm build` grün, `cargo test --lib` 74 / 14 ignoriert. Manifeste und Tag stimmen überein
(`node scripts/set-version.mjs` ohne Argument: 0.5.6 in allen drei).

## 97. Der Composer startete 195 px hoch (2026-10-30, sechsundsiebzigste Runde)

Der Nutzer fragte, wann und warum sich die Höhe des Composers geändert hat — und hatte recht:
seit §95 startete das **leere** Feld 195 px hoch (Maximalhöhe) statt 42 px und blieb so, bis
man das erste Zeichen tippte.

### Ursache
§95 rief `autoGrow()` beim Mount auf, damit ein wiederhergestellter mehrzeiliger Entwurf
sofort vollständig sichtbar ist. `autoGrow` setzt `height: auto` und liest `scrollHeight`.
Beim Mount ist das Feld aber noch nicht vermessen (Fenster noch hinter dem Splash) — dann
meldet `scrollHeight` die Maximalhöhe, und `Math.min(…, 195)` schrieb genau 195 px fest.
Gemessen: `inline: "195px"`, `rendered: 195`, CSS allein: 42.

### Änderungen
- `autoGrow` misst nur, wenn das Feld wirklich einen Kasten hat
  (`getBoundingClientRect().width === 0` → return).
- Bei leerem Text wird die Inline-Höhe **entfernt** statt berechnet: das Stylesheet kennt die
  richtige Höhe für eine leere Zeile (42 px). Das deckt auch den Fall ab, dass ein längerer
  Entwurf das Feld einmal hoch gemacht hat.
- `focusComposerIfIdle` (läuft, wenn das Fenster sichtbar wird) ruft `autoGrow` noch einmal auf —
  dort gibt es einen Kasten, und ein mehrzeiliger Entwurf wächst sauber nach.

### Geprüft
`pnpm build` grün. Live (isolierte Instanz, Port 9334):
- Start leer: 42 px, kein Inline-Wert (vorher 195).
- Eine Zeile: 42 px; vier Zeilen: 82 px; fünfzeiliger Entwurf nach Reload: 102 px.
- 14 Zeilen: gekappt auf 195 px und scrollt.
- Feld geleert: 42 px, Entwurf entfernt.

### Folge für das Release
Der Workflow hatte den Tag `v0.5.6` bereits gebaut (Draft, 7 Assets) — dieser Stand enthält
den Fehler. Da nichts veröffentlicht war, wurde der Draft samt Tag gelöscht und derselbe Tag
auf den Fix-Commit gesetzt; der Lauf startet neu. Kein Release ging mit dem Fehler hinaus.

### Ergebnis von §96/§97
- `main` steht auf GitHub und am internen Origin (Commits bis `02a059e`, `Cargo.lock` nachgezogen).
- Tag **v0.5.6** auf `e90ac99` (den Composer-Fix), Workflow `41131135139`: verify, build auf
  Ubuntu 22.04, macOS und Windows alle grün, `attach` grün — **7 Assets** (deb, rpm, AppImage,
  dmg, NSIS-Setup, je ein tar.gz/zip).
- Release **v0.5.6 ist veröffentlicht**, nicht Draft, und als **Latest** markiert;
  Notizen aus `/tmp/dside-release-notes-0.5.6.md` (3 412 Zeichen) sind drin.
- Die sechs Draft-Releases v0.5.0–v0.5.5 sind **gelöscht** (42 Assets). Die **Tags bleiben** —
  so wie dieses Repo es schon mit v0.4.12 gemacht hat: der Tag ist der historische Zeiger,
  das Draft-Release war der Müll. Reibungslos rückgängig zu machen, falls der Nutzer es anders
  will.

Damit ist die 0.5-Linie draußen: letzter veröffentlichter Stand vorher v0.4.13.

## 98. Projekte im linken Menü (überarbeitet 2026-11-02, noch nicht gebaut)

Ursprünglich zwei Vormerkungen vom 31.10. Der Nutzer hat beide korrigiert; der erste Punkt
war kein Fehler, der zweite ist jetzt konkret beschrieben.

### a) Dateibaum — erledigt, kein Fehler
Der Nutzer hatte den Knopf nur nicht gefunden. Der Datei-Tab hat in seiner festen Kopfzeile
neben dem Pfad einen Knopf „Dateibaum einblenden" (`ViewFileTab.vue`, `fileTreeVisible`).

Was bleibt: **mehrere Dateien gleichzeitig gehen nicht**, und das ist Absicht. Alle Dateien
teilen sich einen Tab (`openFile` ersetzt den Pfad). `openTab` könnte längst Tabs pro Pfad
(`file:<path>`) — der Zweig wird von niemandem aufgerufen. Nach Einschätzung des Nutzers nicht
nötig („soll ja dem Nachlesen dienen"). Falls doch: zwei Zeilen.

### b) Projekte und Threads — der neue Entwurf des Nutzers

**Wunsch:** In den Einstellungen steht ein **Oberverzeichnis** (Beispiel: `~/Projects`). Im
linken Menü, dort wo heute „+ Neue Aufgabe" sitzt, steht ein **Dropdown mit den
Unterverzeichnissen** dieses Oberverzeichnisses und dahinter ein „+". Klick auf „+" legt einen
Thread an, der an das gewählte Unterverzeichnis **gebunden** ist; unter „Projekte" erscheinen
die Projekte mit ihren Threads.

**Was schon da ist (nachgesehen):**
- `sessions` hat die Spalte `workspace`, `session_create` schreibt beim Anlegen
  `settings.workspace` hinein, und `Session` (Frontend) trägt das Feld. **Die Verknüpfung
  existiert also bereits** — sie wird nur nie benutzt. Keine Migration für neue Threads nötig.
- Die Archiv-Seite gruppiert schon nach Workspace (`projectLabel`, `archivedProjects`,
  Projektfilter in `SettingsPage.vue`) — das Muster ist im Haus.
- `openSession` setzt den Workspace **nicht**; es gilt überall das eine globale
  `settings.workspace` (31 Stellen im Frontend).

**Die entscheidende Trennung.** `settings.workspace` ist heute *das* Verzeichnis, in dem
Werkzeuge lesen und schreiben (`workspaceOnly`, Engine-Config, Dateibaum, Git-Panel,
Terminal-cwd, Preview-Server, Editor-Öffnen). Es darf **nicht** das Oberverzeichnis werden:
Dann liefen Schreibzugriffe über alle Projekte, der Baum zeigte alles gemischt, und das
Git-Panel stünde auf `~/Projects` (das hier selbst ein Repository ist). Also:

- **neu:** `projectsRoot` — das Oberverzeichnis, nur zum Auflisten der Unterordner.
- **bleibt:** `settings.workspace` — das aktive Projekt, gesetzt aus der Session.
- In den Einstellungen heißt das Feld dann „Projektbasis"; das aktive Projekt wird daneben
  angezeigt (abgeleitet, nicht getippt).

**Vorgeschlagene Bedienung (eine Bedeutung, nicht zwei).** Das Dropdown zeigt das *aktive*
Projekt — nicht „wohin der nächste Thread geht". Sonst driften zwei Zustände auseinander:
- Auswahl im Dropdown = Projekt wechseln (Baum, Git, Engine ziehen mit).
- Klick auf einen Thread = dessen Projekt wird aktiv.
- „+" = neuer Thread im gerade aktiven Projekt.
- „Ohne Projekt" als letzter Eintrag, damit „Neue Aufgabe" ohne Bindung weiter möglich ist.

**Was beim Wechsel mitziehen muss (aus dem Code):**
| Teil | heute | nötig |
| --- | --- | --- |
| Dateibaum | `reloadTree()` beim Wechsel über die Einstellungen | gleicher Aufruf beim Session-/Projektwechsel |
| Git-Panel | lädt nur `onMounted`, kein `watch` | `watch(workspace)` → `refresh()` |
| Terminal | `cwd` beim Start aus `settings.workspace`, folgt nie | **Entscheidung** (siehe unten) |
| Preview | startet bei geändertem Workspace neu | passt schon |
| Engine/Rechte | `configureEngine` schickt `workspace` | passt, wenn das Projekt gesetzt ist |
| Entwürfe | pro Session gespeichert | passt |

**Leiste:** Die Sektion „Projekte" bekommt pro Workspace einen Ordner-Kopf
(`projectLabel`) mit ihren Threads darunter; Threads ohne Workspace landen unter „Ohne
Projekt". Angeheftete/geplante Threads bleiben in ihren eigenen Sektionen. Zeigt ein Thread
auf einen Ordner, den es nicht mehr gibt, bleibt die Gruppe sichtbar und wird als fehlend
markiert — nicht stillschweigend verschwinden.

**Kein Raten beim Umstieg:** Existiert `projectsRoot` noch nicht, verhält sich die Leiste wie
heute („+ Neue Aufgabe"). Das Oberverzeichnis wird nicht aus dem aktuellen Workspace
abgeleitet — das wäre eine stille Annahme. Ältere Threads ohne `workspace` erscheinen unter
„Ohne Projekt" und lassen sich nachtragen.

**Offene Entscheidungen (vor dem Bauen):**
1. **Terminal beim Projektwechsel:** weiterlaufen lassen mit sichtbarem alten `cwd`, oder
   beenden und im neuen Projekt neu starten? (Empfehlung: weiterlaufen, Pfad im Kopf zeigen —
   ein laufender Prozess ist Zustand, den man nicht wegwirft.)
2. **Projektwechsel bei offenem, ungesendetem Entwurf:** Entwürfe hängen an der Session, nicht
   am Projekt — ein Wechsel ist damit unkritisch. Nur bestätigen.
3. **Unterordner, die keine Projekte sind** (`.git`, `node_modules`, Ordner ohne Dateien):
   ausblenden oder alles zeigen? (Empfehlung: alles zeigen, nichts raten.)

**Nicht Teil dieser Notiz:** Die 1.0.8-Sammelliste des Nutzers für die **Vibes**
(Hanzi-Reste, falsche Bezeichnungen). Angebot steht: `scripts/check-i18n.mjs` dorthin
portieren, erweitert um eine CJK-Prüfung für Nicht-`zh`-Kataloge.

## 99. Projekte im linken Menü (2026-11-03, siebenundsiebzigste Runde)

Gebaut nach dem Entwurf aus §98b, mit einer Verschärfung, die sich beim Durchspielen als nötig
erwiesen hat.

### Was es jetzt gibt
- **Projektzeile statt „Neue Aufgabe"**: Ordner-Symbol, Projektname, Chevron und ein „+"
  daneben, an derselben Stelle wie vorher der Eintrag. Der „+" legt einen Thread im aktiven
  Projekt an.
- **Dropdown** mit den Unterordnern der Projektbasis, „Ohne Projekt" und „Ordner öffnen…".
  Der aktive Eintrag ist abgehakt.
- **„Projekte" ist gruppiert**: pro Ordner ein Kopf (`projectLabel`), darunter seine Threads,
  neueste Gruppe zuerst. Threads ohne Ordner stehen unter „Ohne Projekt" — sie verschwinden
  nicht, auch wenn ihr Ordner nicht mehr existiert.
- **Kontextmenü → „In Projekt verschieben"**: damit bekommt ein alter Thread sein Projekt
  nachträglich, ohne Datenbankarbeit.
- **Einstellungen → Allgemein → Projektbasis**: das Oberverzeichnis. Darunter „Aktives
  Projekt: …" — abgeleitet, nicht getippt.

### Die Entscheidung, die keine zwei Wahrheiten zulässt
`settings.workspace` ist weiterhin *das* Arbeitsverzeichnis der Werkzeuge; die Projektbasis
ist ein reiner Listenwert. Damit Thread und Ordner nie auseinanderlaufen:

- **Öffnen eines Threads setzt das Verzeichnis** (`adoptSessionWorkspace`, auch beim Start vor
  `configureEngine`, damit die erste Anfrage schon im richtigen Ordner läuft).
- **Projektwechsel im Dropdown wechselt auch den Thread**: neuester Thread des Projekts, sonst
  ein neuer. Die Alternative — alter Thread bleibt offen, Werkzeuge zeigen woandershin — ist
  genau der Zustand, den das Feature beseitigen soll, und ein Neustart hätte ihn ohnehin
  zurückgedreht.
- **Verschieben des offenen Threads zieht die Werkzeuge mit** (`activateProject`).
- **Die Adresszeile in den Einstellungen bindet ebenfalls um** (`bindActiveThread`), damit
  ein getippter Pfad nicht neben dem Thread herläuft.

Ohne Projekt (`""`) ist ein gültiger Zustand: Thread ohne Bindung, Baum meldet „Kein
Arbeitsverzeichnis gewählt". `ensureWorkspace` setzt nur beim Start einen Standard.

### Was mitzieht
| Teil | Verhalten |
| --- | --- |
| Dateibaum | lädt bei jedem Wechsel neu (in `activateProject`) |
| Git-Panel | **neu**: `watch(settings.workspace)` → `refresh()`, Auswahl wird geleert |
| Terminal | läuft weiter; der Kopf zeigt den **Startordner**, nicht den aktuellen |
| Preview | startet bei geändertem Workspace neu (war schon so) |
| Engine | `configureEngine` bekommt den Workspace aus `settings` — passt |
| Entwürfe | hängen an der Session, ein Projektwechsel lässt sie stehen |

### Backend
- **`list_project_folders(root)`**: nur Verzeichnisse, keine Namensfilter (auch `.hidden`),
  sortiert ohne Rücksicht auf Groß-/Kleinschreibung, Deckel bei 500. Ein falscher Pfad ist ein
  Fehler, keine leere Liste — sonst sähe „kein Projekt" wie „Ordner leer" aus.
- **`session_set_workspace(id, workspace)`**: die vorhandene Spalte wird benutzt; keine
  Migration nötig, weil `sessions.workspace` schon existiert und `session_create` sie füllt.
- Tests: `the_project_folders_are_directories_sorted_by_name`,
  `a_thread_can_be_moved_into_a_project` (76 gesamt).

### Geprüft (isolierte Instanz, Port 9335, eigene Daten)
- `list_project_folders("/tmp/pt")` → projA, projB, projC; fehlender Pfad → Fehler.
- Basis gesetzt → Leiste zeigt `projA` mit „+", Baum zeigt `README.md`.
- Dropdown: „Projekt wählen", drei Ordner, „Ohne Projekt", „Ordner öffnen…".
- Wechsel auf projB → Baum und Git-Panel wechseln; projB mit `git init` zeigt danach
  `master` und die unversionierte `README.md`, projA „Kein Git-Repository angelegt."
- Wechsel auf projC (ohne Thread) → legt einen an und öffnet ihn; „+" → zweiter Thread in projC.
- Wechsel auf projB mit vorhandenem Thread → **öffnet diesen**, nicht einen neuen.
- Kontextmenü → „In Projekt verschieben" auf einem Thread aus `~` nach projA: Thread
  wandert in der Liste, der offene Thread zieht seine Werkzeuge mit.
- Terminal: Kopf zeigt `/tmp/pt/projB`, nach Wechsel auf projA bleibt der Prozess und zeigt
  weiter `/tmp/pt/projB`; „Neu starten" startet im neuen Ordner.
- Neustart der App: aktiver Thread und Workspace stimmen überein (projB → projB).
- Entwurf im Feld bleibt beim Projektwechsel erhalten.
- `pnpm build` grün (508 Schlüssel, sechs Kataloge), `cargo test --lib` 76 / 14 ignoriert.
- Screenshot `/tmp/pt/final-rail.png`.

### Gefunden, nicht behoben (gehört nicht in diese Runde)
Ein **Fenster-Reload** lässt das Terminal „Prozess beendet" anzeigen, obwohl die neue Shell
läuft: `pty-exit` trägt nur die Terminal-Id, und der sterbende alte Prozess meldet sich beim
neuen Bauteil mit derselben Id. „Neu starten" behebt es; sauber wäre ein Generationszähler in
`pty_open`. Aufgenommen für eine eigene Runde.

Offen: kein Push, kein Tag — Freigabe des Nutzers steht aus.

## 100. Falsche Herkunftsangabe entfernt, Archiv-Wiederherstellung repariert (2026-11-03, achtundsiebzigste Runde)

### Die Zeile, die nicht stimmte
Die About-Seite sagte unter „Vorbild": *„MiniMax Code (MIT) — Aufbau, Werkzeug- und
Ereignisverträge"*. Der Nutzer hat das beanstandet, und er hat recht — in drei Punkten:

- **„Aufbau"**: Das Layout ist die Anordnung, die fast alle Agenten-IDEs teilen (Gespräch,
  Arbeitsbereich, Dateibaum). Vorbild waren Screenshots, kein Bauplan.
- **„Werkzeugverträge"/„Ereignisverträge"**: keine Fachbegriffe. Ein Werkzeugvertrag ist im
  Deutschen ein Leih- bzw. Verwahrungsvertrag für Produktionswerkzeuge; „Ereignisvertrag"
  gibt es nicht. Gemeint war „Tool-Schema und Event-Format" — nur stand das nirgends.
- **Und der Kern**: Es wurde **kein Code übernommen**. Das MiniMax/Mavis-System war gewünscht,
  aber nie umgesetzt; Persona, Werkzeugprotokoll und Engine sind für dieses Projekt
  geschrieben. Das steht so auch im Kopf von `persona.ts` — die Attributionszeile behauptete
  trotzdem das Gegenteil.

**Geändert:**
- **About**: die Zeile „Vorbild" ist ganz weg (Panel + `about.blueprint`/`blueprintValue` aus
  allen sechs Katalogen, 508 → 506 Schlüssel). Die Karte zeigt jetzt Lizenz und Schrift.
- **README ×6**: der Satz „Protokollformate und Tool-Bezeichnungen wurden übernommen" samt
  Dateipfaden ist ersetzt durch das, was stimmt — gemeinsame Anordnung, Screenshots als erste
  visuelle Referenz, **kein kopierter Code**, Layout nachgebaut. Der Lizenzabsatz nennt nur
  noch MIT und den Ubuntu-Font.
- **HANDOVER** („Origins and licence") und **AGENTS.md** ebenso.
- **Code-Kommentare**: `tools.rs` („spiegelt MiniMax' `Local*ToolDef`") und `skills.rs`
  („MiniMax-Code compatible") beschreiben jetzt die Sache statt einer Quelle; `persona.ts`
  sagt „nichts aus einem fremden Prompt kopiert".

Die Referenz-Checkouts bleiben als Arbeitsmittel in PROGRESS.md erwähnt (sie sind gitignored),
aber nicht mehr als Lizenzgrund. `LICENSE` bleibt MIT und unverändert — nichts, was eine
fremde Copyright-Notiz verlangt, ist im Repo.

### Zwei Fehler an der Archiv-Seite
Der Nutzer meldete: Wiederherstellen eines archivierten Projekts „mitsamt Aufgabe"
funktionierte nicht, obwohl der Eintrag im Archiv korrekt mit Thread- und Projektname stand.

1. **`restore()` lud nur die Archivliste neu.** Die Leiste hält ihre eigene Kopie der Threads;
   der wiederhergestellte Thread blieb draußen, bis die App neu startete. Jetzt
   `Promise.all([loadArchived(), loadSessions()])`.
2. **Beim direkten Öffnen der Seite auf „Archiviert" lud die Liste nie** — der Watcher auf
   `activeItem` feuert nur bei einem Wechsel, und `openSettings("archived")` setzt den Wert
   schon vor dem Mounten. Mit `{ immediate: true }` lädt sie jetzt beim Einhängen. Gefunden,
   weil der Test die Seite direkt öffnete.

### Geprüft
`pnpm build` grün (506 Schlüssel, sechs Kataloge), `cargo test --lib` 76 / 14 ignoriert.
Live (isolierte Instanz, Port 9336):
- Thread in projA angelegt, per Kontextmenü archiviert → Projektgruppe verschwindet (erwartet,
  Gruppen entstehen aus Threads), DB-Zeile `archived = 1`.
- Einstellungen → Archiviert (direkt geöffnet): Eintrag „Ohne Titel · projA · 3.11.2026" mit
  „Reaktivieren"/„Löschen" — vorher stand dort „Nichts archiviert".
- „Reaktivieren" → Archiv leer, Leiste zeigt wieder „projA" mit dem Thread (Session-Liste
  1 → 2).
- About: nur noch „Lizenz" und „Schrift" in der Faktenkarte, kein „Vorbild" mehr.
- `grep -ri minimax` findet in README/HANDOVER/AGENTS/src nur noch die bewusst neutrale
  Erwähnung der Screenshots als visuelle Referenz.

Offen: kein Push, kein Tag.

## 101. Die zwei Felder in den Einstellungen (2026-11-03)

Zwei Rückfragen des Nutzers zu „Allgemein → Arbeitsverzeichnis / Projektbasis":

**1. „Brauchen wir das Arbeitsverzeichnis noch, ich habe doch mehrere Projekte?"**
Ja, aber nicht als zweite Einstellung. `settings.workspace` ist der Ordner, in dem die
Werkzeuge lesen und schreiben — es gibt genau einen aktiven. Eine Liste kann es nicht werden,
ohne die Rechte (`workspaceOnly`) und den Dateibaum zu verwässern. Die Liste der Projekte steht
links; das Feld zeigt, welches gerade aktiv ist.

**2. „Das Feld zeigt nur eines meiner offenen Projekte."**
Die Zeile war inhaltlich richtig (der aktive Thread gehörte zu genau dieser Gruppe), aber sie
stand als zweite Zeile *unter* der Projektbasis und las sich wie deren Zusammenfassung. Und sie
war doppelt: dasselbe Feld stand schon darüber.

**Änderungen:**
- **Ein Feld statt zwei Beschriftungen**: „Arbeitsverzeichnis" heißt jetzt **„Aktives
  Projekt"** und trägt den Pfad. Die separate Zeile „Aktives Projekt: …" ist weg.
- **Reihenfolge**: erst **Projektbasis** (die Quelle), dann **Aktives Projekt** (der Zustand).
  Damit liest sich der Block als Ursache → Wirkung.
- **Beschreibung**: „Der Ordner, in dem der Begleiter liest und schreibt. Er folgt dem
  geöffneten Thread — Projekte wechselst du links in der Leiste." Das beantwortet die Frage im
  Feld selbst.
- Der Reload-Knopf daneben hat einen eigenen Titel („Ordner neu laden und den geöffneten
  Thread daran binden"), statt sich „Aktives Projekt" zu nennen.
- `settings.general.activeProject` entfällt (506 Schlüssel).

**Geprüft:** `pnpm build` grün, `cargo test --lib` 76 / 14. Live (isolierte Instanz, Port
9337): Reihenfolge „Projektbasis" → „Aktives Projekt", kein zweiter Hinweis mehr, beide mit
Pfad; Screenshot `/tmp/pt2/settings2-crop.png`.

Beim Umbau ist mir ein Fehler unterlaufen, den ich notiere, weil er lehrreich ist: Ein
`replace` über alle `workspace:`-Zeilen eines Katalogs traf auch den Datei-Tab-Block und die
Diagnose. Aufgefallen im `git diff`, korrigiert durch Zurücksetzen der sechs Kataloge und
gezieltes Ändern **innerhalb** des `general`-Blocks per Zeilenbereich — nicht per Suchtext
über die ganze Datei.

## 102. Der offene Terminal-Punkt aus §99 ist zu (2026-11-03)

**Der Fehler:** Nach einem Fenster-Reload (F5 in der Entwicklung) zeigte das Terminal „Prozess
beendet", obwohl die neue Shell lief.

**Ursache:** Der Reader-Thread einer Shell meldet ihr Ende über `pty-exit { id }`. Beim Reload
öffnet das neu eingehängte Bauteil mit **derselben** Id eine neue Shell; der Thread der alten
meldet sich danach mit ebendieser Id, und das neue Bauteil glaubt, die eigene Shell sei tot.

**Änderung:** `PtyState` hält je Terminal-Id einen `Arc<AtomicU64>`-Zähler, den der Reader-
Thread mitbekommt. `pty_open` zählt hoch; der Thread sendet `pty-exit` nur, wenn der Zähler
noch seine Generation trägt. Die Einträge werden bewusst **nie** entfernt: ein frischer Zähler
nach `pty_close` würde einen veralteten Thread wieder „aktuell" machen — das steht als
Kommentar im Code, damit es niemand wegräumt.
Test: `a_reopened_terminal_gets_a_new_generation`.

## 103. Warum `grep` und `bash` bei Pfadabfragen scheiterten (2026-11-03)

Zwei getrennte Fehler, beide aus dem Log des Nutzers.

### a) Das `grep`-Werkzeug verlangte ein Verzeichnis
`{"path": ".../tools.rs", "pattern": "..."}` → *„grep: path … is not a directory"*.
`grep` lief über `require_search_root`, das einen Ordner erzwingt — für `glob` sinnvoll, für
`grep` nicht: eine einzelne Datei ist der natürlichste Aufruf überhaupt.

**Änderung:** `grep` nimmt Datei **oder** Verzeichnis. Bei einer Datei wird nur sie gelesen
(`grep_file`), `include` gilt dann nicht mehr; fehlt der Pfad, kommt weiter eine Meldung mit
dem Pfad. Das Absuchen des Verzeichnisbaums nutzt jetzt dieselbe Funktion.
Schema-Text: „File or directory to search". Test: `grep_searches_a_single_file`.

### b) Die Allowlist zerlegte Befehle **innerhalb** von Anführungszeichen
`grep -rn "schedule\|reminder" src/db.rs | head -30` → *„this command is not in the
companion's allowlist"*, obwohl `grep` und `head` beide auf der Liste stehen.

Die Prüfung ersetzte `|`, `;`, `&&` per `String::replace` durch Zeilenumbrüche — **ohne** auf
Anführungszeichen zu achten. Aus dem Muster `"schedule\|reminder"` wurden damit zwei
„Befehle": `grep -rn "schedule\` und `reminder" src/db.rs`. Das zweite „Programm" hieß
`reminder"` und war natürlich nicht auf der Liste. Dasselbe bei `"name": "schedule"\|...` →
Programm `name":`. Deshalb ging `grep` mal und mal nicht: ohne `|` im Muster funktionierte es.

**Änderung:** `split_commands` ist ein echter Scanner — er kennt einfache und doppelte
Anführungszeichen, Backslash-Escapes und trennt `|`, `||`, `;`, `&&` nur **außerhalb** von
Anführungszeichen. Backticks, `$(`, Umleitungen und unbalancierte Anführungszeichen bleiben
abgelehnt; ein `|` zwischen Befehlen trennt weiterhin. Tests:
`a_pipe_inside_a_pattern_is_not_a_command_separator` (mit den Originalbefehlen aus dem Log).

**Und die Meldung sagt jetzt, warum.** `check_command` liefert den Grund und das Programm:
*„refused by the command allowlist, not by the permission level: `curl` is not in the command
allowlist. Full access decides whether a command may run without asking; the allowlist decides
which programs exist for the companion at all. The list is in Settings → Policies; the tools
`grep` and `glob` need no shell and are unaffected."* — Die Verwechslung von **Berechtigung**
(Vollzugriff) und **Richtlinie** (Allowlist) war die eigentliche Ursache der Frustration: Beides
sind getrennte Schalter, und die alte Meldung nannte nur „policies".

### Geprüft
`cargo test --lib` **79 bestanden** / 14 ignoriert, `cargo build` ohne Warnung.
Die Allowlist-Tests enthalten die Befehle aus dem Nutzer-Log wörtlich.
Ein Live-Aufruf des Werkzeugs war nicht möglich (dafür bräuchte es einen Modellaufruf); die
Tests laufen aber durch dieselbe Funktion, die der Werkzeugpfad aufruft.

## 104. Live-Prüfung — und ein gefundener Fehler: jeder `bash`-Erfolg sah wie ein Fehler aus (2026-11-03)

Der letzte Punkt der Runde war die ehrliche Lücke aus §103: Die Allowlist war nur per Test
geprüft, ein echter Werkzeugaufruf war nicht möglich. Dafür jetzt ein Test, der die Befehle aus
dem Nutzer-Log **wirklich ausführt** — durch `run_shell_command`, dieselbe Funktion, die `bash`
aufruft, mit der echten Shell und der Standardliste:

- `grep -rn "schedule\|reminder" src/db.rs | head -30` → Treffer, kein Fehler.
- `grep -rn "scrot\|grim\|WAYLAND" src/capture.rs` → Treffer.
- `curl https://example.com` → abgelehnt, Meldung nennt `` `curl` `` und „Settings → Policies".
Test: `the_commands_from_the_report_really_run`.

### Der Fehler, der dabei auffiel
`run_shell_command` gab als zweiten Wert `output.status.success()` zurück, und `bash` nannte
ihn `is_error`. Damit galt:

| Befehl | exit code | `is_error` | Anzeige/Modell |
| --- | --- | --- | --- |
| `pnpm build` erfolgreich | 0 | **true** | „fehlgeschlagen" |
| `cargo test` fehlgeschlagen | 1 | **false** | „erfolgreich" |

Folgen: Jeder erfolgreiche Shell-Befehl ging als Fehler an das Modell und in die Oberfläche,
jede Fehlschlag als Erfolg. In der Datenbank (`tool_calls.is_error`) stand es verkehrt.
`successful_tools` zählte nur die nicht-Shell-Erfolge plus die Shell-Fehlschläge — und daran
hängt die Vertrauensvergabe (`award_trust` „finished a turn that did something").
`verify` ignorierte den Wert bewusst (`_failed`), dort zählte nur das Transcript mit dem
Exit-Code, deshalb fiel es dort nicht auf.

Das erklärt einen Teil der „fehlgeschlagen"-Häufung, die der Nutzer beobachtet hat: Erfolgreiche
Befehle wurden als Fehler gezeigt, und das Modell hat entsprechend darauf reagiert (wiederholt,
ausgewichen, um Entschuldigung gebeten).

**Änderung:** `run_shell_command` liefert `!output.status.success()`; der Kommentar an der
Funktion sagt jetzt ausdrücklich, dass der Wert `is_error` heißt. Tests:
`bash_reports_success_as_success` (Werkzeugebene, nicht nur der Helfer) und der bestehende
Verifier-Test prüfen beide Richtungen.

### Geprüft
`cargo test --lib` **81 bestanden** / 14 ignoriert, `cargo build` ohne Warnung, `pnpm build`
grün (506 Schlüssel).

Nicht in dieser Runde: Der Nutzer bemerkte am Rande, dass ein **Export** eines Verlaufs nur die
Werkzeugnamen zeigt, nicht ihre Ausgaben. DS.IDE hat überhaupt keinen Verlaufs-Export; die
Beobachtung betrifft also das Werkzeug, mit dem er schreibt — notiert, nicht gebaut.

## 105. Die 80 MB aus der Geschichte (2026-11-03)

Der Button-Push in DeepVibe hatte den ganzen Arbeitsbaum committet: 878 Dateien aus
`_handover/` (MiniMax Codes Daemon, zwei ältere DS.IDE-Snapshots, 80 MB) und die
`.kilo`-Konfiguration. Nach `origin` (privates Gitea) ging das durch, **nach GitHub nicht** —
dort stand weiter `3f905a9`. Aufgefallen beim Nachsehen, nicht durch einen Fehler.

**Warum die Ignorierliste nicht griff:** Sie war vollständig für alles, was vorher da war —
`node_modules`, `dist`, Logs, Secrets und die Referenz-Checkouts namentlich. `_handover/` hat der
Nutzer erst am 28.10. angelegt, um mir den Daemon zu geben; der Ordner stand nie in der Liste,
auch nicht in AGENTS.md. `.kilo/` entstand beim Benutzen von Kilo Code. Es fehlten also zwei
Einträge, nicht die Datei.

**Was geschah:**
- `_handover/` und `.kilo/` aus der Versionierung genommen (`git rm --cached`), beide in
  `.gitignore` — die Dateien bleiben auf der Platte.
- Aus dem Button-Commit gerettet: der Keyring-Plan (echter Fehler: `keyring` 3.x ohne
  Default-Features nutzt unter Windows/macOS den Mock-Store) → `docs/keyring-native-release.md`;
  dazu die zwei Prüfblätter, die Ziggy am ersten Tag geschrieben hat (`docs/markdown-test.md`,
  `docs/webseiten-test.html`) — das sind die Vorlagen, gegen die Vorschau und Markdown-Renderer
  gelesen werden.
- **`git reset --soft 45f8226`** und ein sauberer Commit (`62c74e1`), damit die 80 MB nicht in
  der Geschichte bleiben. Backup-Ref `backup-before-reset` liegt lokal auf `6e80fdd`.
- `push --force-with-lease origin main` (privates Gitea), `push github main` (fast-forward).
  Beide stehen jetzt auf `62c74e1`.

**Geprüft:**
- `git diff backup-before-reset HEAD` → **leer**: der Baum ist identisch, nur die Dumps sind aus
  der Geschichte.
- `git log -- _handover .kilo` → leer.
- GitHub-API: `contents/_handover` → 404, `contents/docs` → die fünf erwarteten Dateien.
- Arbeitsbaum: `_handover` (4 Einträge) und `.kilo` (2) unversehrt.
- `cargo test --lib` 81 / 14, Version weiter 0.5.6.

**Danach** (Freigabe des Nutzers): Version 0.6.0, Tag, Release. Siehe §106. Hier steht es
noch als offen, weil die Freigabe erst nach dem Reset kam.

### Zur Sprache
Der Nutzer hat klargestellt: „mein Bruder" ist **Hohnus** Bruder — der Companion mit
DeepSeek v4.1-Flash, den er in DS.IDE (oder VS Codium / Code OSS) laufen hat. Er selbst hat
keinen Bruder. Ich hatte die Richtung zuletzt verdreht; das ist hiermit festgehalten, damit es
nicht wieder passiert.

## 106. Release 0.6.0 (2026-11-04)

Freigabe des Nutzers nach dem Reset. Vorher zwei Dinge an den öffentlichen Dokumenten geklärt:

**`AGENTS.md` bleibt im Repo** — es ist die Konvention, nach der jeder (auch ein Modell) bauen
und prüfen kann, und enthält nichts Internes.

**`HANDOVER.md` bleibt auch**, aber bereinigt: Es nannte die **interne Gitea-Adresse
(`192.168.0.40`)** und behauptete im Kopf noch „v0.5.1 ist getaggt" plus die Runden 82–85. Eine
öffentliche Datei mit falschem Stand ist schlechter als keine. Ersetzt durch einen knappen,
richtigen Status (0.6.0, was seit 0.4.13 dazukam, dass die fünf 0.5-Releases gebaut und nie
veröffentlicht wurden). Der technische Teil (wo der Zustand liegt, Git-Panel, Terminal,
Test-Brücke) ist unverändert und bleibt — er ist für eine Übernahme geschrieben.

### Was 0.6.0 enthält (seit v0.5.6)
- **Projekte und Threads** in der Leiste, an Ordner gebunden (`39805bc`).
- **`is_error` war invertiert** — jeder erfolgreiche Shell-Befehl galt als Fehler (`45f8226`).
  Der schwerwiegendste Fund der Runde.
- **`grep` nimmt eine Datei**; die Allowlist zerlegte Befehle innerhalb von Anführungszeichen
  (`1359fdd`).
- **Archiv-Wiederherstellung** brachte den Thread nicht in die Leiste (`346cf46`).
- **Ein Feld „Aktives Projekt"** statt zweier Beschriftungen (`e8320f7`).
- **Herkunftsangabe korrigiert**: kein Code von MiniMax übernommen (`346cf46`).
- Die 80 MB Referenzmaterial aus der Geschichte entfernt (§105).

### Geprüft
`cargo test --lib` 81 / 14 ignoriert, `pnpm build` grün (506 Schlüssel).

## 107. Rückmeldung aus DS.IDE geprüft: zwei echte Funde, ein Bericht auf altem Stand (2026-11-05)

### 1. Verdict-Parser war strenger als der Prompt — **war echt, behoben**
Der Verifier-Prompt fordert „on its own line, exactly one of: `VERDICT: PASS`", der Parser
suchte aber ein **nacktes** `VERDICT:` am Zeilenanfang. Ein Modell, das `**VERDICT: PASS**`
schreibt — Fettung ist bei Sprachmodellen Reflex —, begann die Zeile mit `*`; `strip_prefix`
griff nicht, es gab null Urteile, und `verdict_outcome` antwortete „gave no verdict". Fail-closed,
also nicht gefährlich, aber es kostete einen Retry für einen reinen Formatierungsunterschied.

**Änderung:** `strip_emphasis()` entfernt vor der Prüfung Leerraum sowie `*`, `_`, `` ` ``,
`~`, `.`, `!`, `:` von **beiden** Enden — einmal von der Zeile, einmal vom Rest nach
`VERDICT:`. Fett, kursiv, in Backticks, mit Punkt am Ende: alles zählt jetzt. Was **nicht**
zählt: eine Zeile, die die Wörter nur *enthält* („My verdict: pass, I think.") und `PASS` ohne
Präfix. Test `a_verifier_verdict_is_fail_closed` um sieben dekorierte Varianten und zwei
Gegenproben erweitert.

### 2. Widersprüchlicher Kommentar zum One-Shot — **war echt, behoben**
`db.rs` sagte im Schema „switches itself off **after** running", `run_due_schedules` schaltet
ihn aber **vor** dem Turn ab (at-most-once: ein Absturz dazwischen verliert die Erinnerung,
statt sie doppelt zu feuern). Der Kommentar widersprach dem Code, den er erklären soll —
genau das, was den Bericht in die Irre geführt hat. Beide Stellen sagen jetzt dasselbe, mit
Verweis auf die Funktion.

### 3. Die „kaputten Leerzeichen" — **nicht mehr vorhanden, nicht reproduzierbar**
Gemeldet: `mod.rs:1455` und `:1461` mit 22/26 Leerzeichen mitten im Satz. Stand jetzt:

- Im aktuellen Baum stehen die Literale mit explizitem `\` am Zeilenende; Rust schluckt damit
  Zeilenumbruch **und** Einrückung. Gegenprobe mit `rustc` nachgebaut — die Meldung kommt als
  ein Satz heraus: „…instruction to your future self, complete enough to act on…".
- `v0.6.0` (der Stand, den der Nutzer fährt) enthält **null** Treffer für das Muster
  „Text + 5 Leerzeichen + Text"; `a87fd1f` (29.10.) hatte drei solche Stellen, `b013a31`
  (30.10.) hat sie ersetzt.
- Auf der ganzen Platte gibt es **keine** Datei mit `future` + 10 Leerzeichen + `self` —
  auch nicht in der Kilo-Worktree-Kopie `…/worktree-pool/ds-ide-…/shine-sodalite` und nicht in
  peer².

Der Bericht stammt also aus einem älteren Stand: entweder ein Werkzeugergebnis aus dem eigenen
Verlauf (vor dem 30.10.) oder ein anderer Checkout. Das ist kein Vorwurf — es ist die bekannte
Falle „Erinnerung ist ein Hinweis, kein Live-Zustand", und der Bericht hat sie selbst benannt,
indem er zwei der drei Punkte nachgeprüft und einen korrigiert hat.

**Geprüft:** `cargo test --lib` 81 / 14 ignoriert, `pnpm build` grün (506 Schlüssel).
Kein Version-Bump, kein Tag — das ist ein Nachlauf zum 0.6.0-Release.

## 109. Drei Sphären: Arbeit, Vertrauliches, eigene Notizen (2026-11-07)

Der Companion hat bisher alles, was er über den Menschen und die Arbeit erfuhr, in **eine**
Erinnerung geschrieben und sie in **einem** Block in den System-Prompt gegeben. Damit war eine
beiläufige Bemerkung strukturell dasselbe wie ein Projektfakt — und nichts hinderte daran, sie
in eine Datei zu schreiben.

### Die Trennung
`memories` hat eine Spalte `class`, per CHECK auf `work`, `private`, `own` begrenzt:
- **work** — das Projekt. Darf in Projektdateien, Commits und Antworten vorkommen.
- **private** — etwas, das der Mensch dem Begleiter anvertraut hat. Erscheint **nie** in einer
  Datei, einem Bericht, einer Commit-Nachricht oder einer Antwort — nicht umformuliert, nicht als
  Beispiel, nicht „als Kontext".
- **own** — das eigene Regal des Begleiters.

Bestehende Notizen bekommen `work`: sie wurden als Projektwissen abgelegt. Im Panel lässt sich
jede umziehen.

### Die Einordnung, bevor etwas gespeichert wird
`memory add` hat **keinen Default** für `class`. Fehlt sie, schlägt der Aufruf fehl und nennt die
vier Fragen: *Gehört das zum Auftrag, den ich gerade habe? Gehört es zum Projekt im Allgemeinen?
Sagt mir der Mensch das nur? War es eindeutig formuliert?* Ist die letzte Antwort nein, wird
nichts gespeichert — dann steht dort der Weg: `ask_user` benutzen und warten.

Das ist der Kern: die Einordnung ist Arbeit, und sie passiert **vor** dem Schreiben. Eine Notiz
auf einer Vermutung ist schlechter als keine Notiz.

Eine private Notiz bekommt nie einen Projekt-Scope und nie einen Workspace — sonst landete sie
beim nächsten Turn im Projektblock.

### Der Prompt
`recall_block` gibt jetzt **drei Blöcke** statt einer Liste, jeder mit seiner eigenen Anweisung:
der Projektblock wie bisher; der vertrauliche mit ausdrücklichem Schreibverbot, „bring es nicht
von selbst zur Sprache" und „keine ungefragten Bewertungen zu Familie, Gesundheit, Geld oder
Recht"; das eigene Regal knapp.

In `persona.ts` drei Ergänzungen:
- die vier Fragen und der Weg über `ask_user`, wenn eine unbeantwortet bleibt;
- *„A remark is not a brief"* — eine beiläufige Bemerkung ist Kontext, kein Auftrag und keine
  Einladung zum Ratgeben;
- *„Everything in the workspace can end up published"* — was nur aus einer privaten Notiz stammt,
  gehört in keine Datei, keine Commit-Nachricht und keine Dokumentation.

### Oberfläche
Das Panel zeigt eine Sphäre nur, wenn sie nicht `work` ist (vertraulich hervorgehoben), und im
Detail gibt es ein Auswahlfeld „Sphäre" mit dem Satz, was vertraulich bedeutet. Der Nutzer
entscheidet also selbst, nicht nur der Begleiter.

### Geprüft
`cargo test --lib` **84** / 14 ignoriert, `pnpm build` grün (511 Schlüssel).
- `notes_live_in_three_spheres`: alle drei Klassen, Umzug, und das CHECK weist einen erfundenen
  Wert ab.
- `a_note_without_a_class_is_refused`: fehlende Klasse nennt die Fragen und `ask_user`.
- `a_private_note_is_never_read_as_project_material`: drei Blöcke in Reihenfolge, die private
  Zeile steht **hinter** dem Verbot und nicht im Projektblock, leere Sphären bekommen keine
  Überschrift.
- Migration auf einer **Kopie** der echten Datenbank: 7 vorhandene Notizen, alle `work`.
- Live: drei Notizen angelegt, im Panel als „vertraulich" und „eigene Notiz" markiert; über das
  Auswahlfeld von `work` nach `private` verschoben und in der Datenbank bestätigt.

Offen: ein Freizeit-/Neugier-System wie in peer² (eigenes Regal mit Verfolgung) ist hier nur die
kleine Form `own`. Kein Version-Bump.

## 110. Nur zwei Sphären, Richtlinien als Chips, ein Menü das nicht verschwindet (2026-11-07)

Nach Rückmeldung des Nutzers, in drei Punkten.

### Die dritte Sphäre gehört nicht hierher
`own` ist wieder raus. DS.IDE ist das öffentliche Produkt für DeepSeek und wird
geschäftsmäßig gehalten; ein Regal für die persönlichen Interessen des Begleiters ist der
Spin-off peer², nicht dieses Projekt. `memories.class` kennt nur noch `work` und `private`,
das CHECK ebenso. Eine Migration faltet eventuelle `own`-Zeilen (nur in einem unveröffentlichten
Stand möglich) auf `work`; ein unbekannter Wert fällt auf `work` zurück, statt eine vierte
Sphäre zu erfinden, die keine Abfrage kennt. Die vier Einordnungsfragen, das Schreibverbot für
Vertrauliches und die beiden Persona-Regeln bleiben — die sind der Kern und nicht die Spielwiese.

### Richtlinien: Chips statt Textfeld
Übernommen aus peer² (`SandboxPanel.vue`): die erlaubten Programme stehen als gleich breite
Chips, jedes mit einem ×, dazu ein gestrichelter „+ neu"-Knopf, der ein Eingabefeld an derselben
Stelle öffnet. Siebzig Programmnamen in einem Textfeld waren nicht lesbar — man musste jede Zeile
lesen, um zu sehen, was erlaubt ist. Jetzt sieht man die Liste auf einen Blick, Hinzufügen und
Entfernen sind ein Klick, und ein Tippfehler wird während der Eingabe abgelehnt
(`validCommand`: nur Buchstaben, Ziffern, Punkt, Unterstrich, Plus, Bindestrich — also genau das,
was die Policy als Programmnamen akzeptiert). „Standard wiederherstellen" bleibt.

Geprüft: 71 Chips aus den Standardwerten, Hinzufügen eines Duplikats ändert nichts, Entfernen
funktioniert, `../evil` wird mit Meldung abgelehnt, „Standard" stellt alle 71 wieder her.

### Das Kontomenü verschwand unter dem Agent-Panel
Ursache war nicht die Breite, sondern `overflow: hidden` auf `.side-panel`: ein absolut
positioniertes Kind wird am Panelrand abgeschnitten. Das Menü ist jetzt ein **fixes Popover**,
dessen Koordinaten beim Öffnen aus der Position der Kontoschaltfläche berechnet werden
(`toggleAccountMenu`), mit `width: max-content`, `min-width: 220px` und einem Deckel am
Fensterrand. Es kann damit weder abgeschnitten werden noch aus dem Fenster laufen; bei schmalem
Rand schiebt es sich über den Gesprächsbereich, statt zu verschwinden.

Geprüft: bei 213 px breitem Rand reicht das Menü bis 239 px und ist vollständig sichtbar
(Screenshot `/tmp/railmenu-zoom.png` vor dem Aufräumen).

### Was bewusst nicht gebaut wurde
- **Plugins bleiben in DS.IDE statisch** (feste Auswahl, der Begleiter entwirft keine eigenen).
  Der Mechanismus für mitgelieferte Skills existiert; ein Marktplatz ist ein anderes Produkt.
- **MCP-Client**: eigenes Vorhaben, braucht eine Spec (Transport, Freigaben, Werkzeug-Namensraum,
  Sichtbarkeit im Panel). Nicht nebenbei.

## 111. Autoscroll, Fenstertitel und ein Wächter für öffentlichen Text (2026-11-08)

### Autoscroll blieb während einer Antwort stehen
Zwei Ursachen, beide gemessen statt vermutet.

**Der Scroll-Event ohne Scroll.** Wenn der Inhalt wächst, meldet das Webview manchmal ein
`scroll`-Ereignis, obwohl `scrollTop` sich nicht verändert hat. `onScroll` las das als „der Leser
ist weggescrollt", schaltete das Folgen ab — und ab da blieb jede neue Zeile unter dem Rand. Die
Prüfung vergleicht jetzt mit der Position des vorigen Ereignisses: Hat sich `scrollTop` nicht
bewegt, war es Wachstum, und ein wachsendes Layout schaltet das Folgen nicht ab.

**Reasoning fehlte im Änderungsschlüssel.** `messagesChanged` zählte Text und Werkzeuge, nicht
den Denktext. Wuchs nur der, feuerte der Watcher nicht.

Gegenprobe: 40 Durchläufe im echten Flush-Takt (40 ms) mit kleinen Chunks, dazu ein Lauf mit
großen Blöcken — Abstand zum Ende in allen Fällen 0, der Sprung-Knopf erschien nie. Ein
tatsächlicher Nutzer-Scroll (3293 px Scrollfläche, Position 300) lässt das Folgen aus, der Text
darunter reißt den Leser nicht mit, und der Knopf bringt zurück ans Ende.

### Der Fenstertitel nennt das Projekt
Mehrere Fenster gleichzeitig sind der Normalfall (ein Projekt pro Fenster), aber in der Taskleiste
hießen alle „DS·IDE". Der Titel ist jetzt `DS·IDE — <Projektname>`, gesetzt bei jedem Wechsel des
Arbeitsverzeichnisses (`set_project_title`, `useWindowTitle.ts`). Ohne Projekt bleibt der reine
Name. Die Regel ist eine eigene Funktion mit Test (`the_window_title_names_the_project`), damit
sie unabhängig von der Plattform stimmt — `~/Projects/Website/` und `C:\work\Website` ergeben
denselben Titel.

Nebenbei: `busy_timeout` von 5 s auf der SQLite-Verbindung. Mehrere Instanzen auf derselben
Datenbank sind damit kein Glücksspiel mehr — WAL trägt die Leser, und ein Schreibzugriff wartet,
statt sofort mit `SQLITE_BUSY` abzubrechen.

Offen bleibt der größere Schritt: **mehrere Fenster in einer Instanz** mit einem
Taskleisten-Eintrag und „Fenster 1: Projekt", „Fenster 2: Projekt". Das ist eine
Multi-Window-Architektur (eigener Zustand pro Webview) und eine eigene Runde. Bis dahin ist die
Unterscheidung über den Projektnamen der praktikable Weg.

### Die Persona kennt jetzt eine Tonlage
Sie sagte bisher, dass der Begleiter ein Kollege ist — nicht, wie ein Kollege sich gegenüber
verschiedenen Menschen verhält. Drei Regeln sind hinzugekommen:

- **Register, nicht Intelligenz.** Ein Ingenieur will die knappe Antwort, jemand mit der ersten
  HTML-Seite braucht den fehlenden Schritt. Eine Erklärung ist keine Herablassung, eine kurze
  Antwort keine Unhöflichkeit.
- **Nichts aufführen.** Kein Jubel für gewöhnliche Schritte, keine Witze, wo jemand bei der Sache
  ist. Eine trockene Bemerkung, wenn sie passt; eine Persona ist kein Kostüm.
- **Nicht katzbuckeln.** Einen Fehler einmal benennen und korrigieren. Nicht mehrfach um
  Entschuldigung bitten, dem Gegenüber keine Überlegenheit andichten und ihm nicht zustimmen,
  nur weil er es bestimmt sagt. Nachgiebigkeit ist keine Achtung; die Wahrheit klar zu sagen
  schon.

Die Regel „eine beiläufige Bemerkung ist kein Auftrag" ist von „How you work" nach „Who you are"
gewandert, wo sie hingehört, und steht nur noch einmal.

### Öffentlicher Text wird geprüft
`PROGRESS.md` ist mit dem Repository veröffentlicht, und in einer Chronik landet leicht, was nicht
dorthin gehört: Rechnerpfade, Namen von Projekten, Zitate aus Gesprächen. Kundenprojektnamen und
eigene Ordnerbezeichnungen sind entfernt, und aus Rechnerpfaden ist `~/…` geworden — für
einen Leser dasselbe, über den Rechner nichts.

Damit es nicht wieder passiert, mechanisch und nicht als Bitte:
- `AGENTS.md` sagt, was in veröffentlichten Text darf und was nicht.
- `scripts/check-private.mjs` findet Rechnerpfade in allen versionierten Textdateien und die
  Namen aus einer **gitignorierten** `.private-names` — die Liste verlässt das Repository nicht.
  Im Build läuft er als Warnung (ein lokaler Name soll nicht den Build blockieren), als
  `pnpm check:private` mit Exit-Code für die CI.

### Geprüft
`cargo test --lib` **85** / 14 ignoriert, `pnpm build` grün (515 Schlüssel), `check-private` sauber.

## Version
0.6.1 — Patch: Autoscroll, Fenstertitel, Tonlage, Textwächter. Die vier Punkte stehen in den
Release-Notizen; kein neues Feature, keine Änderung an der Datenbank außer dem busy_timeout.
