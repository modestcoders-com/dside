# Spec: MCP client

## Goal

Let the user connect MCP servers they configure themselves, with the same rules the `bash` tool
already follows. Nothing is shipped fixed; nothing runs that the user did not name.

## Decided

- **Same rules as `bash`:** the first use of an MCP tool in a session is confirmed, later calls in
  that session run without asking, and in read-only mode no MCP tool is offered at all — nobody
  knows what a foreign tool does.
- **stdio first.** HTTP/SSE later, not now.
- **The config file is written by the user.** No settings form for four fields.

## Naming

`mcp__<server>__<tool>`.

- **Double underscore between the parts**, because it survives a server or tool name that itself
  contains a single underscore (`filesystem__read_file` vs. `filesystem_read_file` would be
  ambiguous).
- **The `mcp__` prefix stays**, so an external tool is recognisable at a glance from a built-in one
  (`mcp__postgres__query` vs. `bash`). That matters here beyond tidiness: built-in tools are
  covered by the app's own contract, MCP tools are not.
- Names are normalised to `[a-z0-9_]`: a server called `GitHub` becomes `github`, a tool called
  `create-issue` becomes `create_issue`. Dashes and dots are readable to people and fragile for
  models' tool-name grammars; the mapping is shown in the panel so the name is not a guess.

## Config

`<app data>/mcp.json`, mode 0600, never logged:

```json
{
  "servers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "…" },
      "cwd": null
    }
  }
}
```

- Read on start and again on an explicit reload in the panel; a malformed file is reported, not
  ignored.
- `env` may carry secrets. The file is therefore 0600, is gitignored, and its contents never reach
  the log, the panel or the model.
- A relative `command` resolves through `PATH` with the child environment sanitised, exactly like
  a `bash` call.

## Lifecycle

- **Lazy:** a server starts on the first turn whose tool list includes one of its tools. Starting
  every configured server at boot would cost seconds and tokens for nothing.
- **One process per server**, owned by the app, killed when the app exits, restarted with backoff
  after a crash (three attempts, then it stays down and says so).
- **Handshake:** `initialize` → `tools/list` once per process; the tool list is cached with the
  process. A server that answers nonsense is dropped, with the reason in the log.

## Foreign text is data

The output of an MCP tool is untrusted, like a web page: it enters the conversation as tool result,
not as instruction. The panel marks results from external servers, and the prompt already carries
the rule that tool output may inform but never instruct — this spec keeps that rule where it
matters.

## Failure behaviour

Every failure is a fact, not an exception: server not found, handshake timeout, server died, tool
missing, tool raised. The tool result names which, and the turn continues. Nothing about a foreign
server may take the conversation down.

## What "done" means

- A configured server's tools appear as `mcp__<server>__<tool>` and nothing else does.
- With no `mcp.json`, nothing changes: no process, no tool, no delay.
- Read-only mode offers no MCP tool, and says so if the model asks for one.
- Tests: name normalisation, config parsing (broken JSON, missing command), tool filtering, and a
  fake stdio server that answers `initialize` and `tools/list`.

## Out of scope

- HTTP/SSE transport, OAuth flows, server-side webhooks.
- Shipping a default server list.
- Letting a server add UI, or letting it register tools outside its prefix.
