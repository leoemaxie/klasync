import type { Participant, RosterStudent, Screen, Session } from "./types";

export class SessionState {
  screen = $state<Screen>("home");
  lecturerName = $state("");
  lecturerEmail = $state("");
  courseCode = $state("CSC 312");
  courseTitle = $state("Human Computer Interaction");
  rosterText = $state("MAT/2023/001,Ada Okafor\nMAT/2023/002,Chinedu Obi");
  roster = $state<RosterStudent[]>([]);
  session = $state<Session | null>(null);
  sessionCode = $state("");
  matric = $state("");
  displayName = $state("");
  joinError = $state("");
  joinedParticipant = $state<Participant | null>(null);
  captions = $state<string[]>(["WAITING FOR LIVE CAPTIONS."]);
  captionIndex = $state(0);
  captionDraft = $state("");
  copied = $state(false);
  accountCreated = $state(false);
  rosterNotice = $state("");
  apiNotice = $state("");
  isSaving = $state(false);
}

export function createSessionState(): SessionState {
  return new SessionState();
}
