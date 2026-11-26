import { http } from './http';
import type { RollingSummary } from './aiStudy';

export interface Poll {
  id: string;
  kind: 'poll' | 'quiz' | 'comprehension';
  question: string;
  options: string[];
  status: 'open' | 'closed';
  source: string;
  created_at: string;
  counts: number[];
  total: number;
  correct_index: number | null;
  explanation: string | null;
  my_choice: number | null;
}

export interface RaisedHand {
  participant_id: string;
  display_name: string;
  matric_number: string;
  raised_at: string;
}

const code = (c: string) => encodeURIComponent(c);

export function fetchLiveSummaries(
  shortCode: string
): Promise<RollingSummary[]> {
  return http<RollingSummary[]>(`/sessions/code/${code(shortCode)}/summaries`);
}

export function fetchPolls(
  shortCode: string,
  participantId?: string
): Promise<Poll[]> {
  const qs = participantId
    ? `?participant_id=${encodeURIComponent(participantId)}`
    : '';
  return http<Poll[]>(`/sessions/code/${code(shortCode)}/polls${qs}`);
}

export function createPoll(
  shortCode: string,
  input: {
    kind: Poll['kind'];
    question: string;
    options: string[];
    correct_index?: number;
    explanation?: string;
  }
): Promise<Poll> {
  return http<Poll>(`/sessions/code/${code(shortCode)}/polls`, {
    method: 'POST',
    body: JSON.stringify(input),
  });
}

export function generatePoll(
  shortCode: string,
  kind: Poll['kind'] = 'quiz'
): Promise<Poll> {
  return http<Poll>(`/sessions/code/${code(shortCode)}/polls/generate`, {
    method: 'POST',
    body: JSON.stringify({ kind }),
  });
}

export function respondToPoll(
  shortCode: string,
  pollId: string,
  participantId: string,
  choiceIndex: number
): Promise<Poll> {
  return http<Poll>(
    `/sessions/code/${code(shortCode)}/polls/${encodeURIComponent(pollId)}/respond`,
    {
      method: 'POST',
      body: JSON.stringify({
        participant_id: participantId,
        choice_index: choiceIndex,
      }),
    }
  );
}

export function closePoll(shortCode: string, pollId: string): Promise<Poll> {
  return http<Poll>(
    `/sessions/code/${code(shortCode)}/polls/${encodeURIComponent(pollId)}/close`,
    { method: 'POST' }
  );
}

export function fetchRaisedHands(shortCode: string): Promise<RaisedHand[]> {
  return http<RaisedHand[]>(`/sessions/code/${code(shortCode)}/hand-raises`);
}
