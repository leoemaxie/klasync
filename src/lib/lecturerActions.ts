import {
  createLiveSession,
  endSession as apiEndSession,
  getParticipants,
  publishCaption as apiPublishCaption,
} from './api';
import { persist } from './rosterUtils';
import { refreshCaptions } from './studentActions';
import type { SessionState } from './sessionState.svelte';
import type { Participant } from './types';

export async function startSession(state: SessionState) {
  state.apiNotice = '';
  if (!state.lecturerName.trim() || !state.lecturerEmail.trim()) {
    state.apiNotice =
      'Enter your name and email before starting a live session.';
    return;
  }
  state.isSaving = true;
  try {
    const invite = await createLiveSession({
      lecturerName: state.lecturerName,
      lecturerEmail: state.lecturerEmail,
      courseCode: state.courseCode,
      courseTitle: state.courseTitle,
      roster: state.roster.map((s) => ({
        matric_number: s.matric,
        full_name: s.name,
      })),
    });
    state.session = {
      title: invite.session.title,
      code: invite.session.short_code,
      live: true,
      createdAt: new Date().toISOString(),
      participants: [],
    };
    state.sessionCode = invite.session.short_code;
    persist(state);
  } catch (error) {
    state.apiNotice =
      error instanceof Error ? error.message : 'Unable to start session.';
  } finally {
    state.isSaving = false;
  }
}

export async function publishCaption(state: SessionState) {
  if (!state.session || !state.captionDraft.trim()) return;
  const draft = state.captionDraft.trim();
  state.apiNotice = '';
  try {
    await apiPublishCaption(state.session.code, draft);
    state.captionDraft = '';
    await refreshCaptions(state);
  } catch {
    if (!state.captions.includes(draft)) {
      state.captions = [...state.captions, draft];
      state.captionIndex = state.captions.length - 1;
    }
    state.captionDraft = '';
    state.apiNotice = '';
  }
}

export async function copyInvite(state: SessionState) {
  if (!state.session) return;
  await navigator.clipboard?.writeText(
    `${location.origin}/#/?join=${state.session.code}`
  );
  state.copied = true;
  setTimeout(() => (state.copied = false), 1800);
}

export async function refreshAttendance(state: SessionState) {
  if (!state.session) return;
  state.apiNotice = '';
  try {
    const remote = await getParticipants(state.session.code);
    const participants: Participant[] = remote.map((p) => ({
      id: p.id,
      matric: p.matric_number,
      name: p.display_name,
      verified: p.verification_status === 'verified',
      joinedAt: new Date().toISOString(),
      heartbeats: p.heartbeat_count,
    }));
    state.session = { ...state.session, participants };
    persist(state);
  } catch (error) {
    state.apiNotice =
      error instanceof Error ? error.message : 'Unable to refresh attendance.';
  }
}

export async function endSession(state: SessionState) {
  if (!state.session) return;
  state.apiNotice = '';
  state.isSaving = true;
  try {
    await apiEndSession(state.session.code);
    state.session = { ...state.session, live: false };
    persist(state);
  } catch (error) {
    state.apiNotice =
      error instanceof Error ? error.message : 'Unable to end session.';
  } finally {
    state.isSaving = false;
  }
}
