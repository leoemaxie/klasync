/**
 * Stale-While-Revalidate (SWR) fetching helper.
 *
 * 1. Immediately returns cached data from IndexedDB/getter if present (0ms UI latency).
 * 2. Asynchronously fetches fresh data from the remote backend API in the background.
 * 3. Updates local IndexedDB cache and triggers onData callback with fresh data.
 */
export async function fetchWithSWR<T>(options: {
  getCached: () => Promise<T | null | undefined>;
  fetchRemote: () => Promise<T>;
  setCached: (data: T) => Promise<void>;
  onData: (data: T, isFromCache: boolean) => void;
  onError?: (error: any) => void;
}): Promise<T> {
  const { getCached, fetchRemote, setCached, onData, onError } = options;

  let hasCachedData = false;

  // Step 1: Instant Cache Read
  try {
    const cached = await getCached();
    if (cached != null) {
      hasCachedData = true;
      onData(cached, true);
    }
  } catch {
    // Ignore cache read errors gracefully
  }

  // Step 2: Background Network Sync
  try {
    const fresh = await fetchRemote();
    await setCached(fresh);
    onData(fresh, false);
    return fresh;
  } catch (err) {
    if (!hasCachedData && onError) {
      onError(err);
    }
    throw err;
  }
}
