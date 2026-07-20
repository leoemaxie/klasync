import type { SessionState } from "./sessionState.svelte";
import type { RosterStudent } from "./types";

export function persist(state: SessionState) {
  if (state.session) {
    localStorage.setItem("klasync-session", JSON.stringify(state.session));
  }
  localStorage.setItem("klasync-roster", JSON.stringify(state.roster));
  localStorage.setItem("klasync-lecturer", state.lecturerName);
}

export function parseRoster(state: SessionState) {
  const parsed = state.rosterText
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [matric, name] = line.split(",").map((v) => v?.trim());
      return { matric, name };
    })
    .filter((item) => item.matric && item.name) as RosterStudent[];
  state.roster = parsed;
  persist(state);
  state.rosterNotice = `${parsed.length} student${parsed.length === 1 ? "" : "s"} ready for verification.`;
}

export function importFile(state: SessionState, event: Event) {
  const file = (event.currentTarget as HTMLInputElement).files?.[0];
  if (!file) return;
  if (!/\.csv$/i.test(file.name)) {
    state.rosterNotice = "This MVP accepts CSV files. Use columns: matric number, full name.";
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    state.rosterText = String(reader.result).replace(/^\uFEFF/, "");
    parseRoster(state);
  };
  reader.readAsText(file);
}
