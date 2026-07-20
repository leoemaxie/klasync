import type { Participant, RosterStudent, Screen, Session } from "./types";

export function createSessionState() {
  let screen = $state<Screen>("home");
  let lecturerName = $state("");
  let lecturerEmail = $state("");
  let courseCode = $state("CSC 312");
  let courseTitle = $state("Human Computer Interaction");
  let rosterText = $state("MAT/2023/001,Ada Okafor\nMAT/2023/002,Chinedu Obi");
  let roster = $state<RosterStudent[]>([]);
  let session = $state<Session | null>(null);
  let sessionCode = $state("");
  let matric = $state("");
  let displayName = $state("");
  let joinError = $state("");
  let joinedParticipant = $state<Participant | null>(null);
  let captions = $state<string[]>(["WAITING FOR LIVE CAPTIONS."]);
  let captionIndex = $state(0);
  let captionDraft = $state("");
  let copied = $state(false);
  let accountCreated = $state(false);
  let rosterNotice = $state("");
  let apiNotice = $state("");
  let isSaving = $state(false);

  return {
    get screen() { return screen; }, set screen(v) { screen = v; },
    get lecturerName() { return lecturerName; }, set lecturerName(v) { lecturerName = v; },
    get lecturerEmail() { return lecturerEmail; }, set lecturerEmail(v) { lecturerEmail = v; },
    get courseCode() { return courseCode; }, set courseCode(v) { courseCode = v; },
    get courseTitle() { return courseTitle; }, set courseTitle(v) { courseTitle = v; },
    get rosterText() { return rosterText; }, set rosterText(v) { rosterText = v; },
    get roster() { return roster; }, set roster(v) { roster = v; },
    get session() { return session; }, set session(v) { session = v; },
    get sessionCode() { return sessionCode; }, set sessionCode(v) { sessionCode = v; },
    get matric() { return matric; }, set matric(v) { matric = v; },
    get displayName() { return displayName; }, set displayName(v) { displayName = v; },
    get joinError() { return joinError; }, set joinError(v) { joinError = v; },
    get joinedParticipant() { return joinedParticipant; }, set joinedParticipant(v) { joinedParticipant = v; },
    get captions() { return captions; }, set captions(v) { captions = v; },
    get captionIndex() { return captionIndex; }, set captionIndex(v) { captionIndex = v; },
    get captionDraft() { return captionDraft; }, set captionDraft(v) { captionDraft = v; },
    get copied() { return copied; }, set copied(v) { copied = v; },
    get accountCreated() { return accountCreated; }, set accountCreated(v) { accountCreated = v; },
    get rosterNotice() { return rosterNotice; }, set rosterNotice(v) { rosterNotice = v; },
    get apiNotice() { return apiNotice; }, set apiNotice(v) { apiNotice = v; },
    get isSaving() { return isSaving; }, set isSaving(v) { isSaving = v; }
  };
}

export type SessionState = ReturnType<typeof createSessionState>;
