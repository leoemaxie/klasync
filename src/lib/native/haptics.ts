/**
 * Klasync Native Haptic Feedback Integration
 */
import { platform } from './platform';

export type HapticImpact =
  | 'light'
  | 'medium'
  | 'heavy'
  | 'success'
  | 'warning'
  | 'error'
  | 'selection';

export function triggerHaptic(type: HapticImpact = 'light'): void {
  if (typeof window === 'undefined' || !platform.hasHaptics) return;

  try {
    switch (type) {
      case 'selection':
      case 'light':
        navigator.vibrate?.(10);
        break;
      case 'medium':
        navigator.vibrate?.(20);
        break;
      case 'heavy':
        navigator.vibrate?.(40);
        break;
      case 'success':
        navigator.vibrate?.([15, 30, 20]);
        break;
      case 'warning':
        navigator.vibrate?.([25, 40, 25]);
        break;
      case 'error':
        navigator.vibrate?.([40, 60, 40, 60]);
        break;
    }
  } catch {
    // Haptics fail silently if restricted by browser security policies
  }
}
