import { http } from './http';
import type { AiJob } from './archive';

export function dispatchAiJob(
  shortCode: string,
  jobId: string
): Promise<AiJob> {
  return http<AiJob>(
    `/sessions/code/${encodeURIComponent(shortCode)}/ai-jobs/${encodeURIComponent(jobId)}/dispatch`,
    {
      method: 'POST',
    }
  );
}

export function getJobStatus(jobId: string): Promise<AiJob> {
  return http<AiJob>(`/jobs/${encodeURIComponent(jobId)}`);
}
