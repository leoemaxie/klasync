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
      title: 'Join Live Lecture Session',
      description:
        'Enter guest short-code & student ID to access live captions',
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
      title: 'Lecturer Control Room',
      description: 'Course setup, roster management, and broadcasting',
      icon: BookOpen,
      category: 'Navigation',
      action: (state) => ({ screen: 'lecturer', isOpen: false }),
    });
  }

  if (!currentUser || currentUser.role === 'student') {
    actions.push({
      id: 'archive',
      title: 'Student Archive & Materials',
      description: 'Review saved lecture notes, slides, and transcripts',
      icon: Users,
      category: 'Navigation',
      action: (state) => ({ screen: 'archive', isOpen: false }),
    });
  }

  return actions;
}
