import json, glob, subprocess, os, sys
os.chdir(os.path.dirname(os.path.abspath(__file__)))
FONT='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'; FONTB='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'; SERIF='/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf'
LM=json.load(open('out/live_marks.json')); SM=json.load(open('out/study_marks.json'))
L={m['name']:m['ms']/1000 for m in LM['marks']}; S={m['name']:m['ms']/1000 for m in SM['marks']}
OFF_D=LM['off']['desktop']/1000; OFF_P=LM['off']['phone']/1000; OFF_S=0.7
VD=glob.glob('out/live_desktop/*.webm')[0]; VP=glob.glob('out/live_phone/*.webm')[0]; VS=glob.glob('out/study_phone/*.webm')[0]
AUDIO_START=L['audio_start']
BG='0x0e0905'; ACC='0xff8c3c'
os.makedirs('out/seg',exist_ok=True)
TOOLS=['Transcript and audio','Chapters','Summary','Concept map','Flashcards','Quiz  (new)','Ask the lecture']

def txt(name, s):
    p=f'out/seg/{name}.txt'; open(p,'w').write(s); return p

def run(cmd):
    r=subprocess.run(cmd,capture_output=True,text=True)
    if r.returncode: print(r.stderr[-1500:]); sys.exit(1)

def ndur(name):
    p=f'out/narr/{name}.wav'
    if not os.path.exists(p): return 0.0
    return float(subprocess.run(['ffprobe','-v','error','-show_entries','format=duration','-of','csv=p=0',p],capture_output=True,text=True).stdout.strip())

def audio_chain(name, dur, first_idx, lecture=None):
    """Narration (delayed 0.4s) over an optional, ducked lecture track; always `dur` long."""
    ins=['-f','lavfi','-t',f'{dur:.3f}','-i','anullsrc=r=48000:cl=mono']; idx=first_idx; parts=[f'[{idx}:a]anull[a0]']; mix=['[a0]']; idx+=1
    if lecture:
        start,length,speed=lecture
        ins+=['-ss',f'{max(0,start):.3f}','-t',f'{length:.3f}','-i','lecture48.wav']
        parts.append(f'[{idx}:a]atempo={speed},volume=0.3,afade=t=in:st=0:d=0.2,afade=t=out:st={max(0,dur-0.3):.3f}:d=0.3[a1]'); mix.append('[a1]'); idx+=1
    if ndur(name):
        ins+=['-i',f'out/narr/{name}.wav']
        parts.append(f'[{idx}:a]aresample=48000,adelay=400|400,volume=1.6[a2]'); mix.append('[a2]'); idx+=1
    parts.append(''.join(mix)+f'amix=inputs={len(mix)}:duration=first:normalize=0,alimiter=limit=0.95[aout]')
    return ins, parts, idx

def seg(name, layout, d_rng, p_rng, vp, caption, label_d='Lecturer  ·  desktop', label_p='Student  ·  phone', speed=1.0, audio=False, tool=None):
    """ranges are (start,end) in each video's own time. A scene lasts as long as its narration needs
    (slowing the footage if the narration is longer than the sped-up clip)."""
    rng = d_rng or p_rng; span=rng[1]-rng[0]
    nd=ndur(name); dur=max(span/speed, nd+0.9) if nd else span/speed
    speed=span/dur
    if speed<0.95: print(f'  note: {name} plays at {speed:.2f}x to fit narration')
    ins=[]
    ins += ['-f','lavfi','-t',f'{dur:.3f}','-i',f'color=c={BG}:s=1920x1080:r=30']
    idx=1; last='0:v'
    def prep(path, rng, w, h):
        nonlocal idx
        ins.extend(['-ss',f'{rng[0]:.3f}','-t',f'{rng[1]-rng[0]:.3f}','-i',path])
        i=idx; idx+=1
        crop='crop=390:664:0:0,' if h>w else ''
        return f'[{i}:v]fps=30,{crop}setpts=(PTS-STARTPTS)/{speed},scale={w}:{h}:flags=lanczos[v{i}]', f'v{i}'
    chain=[]
    if layout=='D':
        f,v=prep(VD,d_rng,1536,960); chain.append(f); chain.append(f'[{last}][{v}]overlay=192:10[o1]'); last='o1'
    elif layout=='P':
        f,v=prep(vp,p_rng,560,953); chain.append(f); chain.append(f"[{last}][{v}]overlay=680:24[o1]"); last='o1'
        dt=[f"drawtext=fontfile={FONTB}:text='{label_p}':fontsize=26:fontcolor={ACC}:x=90:y=60"]
        if tool is not None:
            dt.append(f"drawtext=fontfile={FONTB}:text='Lecture studio':fontsize=30:fontcolor={ACC}:x=1350:y=250")
            for i,nm in enumerate(TOOLS):
                on=(i==tool)
                dt.append(f"drawtext=fontfile={FONTB if on else FONT}:text='{('> ' if on else '   ')+nm}':fontsize=36:fontcolor={'white' if on else '0x8a7a6a'}:x=1350:y={310+i*68}")
        chain.append(f"[{last}]"+','.join(dt)+"[o2]"); last='o2'
    else:
        f,v=prep(VD,d_rng,1200,750); chain.append(f); chain.append(f'[{last}][{v}]overlay=40:110[o1]'); last='o1'
        f,v=prep(vp,p_rng,440,750); chain.append(f); chain.append(f'[{last}][{v}]overlay=1330:110[o2]'); last='o2'
        chain.append(f"[{last}]drawtext=fontfile={FONTB}:text='{label_d}':fontsize=26:fontcolor={ACC}:x=40:y=62,drawtext=fontfile={FONTB}:text='{label_p}':fontsize=26:fontcolor={ACC}:x=1330:y=62[o3]"); last='o3'
    cp=txt(name,caption)
    chain.append(f"[{last}]drawbox=x=0:y=1000:w=1920:h=80:color=black@0.78:t=fill,drawtext=fontfile={FONT}:textfile={cp}:fontsize=32:fontcolor=white:x=(w-text_w)/2:y=1040-text_h/2,fade=t=in:st=0:d=0.3,fade=t=out:st={dur-0.3:.3f}:d=0.3[vout]")
    lec=((d_rng[0]-AUDIO_START),(d_rng[1]-d_rng[0]),speed) if (audio and layout!='P') else None
    ains,achain,idx=audio_chain(name,dur,idx,lec); ins+=ains
    out=f'out/seg/{name}.mp4'
    run(['ffmpeg','-v','error','-y',*ins,'-filter_complex',';'.join(chain+achain),'-map','[vout]','-map','[aout]','-r','30','-c:v','libx264','-preset','veryfast','-crf','20','-pix_fmt','yuv420p','-c:a','aac','-ar','48000','-ac','1','-t',f'{dur:.3f}',out])
    return out

def card(name, title, sub, dur=4.0):
    nd=ndur(name); dur=max(dur, nd+1.4) if nd else dur
    tp=txt(name+'_t',title); sp=txt(name+'_s',sub)
    out=f'out/seg/{name}.mp4'
    vf=(f"drawtext=fontfile={SERIF}:textfile={tp}:fontsize=84:fontcolor=0xfff0e0:x=(w-text_w)/2:y=h/2-90,"
        f"drawtext=fontfile={FONT}:textfile={sp}:fontsize=34:fontcolor={ACC}:x=(w-text_w)/2:y=h/2+40,fade=t=in:st=0:d=0.6,fade=t=out:st={dur-0.6}:d=0.6")
    ins=['-f','lavfi','-t',str(dur),'-i',f'color=c={BG}:s=1920x1080:r=30']
    ains,achain,_=audio_chain(name,dur,1)
    run(['ffmpeg','-v','error','-y',*ins,*ains,'-filter_complex',';'.join([f'[0:v]{vf}[vout]']+achain),'-map','[vout]','-map','[aout]','-r','30','-c:v','libx264','-preset','veryfast','-crf','20','-pix_fmt','yuv420p','-c:a','aac','-ar','48000','-ac','1','-t',str(dur),out])
    return out

d=lambda a,b:(a-OFF_D,b-OFF_D); p=lambda a,b:(a-OFF_P,b-OFF_P); s=lambda a,b:(a-OFF_S,b-OFF_S)
segs=[]
segs.append(card('c_intro','KLASYNC','Live lectures, captioned in real time and ready to study  -  end to end'))
segs.append(seg('s1','D',d(0.5,23.5),None,None,'Lecturer signs in, reviews the class roster and starts a live session',speed=1.6))
segs.append(seg('s2','S',d(23.5,34.0),p(23.5,34.0),VP,'Students join from their phones with the session code',speed=1.0))
segs.append(seg('s3','S',d(34.3,52.0),p(34.3,52.0),VP,'The lecture audio streams in and Gemini captions it live on every phone',speed=1.4,audio=True))
segs.append(seg('s4','S',d(52.0,92.0),p(52.0,92.0),VP,'Check-ins, raised hands and questions reach the lecturer instantly',speed=2.6))
segs.append(seg('s5','S',d(92.0,118.0),p(92.0,118.0),VP,'A rolling AI summary builds as the lecture goes',speed=3.0))
segs.append(seg('s6a','S',d(118.0,136.0),p(118.0,136.0),VP,'Lecturer taps AI quiz...',speed=4.0))
segs.append(seg('s6b','S',d(136.5,150.0),p(136.5,150.0),VP,'Gemini writes a question from what was just taught. Students answer live; results update instantly',speed=1.0))
segs.append(seg('s7','D',d(184.5,203.0),None,None,'End session: attendance is scored automatically and rolled into course analytics',speed=1.3))
segs.append(card('c_mid','After class','The student archive on mobile',dur=3.0))
segs.append(seg('t1','P',None,s(3.0,12.6),VS,'Student signs in to the archive on their phone',speed=1.2,tool=0))
segs.append(seg('t2','P',None,s(12.6,21.2),VS,'Searchable transcript synced to the lecture recording',speed=1.3,tool=0))
segs.append(seg('t3','P',None,s(21.2,26.6),VS,'AI-generated chapters jump to any moment in the lecture',speed=1.0,tool=1))
segs.append(seg('t3b','P',None,s(26.6,32.8),VS,'A summary of the lecture, built as it was taught',speed=1.0,tool=2))
segs.append(seg('t4','P',None,s(32.8,40.6),VS,'An interactive concept map of the lecture',speed=1.1,tool=3))
segs.append(seg('t5','P',None,s(40.6,55.6),VS,'Flashcards generated from the lecture',speed=1.4,tool=4))
segs.append(seg('t6','P',None,s(55.6,84.1),VS,'New: take a Gemini-written quiz on the lecture with instant feedback',speed=1.6,tool=5))
segs.append(seg('t7','P',None,s(84.1,95.1),VS,'Ask the lecture: answers cite the moments in the recording',speed=1.2,tool=6))
segs.append(card('c_out','KLASYNC','Every lecture accessible to all.',dur=4.0))
open('out/concat.txt','w').write(''.join(f"file 'seg/{os.path.basename(x)}'\n" for x in segs))
run(['ffmpeg','-v','error','-y','-f','concat','-safe','0','-i','out/concat.txt','-c','copy','out/klasync_demo_raw.mp4'])
# Even out loudness between the lecturer, the narrator and the silent stretches.
run(['ffmpeg','-v','error','-y','-i','out/klasync_demo_raw.mp4','-c:v','copy','-af','loudnorm=I=-16:TP=-1.5:LRA=11','-c:a','aac','-b:a','160k','out/klasync_demo.mp4'])
print(subprocess.run(['ffprobe','-v','error','-show_entries','format=duration:stream=width,height','-of','csv=p=0','out/klasync_demo.mp4'],capture_output=True,text=True).stdout)
