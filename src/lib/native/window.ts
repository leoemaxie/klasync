/**
 * Klasync Desktop Window Control Integration (Tauri)
 */
import { platform } from './platform';

export function isTauriWindow(): boolean {
  return platform.isTauri;
}

export async function minimizeWindow(): Promise<void> {
  if (!platform.isTauri) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().minimize();
  } catch (err) {
    console.warn('[Klasync Native] Minimize window error:', err);
  }
}

export async function toggleMaximizeWindow(): Promise<void> {
  if (!platform.isTauri) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const win = getCurrentWindow();
    await win.toggleMaximize();
  } catch (err) {
    console.warn('[Klasync Native] Toggle maximize window error:', err);
  }
}

export async function closeWindow(): Promise<void> {
  if (!platform.isTauri) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().close();
  } catch (err) {
    console.warn('[Klasync Native] Close window error:', err);
  }
}

export async function startDraggingWindow(): Promise<void> {
  if (!platform.isTauri) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().startDragging();
  } catch (err) {
    console.warn('[Klasync Native] Start dragging window error:', err);
  }
}
