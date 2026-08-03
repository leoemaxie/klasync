/**
 * Klasync Screen Wake Lock Controller
 * Prevents screen auto-dimming during live lectures and active captions.
 */

type WakeLockSentinel = {
  release: () => Promise<void>;
  released: boolean;
  addEventListener?: (type: string, listener: () => void) => void;
};

let wakeLockSentinel: WakeLockSentinel | null = null;
let isRequested = false;

export async function requestWakeLock(): Promise<boolean> {
  isRequested = true;
  if (typeof window === 'undefined' || !('wakeLock' in navigator)) {
    return false;
  }

  try {
    const nav = navigator as unknown as { wakeLock: { request: (type: string) => Promise<WakeLockSentinel> } };
    wakeLockSentinel = await nav.wakeLock.request('screen');
    
    wakeLockSentinel.addEventListener?.('release', () => {
      wakeLockSentinel = null;
    });

    return true;
  } catch (err) {
    console.warn('[Klasync Native] Screen Wake Lock request failed:', err);
    return false;
  }
}

export async function releaseWakeLock(): Promise<void> {
  isRequested = false;
  if (wakeLockSentinel && !wakeLockSentinel.released) {
    try {
      await wakeLockSentinel.release();
    } catch {
      // Ignore release errors
    } finally {
      wakeLockSentinel = null;
    }
  }
}

// Re-acquire wake lock if tab loses and regains visibility while in live lecture
if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible' && isRequested && !wakeLockSentinel) {
      void requestWakeLock();
    }
  });
}
