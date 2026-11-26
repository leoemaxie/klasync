#!/usr/bin/env python3
"""Gemini text-to-speech for the demo.

    python3 tts.py lecture      # lecture.txt  -> lecture.wav / lecture48.wav (the lecturer's voice)
    python3 tts.py narration    # narration.json -> out/narr/<segment>.wav (the narrator)

Set GEMINI_API_KEY unless the environment injects credentials (a proxy does in
the cloud sandbox). Output is 24 kHz mono PCM wrapped as WAV.
"""
import base64, json, os, subprocess, sys, time, urllib.request, wave

HERE = os.path.dirname(os.path.abspath(__file__))
# gemini-3.8-flash-tts pads short lines with noise, so both voices use the 3.1 preview.
LECTURE_MODEL = os.environ.get("LECTURE_TTS_MODEL", "gemini-3.1-flash-tts-preview")
NARRATION_MODEL = os.environ.get("NARRATION_TTS_MODEL", "gemini-3.1-flash-tts-preview")


LECTURE_STYLE = "Accent: Nigerian English, as spoken by a university lecturer in Nigeria. Tone: calm, clear and authoritative, at a natural measured teaching pace.\n\nTranscript:"


def endpoint(model):
    return f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"


def speak(text, voice, out_wav, style="", model=None, retries=4):
    body = {
        "contents": [{"parts": [{"text": (style + " " if style else "") + text}]}],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
            "speechConfig": {"voiceConfig": {"prebuiltVoiceConfig": {"voiceName": voice}}},
        },
    }
    headers = {"content-type": "application/json"}
    if os.environ.get("GEMINI_API_KEY"):
        headers["x-goog-api-key"] = os.environ["GEMINI_API_KEY"]
    for attempt in range(retries):
        try:
            req = urllib.request.Request(endpoint(model or LECTURE_MODEL), data=json.dumps(body).encode(), headers=headers)
            data = json.load(urllib.request.urlopen(req, timeout=300))
            part = data["candidates"][0]["content"]["parts"][0]["inlineData"]
            break
        except Exception as error:  # transient 5xx / empty candidates: retry
            if attempt == retries - 1:
                raise
            print("retry:", error)
            time.sleep(3 * (attempt + 1))
    pcm = base64.b64decode(part["data"])
    with wave.open(out_wav, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(24000); w.writeframes(pcm)
    return len(pcm) / 48000


def lecture():
    text = open(os.path.join(HERE, "lecture.txt")).read().strip()
    out = os.path.join(HERE, "lecture.wav")
    secs = speak(text, "Charon", out, LECTURE_STYLE, LECTURE_MODEL)
    # Chrome's fake microphone wants 48 kHz.
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", out, "-ar", "48000", "-ac", "1", os.path.join(HERE, "lecture48.wav")], check=True)
    print(f"lecture: {secs:.1f}s")


def narration():
    cfg = json.load(open(os.path.join(HERE, "narration.json")))
    os.makedirs(os.path.join(HERE, "out", "narr"), exist_ok=True)
    for key, text in cfg["lines"].items():
        out = os.path.join(HERE, "out", "narr", key + ".wav")
        if os.path.exists(out) and "--force" not in sys.argv:
            continue
        # Occasionally a clip trails off into noise; a sane line runs ~2.5 words/s.
        limit = len(text.split()) * 0.7 + 2.5
        # The safety filter now and then rejects harmless lines for a given prompt
        # wording; fall back to simpler accent wording rather than failing.
        styles = [cfg["style"], "Say in a warm Nigerian English accent:", "Say in a Nigerian English accent:"]
        for attempt in range(6):
            style = styles[min(attempt // 2, len(styles) - 1)]
            try:
                secs = speak(text, cfg["voice"], out, style, NARRATION_MODEL, retries=1)
            except Exception as error:
                print(f"{key}: attempt {attempt + 1} failed ({error}), trying again")
                continue
            if secs <= limit:
                break
            print(f"{key}: {secs:.1f}s is too long for {len(text.split())} words, retrying")
        else:
            sys.exit(f"{key}: could not generate narration")
        print(f"{key}: {secs:.1f}s")


if __name__ == "__main__":
    {"lecture": lecture, "narration": narration}.get(sys.argv[1] if len(sys.argv) > 1 else "", lambda: sys.exit(__doc__))()
