# Spec: selectable plugins that may carry code

## Goal

Let the user add capabilities DS·IDE does not ship, without turning the app into an execution
surface nobody can audit. A plugin is a folder with a manifest, optional notes and an optional
program. Plugins are opt-in per project, listed in the panel, and removable at any time.

## Decided

- **Plugins may carry code.** Therefore they are in the same class as MCP servers: foreign code
  acting on the user's machine. Every call passes the permission model.
- **Built-in skills stay built-in.** They are what makes DS·IDE DS·IDE; they are not plugins and
  are not moved into the plugin mechanism.
- **Deselecting works immediately.** Disabling a plugin removes its tools from the offer, drops any
  "allowed for this session" grant, and leaves its files on disk. Off is not uninstall.
- **No store in the first version.** A store for code-executing content means review, signing and
  recall, and a promise about liability. With no users that is overhead pretending to be a
  feature. Plugins come from a folder.

## The shape

One mechanism instead of two. A plugin is:

```
<plugin root>/<slug>/
  plugin.json          manifest
  SERVER.md            optional notes the companion reads when the plugin is enabled
  server               optional executable (any language), speaks JSON-RPC over stdio
```

`plugin.json`:

```json
{
  "id": "release-helper",
  "name": "Release helper",
  "version": "1.0.0",
  "author": "someone",
  "description": "One sentence for the panel.",
  "entry": "server",
  "tools": ["bump_version", "changelog"]
}
```

The optional `server` speaks the **same** JSON-RPC over stdio that the MCP client speaks
(`docs/specs/mcp-client.md`): `initialize`, `tools/list`, `tools/call`. That is the reason this
shape wins — one transport, one permission gate, one way to report a failure. A plugin without
`entry` is text only (`SERVER.md`), which is what today's skills are; it needs no code path of its
own.

**Why a subprocess and not JavaScript in the webview:** the webview holds the Tauri IPC handle. A
plugin running there could call every `invoke` the app offers, including `settings_set` and the
secret store. A subprocess has no such handle; it can do only what the permission model lets its
tool calls do. WASM is the safer-looking option but forces every author into one toolchain for the
sake of a sandbox we do not claim to provide — the honest boundary is the same one `bash` has.

## Where plugins live

- Default root: the app data directory, `plugins/` next to the database.
- The panel shows the root and has a button to open it in the file manager; a second root can be
  named in settings for a shared or version-controlled folder. One root per installation, not one
  per project — a project selects *which* plugins are on, not *where* they come from.
- Discovery reads `<root>/*/plugin.json`, tolerates broken manifests (skip and log), and never
  executes anything during discovery.

## Naming

`plugin__<id>__<tool>`, normalised to `[a-z0-9_]` — the same shape and the same reason as the MCP
prefix: an external tool must be recognisable at a glance, because the app's own contract does not
cover what a stranger's program does. Without a prefix a plugin could call a tool `bash` and shadow
the built-in one.

The split is done against the known plugin ids, not on the `__` separator, so a tool whose own name
contains an underscore stays unambiguous. The panel shows the resulting name, so it is not a guess.

## Permissions and trust

- `entry` is executed only after the user enables the plugin in that project; enabling is an
  explicit click in the panel, and the panel shows id, version, author, the tool names and the
  absolute path.
- A tool call from a plugin goes through the same gate as `bash`: first use in a session is
  confirmed (unless the stage is full access), later calls in that session run without asking, and
  read-only mode does not offer plugin tools at all.
- **No shell, no plugins.** Everyday mode takes `bash` away; plugin tools disappear with it. "No
  shell in this mode" and "a stranger's program in this mode" would otherwise be two different
  promises about the same thing.
- Plugin tools run with the workspace as their working directory and with the child environment
  sanitised (`child_env.rs`), so a bundle's `LD_LIBRARY_PATH` cannot leak into them.
- No plugin update happens on its own. Replacing a plugin's files is the user's act; the panel
  shows a changed hash after it happened.
- **Notes go into the prompt, not through a tool.** An enabled plugin's `SERVER.md` is appended to
  the system prompt under a heading that says it came from the plugin's authors and cannot change
  the companion's rules. A folder with only notes therefore needs no program and no round trip.
- **The panel can test a plugin without a conversation.** A button starts the server once, runs
  the handshake and lists what it offers against what the manifest allows: allowed, refused,
  missing. This is how a broken plugin is found without waiting for a model turn.

## Enabling per project

Enabled ids are stored per project (the workspace path), not globally: the same plugin can be on
in one project and off in another. The store is a settings key with a path→ids map, so switching
projects changes what the companion can do without touching the filesystem.

## What "done" means

- A plugin folder with only `SERVER.md` is readable as notes and changes nothing else.
- A plugin with a `server` exposes exactly the tools its manifest names; a tool it does not name is
  refused even if the server offers it.
- Disabling removes the tools from the next turn's definitions and the panel says so.
- Tests: manifest parsing (broken JSON, missing id, unknown entry), tool filtering, and the "off
  is not uninstall" rule.

## Built (2026-11-25)

`src-tauri/src/jsonrpc.rs` (the shared stdio client) and `src-tauri/src/plugins.rs`, wired into
the turn in `engine/mod.rs`, with `PluginsPanel.vue` under Settings. 17 tests cover discovery,
naming, the offer path (`definitions_in`), the prompt notes, a real server through a probe, and a
real `tools/call` round trip.

Still open: a **real model turn** that uses a plugin tool. The offer path is tested with a real
child process, but no conversation has called one yet — that needs a model call, and therefore the
user.

## Out of scope

- A marketplace, ratings, or auto-update of plugins.
- Plugin-supplied UI. A plugin may not add views to the work area in the first version.
- Running plugins on machines the user has not enabled them on.
