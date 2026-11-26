#!/usr/bin/env python3
"""Cuts the raw captures into the narrated demo: out/klasync_demo.mp4.

Scenes are defined at the bottom. Each scene lasts as long as its narration
needs: sped-up footage is slowed down (never below 0.8x) and held on its last
frame if the narration is still longer. In scenes that carry the lecturer's
voice, the narrator briefly ducks it, then the lecture plays on at full volume.
"""
import glob, json, os, subprocess, sys

os.chdir(os.path.dirname(os.path.abspath(__file__)))
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
FONTB = '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
SERIF = '/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf'
BG, ACC = '0x0e0905', '0xff8c3c'
NAR_TEMPO = 1.15   # narration is sped up slightly (pitch preserved) to keep the cut tight
TOOLS = ['Transcript and audio', 'Chapters', 'Summary', 'Concept map', 'Flashcards', 'Quiz  (new)', 'Ask the lecture']

LM = json.load(open('out/live_marks.json'))
SM = json.load(open('out/study_marks.json'))
L = {m['name']: m['ms'] / 1000 for m in LM['marks']}
S = {m['name']: m['ms'] / 1000 for m in SM['marks']}
OFF_D, OFF_P, OFF_S = LM['off']['desktop'] / 1000, LM['off']['phone'] / 1000, S['start'] - 0.05
VD = glob.glob('out/live_desktop/*.webm')[0]
VP = glob.glob('out/live_phone/*.webm')[0]
VS = glob.glob('out/study_phone/*.webm')[0]
AUDIO_START = L['audio_start']
os.makedirs('out/seg', exist_ok=True)


def run(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        print(r.stderr[-1500:])
        sys.exit(1)


def txt(name, s):
    p = f'out/seg/{name}.txt'
    open(p, 'w').write(s)
    return p


def ndur(name):
    p = f'out/narr/{name}.wav'
    if not os.path.exists(p):
        return 0.0
    out = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', p], capture_output=True, text=True).stdout
    return float(out.strip()) / NAR_TEMPO


def audio_chain(name, dur, first_idx, lecture=None, nd=0.0):
    """Silence bed + optional lecturer track + narration (delayed 0.4 s).
    lecture = (source_start, length, speed, mode) with mode 'duck' (under the
    narration, then full) or 'bed' (quiet throughout)."""
    ins = ['-f', 'lavfi', '-t', f'{dur:.3f}', '-i', 'anullsrc=r=48000:cl=mono']
    idx = first_idx
    parts = [f'[{idx}:a]anull[a0]']
    mix = ['[a0]']
    idx += 1
    if lecture:
        start, length, speed, mode = lecture
        ins += ['-ss', f'{max(0, start):.3f}', '-t', f'{length:.3f}', '-i', 'lecture48.wav']
        if mode == 'duck' and nd:
            nend = 0.3 + nd + 0.3
            vol = f"volume='1.7*(0.1+0.9*clip((t-{nend:.2f})/0.7,0,1))':eval=frame"
        elif mode == 'bed':
            vol = 'volume=0.4'
        else:
            vol = 'volume=1.7'
        parts.append(f'[{idx}:a]atempo={speed},{vol},afade=t=in:st=0:d=0.2,afade=t=out:st={max(0, dur - 0.4):.3f}:d=0.4[a1]')
        mix.append('[a1]')
        idx += 1
    if nd:
        ins += ['-i', f'out/narr/{name}.wav']
        parts.append(f'[{idx}:a]aresample=48000,atempo={NAR_TEMPO},adelay=300|300,volume=1.5[a2]')
        mix.append('[a2]')
        idx += 1
    parts.append(''.join(mix) + f'amix=inputs={len(mix)}:duration=first:normalize=0,alimiter=limit=0.95[aout]')
    return ins, parts, idx


def seg(name, layout, d_rng, p_rng, vp, caption, speed=1.0, lecture=None, tool=None, tail=0.6):
    """layout: D desktop only, P phone only (with studio tool list), S side by side.
    Ranges are (start, end) seconds in each source video."""
    rng = d_rng or p_rng
    span = rng[1] - rng[0]
    nd = ndur(name)
    dur = max(span / speed, nd + tail) if nd else span / speed
    print(f'  {name:7} span {span:5.1f}s  speed {speed:.1f}  footage {span/speed:5.1f}s  narration {nd:4.1f}s  -> scene {dur:5.1f}s')
    eff = max(span / dur, 0.8)
    pad = max(0.0, dur - span / eff)
    if eff < 0.95:
        print(f'  note: {name} plays at {eff:.2f}x' + (f' and holds {pad:.1f}s' if pad > 0.05 else ''))
    ins = ['-f', 'lavfi', '-t', f'{dur:.3f}', '-i', f'color=c={BG}:s=1920x1080:r=30']
    idx = 1
    last = '0:v'
    chain = []

    def prep(path, r, w, h):
        nonlocal idx
        ins.extend(['-ss', f'{max(0, r[0]):.3f}', '-t', f'{r[1] - r[0]:.3f}', '-i', path])
        i = idx
        idx += 1
        crop = 'crop=390:664:0:0,' if h > w else ''
        hold = f',tpad=stop_mode=clone:stop_duration={pad:.3f}' if pad > 0.05 else ''
        return f'[{i}:v]fps=30,{crop}setpts=(PTS-STARTPTS)/{eff:.4f}{hold},scale={w}:{h}:flags=lanczos[v{i}]', f'v{i}'

    if layout == 'D':
        f, v = prep(VD, d_rng, 1536, 960)
        chain += [f, f'[{last}][{v}]overlay=192:10[o1]']
        last = 'o1'
    elif layout == 'P':
        f, v = prep(vp, p_rng, 560, 953)
        chain += [f, f'[{last}][{v}]overlay=680:24[o1]']
        last = 'o1'
        dt = [f"drawtext=fontfile={FONTB}:text='Student  ·  phone':fontsize=26:fontcolor={ACC}:x=90:y=60"]
        if tool is not None:
            dt.append(f"drawtext=fontfile={FONTB}:text='Lecture studio':fontsize=30:fontcolor={ACC}:x=1350:y=250")
            for i, nm in enumerate(TOOLS):
                on = i == tool
                dt.append(f"drawtext=fontfile={FONTB if on else FONT}:text='{('> ' if on else '   ') + nm}':fontsize=36:fontcolor={'white' if on else '0x8a7a6a'}:x=1350:y={310 + i * 68}")
        chain.append(f'[{last}]' + ','.join(dt) + '[o2]')
        last = 'o2'
    else:
        f, v = prep(VD, d_rng, 1200, 750)
        chain += [f, f'[{last}][{v}]overlay=40:110[o1]']
        last = 'o1'
        f, v = prep(vp, p_rng, 440, 750)
        chain += [f, f'[{last}][{v}]overlay=1330:110[o2]']
        last = 'o2'
        chain.append(f"[{last}]drawtext=fontfile={FONTB}:text='Lecturer  ·  desktop':fontsize=26:fontcolor={ACC}:x=40:y=62,drawtext=fontfile={FONTB}:text='Student  ·  phone':fontsize=26:fontcolor={ACC}:x=1330:y=62[o3]")
        last = 'o3'
    cp = txt(name, caption)
    chain.append(f"[{last}]drawbox=x=0:y=1000:w=1920:h=80:color=black@0.78:t=fill,drawtext=fontfile={FONT}:textfile={cp}:fontsize=32:fontcolor=white:x=(w-text_w)/2:y=1040-text_h/2,fade=t=in:st=0:d=0.3,fade=t=out:st={dur - 0.3:.3f}:d=0.3[vout]")
    lec = None
    if lecture and layout != 'P':
        lec = (d_rng[0] - AUDIO_START, span, eff, lecture)
    ains, achain, idx = audio_chain(name, dur, idx, lec, nd)
    ins += ains
    out = f'out/seg/{name}.mp4'
    run(['ffmpeg', '-v', 'error', '-y', *ins, '-filter_complex', ';'.join(chain + achain), '-map', '[vout]', '-map', '[aout]', '-r', '30', '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '20', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-ar', '48000', '-ac', '1', '-t', f'{dur:.3f}', out])
    return out


def card(name, title, sub, bullets=(), dur=4.0):
    nd = ndur(name)
    dur = max(dur, nd + 1.2) if nd else dur
    parts = [
        f"drawtext=fontfile={SERIF}:textfile={txt(name + '_t', title)}:fontsize=110:fontcolor=0xfff0e0:x=(w-text_w)/2:y=300",
        f"drawtext=fontfile={FONT}:textfile={txt(name + '_s', sub)}:fontsize=44:fontcolor={ACC}:x=(w-text_w)/2:y=450",
    ]
    for i, b in enumerate(bullets):
        t0 = 1.4 + i * max(1.4, (dur - 4.0) / max(1, len(bullets)))
        parts.append(f"drawtext=fontfile={FONT}:textfile={txt(f'{name}_b{i}', b)}:fontsize=36:fontcolor=0xd8c8b8:x=(w-text_w)/2:y={600 + i * 62}:enable='gte(t,{t0:.2f})'")
    parts.append(f'fade=t=in:st=0:d=0.6,fade=t=out:st={dur - 0.6:.2f}:d=0.6')
    out = f'out/seg/{name}.mp4'
    ins = ['-f', 'lavfi', '-t', f'{dur:.3f}', '-i', f'color=c={BG}:s=1920x1080:r=30']
    ains, achain, _ = audio_chain(name, dur, 1, None, nd)
    run(['ffmpeg', '-v', 'error', '-y', *ins, *ains, '-filter_complex', ';'.join(['[0:v]' + ','.join(parts) + '[vout]'] + achain), '-map', '[vout]', '-map', '[aout]', '-r', '30', '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '20', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-ar', '48000', '-ac', '1', '-t', f'{dur:.3f}', out])
    return out


def d(a, b): return (a - OFF_D, b - OFF_D)
def p(a, b): return (a - OFF_P, b - OFF_P)
def s(a, b): return (a - OFF_S, b - OFF_S)
def both(a, b): return d(a, b), p(a, b)   # same moment on the two live videos


TAG = 'Assistive Platform for Inclusive and Accessible Education'
segs = []
segs.append(card('c_intro', 'KLASYNC', TAG, ['Live captions for every student', 'Searchable transcripts and study tools', 'Attendance and insight for lecturers'], dur=7.0))
segs.append(seg('s1a', 'D', d(L['login_start'] - 0.8, L['device_start']), None, None, 'Lecturer signs in and loads the class roster', speed=1.4))
segs.append(seg('s1b', 'D', d(L['device_start'], L['room_start']), None, None, 'Microphone options: laptop mic or a dedicated wireless mic', speed=1.1))
segs.append(seg('s1c', 'D', d(L['room_start'], L['session_started'] + 2.5), None, None, 'One tap starts a live session with a join code and QR code', speed=1.4))
D_, P_ = both(L['join_start'] - 0.5, L['phone_joined'] + 1.5)
segs.append(seg('s2a', 'S', D_, P_, VP, 'Students join with the code: no account, no app, works on limited data', speed=1.0))
D_, P_ = both(L['audio_start'], L['access_end'] + 0.8)
segs.append(seg('s2b', 'S', D_, P_, VP, 'Larger text and a dyslexia-friendly font for easier reading', speed=1.3, lecture='bed'))
D_, P_ = both(L['access_end'] + 0.8, L['audio_start'] + 25)
segs.append(seg('s3', 'S', D_, P_, VP, 'The lecture is captioned live, word for word', speed=1.0, lecture='duck'))
D_, P_ = both(L['audio_start'] + 25, L['lect_question_view'] + 1)
segs.append(seg('s4', 'S', D_, P_, VP, 'Check in, raise a hand, ask a question: the lecturer sees it instantly', speed=3.0))
D_, P_ = both(L['lect_question_view'] + 1, L['ai_quiz_click'] - 19)
segs.append(seg('s5', 'S', D_, P_, VP, 'A live summary lets anyone catch up without losing the thread', speed=3.4))
D_, P_ = both(L['ai_quiz_click'] - 18, L['ai_quiz_click'] - 0.5)
segs.append(seg('s6a', 'S', D_, P_, VP, 'Checking understanding on the spot', speed=4.5))
D_, P_ = both(L['quiz_live_lecturer'] - 0.5, L['lect_results'] + 3)
segs.append(seg('s6b', 'S', D_, P_, VP, 'A quick quiz from what was just taught: answers and results arrive live', speed=1.3))
segs.append(seg('s7', 'D', d(L['audio_done'] + 1, L['analytics']), None, None, 'Attendance is scored automatically and rolled into course analytics', speed=1.8))
segs.append(card('c_mid', 'After class', 'The student archive, on mobile', dur=2.0))
segs.append(seg('t1', 'P', None, s(S['signin_start'], S['transcript_start']), VS, 'A student who missed class signs in to their archive', speed=1.3, tool=0))
segs.append(seg('t2', 'P', None, s(S['transcript_start'], S['chapters_start']), VS, 'Searchable transcript, synced to the recording', speed=1.5, tool=0))
segs.append(seg('t3', 'P', None, s(S['chapters_start'], S['summary_start']), VS, 'Chapters jump to any moment in the lecture', speed=1.1, tool=1))
segs.append(seg('t3b', 'P', None, s(S['summary_start'], S['map_start']), VS, 'A summary of the key points', speed=1.4, tool=2))
segs.append(seg('t4', 'P', None, s(S['map_start'], S['cards_start']), VS, 'An interactive concept map: see how the ideas connect', speed=2.6, tool=3))
segs.append(seg('t5', 'P', None, s(S['cards_start'], S['quiz_start']), VS, 'Flashcards for short, repeatable practice', speed=2.1, tool=4))
segs.append(seg('t6', 'P', None, s(S['quiz_start'], S['ask_start']), VS, 'New: a quiz with instant feedback and an explanation for every answer', speed=3.4, tool=5))
segs.append(seg('t7', 'P', None, s(S['ask_start'], S['study_end']), VS, 'Ask the lecture: answers cite the exact moment', speed=1.5, tool=6))
segs.append(card('c_out', 'KLASYNC', TAG, ['Every lecture, for every learner.'], dur=4.5))

open('out/concat.txt', 'w').write(''.join(f"file 'seg/{os.path.basename(x)}'\n" for x in segs))
run(['ffmpeg', '-v', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', 'out/concat.txt', '-c', 'copy', 'out/klasync_demo_raw.mp4'])
# Even out loudness between the lecturer, the narrator and the quiet stretches.
run(['ffmpeg', '-v', 'error', '-y', '-i', 'out/klasync_demo_raw.mp4', '-c:v', 'copy', '-af', 'loudnorm=I=-16:TP=-1.5:LRA=11', '-c:a', 'aac', '-b:a', '160k', 'out/klasync_demo.mp4'])
print(subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration:stream=width,height', '-of', 'csv=p=0', 'out/klasync_demo.mp4'], capture_output=True, text=True).stdout)
