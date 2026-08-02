import { http } from './http';

export type LifecyclePatch = {
  title?: string;
  scheduled_start_at?: string;
  timezone?: string;
};

export type LifecycleView = {
  id: string;
  title: string;
  status: string;
  scheduled_start_at?: string;
  timezone: string;
  archived_at?: string;
  deleted_at?: string;
  reopen_count: number;
};

export function updateSessionLifecycle(
  shortCode: string,
  input: LifecyclePatch
): Promise<LifecycleView> {
  return http<LifecycleView>(
    `/sessions/code/${encodeURIComponent(shortCode)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(input),
    }
  );
}

export function pauseSession(shortCode: string): Promise<LifecycleView> {
  return http<LifecycleView>(
    `/sessions/code/${encodeURIComponent(shortCode)}/pause`,
    {
      method: 'POST',
    }
  );
}

export function resumeSession(shortCode: string): Promise<LifecycleView> {
  return http<LifecycleView>(
    `/sessions/code/${encodeURIComponent(shortCode)}/resume`,
    {
      method: 'POST',
    }
  );
}

export function archiveSession(shortCode: string): Promise<LifecycleView> {
  return http<LifecycleView>(
    `/sessions/code/${encodeURIComponent(shortCode)}/archive`,
    {
      method: 'POST',
    }
  );
}

export function reopenSession(shortCode: string): Promise<LifecycleView> {
  return http<LifecycleView>(
    `/sessions/code/${encodeURIComponent(shortCode)}/reopen`,
    {
      method: 'POST',
    }
  );
}

export function deleteSession(shortCode: string): Promise<void> {
  return http<void>(`/sessions/code/${encodeURIComponent(shortCode)}/delete`, {
    method: 'POST',
  });
}
