"""Continuous resource-space film. Rebuild: python docs/video/render_video.py [--preview]."""
from pathlib import Path
import argparse, functools, json, math, os, subprocess, re, wave
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import imageio_ffmpeg
ROOT=Path(__file__).resolve().parents[2]; HERE=Path(__file__).resolve().parent
ASSETS=HERE/'assets'; OUT=ROOT/'docs/media'; OUT.mkdir(exist_ok=True)
SPEC=json.loads((HERE/'storyboard.json').read_text()); W,H,FPS=1920,1080,30
FFMPEG=os.environ.get('FFMPEG_BINARY',imageio_ffmpeg.get_ffmpeg_exe())
CN=os.environ.get('AGENTLENS_CN_FONT','/System/Library/Fonts/PingFang.ttc')
EN=os.environ.get('AGENTLENS_EN_FONT','/System/Library/Fonts/SFNS.ttf')
WHITE=(229,242,246); MINT=(111,255,208); BLUE=(118,167,255); AMBER=(255,191,108); PINK=(222,146,254)
COLORS=[MINT,BLUE,AMBER,PINK]
@functools.lru_cache(100)
def font(n,bold=False):return ImageFont.truetype(CN,n,index=8 if bold else 2)
def label(im,xy,s,size=28,fill=WHITE,anchor=None,bold=False):ImageDraw.Draw(im).text(xy,s,font=font(size,bold),fill=fill,anchor=anchor)
def blend(a,b,p):return np.asarray(a)*(1-p)+np.asarray(b)*p
def ease(p):p=np.clip(p,0,1);return p*p*(3-2*p)
def interval(t,a,b):return float(ease((t-a)/(b-a)))
scenes=SPEC['scenes'];cursor=0
for s in scenes:
 s['start']=cursor;s['voiceStart']=cursor+.20;s['duration']=s['voiceDuration']+.55;cursor+=s['duration']
DURATION=math.ceil((cursor+1.3)*FPS)/FPS
assert DURATION<80
# Word timed captions. Keep source punctuation and avoid overlapping cues.
CAP=[]
for s in scenes:
 words=json.loads((ASSETS/f"voice-{s['id']}.json").read_text());src=s['narration'];pos=0;group=[];startpos=0
 for i,w in enumerate(words):
  a=src.find(w['text'],pos);a=max(pos,a);b=a+len(w['text'])
  while b<len(src) and src[b] in '，。；！？、,.!?;':b+=1
  if not group:startpos=a
  group.append(w);pos=b;txt=src[startpos:b]
  if (len(txt)>=10 and txt[-1:] in '，。；！？') or len(txt)>23 or i==len(words)-1:
   CAP.append(dict(start=s['voiceStart']+group[0]['offset']/1e7,end=s['voiceStart']+(w['offset']+w['duration'])/1e7+.04,text=txt));group=[]
for a,b in zip(CAP,CAP[1:]):a['end']=min(a['end'],b['start']-.005)
def stamp(t):
 m=round(t*1000);return f'{m//3600000:02}:{m//60000%60:02}:{m//1000%60:02},{m%1000:03}'
srt='\n\n'.join(f"{i+1}\n{stamp(c['start'])} --> {stamp(c['end'])}\n{c['text']}" for i,c in enumerate(CAP))+'\n'
(OUT/'agentlens-intro-zh.srt').write_text(srt)
(OUT/'agentlens-intro-zh.vtt').write_text('WEBVTT\n\n'+re.sub(r'(\d\d:\d\d:\d\d),(\d{3})',r'\1.\2',srt))
# A single persistent world: files retain identity as their positions change.
rng=np.random.default_rng(2411);N=108
initial=rng.uniform([-1600,-650,-450],[1600,650,1100],(N,3))
phase=rng.uniform(0,2*math.pi,N);radius=rng.uniform(115,320,N);group=np.arange(N)%3
sizes=rng.uniform(12,27,N);stars=rng.uniform([-2500,-1400,-800],[2500,1400,2200],(280,3))
y,x=np.mgrid[0:H,0:W];glow=np.exp(-((x-1000)/850)**2-((y-490)/540)**2)
bg=np.zeros((H,W,3),np.uint8)
for c,(base,extra) in enumerate([(3,4),(7,15),(13,19)]):bg[:,:,c]=base+glow*extra
BG=Image.fromarray(bg)
SHOTS={n:Image.open(ASSETS/f'{n}.png').convert('RGB') for n in ['overview','processes','storage','cleanup']}
if (ASSETS/'adoption.png').exists():SHOTS['adoption']=Image.open(ASSETS/'adoption.png').convert('RGB')
else:SHOTS['adoption']=SHOTS['overview']
@functools.lru_cache(12)
def resized_shot(name,width,frame=0):
 frames=sorted((ASSETS/'motion'/name).glob('*.jpg'))
 shot=Image.open(frames[min(frame,len(frames)-1)]).convert('RGB') if frames else SHOTS[name]
 return shot.resize((width,round(width*shot.height/shot.width)),Image.Resampling.LANCZOS)
def projection(points,t):
 points=np.asarray(points,dtype=float);yaw=.28*math.sin(t*.13)+.07;pitch=.13
 cx,sx=math.cos(yaw),math.sin(yaw);xx=points[...,0]*cx+points[...,2]*sx;zz=-points[...,0]*sx+points[...,2]*cx
 yy=points[...,1]*math.cos(pitch)-zz*math.sin(pitch);zz=points[...,1]*math.sin(pitch)+zz*math.cos(pitch)
 depth=np.maximum(450,zz+1850-100*math.sin(t*.06));scale=1100/depth
 return np.stack([W*.52+xx*scale,H*.48+yy*scale],axis=-1),scale,depth

def render(t):
 idx=max(i for i,s in enumerate(scenes) if s['start']<=min(t,scenes[-1]['start']))
 s=scenes[idx];local=t-s['start'];p=local/s['duration'];im=BG.copy();d=ImageDraw.Draw(im)
 # Directional stars and a moving floor establish camera movement throughout.
 pts,sc,dep=projection(stars+np.array([0,0,(t*25)%900]),t)
 for j,(px,py) in enumerate(pts):
  if 0<px<W and 80<py<H-120:
   v=int(42+45*sc[j]);d.line((px,py,px-2,py+sc[j]*5),fill=(v//2,v,v+15),width=1)
 for k in range(14):
  z=-500+k*210+(t*30)%210;ps,_,_=projection([[-2400,650,z],[2400,650,z]],t);d.line(list(map(tuple,ps)),fill=(13,35,43),width=1)
 for xx in range(-2400,2600,240):
  ps,_,_=projection([[xx,650,-500],[xx,650,2500]],t);d.line(list(map(tuple,ps)),fill=(11,28,36),width=1)
 # Persistent lens: ellipse rings, optical ribs, travelling scan arc and light ray.
 center=(W*.52,H*.46);cx,cy=center
 lens_a=interval(t,4,10)*(1-interval(t,DURATION-4,DURATION-1))
 glow_layer=Image.new('RGB',(W,H));gd=ImageDraw.Draw(glow_layer)
 r=260+25*math.sin(t*.32)
 for k in range(8):
  rr=r+k*8;box=(cx-rr,cy-rr*.49,cx+rr,cy+rr*.49)
  gd.arc(box,10+t*27+k*4,270+t*27+k*4,fill=(int(30*lens_a),int(90*lens_a),int(77*lens_a)),width=5)
 im=Image.blend(im,Image.blend(im,glow_layer.filter(ImageFilter.GaussianBlur(16)),.50),.25);d=ImageDraw.Draw(im)
 if lens_a:
  for k in range(3):
   rr=r+k*18;box=(cx-rr,cy-rr*.49,cx+rr,cy+rr*.49)
   d.ellipse(box,outline=tuple(int(v*lens_a*.7) for v in MINT),width=1)
   d.arc(box,t*40+k*45,t*40+60+k*45,fill=MINT,width=3)
  for ang in np.linspace(0,2*math.pi,36,endpoint=False):
   ca,sa=math.cos(ang+t*.1),math.sin(ang+t*.1)
   d.line((cx+ca*(r+8),cy+sa*(r+8)*.49,cx+ca*(r+22),cy+sa*(r+22)*.49),fill=(51,103,98),width=2)
 # Scatter becomes an attributed orbit map, then moves through evidence gates.
 hubs=np.array([[-670,130,100],[0,-250,200],[670,130,100]])
 orbit=np.zeros_like(initial)
 for i in range(N):
  a=phase[i]+t*(.15+(i%5)*.012);orbit[i]=hubs[group[i]]+[math.cos(a)*radius[i],math.sin(a)*radius[i]*.5,math.sin(a)*radius[i]*.6]
 positions=blend(initial+np.stack([np.sin(phase+t*.1)*50,np.cos(phase+t*.18)*40,np.zeros(N)],1),orbit,interval(t,scenes[1]['start'],scenes[1]['start']+4))
 sort_start=scenes[4]['start'];act_start=scenes[5]['start']
 split=positions.copy()
 for i in range(N):
  lane=i%3;ang=phase[i]+t*.14
  split[i]=[(-750,0,750)[lane]+math.cos(ang)*radius[i]*.7,math.sin(ang)*radius[i]*.65,200+math.sin(ang)*radius[i]]
 positions=blend(positions,split,interval(t,sort_start,sort_start+3))
 # Safe candidates arc into a recycle orbit, protected and active lanes remain.
 if t>act_start:
  for i in range(N):
   if i%3==2:
    q=interval(t,act_start+(i%11)*.2,act_start+4+(i%11)*.2)
    target=np.array([1050+math.cos(t+i)*65,200+math.sin(t+i)*65,150.])
    positions[i]=blend(positions[i],target,q);positions[i,1]-=math.sin(q*math.pi)*340
 pts,sc,dep=projection(positions,t)
 # Orbital tracks connect actual points to their tool's hub.
 if scenes[1]['start']<t<sort_start:
  for g in range(3+(t>=scenes[3]['start'])):
   h=hubs[g] if g<3 else np.array([1040,-250,120]);c=COLORS[g]
   ring=[[h[0]+math.cos(a)*340,h[1]+math.sin(a)*150,h[2]+math.sin(a)*190] for a in np.linspace(0,2*math.pi,100)]
   pp,_,_=projection(ring,t);d.line(list(map(tuple,pp)),fill=tuple(int(v*.32) for v in c),width=1)
   xy,_,_=projection([h],t);hx,hy=xy[0]
   d.ellipse((hx-7,hy-7,hx+7,hy+7),fill=c)
   label(im,(hx,hy-125),['Codex','Claude Code','Cursor','Lumo'][g],27,c,anchor='mm',bold=True)
   if g==3:label(im,(hx,hy-90),'已接入',20,c,anchor='mm')
 # A travelling optical plane gives the reveal a physical direction.
 if scenes[1]['start']<t<scenes[1]['start']+5:
  q=(t-scenes[1]['start'])/5;beamx=-1400+2800*q
  plane,_,_=projection([[beamx,-650,-100],[beamx,600,-100]],t)
  for k in range(14,0,-1):d.line(list(map(tuple,plane)),fill=(12,45+k*3,47+k*2),width=k*3)
  d.line(list(map(tuple,plane)),fill=MINT,width=2)
 # Draw tiny extruded file objects, depth-sorted; every object keeps its identity.
 for i in np.argsort(-dep):
  px,py=pts[i];sz=max(3,sizes[i]*sc[i]);c=COLORS[group[i]]
  if t>=sort_start:c=[BLUE,AMBER,MINT][i%3]
  if not (-30<px<W+30 and 80<py<H-130):continue
  if idx in [2,3]:c=tuple(int(v*.65) for v in c)
  d=ImageDraw.Draw(im);d.polygon([(px,py-sz),(px+sz,py-sz*.45),(px+sz,py+sz*.5),(px,py+sz)],fill=tuple(int(v*.45) for v in c),outline=c)
  d.polygon([(px-sz,py-sz*.45),(px,py-sz),(px,py+sz),(px-sz,py+sz*.5)],fill=tuple(int(v*.12) for v in c),outline=tuple(int(v*.65) for v in c))
  if i%7==0 and idx==0:label(im,(px+sz+10,py-12),['cache','node','logs','session'][i%4],17,tuple(int(v*.65) for v in c))
 # A process pulse tree is part of the same resource space.
 if idx==2:
  fade=math.sin(min(1,p)*math.pi)**.4
  for k,(name,c) in enumerate(zip(['仍在工作','常驻保留','继续观测','建议处理'],COLORS)):
   x0=390+k*385;y0=440+math.sin(k*1.8+t*.3)*45
   d.line((W*.52,230,x0,y0),fill=tuple(int(v*.45*fade) for v in c),width=2)
   for j in range(28):
    xx=x0-100+j*7;yy=y0+60+math.sin(j*.8-t*(3 if k==0 else .3))*(24 if k==0 else 3)
    d.ellipse((xx-2,yy-2,xx+2,yy+2),fill=c)
   d.ellipse((x0-23,y0-23,x0+23,y0+23),outline=c,width=2)
   label(im,(x0,y0+115),name,29,c,anchor='mm',bold=True)
 # Three separate evidence lanes make the protection logic visible.
 if idx>=4 and idx<6:
  for lane,(name,c) in enumerate(zip(['重要数据 · 保护','工具运行中 · 保留','核验通过 · 候选'],[BLUE,AMBER,MINT])):
   pp,_,_=projection([[(-750,0,750)[lane],-355,160]],t);a,b=pp[0]
   label(im,(a,b),name,28,c,anchor='mm',bold=True)
  if idx==5:
   px,py=projection([[1050,200,150]],t)[0][0]
   d.ellipse((px-85,py-50,px+85,py+50),outline=MINT,width=3)
   d.arc((px-85,py-90,px+85,py+10),180,360,fill=MINT,width=3)
   label(im,(px,py+105),'系统垃圾桶',24,MINT,anchor='mm')
 # Actual app UI emerges through the lens; preserve app labels and preview disclosure.
 shot_name={1:'overview',2:'processes',3:'adoption',4:'storage',5:'cleanup'}.get(idx)
 show=0
 if shot_name:
  show=interval(p,.30,.42)*(1-interval(p,.64,.77))
 if show>.001:
  sw=round((1040+120*show)/8)*8;shot=resized_shot(shot_name,sw,round(max(0,local-s['duration']*.3)*12))
  sh=min(760,shot.height);shot=shot.crop((0,0,sw,sh));sx=round(W*.5-sw*.5);sy=round(330-70*show)
  overlay=Image.new('RGBA',(W,H));mask=Image.new('L',(sw,sh));ImageDraw.Draw(mask).rounded_rectangle((0,0,sw-1,sh-1),15,fill=int(show*255))
  shadow=Image.new('RGBA',(W,H));ImageDraw.Draw(shadow).rounded_rectangle((sx-12,sy-8,sx+sw+12,sy+sh+15),24,fill=(0,0,0,180));im=Image.alpha_composite(im.convert('RGBA'),shadow.filter(ImageFilter.GaussianBlur(22))).convert('RGB')
  im.paste(shot,(sx,sy),mask);d=ImageDraw.Draw(im)
  d.rounded_rectangle((sx,sy,sx+sw,sy+sh),15,outline=tuple(int(v*.65) for v in MINT),width=1)
  label(im,(sx,sy-34),'真实界面 · 预览数据演示',18,(126,173,177))
 # Sparse words, no chapter cards. Their appearance follows physical events.
 if idx<6:
  alpha=interval(local,0,.7)*(1-interval(p,.85,1))
  label(im,(100,132),s['title'],62,tuple(int(v*alpha) for v in WHITE),bold=True)
  if idx==0:label(im,(105,219),'WHO IS STILL HERE?',20,(119,160,170))
 else:
  fade=interval(local,0,1.4)
  shade=Image.new('RGB',(W,H),(3,10,16));im=Image.blend(im,shade,.72*fade)
  rr=108;cx,cy=960,380;d=ImageDraw.Draw(im)
  for k in range(3):d.arc((cx-rr-k*8,cy-rr-k*8,cx+rr+k*8,cy+rr+k*8),t*20+k*40,t*20+260+k*40,fill=MINT,width=3 if k==0 else 1)
  d.line((1036,456,1086,506),fill=MINT,width=8)
  label(im,(960,565),'AgentLens',98,WHITE,anchor='mm',bold=True)
  label(im,(960,666),'让 AI 开发的资源消耗，清晰可见。',38,MINT,anchor='mm')
  label(im,(960,741),'Your agents, in focus.',25,(134,174,182),anchor='mm')
  label(im,(960,821),'github.com/bubugx/agentLens',23,(158,185,193),anchor='mm')
 # The same small optical mark stays on screen throughout the journey.
 d=ImageDraw.Draw(im);d.ellipse((100,54,121,75),outline=MINT,width=2);d.line((118,72,127,81),fill=MINT,width=2)
 label(im,(143,49),'AgentLens',23,WHITE,bold=True)
 label(im,(1820,55),'LOCAL / IN FOCUS',17,(99,146,153),anchor='ra')
 for c in CAP:
  if c['start']<=t<c['end']:
   txt=c['text'];tw=d.textlength(txt,font=font(32));d.rounded_rectangle((W/2-tw/2-24,976,W/2+tw/2+24,1036),14,fill=(3,10,16));label(im,(960,1007),txt,32,WHITE,anchor='mm');break
 d.line((100,1056,1820,1056),fill=(24,48,57),width=2)
 d.line((100,1056,100+1720*min(1,t/DURATION),1056),fill=MINT,width=2)
 return im

def audio():
 sr=48000;n=math.ceil(DURATION*sr);mix=np.zeros((n,2),np.float32)
 for s in scenes:
  out=ASSETS/f"voice-{s['id']}.wav"
  subprocess.run([FFMPEG,'-y','-v','error','-i',str(ASSETS/f"voice-{s['id']}.mp3"),'-ar',str(sr),'-ac','1',str(out)],check=True)
  with wave.open(str(out)) as f:a=np.frombuffer(f.readframes(f.getnframes()),'<i2').astype(np.float32)/32768
  start=round(s['voiceStart']*sr);end=min(n,start+len(a));mix[start:end,:]+=a[:end-start,None]*.94
 # Soft original electronic score: pulse, sub, glass harmonics; no external music assets.
 time=np.arange(n)/sr;music=np.zeros(n)
 for f in [55,82.4069,110,164.8138]:music+=.007*np.sin(2*np.pi*f*time+np.sin(time*.17))
 beat=time%(60/112);music+=.024*np.exp(-beat*18)*np.sin(2*np.pi*(56*time+7*(1-np.exp(-beat*13))))
 for i,f in enumerate([440,659.25,880,987.77]):
  env=np.exp(-((time-i*.134)%2.14)*6);music+=.009*env*np.sin(2*np.pi*f*time)
 fade=np.minimum(1,time/2)*np.minimum(1,(DURATION-time)/2);music*=fade
 mix[:,0]+=music;mix[:,1]+=np.roll(music,450)
 peak=float(np.abs(mix).max());mix*=min(1,.92/max(peak,.001))
 path=ASSETS/'mix.wav'
 with wave.open(str(path),'wb') as f:f.setnchannels(2);f.setsampwidth(2);f.setframerate(sr);f.writeframes((np.clip(mix,-1,1)*32767).astype('<i2').tobytes())
 return path

def normalize_audio(target):
 log=subprocess.run([FFMPEG,'-hide_banner','-i',str(target),'-vn','-af','loudnorm=I=-16:TP=-1.5:LRA=7:print_format=json','-f','null','-'],capture_output=True,text=True,check=True).stderr
 m=json.loads(log[log.rfind('{'):log.rfind('}')+1])
 settings=f"loudnorm=I=-16:TP=-1.5:LRA=7:measured_I={m['input_i']}:measured_TP={m['input_tp']}:measured_LRA={m['input_lra']}:measured_thresh={m['input_thresh']}:offset={m['target_offset']}:linear=true"
 temp=target.with_name(target.stem+'.normalized.mp4')
 subprocess.run([FFMPEG,'-y','-v','warning','-i',str(target),'-c:v','copy','-af',settings,'-ar','48000','-c:a','aac','-b:a','192k','-movflags','+faststart',str(temp)],check=True)
 temp.replace(target)

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--preview',action='store_true');args=parser.parse_args()
 times=[s['start']+s['duration']*(.2 if i in [2,4,5] else .5) for i,s in enumerate(scenes)]
 thumbs=[]
 for i,t in enumerate(times):
  im=render(t);im.save(OUT/f"frame-{scenes[i]['id']}.jpg",quality=93);thumbs.append(im.resize((640,360)))
 sheet=Image.new('RGB',(1280,360*4),(3,10,16))
 for i,im in enumerate(thumbs):sheet.paste(im,((i%2)*640,(i//2)*360))
 sheet.save(OUT/'agentlens-contact-sheet.jpg',quality=92)
 render(scenes[-1]['start']+2).save(OUT/'agentlens-poster.jpg',quality=94)
 if args.preview:print(f'Preview: {DURATION:.2f}s');return
 track=audio();target=OUT/'agentlens-intro-zh.mp4'
 proc=subprocess.Popen([FFMPEG,'-y','-v','warning','-f','rawvideo','-vcodec','rawvideo','-s',f'{W}x{H}','-pix_fmt','rgb24','-r',str(FPS),'-i','-','-i',str(track),'-c:v','libx264','-preset','fast','-crf','19','-pix_fmt','yuv420p','-c:a','aac','-b:a','192k','-t',str(DURATION),'-movflags','+faststart',str(target)],stdin=subprocess.PIPE)
 for i in range(round(DURATION*FPS)):
  proc.stdin.write(render(i/FPS).tobytes())
  if i%(FPS*5)==0:print(f'Render {i/FPS:.0f}/{DURATION:.1f}s',flush=True)
 proc.stdin.close();assert proc.wait()==0
 normalize_audio(target)
 metadata=dict(title=SPEC['title'],concept=SPEC['concept'],duration=DURATION,width=W,height=H,fps=FPS,voice=SPEC['voice'],rate=SPEC['rate'],narrationSeconds=round(sum(s['voiceDuration'] for s in scenes),3),scenes=scenes,captions=len(CAP))
 (OUT/'video-metadata.json').write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n');print(target,flush=True)
if __name__=='__main__':main()
