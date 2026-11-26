import { computed, reactive, ref, watch } from "vue";
import { t } from "../i18n";
import { saveUiState } from "../lib/uiState";
import type { ChatMessage, MessagePart, Session, ToolActivity } from "../types";

/**
 * How many model calls one turn may make. A turn is not one request: a model that uses tools
 * asks, the tool answers, the model asks again — every request is one round. This is the ceiling
 * that stops a looping model from keeping a turn alive forever. It lives here because the message
 * the user sees when the ceiling is hit has to name the same number.
 */
export const MAX_ROUNDS_PER_TURN = 24;

export const sessions = ref<Session[]>([]);

export const activeSessionId = ref<string>("s1");

export const activeSession = computed(
  () => sessions.value.find((session) => session.id === activeSessionId.value) ?? null,
);

const messagesBySession = reactive<Record<string, ChatMessage[]>>({ s1: [] });
export { messagesBySession };

export const messages = computed<ChatMessage[]>(
  () => messagesBySession[activeSessionId.value] ?? [],
);

function bucket(): ChatMessage[] {
  const id = activeSessionId.value;
  return messagesBySession[id] ?? (messagesBySession[id] = []);
}

let seq = 0;
let currentAssistantId: string | null = null;

function currentAssistant(): ChatMessage | undefined {
  if (!currentAssistantId) return undefined;
  return bucket().find((message) => message.id === currentAssistantId);
}

export function appendUserMessage(
  text: string,
  attachments: { path: string; name: string; mime: string; preview: string }[] = [],
): void {
  seq += 1;
  // The same shape the database stores, so a reloaded conversation looks identical.
  const parts: MessagePart[] = attachments.map((attachment) => ({
    kind: "image",
    text: "",
    path: attachment.path,
    name: attachment.name,
    mime: attachment.mime,
    preview: attachment.preview,
  }));
  if (text) parts.push({ kind: "text", text });
  bucket().push({ id: `u${seq}`, role: "user", text, parts });
}

export function beginAssistantMessage(): void {
  seq += 1;
  const message: ChatMessage = {
    id: `a${seq}`,
    role: "assistant",
    text: "",
    streaming: true,
    tools: [],
    parts: [],
  };
  bucket().push(message);
  currentAssistantId = message.id;
}

/**
 * Appends to the last part of that kind, or starts a new one. That "or starts a new one" is
 * the whole point: text that arrives after a tool call belongs below it, not merged into the
 * text that came before.
 */
function appendToPart(message: ChatMessage, kind: "text" | "reasoning", chunk: string): void {
  const parts = message.parts ?? (message.parts = []);

  if (kind === "reasoning") {
    // Thinking is one block, always at the top. A model that thinks in several passes sends
    // its deltas interleaved with text — "text, reasoning, text, reasoning…" — and appending a
    // new block per switch turned a single answer into confetti: three words of prose, a
    // "thinking" fold, three more words, another fold. Found in a screenshot of a real answer.
    const existing = parts.find((part) => part.kind === "reasoning");
    if (existing) {
      existing.text += chunk;
      return;
    }
    parts.unshift({ kind: "reasoning", text: chunk });
    return;
  }

  // Text does start a new block after a tool chip: what was said after a command belongs
  // below it, not merged into the paragraph from before.
  const last = parts[parts.length - 1];
  if (last?.kind === kind) {
    last.text += chunk;
    return;
  }
  parts.push({ kind, text: chunk });
}

/**
 * Starts an answer if none is open. A turn can also begin without the composer — a scheduled
 * task, for one — and then nobody called `beginAssistantMessage`: its events arrived for the
 * active session and were dropped, so the answer only appeared after a reload.
 */
export function ensureAssistantMessage(): void {
  if (!currentAssistantId) beginAssistantMessage();
}

export function appendAssistantDelta(chunk: string): void {
  const message = currentAssistant();
  if (!message) return;
  message.text += chunk;
  appendToPart(message, "text", chunk);
}

export function appendAssistantReasoning(chunk: string): void {
  const message = currentAssistant();
  if (!message) return;
  message.reasoning = (message.reasoning ?? "") + chunk;
  appendToPart(message, "reasoning", chunk);
}

export function upsertToolActivity(activity: ToolActivity): void {
  const message = currentAssistant();
  if (!message) return;
  const tools = message.tools ?? (message.tools = []);
  const index = tools.findIndex((entry) => entry.id === activity.id);
  if (index >= 0) tools[index] = { ...tools[index], ...activity };
  else tools.push(activity);

  // The part keeps its place in the timeline while the tool runs and finishes.
  const parts = message.parts ?? (message.parts = []);
  const existing = parts.find((part) => part.kind === "tool" && part.tool.id === activity.id);
  if (existing && existing.kind === "tool") {
    existing.tool = { ...existing.tool, ...activity };
    return;
  }
  parts.push({ kind: "tool", tool: activity });
}

export function endAssistantMessage(stopReason?: string): void {
  const message = currentAssistant();
  if (message) {
    message.streaming = false;
    // "error" is the *terminal marker* of a failed turn, not an error message: the error
    // itself arrives as its own event and is already on the bubble. Letting the marker win
    // replaced a real message ("LLM HTTP 400: …") with the word "error".
    // "no_answer" is terminal too: the engine has already explained in its own notice why the
    // turn stopped, and the marker must not replace that sentence with a token.
    // "max_turns" is not a failure either — the turn simply used up its rounds — so it gets a
    // sentence instead of the engine's identifier, which used to appear alone in red.
    if (
      stopReason &&
      !["end_turn", "abort", "error", "no_answer"].includes(stopReason)
    ) {
      message.error =
        stopReason === "max_turns"
          ? t("engine.maxTurns", { rounds: MAX_ROUNDS_PER_TURN })
          : stopReason;
    }
  }
  currentAssistantId = null;
}

export function failAssistantMessage(error: string): void {
  const message = currentAssistant();
  if (!message) return;
  message.streaming = false;
  message.error = error;
  currentAssistantId = null;
}

/** Drops one message from the open conversation — the failed bubble before a retry. */
export function dropMessage(id: string): void {
  const list = bucket();
  const index = list.findIndex((message) => message.id === id);
  if (index >= 0) list.splice(index);
  if (currentAssistantId === id) currentAssistantId = null;
}

export function clearConversation(): void {
  messagesBySession[activeSessionId.value] = [];
  currentAssistantId = null;
}

export function selectSession(id: string): void {
  activeSessionId.value = id;
  currentAssistantId = null;
}

/**
 * Remembers which conversation was open. Called before the first session list is loaded, so
 * a stale id (deleted, archived) is simply replaced by the newest session at that point.
 */
export function initSessionSelection(stored: unknown): void {
  if (typeof stored === "string" && stored) activeSessionId.value = stored;
  watch(activeSessionId, (id) => saveUiState("activeSession", id));
}
