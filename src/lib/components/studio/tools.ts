import {
  FileText,
  ListChecks,
  ListTree,
  Network,
  PenLine,
  Layers,
  ClipboardCheck,
  MessageCircleQuestion,
  BookOpenText,
} from '@lucide/svelte';
import type { Component } from 'svelte';

export type ToolId =
  | 'transcript'
  | 'chapters'
  | 'summary'
  | 'visuals'
  | 'notes'
  | 'ask'
  | 'flashcards'
  | 'quiz'
  | 'handout';

export type ToolDef = {
  id: ToolId;
  label: string;
  key: string;
  icon: Component<{ size?: number | string }>;
};

export const TOOLS: ToolDef[] = [
  { id: 'transcript', label: 'Transcript', key: '1', icon: FileText },
  { id: 'notes', label: 'Notes', key: '2', icon: PenLine },
  { id: 'ask', label: 'Ask', key: '3', icon: MessageCircleQuestion },
  { id: 'summary', label: 'Summary', key: '4', icon: ListChecks },
  { id: 'visuals', label: 'Map', key: '5', icon: Network },
  { id: 'flashcards', label: 'Cards', key: '6', icon: Layers },
  { id: 'chapters', label: 'Chapters', key: '7', icon: ListTree },
  { id: 'quiz', label: 'Quiz', key: '8', icon: ClipboardCheck },
  { id: 'handout', label: 'Handout', key: '9', icon: BookOpenText },
];

export const toolById = (id: ToolId) => TOOLS.find((t) => t.id === id)!;

export const tabId = (pane: string, id: ToolId) => `${pane}-tab-${id}`;
