import type { SessionState } from './sessionState.svelte';
import type { RosterStudent } from './types';

export function persist(state: SessionState) {
  if (state.session) {
    localStorage.setItem('klasync-session', JSON.stringify(state.session));
  }
  localStorage.setItem('klasync-roster', JSON.stringify(state.roster));
  localStorage.setItem('klasync-lecturer', state.lecturerName);
  if (state.currentUser) {
    localStorage.setItem('klasync-user', JSON.stringify(state.currentUser));
  }
}

export function parseRosterTextToStudents(rawText: string): RosterStudent[] {
  const cleanedText = rawText
    .replace(/^\uFEFF/, '')
    .replace(/[\u200B-\u200D\uFEFF]/g, '');
  const rawLines = cleanedText
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  if (rawLines.length === 0) return [];

  // Determine delimiter from first line (comma, semicolon, or tab)
  const firstLine = rawLines[0];
  let delimiter = ',';
  if (firstLine.includes('\t')) delimiter = '\t';
  else if (firstLine.includes(';')) delimiter = ';';

  // Check if line is a header
  const parseRow = (line: string) => {
    return line
      .split(delimiter)
      .map((col) => col.replace(/^["']|["']$/g, '').trim());
  };

  let startIdx = 0;
  let matricCol = 0;
  let nameCol = 1;

  const headerCols = parseRow(firstLine.toLowerCase());
  const hasHeaderMatric = headerCols.some(
    (c) => c.includes('matric') || c.includes('id') || c.includes('reg')
  );
  const hasHeaderName = headerCols.some(
    (c) => c.includes('name') || c.includes('student')
  );

  if (hasHeaderMatric || hasHeaderName) {
    startIdx = 1; // Skip header
    headerCols.forEach((col, idx) => {
      if (col.includes('matric') || col.includes('id') || col.includes('reg'))
        matricCol = idx;
      else if (col.includes('name') || col.includes('student')) nameCol = idx;
    });
  }

  const students: RosterStudent[] = [];
  for (let i = startIdx; i < rawLines.length; i++) {
    const cols = parseRow(rawLines[i]);
    const matric = cols[matricCol] ?? cols[0] ?? '';
    const name = cols[nameCol] ?? cols[1] ?? '';
    if (matric && name) {
      students.push({ matric, name });
    }
  }

  return students;
}

export function parseRoster(state: SessionState) {
  const parsed = parseRosterTextToStudents(state.rosterText);
  state.roster = parsed;
  persist(state);
  state.rosterNotice = `${parsed.length} student${parsed.length === 1 ? '' : 's'} prepared for verification.`;
}

export function importFile(state: SessionState, eventOrFile: Event | File) {
  let file: File | undefined;

  if (eventOrFile instanceof File) {
    file = eventOrFile;
  } else if (
    eventOrFile &&
    (eventOrFile.target as HTMLInputElement)?.files?.[0]
  ) {
    file = (eventOrFile.target as HTMLInputElement).files![0];
  }

  if (!file) return;

  const isCsvOrText = /\.csv$|\.txt$|\.tsv$/i.test(file.name);
  const isXlsx = /\.xlsx$/i.test(file.name);

  if (!isCsvOrText && !isXlsx) {
    state.rosterNotice =
      'Unsupported file format. Please upload a .csv, .tsv, or .xlsx file.';
    return;
  }

  const reader = new FileReader();
  reader.onload = () => {
    try {
      const content = String(reader.result ?? '');
      state.rosterText = content.replace(/^\uFEFF/, '');
      parseRoster(state);
      state.rosterNotice = `Successfully imported ${state.roster.length} student${state.roster.length === 1 ? '' : 's'} from ${file.name}.`;
    } catch {
      state.rosterNotice =
        'Error reading file contents. Ensure the file contains readable student records.';
    }
  };

  reader.onerror = () => {
    state.rosterNotice = 'Error loading file from disk.';
  };

  reader.readAsText(file);
}
