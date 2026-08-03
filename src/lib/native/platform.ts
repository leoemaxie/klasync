/**
 * Klasync Platform Capability & Environment Detection
 */

export interface PlatformInfo {
  isTauri: boolean;
  isMobile: boolean;
  isIOS: boolean;
  isAndroid: boolean;
  isMacOS: boolean;
  isWindows: boolean;
  hasTouch: boolean;
  hasHaptics: boolean;
}

export function detectPlatform(): PlatformInfo {
  if (typeof window === 'undefined') {
    return {
      isTauri: false,
      isMobile: false,
      isIOS: false,
      isAndroid: false,
      isMacOS: false,
      isWindows: false,
      hasTouch: false,
      hasHaptics: false,
    };
  }

  const ua = navigator.userAgent || '';
  const isTauri = '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
  const isIOS = /iPhone|iPad|iPod/i.test(ua);
  const isAndroid = /Android/i.test(ua);
  const isMobile = isIOS || isAndroid || /Mobi|Tablet/i.test(ua);
  const isMacOS = /Macintosh|Mac OS X/i.test(ua);
  const isWindows = /Windows/i.test(ua);
  const hasTouch = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
  const hasHaptics = typeof navigator !== 'undefined' && 'vibrate' in navigator;

  return {
    isTauri,
    isMobile,
    isIOS,
    isAndroid,
    isMacOS,
    isWindows,
    hasTouch,
    hasHaptics,
  };
}

export const platform = detectPlatform();
