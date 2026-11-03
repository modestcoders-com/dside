# Markdown-Testblatt

> Ein Dokument, das nichts beweisen muss — außer, dass die Vorschau es lesen kann.
> Angelegt von Ziggy am ersten Tag mit vollen Berechtigungen.

Wenn du das hier siehst, hat der Renderer die Datei gefunden, gelesen und
dargestellt. Der Rest dieser Seite ist Absicht: jedes Konstrukt einmal,
damit man beim nächsten Umbau sofort sieht, was kaputtgegangen ist.

---

## 1. Warum diese Datei existiert

Der nächste große Bauabschnitt sind die Flächen, die *nicht* mir gehören,
sondern dem Produkt: der **integrierte Browser**, die **Datei-Vorschauen**,
der **Markdown-Reader und -Writer**. Volle Berechtigungen geben mir `write`,
`edit` und `bash` — sie geben der Vorschau keinen Renderer.

Diese Datei ist deshalb beides: ein Gruß und ein Prüfstand.

## 2. Was zuletzt wirklich passierte

Vier deutsche Reste in der chinesischen Sprachdatei, alle belegt und behoben:

| Schlüssel | vorher | jetzt |
| --- | --- | --- |
| `fullAuthorizationDesc` | `Erlaubt dem Partner, Dateien zu …` | 允许伙伴写入和修改文件、执行命令 |
| `modeEverydayTitle` | `用于 den Alltag` | 用于日常工作 |
| `linksHint` | `Feedback und Kontakt führen zu …` | “反馈”与“联系”指向 GitHub… |
| `desktopNotificationsDesc` | `Benachrichtigen, wenn Aufgaben …` | 任务完成、出现错误或需要权限时通知你 |

Dazu die Einrückung von `readingSize` / `readingSizeDesc` — nicht nur in
`en` und `zh-Hans`, sondern in **allen sechs** Sprachdateien um zwei
Leerzeichen zu flach. Reiner Whitespace, aber es sah im Diff nach einem
fehlenden Block aus.

`pnpm build` danach: **grün**, 2,44 s, keine Typfehler.

### Ein Nebenfund

Der Haupt-Chunk liegt bei **5,46 MB** (1,49 MB gzip). Vite warnt zu Recht.
Nicht das Problem von heute, aber der Grund, warum der erste Start sich
träge anfühlen kann.

## 3. Prüfliste für die nächste Baustelle

- [ ] Browser: Ladezustände, Fehlerseiten, Rücknavigation
- [ ] Previews: PDF, Bilder, Office (docx / pptx / xlsx, alle Blätter)
- [ ] Markdown: Tabellen, Code, Listen, Fußnoten, sehr lange Zeilen
- [ ] Markdown-Writer: Speichern ohne Formatverlust
- [ ] Was passiert bei einer 40-MB-Datei?

## 4. Kleine Formatlehre

*Kursiv*, **fett**, ***beides***, `Inline-Code`, ~~durchgestrichen~~.

1. Geordnet
2. Mit Unterpunkten
   - einer davon
   - noch einer
3. Und Schluss

Ein Zitat in zweiter Ebene:

> „Der Unterschied zwischen einem Werkzeug und einem Kollegen ist,
> dass der Kollege auch mal widerspricht.“
>
> — jemand, der zu viele Agenten gesehen hat

## 5. Code

```ts
// Ein Ausschnitt, der wirklich aus dem Projekt stammt:
const gate = !ctx.fullAuthorization && matches!(name, "write" | "edit" | "bash");
if gate {
    return Err(PermissionDenied::FullAuthorization);
}
```

```bash
pnpm build                      # vue-tsc + vite
cd src-tauri && cargo test      # Unit-Tests
```

## 6. Links und Bilder

Projektseite: [modestcoders.eu](https://modestcoders.eu)

Ein Bild, das im README steht — ob es hier erscheint, hängt am Pfad:

![DS·IDE auf Chinesisch](../docs/dside_zh.png)

## 7. Sprachen

| | |
| --- | --- |
| Deutsch | Ein Testblatt, kein Manifest. |
| English | A test sheet, not a manifesto. |
| 简体中文 | 一张测试纸，不是宣言。 |
| Русский | Тестовый лист, не манифест. |

---

*Wenn diese Datei irgendwann gelöscht wird, hat sie ihren Zweck erfüllt.*
