import { Radio, BookOpen, Users } from '@lucide/svelte';
import type { Screen } from '$lib/types';
import type { AuthUser } from '$lib/api/auth';
import type { Component } from 'svelte';

export interface ShortcutAction {
  id: string;
  title: string;
  description: string;
  icon: Component<any>;
  category: 'Navigation' | 'Action';
  action: (state: { screen: Screen; isOpen: boolean }) => {
    screen: Screen;
    isOpen: boolean;
  };
}

export function getShortcutActions(
  currentUser: AuthUser | null
): ShortcutAction[] {
  const actions: ShortcutAction[] = [
    {
      id: 'join',
      title: 'Join Session',
      description: 'Enter session code to join',
      icon: Radio,
      category: 'Navigation',
      action: (state) => ({ screen: 'join', isOpen: false }),
    },
  ];

  if (
    !currentUser ||
    currentUser.role === 'lecturer' ||
    currentUser.role === 'admin'
  ) {
    actions.push({
      id: 'lecturer',
      title: 'Lecturer Workspace',
      description: 'Manage courses, roster, and live lectures',
      icon: BookOpen,
      category: 'Navigation',
      action: (state) => ({ screen: 'lecturer', isOpen: false }),
    });
  }

  if (!currentUser || currentUser.role === 'student') {
    actions.push({
      id: 'archive',
      title: 'Study Archive',
      description: 'Review saved notes, transcripts, and flashcards',
      icon: Users,
      category: 'Navigation',
      action: (state) => ({ screen: 'archive', isOpen: false }),
    });
  }

  return actions;
}
