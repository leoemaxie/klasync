import type { Resource } from './archive';
import { API_BASE, WS_BASE } from './http';

export async function uploadSessionAudio(
  shortCode: string,
  file: File
): Promise<Resource> {
  const formData = new FormData();
  formData.append('file', file);
  const response = await fetch(
    `${API_BASE}/sessions/code/${encodeURIComponent(shortCode)}/audio/upload`,
    {
      method: 'POST',
      credentials: 'include',
      body: formData,
    }
  );
  if (!response.ok) {
    const payload = await response.json().catch(() => ({}));
    throw new Error(payload.error ?? 'Audio file upload failed');
  }
  return response.json() as Promise<Resource>;
}

export function getAudioChunkUrl(shortCode: string, seq: number): string {
  return `${API_BASE}/sessions/code/${encodeURIComponent(shortCode)}/audio/chunk/${seq}`;
}

export async function uploadAudioChunk(
  shortCode: string,
  chunk: Blob,
  seq: number
): Promise<void> {
  const formData = new FormData();
  formData.append('chunk', chunk);
  formData.append('seq', seq.toString());
  await fetch(
    `${API_BASE}/sessions/code/${encodeURIComponent(shortCode)}/audio/chunk`,
    {
      method: 'POST',
      credentials: 'include',
      body: formData,
    }
  );
}

export interface IngestedCaption {
  text: string;
  speaker?: string;
  timestamp?: string;
}

export interface AudioWebSocketOptions {
  shortCode: string;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (event: Event) => void;
  onCaptionIngested?: (caption: IngestedCaption) => void;
}

export interface AudioStreamer {
  stop: () => void;
  isStreaming: () => boolean;
}

export function connectAudioWebSocket(options: AudioWebSocketOptions) {
  const wsUrl = `${WS_BASE}/sessions/code/${encodeURIComponent(options.shortCode)}/audio/ws`;

  const ws = new WebSocket(wsUrl);
  ws.binaryType = 'arraybuffer';

  ws.onopen = () => {
    options.onOpen?.();
  };

  ws.onclose = () => {
    options.onClose?.();
  };

  ws.onerror = (evt) => {
    options.onError?.(evt);
  };

  ws.onmessage = (event) => {
    try {
      if (typeof event.data === 'string') {
        const payload = JSON.parse(event.data);
        if (payload && (payload.text || payload.caption)) {
          options.onCaptionIngested?.({
            text: payload.text ?? payload.caption,
            speaker: payload.speaker ?? 'AI Live STT',
            timestamp: payload.timestamp ?? new Date().toISOString(),
          });
        }
      }
    } catch {
      /* ignore non-JSON messages */
    }
  };

  return {
    sendAudioChunk: async (data: Blob | ArrayBuffer) => {
      if (ws.readyState === WebSocket.OPEN) {
        if (data instanceof Blob) {
          const buffer = await data.arrayBuffer();
          ws.send(buffer);
        } else {
          ws.send(data);
        }
      }
    },
    close: () => {
      if (
        ws.readyState === WebSocket.OPEN ||
        ws.readyState === WebSocket.CONNECTING
      ) {
        ws.close();
      }
    },
    get readyState() {
      return ws.readyState;
    },
  };
}

export function getSupportedAudioMimeType(): string | undefined {
  if (typeof MediaRecorder === 'undefined') return undefined;
  const types = [
    'audio/webm;codecs=opus',
    'audio/webm',
    'audio/ogg;codecs=opus',
    'audio/mp4',
    'audio/aac',
  ];
  for (const t of types) {
    if (MediaRecorder.isTypeSupported(t)) {
      return t;
    }
  }
  return undefined;
}

export function startMicrophoneAudioStream(
  shortCode: string,
  mediaStream: MediaStream,
  onCaptionIngested?: (caption: IngestedCaption) => void,
  onError?: (err: Error) => void
): AudioStreamer {
  const wsClient = connectAudioWebSocket({
    shortCode,
    onError: () => onError?.(new Error('Audio WebSocket error')),
    onCaptionIngested,
  });

  const mimeType = getSupportedAudioMimeType();
  let mediaRecorder: MediaRecorder | null = null;
  let active = true;

  try {
    mediaRecorder = new MediaRecorder(
      mediaStream,
      mimeType ? { mimeType } : undefined
    );

    mediaRecorder.ondataavailable = async (event: BlobEvent) => {
      if (active && event.data && event.data.size > 0) {
        try {
          await wsClient.sendAudioChunk(event.data);
        } catch {
          /* ignore send failure */
        }
      }
    };

    mediaRecorder.start(250);
  } catch (err) {
    onError?.(
      err instanceof Error ? err : new Error('Failed to start MediaRecorder')
    );
    wsClient.close();
    active = false;
  }

  // WebSpeechRecognition fallback for real-time speech transcription
  let recognition: any = null;
  const SpeechRecognition =
    typeof window !== 'undefined'
      ? (window as any).SpeechRecognition ||
        (window as any).webkitSpeechRecognition
      : null;

  if (SpeechRecognition) {
    try {
      recognition = new SpeechRecognition();
      recognition.continuous = true;
      recognition.interimResults = false;
      recognition.lang = 'en-US';

      recognition.onresult = (event: any) => {
        for (let i = event.resultIndex; i < event.results.length; ++i) {
          if (event.results[i].isFinal) {
            const transcript = event.results[i][0].transcript.trim();
            if (transcript) {
              onCaptionIngested?.({
                text: transcript,
                speaker: 'Lecturer Mic',
                timestamp: new Date().toISOString(),
              });
            }
          }
        }
      };

      recognition.onerror = () => {};
      recognition.start();
    } catch {}
  }

  return {
    stop: () => {
      active = false;
      if (recognition) {
        try {
          recognition.stop();
        } catch {}
      }
      if (mediaRecorder && mediaRecorder.state !== 'inactive') {
        try {
          mediaRecorder.stop();
        } catch {}
      }
      wsClient.close();
    },
    isStreaming: () => active && wsClient.readyState === WebSocket.OPEN,
  };
}
