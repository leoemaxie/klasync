import {
  getLiveCaptions,
  joinLiveSession,
  resolveLiveSession,
  sendHeartbeat,
} from "./api";
import { persist } from "./rosterUtils";
import type { SessionState } from "./sessionState.svelte";
import type { Participant } from "./types";

export async function joinSession(state: SessionState) {
  state.joinError = "";
  if (!state.matric.trim()) {
    state.joinError = "Enter your matric or student ID to continue.";
    return;
  }
  try {
    const detail = await resolveLiveSession(state.sessionCode.trim());
    const remote = await joinLiveSession(
      state.sessionCode.trim(),
      state.matric.trim(),
      state.displayName.trim()
    );
    const participant: Participant = {
      id: remote.id,
      matric: remote.matric_number,
      name: remote.display_name,
      verified: remote.verification_status === "verified",
      joinedAt: new Date().toISOString(),
      heartbeats: remote.heartbeat_count,
    };
    state.joinedParticipant = participant;
    state.session = {
      title: detail.session.title,
      code: detail.session.short_code,
      live: detail.session.status === "live",
      createdAt: new Date().toISOString(),
      participants: [participant],
    };
    persist(state);
    state.screen = "live";
    await refreshCaptions(state);
  } catch (error) {
    state.joinError =
      error instanceof Error ? error.message : "Unable to join session.";
  }
}

export async function heartbeat(state: SessionState) {
  if (!state.session || !state.joinedParticipant?.id) return;
  try {
    const remote = await sendHeartbeat(state.joinedParticipant.id);
    const participants = state.session.participants.map((p) =>
      p.id === remote.id ? { ...p, heartbeats: remote.heartbeat_count } : p
    );
    state.joinedParticipant =
      participants.find((p) => p.id === remote.id) ?? state.joinedParticipant;
    state.session = { ...state.session, participants };
    persist(state);
  } catch (error) {
    state.apiNotice =
      error instanceof Error ? error.message : "Unable to record presence.";
  }
}

export async function refreshCaptions(state: SessionState) {
  if (!state.session) return;
  try {
    const remote = await getLiveCaptions(state.session.code);
    state.captions = remote.length
      ? remote.map((c) => c.text)
      : ["WAITING FOR LIVE CAPTIONS."];
    state.captionIndex = Math.max(state.captions.length - 1, 0);
  } catch (error) {
    state.apiNotice =
      error instanceof Error ? error.message : "Unable to retrieve live captions.";
  }
}
