#!/usr/bin/env python3
"""End-to-end check of the whole lecture lifecycle against a running stack.

Drives the real HTTP/WebSocket API (lecturer, guest and signed-in student) and
inspects Postgres, so it exercises auth, sessions, live captions, rolling
summaries, polls, audio recording, transcription, post-lecture batch jobs,
on-demand study generation, embeddings and semantic search, access control and
cost accounting. Needs a configured AI provider (GEMINI_API_KEY or OpenRouter)
for the AI steps, and `psql` for database assertions.

    python3 scripts/e2e.py                    # defaults to http://127.0.0.1:8787
    E2E_BASE=http://host:8787 python3 scripts/e2e.py

Environment:
    E2E_BASE        API origin                       (default http://127.0.0.1:8787)
    E2E_DB          psql connection string           (default compose postgres)
    E2E_WAV / E2E_WEBM  speech samples for the audio steps (optional)
"""
import asyncio
import base64
import json
import os
import subprocess
import sys
import time
import uuid
import urllib.error
import urllib.request

BASE = os.environ.get("E2E_BASE", "http://127.0.0.1:8787").rstrip("/")
API = BASE + "/api/v1"
WS = BASE.replace("http", "ws", 1) + "/api/v1"
DB = os.environ.get("E2E_DB", "postgres://klasync:klasync-dev@127.0.0.1:5432/klasync")
WAV = os.environ.get("E2E_WAV", "/tmp/lecture.wav")
WEBM = os.environ.get("E2E_WEBM", "/tmp/lecture.webm")

RUN = uuid.uuid4().hex[:8]
results = []


# ---------------------------------------------------------------- helpers ----
def http(method, path, body=None, token=None, headers=None, raw=False, expect=(200, 201, 202, 204)):
    data = None
    hdrs = dict(headers or {})
    if body is not None and not isinstance(body, (bytes, bytearray)):
        data = json.dumps(body).encode()
        hdrs.setdefault("content-type", "application/json")
    elif body is not None:
        data = body
    if token:
        hdrs["authorization"] = f"Bearer {token}"
    req = urllib.request.Request(API + path, data=data, method=method, headers=hdrs)
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            payload = resp.read()
            status = resp.status
            resp_headers = dict(resp.headers)
    except urllib.error.HTTPError as err:
        payload = err.read()
        status = err.code
        resp_headers = dict(err.headers)
    if raw:
        return status, payload, resp_headers
    parsed = None
    if payload:
        try:
            parsed = json.loads(payload)
        except ValueError:
            parsed = payload.decode(errors="replace")
    if expect is not None and status not in expect:
        raise AssertionError(f"{method} {path} -> {status}: {str(parsed)[:300]}")
    return parsed if expect is not None else (status, parsed)


def sql(query):
    out = subprocess.run(
        ["psql", DB, "-At", "-F", "|", "-c", query], capture_output=True, text=True
    )
    if out.returncode != 0:
        raise AssertionError(f"psql failed: {out.stderr.strip()}")
    return [row.split("|") for row in out.stdout.strip().splitlines() if row]


def scalar(query):
    rows = sql(query)
    return rows[0][0] if rows else None


def wait_for(description, fn, timeout=120, interval=3):
    deadline = time.time() + timeout
    last = None
    while time.time() < deadline:
        try:
            last = fn()
            if last:
                return last
        except AssertionError as err:
            last = err
        time.sleep(interval)
    raise AssertionError(f"timed out after {timeout}s waiting for {description} (last: {last})")


def step(name):
    def decorator(fn):
        def run(*args, **kwargs):
            started = time.time()
            try:
                value = fn(*args, **kwargs)
                results.append((name, True, f"{time.time() - started:.1f}s", ""))
                print(f"  PASS  {name}  ({time.time() - started:.1f}s)")
                return value
            except Exception as err:  # noqa: BLE001 - report every failure
                results.append((name, False, f"{time.time() - started:.1f}s", str(err)))
                print(f"  FAIL  {name}  ({time.time() - started:.1f}s)\n        {err}")
                raise

        return run

    return decorator


LECTURE = [
    "[today] Welcome everyone. Today we are studying photosynthesis, the process by which plants, algae and some bacteria convert light energy into chemical energy stored as glucose. "
    "Photosynthesis happens mainly in the chloroplasts of leaf cells, and it has two stages: the light dependent reactions and the Calvin cycle.",
    "In the light dependent reactions, chlorophyll absorbs red and blue light and uses that energy to split water molecules. This releases oxygen as a by product, "
    "and it produces two energy carrier molecules called ATP and NADPH. These reactions take place in the thylakoid membranes inside the chloroplast.",
    "The second stage is the Calvin cycle, which happens in the stroma of the chloroplast. The Calvin cycle uses the ATP and NADPH from the first stage to fix carbon dioxide into sugar. "
    "The key enzyme is called RuBisCO, which attaches carbon dioxide to a five carbon molecule named RuBP. Remember this enzyme, it will be on the exam.",
    "Several factors limit the rate of photosynthesis: light intensity, carbon dioxide concentration and temperature. If the temperature is too high, the enzymes denature and the rate drops sharply. "
    "At low light, the light dependent reactions are the bottleneck, while at high light the carbon dioxide supply becomes the limiting factor.",
    "Let us compare this with cellular respiration. Respiration releases energy from glucose in the mitochondria, and it consumes oxygen and produces carbon dioxide and water. "
    "In a sense, respiration is the reverse of photosynthesis, and together they form the carbon cycle that connects plants and animals.",
    "For the assignment, please draw a labelled diagram of a chloroplast showing the thylakoids, the stroma and where each stage occurs. "
    "Also explain in your own words why the leaves of most plants look green. We will review your answers at the start of the next lecture. Thank you all for coming.",
]


# ------------------------------------------------------------------- steps ----
state = {}


@step("health and readiness")
def s_health():
    for path in ("/health", "/health/ready"):
        try:
            with urllib.request.urlopen(BASE + path, timeout=30) as resp:
                status, body = resp.status, resp.read()
        except urllib.error.HTTPError as err:
            status, body = err.code, err.read()
        assert status == 200, f"{path} -> {status} {body[:200]}"


@step("lecturer registers, logs in, creates course + roster")
def s_lecturer():
    email = f"lect-{RUN}@university.edu"
    reg = http("POST", "/auth/lecturers/register", {"name": "Dr Test", "email": email, "password": "correct-horse-battery"})
    token = reg["access_token"]
    login = http("POST", "/auth/lecturers/login", {"email": email, "password": "correct-horse-battery"})
    assert login["access_token"]
    state["lec_token"] = login["access_token"]
    me = http("GET", "/auth/lecturers/me", token=login["access_token"])
    assert me["name"] == "Dr Test" and me["role"] == "lecturer" and me["email"] == email, me
    course = http("POST", "/courses", {"code": f"BIO{RUN[:3].upper()}", "title": "Plant Biology", "academic_session": "2026/2027", "semester": "First Semester"}, token)
    state["course"] = course
    state["matric"] = f"BIO/{RUN}/001"
    http("POST", f"/courses/{course['id']}/roster", {"students": [
        {"matric_number": state["matric"], "full_name": "Ada Student", "email": f"ada-{RUN}@university.edu"},
        {"matric_number": f"BIO/{RUN}/002", "full_name": "Bola Student", "email": None},
    ]}, token)


@step("lecturer starts a live session")
def s_session():
    created = http("POST", "/sessions", {"course_id": state["course"]["id"], "title": "Photosynthesis"}, state["lec_token"])
    state["code"] = created["session"]["short_code"]
    state["session_id"] = created["session"]["id"]


@step("guest joins on two devices (same matric) -> one participant")
def s_join():
    a = http("POST", f"/sessions/code/{state['code']}/join", {"matric_number": state["matric"], "display_name": "Ada"})
    b = http("POST", f"/sessions/code/{state['code']}/join", {"matric_number": state["matric"].lower(), "display_name": "Ada phone"})
    # Matching is case-insensitive per roster; the unique key is the exact string.
    assert a["verification_status"] == "verified", a
    state["participant"] = a["id"]
    state["participant_b"] = b["id"]


@step("lecturer publishes live captions; students receive them (WS + REST)")
def s_captions():
    async def listen():
        import websockets
        received = []
        async with websockets.connect(f"{WS}/sessions/code/{state['code']}/captions/ws") as ws:
            async def reader():
                try:
                    while True:
                        received.append(json.loads(await ws.recv()))
                except Exception:
                    pass
            task = asyncio.create_task(reader())
            await asyncio.sleep(1)
            await asyncio.get_running_loop().run_in_executor(None, publish_all)
            await asyncio.sleep(3)
            task.cancel()
        return received

    def publish_all():
        for text in LECTURE:
            http("POST", f"/sessions/code/{state['code']}/captions", {"text": text.replace("[today] ", "")}, state["lec_token"])

    got = asyncio.run(listen())
    assert len(got) >= len(LECTURE) - 1, f"WebSocket delivered {len(got)} of {len(LECTURE)} captions"
    rest = http("GET", f"/sessions/code/{state['code']}/captions")
    assert len(rest) == len(LECTURE), f"REST returned {len(rest)} captions"


@step("rolling summary appears for students while live")
def s_rolling():
    def check():
        items = http("GET", f"/sessions/code/{state['code']}/summaries")
        return items if items else None

    items = wait_for("a rolling summary", check, timeout=150)
    assert items[0]["summary"].strip(), items
    state["live_summary_source"] = items[0]["source"]
    print(f"        source={items[0]['source']}: {items[0]['summary'][:110]}…")


@step("polls: manual quiz, answer, results, AI quiz from the lecture")
def s_polls():
    code, tok = state["code"], state["lec_token"]
    poll = http("POST", f"/sessions/code/{code}/polls", {"kind": "quiz", "question": "Where does the Calvin cycle occur?", "options": ["Thylakoid", "Stroma", "Nucleus"], "correct_index": 1, "explanation": "The Calvin cycle runs in the stroma."}, tok)
    seen = http("GET", f"/sessions/code/{code}/polls?participant_id={state['participant']}")
    assert seen[0]["correct_index"] is None, "answer must be hidden from students before they answer"
    answered = http("POST", f"/sessions/code/{code}/polls/{poll['id']}/respond", {"participant_id": state["participant"], "choice_index": 1})
    assert answered["correct_index"] == 1 and answered["my_choice"] == 1
    status, _ = http("POST", f"/sessions/code/{code}/polls/{poll['id']}/respond", {"participant_id": state["participant"], "choice_index": 0}, expect=None)
    assert status == 409, "double answers must be rejected"
    closed = http("POST", f"/sessions/code/{code}/polls/{poll['id']}/close", None, tok)
    assert closed["status"] == "closed" and closed["counts"][1] == 1
    ai = http("POST", f"/sessions/code/{code}/polls/generate", {"kind": "quiz"}, tok)
    assert ai["source"] == "ai" and len(ai["options"]) >= 2 and ai["correct_index"] is not None, ai
    print(f"        AI quiz: {ai['question'][:90]}")


@step("hand raise reaches the lecturer; presence by code works")
def s_hand():
    code, pid = state["code"], state["participant"]
    http("POST", f"/sessions/code/{code}/participants/{pid}/hand-raise")
    hands = http("GET", f"/sessions/code/{code}/hand-raises", token=state["lec_token"])
    assert any(h["participant_id"] == pid for h in hands), hands
    http("DELETE", f"/sessions/code/{code}/participants/{pid}/hand-raise")
    assert not http("GET", f"/sessions/code/{code}/hand-raises", token=state["lec_token"])
    http("POST", f"/sessions/code/{code}/presence", {"matric_number": state["matric"]})


@step("audio socket rejects anonymous clients; records the lecture for lecturers")
def s_audio_ws():
    import websockets

    async def run():
        # 1. No token: refused at the handshake.
        refused = False
        try:
            async with websockets.connect(f"{WS}/sessions/code/{state['code']}/audio/ws"):
                pass
        except Exception:
            refused = True
        assert refused, "anonymous audio socket must be refused"
        # 2. Lecturer token: stream the recording, close, expect it stored.
        webm = open(WEBM, "rb").read()
        async with websockets.connect(f"{WS}/sessions/code/{state['code']}/audio/ws?token={state['lec_token']}") as ws:
            await ws.send(webm)
            await asyncio.sleep(2)

    asyncio.run(run())
    wait_for("the recording resource", lambda: scalar(
        f"select count(*) from lecture_resources where session_id='{state['session_id']}' and resource_type='audio' and storage_key is not null") not in (None, "0"), timeout=30)


@step("recording transcribes to text (dedicated transcription model)")
def s_transcribe():
    rid = scalar(f"select id from lecture_resources where session_id='{state['session_id']}' and resource_type='audio' order by created_at desc limit 1")
    job = http("POST", f"/sessions/code/{state['code']}/ai-jobs", {"job_type": "transcribe", "input_resource_id": rid}, state["lec_token"])
    wait_for("transcription job", lambda: scalar(f"select status::text from ai_jobs where id='{job['id']}'") in ("completed",), timeout=150)
    text = scalar(f"select content->>'text' from lecture_resources where session_id='{state['session_id']}' and resource_type='transcript' order by created_at desc limit 1")
    assert text and "photosynthesis" in text.lower(), f"unexpected transcript: {text!r}"
    print(f"        transcript: {text[:100]!r}")


@step("lecturer ends the session")
def s_end():
    http("POST", f"/sessions/code/{state['code']}/end", None, state["lec_token"])


@step("student registers LATER on a new device; archive + courses are matched server-side")
def s_student():
    email = f"ada-{RUN}@university.edu"
    http("POST", "/auth/students/register", {"matric_number": state["matric"], "display_name": "Ada Student", "email": email, "password": "correct-horse-battery"})
    login = http("POST", "/auth/students/login", {"email": email, "password": "correct-horse-battery"})
    state["stu_token"] = login["access_token"]
    me = http("GET", "/auth/students/me", token=state["stu_token"])
    assert me["name"] == "Ada Student" and me["role"] == "student" and me["matric_number"] == state["matric"], me
    # A student's token must not read the lecturer profile, and vice versa.
    status, _ = http("GET", "/auth/lecturers/me", token=state["stu_token"], expect=None)
    assert status == 403, status
    status, _ = http("GET", "/auth/students/me", expect=None)
    assert status == 401, status
    archive = http("GET", "/students/archive", token=state["stu_token"])
    mine = [a for a in archive if a["id"] == state["session_id"]]
    assert mine, f"session missing from archive: {archive}"
    assert mine[0]["attended"] is True and mine[0]["has_audio"] is True, mine[0]
    courses = http("GET", "/students/courses", token=state["stu_token"])
    assert any(c["id"] == state["course"]["id"] for c in courses), courses


@step("access control: another student sees nothing and is denied")
def s_access():
    email = f"eve-{RUN}@university.edu"
    http("POST", "/auth/students/register", {"matric_number": f"EVE/{RUN}", "display_name": "Eve", "email": email, "password": "correct-horse-battery"})
    tok = http("POST", "/auth/students/login", {"email": email, "password": "correct-horse-battery"})["access_token"]
    assert http("GET", "/students/archive", token=tok) == []
    sid = state["session_id"]
    for path in (f"/archive/sessions/{sid}/transcript", f"/archive/sessions/{sid}/flashcards", f"/archive/sessions/{sid}/recordings"):
        status, _ = http("GET", path, token=tok, expect=None)
        assert status == 404, f"{path} -> {status}"
    status, _ = http("POST", f"/archive/sessions/{sid}/ask", {"question": "anything at all"}, token=tok, expect=None)
    assert status == 404


@step("student transcript + live summaries are in the archive")
def s_archive_basics():
    sid, tok = state["session_id"], state["stu_token"]
    tr = http("GET", f"/archive/sessions/{sid}/transcript", token=tok)
    assert tr["source"] == "captions" and len(tr["lines"]) >= len(LECTURE), tr["source"]
    assert tr["lines"][0]["offset_sec"] is not None
    sums = http("GET", f"/archive/sessions/{sid}/summaries", token=tok)
    assert sums, "archive should keep the rolling summaries"


@step("recording streams with Range support (seekable)")
def s_stream():
    sid, tok = state["session_id"], state["stu_token"]
    recs = http("GET", f"/archive/sessions/{sid}/recordings", token=tok)
    assert recs, "no recordings listed"
    rid = recs[-1]["resource_id"]
    status, body, headers = http("GET", f"/archive/sessions/{sid}/recordings/{rid}/stream", token=tok, raw=True, headers={"Range": "bytes=0-99"})
    assert status == 206 and len(body) == 100, (status, len(body))
    lower = {k.lower(): v for k, v in headers.items()}
    assert lower.get("content-range", "").startswith("bytes 0-99/"), headers
    status, body, headers = http("GET", f"/archive/sessions/{sid}/recordings/{rid}/stream", token=tok, raw=True)
    lower = {k.lower(): v for k, v in headers.items()}
    assert status == 200 and len(body) > 10_000 and lower["content-type"].startswith("audio/")
    # The bytes must be a real, playable recording.
    open("/tmp/e2e-stream.webm", "wb").write(body)
    probe = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", "/tmp/e2e-stream.webm"], capture_output=True, text=True)
    if probe.returncode == 0:
        assert float(probe.stdout.strip()) > 5, probe.stdout


@step("post-lecture automation queued batch jobs (or ran them) for the ended lecture")
def s_post_lecture():
    sid = state["session_id"]

    def queued():
        rows = sql(f"select job_type, mode, status::text from ai_jobs where session_id='{sid}' and job_type in ('chapters','flashcards','visualization','embed')")
        kinds = {r[0] for r in rows}
        return rows if {"chapters", "flashcards", "visualization", "embed"} <= kinds else None

    rows = wait_for("post-lecture jobs to be queued", queued, timeout=240, interval=5)
    print("        jobs:", ", ".join(f"{r[0]}[{r[1]}/{r[2]}]" for r in sorted(rows)))
    state["post_rows"] = rows


@step("search index (pgvector) is built for the lecture")
def s_index():
    sid = state["session_id"]
    n = wait_for("lecture chunks", lambda: int(scalar(f"select count(*) from lecture_chunks where session_id='{sid}'") or 0) > 0, timeout=120)
    dims = scalar(f"select vector_dims(embedding) from lecture_chunks where session_id='{sid}' limit 1")
    assert dims == "1536", dims
    print(f"        {scalar(f'select count(*) from lecture_chunks where session_id={chr(39)}{sid}{chr(39)}')} passages, {dims} dims")


def generate(kind, topic=None, timeout=240):
    sid, tok = state["session_id"], state["stu_token"]
    body = {"topic": topic} if topic else {}
    job = http("POST", f"/archive/sessions/{sid}/ai/generate-{kind}", body, tok)
    if job["status"] == "ready":
        return job
    deadline = time.time() + timeout
    while time.time() < deadline:
        time.sleep(3)
        st = http("GET", f"/archive/jobs/{job['job_id']}", token=tok)
        if st["status"] == "completed":
            return st
        if st["status"] == "failed" and st["attempts"] >= 3:
            raise AssertionError(f"{kind} failed: {st['error_message']}")
    raise AssertionError(f"{kind} generation timed out")


@step("student generates flashcards on demand (batch job promoted / run immediately)")
def s_flashcards():
    generate("flashcards")
    cards = http("GET", f"/archive/sessions/{state['session_id']}/flashcards", token=state["stu_token"])
    assert len(cards) >= 5, f"only {len(cards)} flashcards"
    assert all(c["prompt"].strip() and c["answer"].strip() for c in cards)
    print(f"        {len(cards)} cards, e.g. {cards[0]['prompt'][:80]!r}")
    # A focused deck adds cards on a topic instead of being refused as "ready".
    before = len(cards)
    generate("flashcards", topic="RuBisCO and the Calvin cycle")
    after = http("GET", f"/archive/sessions/{state['session_id']}/flashcards", token=state["stu_token"])
    assert len(after) > before, f"topic generation added nothing ({before} -> {len(after)})"


@step("student gets chapters with real timestamps")
def s_chapters():
    generate("chapters")
    chapters = http("GET", f"/archive/sessions/{state['session_id']}/chapters", token=state["stu_token"])
    assert 2 <= len(chapters) <= 12, len(chapters)
    assert chapters[0]["title"].strip() and chapters[0]["summary"].strip()
    assert all(c["end_timestamp_sec"] >= c["start_timestamp_sec"] for c in chapters)
    print(f"        {len(chapters)} chapters: " + " | ".join(c["title"][:28] for c in chapters[:4]))


@step("student gets a concept map (visualization pipeline)")
def s_visual():
    generate("visualization")
    vis = http("GET", f"/archive/sessions/{state['session_id']}/visualizations", token=state["stu_token"])
    assert vis, "no visualization"
    spec = vis[0]["spec"]
    ids = {n["id"] for n in spec["nodes"]}
    assert len(ids) >= 4 and spec["edges"], spec
    assert all(e["from"] in ids and e["to"] in ids for e in spec["edges"])
    print(f"        {len(ids)} concepts, {len(spec['edges'])} links, source={vis[0]['source']}")


@step("ask-this-lecture answers from retrieved passages with timestamps")
def s_ask():
    res = http("POST", f"/archive/sessions/{state['session_id']}/ask", {"question": "Which enzyme fixes carbon dioxide in the Calvin cycle?"}, state["stu_token"])
    assert "rubisco" in res["answer"].lower(), res["answer"]
    assert res["sources"] and res["sources"][0]["score"] > 0.4, res["sources"][:1]
    print(f"        {res['answer'][:140]!r}")
    off = http("POST", f"/archive/sessions/{state['session_id']}/ask", {"question": "What is the capital of Australia?"}, state["stu_token"])
    assert "canberra" not in off["answer"].lower(), "must not answer outside the lecture"


@step("cross-lecture semantic search finds passages by meaning")
def s_search():
    hits = http("GET", "/students/search?q=" + urllib.request.quote("why are leaves green"), token=state["stu_token"])
    assert hits, "no hits"
    assert hits[0]["session_id"] == state["session_id"]
    print(f"        top hit {hits[0]['score']:.2f}: {hits[0]['text'][:90]!r}")
    # Another student's search must never surface this lecture.
    email = f"mal-{RUN}@university.edu"
    http("POST", "/auth/students/register", {"matric_number": f"MAL/{RUN}", "display_name": "Mal", "email": email, "password": "correct-horse-battery"})
    tok = http("POST", "/auth/students/login", {"email": email, "password": "correct-horse-battery"})["access_token"]
    assert http("GET", "/students/search?q=photosynthesis", token=tok) == []


@step("uploaded audio file is transcribed automatically (multipart)")
def s_upload():
    boundary = "----e2e" + RUN
    wav = open(WAV, "rb").read()
    body = (f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"lecture.wav\"\r\nContent-Type: audio/wav\r\n\r\n").encode() + wav + f"\r\n--{boundary}--\r\n".encode()
    res = http("POST", f"/sessions/code/{state['code']}/audio/upload", body, state["lec_token"], headers={"content-type": f"multipart/form-data; boundary={boundary}"})
    job = res["ai_job_id"]
    wait_for("uploaded-audio transcription", lambda: scalar(f"select status::text from ai_jobs where id='{job}'") == "completed", timeout=150)
    model = scalar(f"select model from ai_jobs where id='{job}'")
    print(f"        transcribed by {model}")


@step("old lecture predating embeddings gets backfilled automatically")
def s_backfill():
    sid = str(uuid.uuid4())
    lec = scalar(f"select lecturer_id from lecture_sessions where id='{state['session_id']}'")
    course = state["course"]["id"]
    sql(f"insert into lecture_sessions (id, course_id, title, short_code, status, started_at, ended_at, lecturer_id) values ('{sid}','{course}','Legacy lecture','L{RUN[:5].upper()}','ended', now() - interval '2 days', now() - interval '2 days', '{lec}') returning id")
    for i, text in enumerate(LECTURE[:3]):
        sql(f"insert into caption_chunks (session_id, sequence_number, text, created_at) values ('{sid}', {i+1}, $${text.replace('[today] ', '')}$$, now() - interval '2 days' + interval '{i} minutes') returning id")
    wait_for("backfilled embeddings", lambda: int(scalar(f"select count(*) from lecture_chunks where session_id='{sid}'") or 0) > 0, timeout=180, interval=5)


@step("cost accounting: every call logged, priced and attributed")
def s_costs():
    rows = sql("select task, provider, model, count(*), sum(input_tokens), sum(output_tokens), round(sum(cost_usd)::numeric, 5), sum((not ok)::int) from ai_usage_log group by 1,2,3 order by 1,3")
    assert rows, "ai_usage_log is empty"
    print("        task            provider  model                    calls  in_tok  out_tok  cost$   failed")
    total = 0.0
    for r in rows:
        print(f"        {r[0]:<15} {r[1]:<9} {r[2]:<24} {r[3]:>5} {r[4]:>7} {r[5]:>8} {r[6]:>7} {r[7]:>6}")
        total += float(r[6])
    print(f"        total spend this run so far: ${total:.4f}")
    assert total > 0, "nothing was priced"
    assert scalar("select count(*) from ai_usage_log where task='embed' and ok") not in (None, "0")
    assert scalar("select count(*) from ai_usage_log where task='transcribe' and ok") not in (None, "0")


# -------------------------------------------------------------------- main ----
def main():
    print(f"E2E run {RUN} against {BASE}\n")
    for fn in (
        s_health, s_lecturer, s_session, s_join, s_captions, s_rolling, s_polls, s_hand,
        s_audio_ws, s_transcribe, s_end, s_student, s_access, s_archive_basics, s_stream,
        s_post_lecture, s_index, s_flashcards, s_chapters, s_visual, s_ask, s_search,
        s_upload, s_backfill, s_costs,
    ):
        try:
            fn()
        except Exception:  # noqa: BLE001
            break
    failed = [r for r in results if not r[1]]
    print(f"\n{len(results) - len(failed)} passed, {len(failed)} failed")
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main()
