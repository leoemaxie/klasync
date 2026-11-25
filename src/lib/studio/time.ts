/** mm:ss, or h:mm:ss once past an hour. */
export function clock(totalSeconds: number | null | undefined): string {
  if (totalSeconds == null || !Number.isFinite(totalSeconds)) return '--:--';
  const s = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const mm = String(m).padStart(2, '0');
  const ss = String(sec).padStart(2, '0');
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

/** "12:05" / "1:02:03" → seconds, or null when not a timestamp. */
export function parseClock(text: string): number | null {
  const parts = text.trim().split(':');
  if (parts.length < 2 || parts.length > 3) return null;
  const nums = parts.map((p) => (/^\d{1,2}$/.test(p) ? Number(p) : NaN));
  if (nums.some(Number.isNaN)) return null;
  return nums.reduce((acc, n) => acc * 60 + n, 0);
}

/** Spoken form for screen readers: "1 minute 5 seconds". */
export function spokenClock(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const m = Math.floor(s / 60);
  const sec = s % 60;
  const parts: string[] = [];
  if (m) parts.push(`${m} ${m === 1 ? 'minute' : 'minutes'}`);
  if (sec || !m) parts.push(`${sec} ${sec === 1 ? 'second' : 'seconds'}`);
  return parts.join(' ');
}
