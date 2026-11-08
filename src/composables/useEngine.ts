import { reactive, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  ChatMessage,
  EngineEvent,
  MessagePart,
  Session,
  SessionGroup,
  SessionResume,
  ToolActivity,
} from "../types";
import {
  activeSessionId,
  appendAssistantDelta,
  appendAssistantReasoning,
  appendUserMessage,
  beginAssistantMessage,
  clearConversation,
  dropMessage,
  endAssistantMessage,
  ensureAssistantMessage,
  failAssistantMessage,
  messages,
  messagesBySession,
  selectSession,
  sessions,
  upsertToolActivity,
} from "../stores/sessions";
import { defaultModelFor, settings } from "../stores/settings";
import { bumpTurnDiff } from "../stores/workArea";

// A stage change (Planmodus/Vor Änderungen fragen/Vollzugriff) also ends any "allow for this session" grant.
watch(
  () => settings.permission,
  () => {
    void invoke("engine_clear_session_grants").catch(() => undefined);
  },
);
import { ensureWorkspace, reloadTree } from "./useWorkspace";
import { buildPersona } from "../engine/persona";
import { EVERYDAY_DISABLED_TOOLS, everydayMode } from "../engine/mode";
import { partnerName, refreshPartnerName } from "../stores/partner";
import { loadModels, modelChoices } from "../stores/models";
import { saveUiState } from "../lib/uiState";
import { samePath } from "../lib/projects";
import { followToolResult, followToolStart } from "./useFollow";
import { t } from "../i18n";

interface DbSessionRow {
  id: string;
  title: string;
  group: string;
  workspace: string;
  updatedAt: number;
}

interface DbToolRow {
  id: string;
  name: string;
  arguments: string | null;
  output: string | null;
  isError: boolean;
  durationMs: number | null;
}

interface DbHistoryRow {
  id: string;
  role: string;
  text: string;
  tools: DbToolRow[];
  /** JSON timeline, as the engine wrote it. Null for anything from before that existed. */
  parts: string | null;
}

export const running = ref(false);
export const engineReady = ref(false);
export const lastError = ref<string | null>(null);
export const secretBackend = ref<"keyring" | "file" | "none">("none");

/** Bumped whenever the partner rewrites its plan, so the Progress panel can reload. */
export const todosRevision = ref(0);

/** Set while a turn is blocked on the user (`ask_user`). Cleared by answering. */
export const pendingAction = ref<EngineEvent["action"] | null>(null);

/**
 * What the provider reported about the last request. `promptTokens` is the whole conversation
 * that was sent, which is what "how full is the context" actually means.
 */
export const lastUsage = ref<{ promptTokens: number; totalTokens: number } | null>(null);

type SecretStatus = { present: boolean; backend: string };

/** The API key used to live in localStorage. Move it to the keyring once, then forget it. */
const LEGACY_SETTINGS_KEY = "ds-ide.settings";
let legacyKey: string | null = null;

function extractLegacyKey(): void {
  try {
    const raw = localStorage.getItem(LEGACY_SETTINGS_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const value = parsed.apiKey;
    if (typeof value !== "string" || !value.trim()) return;
    legacyKey = value;
    delete parsed.apiKey;
    localStorage.setItem(LEGACY_SETTINGS_KEY, JSON.stringify(parsed));
  } catch {
    // unreadable legacy blob — nothing to migrate
  }
}

function applySecret(status: SecretStatus): void {
  engineReady.value = status.present;
  secretBackend.value = (status.backend as "keyring" | "file" | "none") ?? "none";
}

export async function refreshSecret(): Promise<void> {
  const status = await invoke<SecretStatus>("secret_status");
  if (!status.present && legacyKey) {
    const migrated = await invoke<SecretStatus>("secret_set", { value: legacyKey });
    legacyKey = null;
    applySecret(migrated);
    return;
  }
  applySecret(status);
}

export async function saveApiKey(value: string): Promise<void> {
  applySecret(await invoke<SecretStatus>("secret_set", { value }));
}

export async function clearApiKey(): Promise<void> {
  applySecret(await invoke<SecretStatus>("secret_clear"));
}

let unlisten: (() => void) | null = null;

/**
 * Stream chunks are collected briefly before they reach the store: the provider sends
 * dozens per second, and each one used to re-render the whole conversation and re-parse
 * every finished answer as markdown. Forty milliseconds is below noticing and cuts the
 * work by an order of magnitude.
 */
const STREAM_FLUSH_MS = 40;

let streamBuffer: { kind: "text" | "reasoning"; text: string }[] = [];
let streamTimer: number | undefined;

function queueChunk(kind: "text" | "reasoning", chunk: string): void {
  const last = streamBuffer[streamBuffer.length - 1];
  if (last?.kind === kind) last.text += chunk;
  else streamBuffer.push({ kind, text: chunk });
  if (streamTimer === undefined) {
    streamTimer = window.setTimeout(flushStream, STREAM_FLUSH_MS);
  }
}

/**
 * Hands the buffer over in its original order. Every event that changes the timeline calls
 * this first, so a tool chip can never appear above text that was written before it.
 */
function flushStream(): void {
  if (streamTimer !== undefined) {
    window.clearTimeout(streamTimer);
    streamTimer = undefined;
  }
  if (streamBuffer.length === 0) return;
  const chunks = streamBuffer;
  streamBuffer = [];
  for (const chunk of chunks) {
    if (chunk.kind === "text") appendAssistantDelta(chunk.text);
    else appendAssistantReasoning(chunk.text);
  }
}


export async function configureEngine(): Promise<void> {
  await invoke("engine_set_config", {
    config: {
      baseUrl: settings.baseUrl || "https://api.deepseek.com",
      model: settings.model || defaultModelFor(settings.mode),
      systemPrompt: buildPersona({
        name: partnerName.value,
        workspace: settings.workspace,
        userPrompt: settings.partnerPrompt,
      }),
      workspace: settings.workspace,
      permission: settings.permission,
      commandPolicy: settings.commandPolicy,
      allowedCommands: settings.allowedCommands,
      workspaceOnly: settings.workspaceOnly,
      browserControl: settings.browserControl,
      maxTurns: 24,
      reasoningEffort: settings.reasoningEffort || null,
      // The countdown in the chat panel reads the same value, so the promise and the wait agree.
      askTimeoutSeconds: Math.max(30, Math.min(3600, Math.round(settings.askTimeoutMinutes * 60))),
      // Compression needs the window to know when a conversation is too long.
      contextWindow: modelChoices().find((entry) => entry.id === settings.model)?.contextWindow ?? null,
      // Everyday mode takes the shell, code review and sub-agents away from the partner.
      disabledTools: everydayMode.value ? [...EVERYDAY_DISABLED_TOOLS] : [],
    },
  });
  await refreshSecret();
}

function handleEvent(event: EngineEvent): void {
  if (event.sessionId !== activeSessionId.value) return;

  // Anything that belongs to an answer makes sure one is open first.
  if (
    event.kind === "stream" ||
    event.kind === "reasoning" ||
    event.kind === "tool_start" ||
    event.kind === "tool_result"
  ) {
    ensureAssistantMessage();
    running.value = true;
  }

  switch (event.kind) {
    case "stream":
      if (event.text) queueChunk("text", event.text);
      break;
    case "reasoning":
      if (event.text) queueChunk("reasoning", event.text);
      break;
    case "tool_start":
      flushStream();
      if (event.tool) {
        upsertToolActivity({
          id: event.tool.id,
          name: event.tool.name,
          arguments: event.tool.arguments,
          running: true,
        });
        followToolStart(event.tool.id, event.tool.name, event.tool.arguments);
      }
      break;
    case "tool_result":
      flushStream();
      if (event.tool) {
        followToolResult(event.tool.id, event.tool.isError === true);
        upsertToolActivity({
          id: event.tool.id,
          name: event.tool.name,
          output: event.tool.output,
          isError: event.tool.isError,
          durationMs: event.tool.durationMs,
          image: event.tool.image,
          running: false,
        });
        if (event.tool.name === "todowrite") todosRevision.value += 1;
      }
      break;
    case "error":
      flushStream();
      lastError.value = event.error ?? "unknown engine error";
      // A turn can fail before its first token (a rejected request, a missing key). Without
      // this the error had no bubble to land in — a scheduled run failed silently.
      ensureAssistantMessage();
      failAssistantMessage(lastError.value);
      pendingAction.value = null;
      running.value = false;
      break;
    case "action_required":
      flushStream();
      pendingAction.value = event.action ?? null;
      break;
    case "title": {
      const session = sessions.value.find((entry) => entry.id === event.sessionId);
      if (session && event.text) session.title = event.text;
      break;
    }
    case "usage":
      if (event.usage) {
        lastUsage.value = {
          promptTokens: event.usage.promptTokens,
          totalTokens: event.usage.totalTokens,
        };
        // Remembered across restarts: the conversation it describes is reloaded too, so the
        // ring should not read "nothing has run yet" after every start.
        saveUiState("lastUsage", lastUsage.value);
      }
      break;

    case "turn_end":
      flushStream();
      if (event.stopReason === "abort") {
        // A stop can land while a tool waits for approval; that tool never produces a
        // result. It must not keep spinning in the timeline as if it were still running.
        for (const message of messages.value) {
          for (const tool of message.tools ?? []) {
            if (tool.running) {
              tool.running = false;
              tool.cancelled = true;
            }
          }
        }
      }
      endAssistantMessage(event.stopReason);
      bumpTurnDiff();
      pendingAction.value = null;
      running.value = false;
      // The partner may have chosen its name during this turn. Reading it is enough: the
      // next turn configures the engine again anyway (see `sendPrompt`).
      void refreshPartnerName();
      break;
    default:
      break;
  }
}

/** Restores the last usage report so the context ring has a value before the first turn. */
export function initLastUsage(stored: unknown): void {
  if (!stored || typeof stored !== "object") return;
  const value = stored as { promptTokens?: unknown; totalTokens?: unknown };
  if (typeof value.promptTokens === "number") {
    lastUsage.value = {
      promptTokens: value.promptTokens,
      totalTokens: typeof value.totalTokens === "number" ? value.totalTokens : value.promptTokens,
    };
  }
}

let initialized = false;
let initializing: Promise<void> | null = null;

/**
 * One-time boot: listener, workspace, keyring, partner, model list, sessions and the first
 * history. The chat panel mounts again after Settings; a second run would reload the stored
 * history and wipe live state such as a failed turn's retry bubble.
 */
export async function initEngine(): Promise<void> {
  if (initialized) return;
  if (initializing) return initializing;
  initializing = (async () => {
    if (!unlisten) {
      unlisten = await listen<EngineEvent>("engine://event", (event) => handleEvent(event.payload));
    }
    extractLegacyKey();
    await ensureWorkspace();
    await refreshSecret();
    await refreshPartnerName();
    // Ask the provider which models exist before the first turn: an id it does not know would
    // fail every request, and the field would look fine while doing it.
    await loadModels();
    await loadSessions();
    // The thread that was open decides the project before the engine is configured, so the
    // first request already carries the right folder.
    adoptSessionWorkspace(activeSessionId.value);
    await configureEngine();
    // Check the workspace at startup, not only when the file tab first mounts: the
    // composer's status line shows the difference between "empty" and "gone".
    await reloadTree();
    await loadHistory(activeSessionId.value);
  })();
  try {
    await initializing;
    initialized = true;
  } finally {
    initializing = null;
  }
}

export async function loadSessions(): Promise<void> {
  const rows = await invoke<DbSessionRow[]>("sessions_list");

  if (rows.length === 0) {
    const id = `s${Date.now()}`;
    await invoke("session_create", { id, title: "", workspace: settings.workspace });
    sessions.value = [
      { id, title: "Neue Aufgabe", group: "project", workspace: settings.workspace },
    ];
    activeSessionId.value = id;
    return;
  }

  sessions.value = rows.map(
    (row): Session => ({
      id: row.id,
      title: row.title || "Neue Aufgabe",
      group: row.group as SessionGroup,
      workspace: row.workspace,
    }),
  );

  if (!rows.some((row) => row.id === activeSessionId.value)) {
    activeSessionId.value = rows[0].id;
  }
}

/**
 * The compressed summary per session, for the "where we were" card. `null` until the
 * conversation has been compressed once.
 */
export const resumesBySession = reactive<Record<string, SessionResume | null>>({});

/**
 * How many messages one page holds. A long thread used to arrive whole — hundreds of rows,
 * each with its tool calls, all rendered at once — so the window opened slowly and scrolling
 * dragged. It now opens with the newest page and reads older ones on demand.
 */
const HISTORY_PAGE = 80;

/** Sessions that still have older messages behind the first one on screen. */
export const historyHasMore = reactive<Record<string, boolean>>({});

function mapHistoryRows(rows: DbHistoryRow[]): ChatMessage[] {
  return rows.filter(hasSomethingToSay).map((row) => {
    const tools = row.tools.map(
      (tool): ToolActivity => ({
        id: tool.id,
        name: tool.name,
        arguments: tool.arguments ?? undefined,
        output: tool.output ?? undefined,
        isError: tool.isError,
        durationMs: tool.durationMs ?? undefined,
      }),
    );
    return {
      id: row.id,
      role: row.role === "user" ? ("user" as const) : ("assistant" as const),
      text: row.text,
      tools,
      parts: timelineFor(row, tools),
    };
  });
}

export async function loadHistory(sessionId: string): Promise<void> {
  const rows = await invoke<DbHistoryRow[]>("session_history", {
    sessionId,
    limit: HISTORY_PAGE,
  });
  // A full page means there is probably more behind it; a short one is the beginning.
  historyHasMore[sessionId] = rows.length === HISTORY_PAGE;
  messagesBySession[sessionId] = mapHistoryRows(rows);

  try {
    resumesBySession[sessionId] = await invoke<SessionResume | null>("session_resume", {
      sessionId,
    });
  } catch {
    resumesBySession[sessionId] = null;
  }
}

/**
 * Reads the next older page and puts it in front. The caller keeps the scroll position: the
 * list grows upwards, so the line the reader was on must not move.
 */
export async function loadOlderHistory(sessionId: string): Promise<number> {
  const current = messagesBySession[sessionId] ?? [];
  const first = current[0]?.id;
  if (!first || !historyHasMore[sessionId]) return 0;
  const rows = await invoke<DbHistoryRow[]>("session_history", {
    sessionId,
    beforeId: first,
    limit: HISTORY_PAGE,
  });
  historyHasMore[sessionId] = rows.length === HISTORY_PAGE;
  const older = mapHistoryRows(rows);
  messagesBySession[sessionId] = [...older, ...current];
  return older.length;
}

/**
 * A turn that was cut off — the app closed while the model was still answering — leaves an
 * assistant row that was created but never filled. Reloading showed it as an empty bubble.
 */
function hasSomethingToSay(row: DbHistoryRow): boolean {
  if (row.role === "user") return true;
  if (row.text.trim().length > 0) return true;
  if (row.tools.length > 0) return true;
  // An empty timeline is `[]`, which is as good as none.
  const parts = row.parts?.trim();
  return Boolean(parts && parts !== "[]");
}

/**
 * The stored timeline, or one rebuilt from text and tools. Rows written before the timeline
 * existed only have the two separate fields, and they used to be shown as text-then-tools —
 * keeping that as the fallback means an old conversation still reads the way it did.
 */
function timelineFor(row: DbHistoryRow, tools: ToolActivity[]): MessagePart[] {
  if (row.parts) {
    try {
      const parsed = JSON.parse(row.parts) as MessagePart[];
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    } catch {
      // fall through to the rebuilt version
    }
  }
  const rebuilt: MessagePart[] = [];
  if (row.text) rebuilt.push({ kind: "text", text: row.text });
  for (const tool of tools) rebuilt.push({ kind: "tool", tool });
  return rebuilt;
}

export async function openSession(sessionId: string): Promise<void> {
  flushStream();
  selectSession(sessionId);
  // Choosing a thread chooses its project. A global folder that disagrees with the
  // conversation on screen is the state this feature exists to remove.
  if (adoptSessionWorkspace(sessionId)) await reloadTree();
  await loadHistory(sessionId);
}

/**
 * Points the tools at the folder this thread was started in. Returns true when the workspace
 * actually changed, so the caller knows whether the tree has to be read again.
 *
 * A thread without a workspace (written before projects existed) leaves the current folder
 * alone: guessing one from the path would be a silent assumption, and the rail offers an
 * explicit "move into project" instead.
 */
function adoptSessionWorkspace(sessionId: string): boolean {
  const session = sessions.value.find((entry) => entry.id === sessionId);
  const workspace = session?.workspace?.trim() ?? "";
  if (!workspace || samePath(workspace, settings.workspace)) return false;
  settings.workspace = workspace;
  return true;
}

export async function createSession(): Promise<void> {
  flushStream();
  const id = `s${Date.now()}`;
  await invoke("session_create", { id, title: "", workspace: settings.workspace });
  await loadSessions();
  activeSessionId.value = id;
  clearConversation();
}

export interface OutgoingAttachment {
  path: string;
  name: string;
  mime: string;
  preview: string;
}

export async function sendPrompt(
  text: string,
  attachments: OutgoingAttachment[] = [],
): Promise<void> {
  if (running.value) return;
  lastError.value = null;
  flushStream();

  await configureEngine();
  if (!engineReady.value) {
    lastError.value = t("engine.needsKey");
    return;
  }

  appendUserMessage(text, attachments);
  beginAssistantMessage();
  running.value = true;

  try {
    await invoke("engine_send", {
      sessionId: activeSessionId.value,
      prompt: text,
      attachments,
    });
  } catch (error) {
    running.value = false;
    lastError.value = String(error);
    failAssistantMessage(lastError.value);
  }
}

export async function retryTurn(messageId: string): Promise<void> {
  if (running.value) return;
  const sessionId = activeSessionId.value;
  const message = messagesBySession[sessionId]?.find((entry) => entry.id === messageId);
  if (!message?.error) return;

  dropMessage(messageId);
  lastError.value = null;
  flushStream();
  running.value = true;

  try {
    await configureEngine();
    await invoke("engine_retry", { sessionId });
  } catch (error) {
    // The backend rolls the failed tail back before it starts. If even that failed, reload
    // what is actually stored and leave the error as a bubble that can be retried again.
    running.value = false;
    await loadHistory(sessionId);
    ensureAssistantMessage();
    failAssistantMessage(String(error));
  }
}

export async function abortTurn(): Promise<void> {
  await invoke("engine_abort", { sessionId: activeSessionId.value });
}

/**
 * Answer a pending question so the blocked turn can continue.
 *
 * Throws when the answer did not arrive — the question can have expired on the engine side,
 * and saying "sent" while it went nowhere is how a typed answer disappears. The caller keeps
 * the text and tells the user; the card stays until a real answer or the end of the turn.
 */
export async function answerAction(actionId: string, answer: string): Promise<void> {
  await invoke("engine_answer", { actionId, answer });
  pendingAction.value = null;
}

export async function resetConversation(): Promise<void> {
  await invoke("engine_reset", { sessionId: activeSessionId.value });
}
