"""Render AgentLens's deterministic 1080p introduction from the local storyboard.

Usage: python docs/video/render_video.py [--preview]
The preview writes a contact sheet without encoding the full movie.
"""
from pathlib import Path
import argparse, functools, json, math, os, re, subprocess, wave
import numpy as np
from PIL import Image, ImageDraw, ImageFont, ImageFilter
import imageio_ffmpeg

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
ASSETS = HERE / 'assets'
OUT = ROOT / 'docs/media'
OUT.mkdir(exist_ok=True)
SPEC = json.loads((HERE/'storyboard.json').read_text())
W,H,FPS=1920,1080,30
FFMPEG=os.environ.get('FFMPEG_BINARY',imageio_ffmpeg.get_ffmpeg_exe())
CYAN=(105,237,224); WHITE=(235,245,250); MUTED=(141,164,185); AMBER=(244,185,114)
CN=os.environ.get('AGENTLENS_CN_FONT','/System/Library/Fonts/PingFang.ttc')
EN=os.environ.get('AGENTLENS_EN_FONT','/System/Library/Fonts/SFNS.ttf')
MONO=os.environ.get('AGENTLENS_MONO_FONT','/System/Library/Fonts/SFNSMono.ttf')
@functools.lru_cache(128)
def font(size,kind='cn',weight=0):
 return ImageFont.truetype({'cn':CN,'en':EN,'mono':MONO}[kind],int(size),index=8 if weight and kind=='cn' else 2 if kind=='cn' else 0)
def ease(x):
 x=max(0,min(1,x));return 1-(1-x)**3
def smooth(x):
 x=max(0,min(1,x));return x*x*(3-2*x)
def rgba(c,a=255):return tuple(c)+(int(a),)
def text(im,xy,s,size=30,fill=WHITE,kind='cn',weight=0,anchor=None):
 d=ImageDraw.Draw(im);d.text(xy,s,font=font(size,kind,weight),fill=fill,anchor=anchor,spacing=int(size*.25))
def line(im,points,c=CYAN,width=2):ImageDraw.Draw(im).line(points,fill=c,width=width)
def panel(im,box,fill=(12,25,38,245),outline=(54,81,98,255),radius=22):
 def opaque(c):
  if c is None or len(c)<4:return c
  a=c[3]/255;return tuple(round(c[i]*a+(9,21,33)[i]*(1-a)) for i in range(3))+(255,)
 ImageDraw.Draw(im).rounded_rectangle(tuple(map(int,box)),radius,opaque(fill),opaque(outline),width=1)
def pill(im,x,y,label,color=CYAN,size=22):
 tw=ImageDraw.Draw(im).textlength(label,font=font(size));panel(im,(x,y,x+tw+36,y+44),rgba(color,14),rgba(color,80),12);text(im,(x+18,y+6),label,size,color)
def circle(im,c,r,outline=CYAN,width=2,fill=None):
 ImageDraw.Draw(im).ellipse((c[0]-r,c[1]-r,c[0]+r,c[1]+r),fill,outline,width)

# A restrained navy field with a diffuse teal light; reused instead of recomputed per frame.
y,x=np.mgrid[0:H,0:W]
glow=np.exp(-(((x-1330)/850)**2+((y-440)/620)**2))
base=np.zeros((H,W,3),np.uint8)
for k,(b,a) in enumerate([(5,9),(11,21),(21,27)]):base[:,:,k]=b+a*glow
BG=Image.fromarray(base).convert('RGBA')
ICON=Image.open(ROOT/'src-tauri/icons/icon.png').convert('RGBA')
SHOTS={n:Image.open(ASSETS/(n+'.png')).convert('RGB') for n in ['overview','storage','processes','process-detail','cleanup','activity']}
# Crop to meaningful parts of the actual app, retaining its native preview badge.
CROPS={
 'overview':(280,0,2160,1070),
 'storage':(280,0,2160,1280),
 'process-detail':(280,0,2160,1320),
 'cleanup':(280,0,2160,1190),
 'activity':(280,0,2160,1250),
}
SHOTS={n:im.crop(CROPS.get(n,(0,0,*im.size))) for n,im in SHOTS.items()}

scenes=SPEC['scenes']; cursor=.0
for i,s in enumerate(scenes):
 s['start']=round(cursor,3);s['duration']=round(s['voiceDuration']+(.95 if i==len(scenes)-1 else .46),3)
 s['voiceStart']=round(cursor+.14,3);cursor+=s['duration']
DURATION=math.ceil(cursor*FPS)/FPS
assert DURATION<80, f'{DURATION}s exceeds delivery limit'

# Captions follow synthesized word timings, grouped into short lines with original punctuation.
captions=[]
for s in scenes:
 words=json.loads((ASSETS/f"voice-{s['id']}.json").read_text())
 src=s['narration'];pos=0;group=[];group_start=0
 for wi,w in enumerate(words):
  needle=w['text'];idx=src.find(needle,pos)
  if idx<0:idx=pos
  end=idx+len(needle)
  while end<len(src) and src[end] in '，。；！？、,.!?;':end+=1
  if not group:group_start=idx
  group.append(w);pos=end
  excerpt=src[group_start:end]
  nextlen=len(words[wi+1]['text']) if wi+1<len(words) else 0
  split_here = wi in {5,12,21,26} if s['id']=='intro' else ((len(excerpt)>=12 and excerpt[-1:] in '，。；！？') or len(excerpt)+nextlen>27 or wi==len(words)-1)
  if split_here:
   start=s['voiceStart']+group[0]['offset']/1e7
   stop=s['voiceStart']+(group[-1]['offset']+group[-1]['duration'])/1e7+.06
   captions.append({'start':start,'end':stop,'text':excerpt});group=[]

# Adjacent word groups can touch; keep SRT cues strictly non-overlapping.
for i,c in enumerate(captions[:-1]):
 c['end']=min(c['end'],captions[i+1]['start']-.005)

def stamp(sec):
 ms=round(sec*1000);return f'{ms//3600000:02}:{ms//60000%60:02}:{ms//1000%60:02},{ms%1000:03}'
subtitle_text='\n\n'.join(f"{i+1}\n{stamp(c['start'])} --> {stamp(c['end'])}\n{c['text']}" for i,c in enumerate(captions))+'\n'
(OUT/'agentlens-intro-zh.srt').write_text(subtitle_text)
(OUT/'agentlens-intro-zh.vtt').write_text('WEBVTT\n\n'+re.sub(r'(\d\d:\d\d:\d\d),(\d{3})',r'\1.\2',subtitle_text))

def background(t):
 im=BG.copy();d=ImageDraw.Draw(im)
 for xx in range(0,W+80,80):d.line((xx,100,xx,H-110),fill=(20,37,50,150))
 for yy in range(130,H-90,80):d.line((60,yy,W-60,yy),fill=(20,37,50,150))
 # Fine travelling light traces and orbiting particles.
 for i in range(14):
  xx=(i*163+t*(13+i%4*8))%W;yy=180+(i*173)%700
  d.line((xx,yy,xx+22,yy),fill=(36,90,96,180),width=1)
  d.ellipse((xx-2,yy-2,xx+2,yy+2),fill=(93,188,181,160))
 line(im,[(76,99),(1844,99)],(32,59,73),1)
 text(im,(116,40),'AGENTLENS',23,WHITE,'mono')
 circle(im,(87,58),9,CYAN,2);line(im,[(94,65),(101,72)],CYAN,3)
 text(im,(1845,42),'RESOURCE INTELLIGENCE / LOCAL FIRST',17,MUTED,'mono',anchor='ra')
 return im

def header(im,s,p,idx):
 y=int(184+28*(1-ease(p/.75)))
 text(im,(94,y-42),s['chapter'],22,CYAN,'cn',1)
 if s['id'] not in ['intro','outro']:
  text(im,(90,y+22),s['title'],64,WHITE,weight=1)
 text(im,(94,930),f'{idx+1:02} / 09',20,MUTED,'mono')
 text(im,(1840,930),'AGENTLENS  •  v0.1.0',17,MUTED,'mono',anchor='ra')

def screenshot(im,name,p,box=(708,218,1832,872)):
 x0,y0,x1,y1=box;w=int(x1-x0);h=int(y1-y0)
 shot=SHOTS[name];zoom=1.0+.035*smooth(p/8)
 sw=int(w*zoom);sh=int(sw*shot.height/shot.width)
 resized=shot.resize((sw,sh),Image.Resampling.BICUBIC)
 cut=resized.crop((int((sw-w)/2),int(max(0,sh-h)*.20),int((sw+w)/2),int(max(0,sh-h)*.20)+h))
 mask=Image.new('L',(w,h));ImageDraw.Draw(mask).rounded_rectangle((0,0,w-1,h-1),18,fill=255)
 # Soft shadow, slight vertical float and a crisp glass frame.
 yy=int(y0+22*(1-ease(p/.8))+3*math.sin(p*.55))
 panel(im,(x0-10,yy-10,x1+10,yy+h+10),(0,0,0,130),(42,81,95,255),26)
 im.paste(cut,(x0,yy),mask)
 pill(im,x1-225,yy+18,'实际界面 · 示例数据',CYAN,17)
 # Moving line around a small portion of the perimeter.
 d=ImageDraw.Draw(im);v=(p*.08)%1
 d.line((x0+30,yy-10,x0+30+(w-60)*v,yy-10),fill=CYAN,width=2)

def bullet(im,x,y,n,title,desc,active=1):
 color=CYAN if active else MUTED
 circle(im,(x+18,y+24),18,color,1);text(im,(x+18,y+22),str(n),18,color,'mono',anchor='mm')
 text(im,(x+54,y),title,30,WHITE,weight=1);text(im,(x+54,y+48),desc,22,MUTED)

def lens(im,c,r,p):
 d=ImageDraw.Draw(im)
 for j in range(4):
  rr=r+j*30;circle(im,c,rr,(24+j*5,65+j*6,76+j*6),1)
  start=(p*16+j*73)%360
  d.arc((c[0]-rr,c[1]-rr,c[0]+rr,c[1]+rr),start,start+48,fill=CYAN if j%2==0 else (70,131,150),width=3)
 for a in range(0,360,6):
  q=math.radians(a);rr=r+109
  line(im,[(c[0]+rr*math.cos(q),c[1]+rr*math.sin(q)),(c[0]+(rr+7 if a%30 else rr+15)*math.cos(q),c[1]+(rr+7 if a%30 else rr+15)*math.sin(q))],(45,82,94),1)
 icon=ICON.resize((int(r*1.3),int(r*1.3)),Image.Resampling.LANCZOS)
 im.alpha_composite(icon,(int(c[0]-icon.width/2),int(c[1]-icon.height/2)))

def render_scene(idx,t):
 s=scenes[idx];p=t-s['start'];im=background(t);header(im,s,p,idx);d=ImageDraw.Draw(im);sid=s['id']
 if sid=='problem':
  text(im,(96,445),'你的 AI 开发环境，正在变重。',30,MUTED)
  for j,(lab,val,col) in enumerate([('STORAGE / 磁盘','缓存与会话',CYAN),('MEMORY / 内存','Agent 与子进程',(146,171,250)),('BACKGROUND / 后台','任务结束之后',AMBER)]):
   yy=int(230+j*205+30*(1-ease((p-j*.14)/.8)))
   panel(im,(805,yy,1795,yy+175));text(im,(839,yy+25),lab,20,col,'mono');text(im,(839,yy+65),val,36,WHITE,weight=1)
   vals=[.25+.04*k+.11*math.sin(k*.7+j) for k in range(28)]
   pts=[(1270+k*16,yy+130-v*70-8*math.sin(p*.5+k*.15)) for k,v in enumerate(vals)]
   line(im,pts,col,3);circle(im,pts[-1],5,col,1,col)
  for j,lab in enumerate(['谁在占用？','从哪里来？','能否清理？']):pill(im,96,575+j*66,lab,CYAN if j==int(p/2.2)%3 else MUTED,26)
 elif sid=='intro':
  text(im,(92,266),'AgentLens',104,WHITE,'en',1);text(im,(96,402),'AI Agent 资源监测与清理工作台',36,WHITE,weight=1)
  text(im,(96,467),'YOUR AGENTS, IN FOCUS.',25,CYAN,'mono');text(im,(96,545),'本地运行  /  有据可查  /  操作可复核',27,MUTED)
  lens(im,(1390,505),148,p)
  labels=['Codex','Claude Code','Cursor','Gemini CLI','OpenCode','Aider']
  for j,lab in enumerate(labels):
   a=j*math.pi/3-.7+p*.025;x=1390+325*math.cos(a);y=505+306*math.sin(a)
   ww=ImageDraw.Draw(im).textlength(lab,font=font(23,'en'))+44
   panel(im,(x-ww/2,y-25,x+ww/2,y+25),(13,33,45,255),(61,110,120,255),12);text(im,(x,y),lab,23,WHITE,'en',anchor='mm')
  pill(im,96,659,'6 类工具识别',CYAN,24)
 elif sid=='overview':
  for j,(a,b) in enumerate([('占用总览','磁盘 · RSS · CPU'),('变化趋势','持续增长，及时检查'),('后台速览','开发时也能掌握变化')]):bullet(im,96,475+j*115,j+1,a,b)
  screenshot(im,'overview',p)
 elif sid=='trace':
  for j,(a,b) in enumerate([('自动发现项目','路径、标记与会话 cwd'),('沿线索逐层下钻','工具 → 项目 → 目录 → 文件'),('显示判断原因','保留什么，候选是什么')]):bullet(im,96,475+j*115,j+1,a,b)
  screenshot(im,'storage',p)
 elif sid=='process':
  for j,(a,b) in enumerate([('先理解用途','Agent / MCP / 开发 / 构建'),('检查身份与影响','父进程、工作目录、资源变化'),('确认后请求停止','执行前核对 PID + 启动时间')]):bullet(im,96,475+j*115,j+1,a,b)
  screenshot(im,'process-detail',p)
 elif sid=='architecture':
  text(im,(1010,248),'LOCAL DATA PIPELINE',25,CYAN,'mono')
  labels=[('Vue 3','交互与可视化','UI / TypeScript'),('Tauri 2','连接桌面能力','IPC / WebView'),('Rust','目录扫描与资源采样','sysinfo / notify'),('SQLite','本地索引与历史','WAL / journal')]
  for j,(name,desc,sub) in enumerate(labels):
   xx=96+j*440;yy=490+int(30*(1-ease((p-j*.15)/.8)))
   panel(im,(xx,yy,xx+398,yy+245),(12,29,42,250),(47,99,110,255),24)
   text(im,(xx+28,yy+22),f'0{j+1}',22,CYAN,'mono');text(im,(xx+28,yy+65),name,52,WHITE,'en');text(im,(xx+28,yy+141),desc,26,WHITE);text(im,(xx+28,yy+194),sub,20,MUTED,'mono')
   if j<3:
    line(im,[(xx+400,yy+120),(xx+436,yy+120)],(51,109,121),2)
    dot=xx+400+36*((p*.7-j*.2)%1);circle(im,(dot,yy+120),4,CYAN,1,CYAN)
  pill(im,96,790,'文件事件 + 周期复核',CYAN,25);pill(im,515,790,'扫描 / 采样独立运行',CYAN,25);pill(im,965,790,'数据保存在本机',CYAN,25)
 elif sid=='cleanup':
  steps=['保留期与规则','稳定观测 + 未运行','用户确认 + 身份复核','移入系统垃圾桶']
  for j,lab in enumerate(steps):
   yy=465+j*78;active=p>(j*.65)
   circle(im,(118,yy+20),16,CYAN if active else MUTED,2)
   if active:line(im,[(111,yy+20),(116,yy+25),(125,yy+14)],CYAN,2)
   text(im,(158,yy-2),lab,28,WHITE if active else MUTED)
   if j<3:line(im,[(118,yy+38),(118,yy+61)],(45,86,98),2)
  screenshot(im,'cleanup',p,(708,218,1832,785))
  panel(im,(708,816,1832,892),(16,35,45,255),(72,114,121,255),15)
  text(im,(735,837),'清空系统垃圾桶后，才真正释放磁盘空间。',28,CYAN,weight=1)
 elif sid=='privacy':
  lens(im,(1380,480),130,p)
  text(im,(1380,756),'ON YOUR DEVICE',22,CYAN,'mono',anchor='mm')
  for j,(title,desc) in enumerate([('无需账号','没有监测数据上传'),('有限元数据','不保存提示词和对话'),('有依据的管理','资源可见 · 来源可查 · 操作可复核')]):bullet(im,96,475+j*115,j+1,title,desc)
 elif sid=='outro':
  icon=ICON.resize((150,150),Image.Resampling.LANCZOS);im.alpha_composite(icon,(93,230))
  text(im,(282,231),'AgentLens',112,WHITE,'en',1)
  text(im,(94,456),s['title'],78,WHITE,weight=1)
  text(im,(100,687),'YOUR AGENTS, IN FOCUS.',28,CYAN,'mono')
  lens(im,(1485,510),130,p)
  panel(im,(96,775,945,852),(10,28,39,255),(40,87,99,255),14)
  text(im,(121,797),'$ npm ci   &&   npm run desktop',26,CYAN,'mono')
 # Caption bar is kept separate from the interface and typography safe area.
 subtitle=next((c['text'] for c in captions if c['start']<=t<c['end']),None)
 if subtitle:
  tw=d.textlength(subtitle,font=font(31));bw=tw+70
  panel(im,((W-bw)/2,974,(W+bw)/2,1042),(2,8,15,238),(39,65,78,255),14)
  text(im,(W/2,1006),subtitle,31,WHITE,anchor='mm')
 # A permanent disclosure prevents demo numbers from being presented as measurements.
 if sid in ['overview','trace','process','cleanup']:
  text(im,(1840,900),'界面数值为演示数据，不代表实测清理收益',18,MUTED,anchor='ra')
 line(im,[(0,H-3),(int(W*t/DURATION),H-3)],CYAN,3)
 return im

def frame(t):
 idx=next((i for i,s in enumerate(scenes) if s['start']<=t<s['start']+s['duration']),len(scenes)-1)
 im=render_scene(idx,t)
 # Fast cross dissolve, with a restrained scan streak through the cut.
 elapsed=t-scenes[idx]['start']
 if idx and elapsed<.27:
  prev=render_scene(idx-1,scenes[idx]['start']-.001)
  im=Image.blend(prev,im,smooth(elapsed/.27))
  xx=int(W*elapsed/.27);line(im,[(xx,115),(xx,912)],(61,134,142,140),2)
 fade=min(1,t/.35,(DURATION-t)/.5)
 if fade<1:im=Image.blend(Image.new('RGBA',(W,H),(5,11,21,255)),im,max(0,fade))
 return im.convert('RGB')

def audio():
 sr=48000;n=round(DURATION*sr);voice=np.zeros(n,np.float32)
 for s in scenes:
  raw=subprocess.check_output([FFMPEG,'-v','error','-i',str(ASSETS/f"voice-{s['id']}.mp3"),'-f','f32le','-ar',str(sr),'-ac','1','-'])
  samples=np.frombuffer(raw,np.float32).copy();peak=max(.01,np.max(np.abs(samples)));samples*=.79/peak
  start=round(s['voiceStart']*sr);length=min(len(samples),n-start);voice[start:start+length]+=samples[:length]
 ts=np.arange(n,dtype=np.float32)/sr;music=np.zeros(n,np.float32)
 # Original procedural soundtrack: sparse synth pad, arpeggio and low pulse.
 for k,f in enumerate([130.8128,195.9977,261.6256,293.6648]):
  music+=(.0065*np.sin(2*np.pi*f*ts+.15*np.sin(ts*.4+k))+.003*np.sin(2*np.pi*(f*1.002)*ts))*(.75+.25*np.sin(ts*.35+k))
 rng=np.random.default_rng(27)
 for beat in np.arange(0,DURATION,.51724):
  st=int(beat*sr);ln=min(int(.27*sr),n-st);tt=np.arange(ln)/sr
  music[st:st+ln]+=.022*np.sin(2*np.pi*(53*tt+4*(1-np.exp(-tt*25))))*np.exp(-tt*20)
 for k,beat in enumerate(np.arange(.26,DURATION,.25862)):
  st=int(beat*sr);ln=min(int(.23*sr),n-st);tt=np.arange(ln)/sr;f=[523.251,783.991,1046.502,587.33,659.255,783.991,1174.66,1046.5][k%8]
  music[st:st+ln]+=.008*np.sin(2*np.pi*f*tt)*np.exp(-tt*23)*(1-np.exp(-tt*200))
 for s in scenes[1:]:
  st=int(max(0,s['start']-.12)*sr);ln=min(int(.43*sr),n-st);tt=np.arange(ln)/sr
  noise=rng.standard_normal(ln).astype(np.float32);noise=np.convolve(noise,np.ones(8)/8,mode='same')
  music[st:st+ln]+=.026*noise*np.sin(np.pi*np.minimum(1,tt/.43))**2
 # Lower the synth bed further during speech; leave clear gaps at cuts.
 envelope=np.convolve(np.abs(voice[::480]),np.ones(8)/8,mode='same')
 duck=np.interp(np.arange(n),np.arange(len(envelope))*480,np.clip(1-envelope*5,.42,1))
 music*=duck*np.minimum(1,ts/1.5)*np.minimum(1,(DURATION-ts)/1.5)
 left=voice+music;right=voice+.96*music
 stereo=np.stack([left,right],axis=1);stereo=np.clip(stereo,-.98,.98)
 path=OUT/'agentlens-mix.wav'
 with wave.open(str(path),'wb') as w:w.setnchannels(2);w.setsampwidth(2);w.setframerate(sr);w.writeframes((stereo*32767).astype('<i2').tobytes())
 return path

def contact():
 sheet=Image.new('RGB',(1440,810),(5,11,21))
 for i,s in enumerate(scenes):
  im=frame(s['start']+min(3.0,s['duration']*.52));im.thumbnail((480,270));sheet.paste(im,((i%3)*480,(i//3)*270))
 sheet.save(OUT/'agentlens-contact-sheet.jpg',quality=92)
 frame(scenes[1]['start']+3).save(OUT/'agentlens-poster.jpg',quality=95)
 for name in ['overview','architecture','cleanup']:
  s=next(s for s in scenes if s['id']==name);frame(s['start']+3).save(OUT/f'frame-{name}.jpg',quality=95)

if __name__=='__main__':
 ap=argparse.ArgumentParser();ap.add_argument('--preview',action='store_true');args=ap.parse_args()
 contact()
 metadata={'duration':DURATION,'width':W,'height':H,'fps':FPS,'voice':SPEC['voice'],'voiceRate':SPEC['rate'],'chapters':[{'id':s['id'],'start':s['start'],'duration':s['duration']} for s in scenes],'subtitles':len(captions)}
 (OUT/'video-metadata.json').write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+'\n')
 print(json.dumps(metadata,ensure_ascii=False),flush=True)
 if not args.preview:
  wav=audio();temp=OUT/'agentlens-picture.mp4';total=round(DURATION*FPS)
  cmd=[FFMPEG,'-y','-v','warning','-f','rawvideo','-vcodec','rawvideo','-pix_fmt','rgb24','-s',f'{W}x{H}','-r',str(FPS),'-i','-','-an','-c:v','libx264','-preset','fast','-crf','19','-pix_fmt','yuv420p','-movflags','+faststart',str(temp)]
  proc=subprocess.Popen(cmd,stdin=subprocess.PIPE)
  try:
   for k in range(total):
    proc.stdin.write(frame(k/FPS).tobytes())
    if k%(FPS*5)==0:print(f'Render {k//FPS}s / {DURATION:.2f}s',flush=True)
  finally:proc.stdin.close()
  assert proc.wait()==0,'Video encoder failed'
  output=OUT/'agentlens-intro-zh.mp4'
  subprocess.run([FFMPEG,'-y','-v','warning','-i',str(temp),'-i',str(wav),'-map','0:v:0','-map','1:a:0','-c:v','copy','-c:a','aac','-b:a','192k','-t',str(DURATION),'-movflags','+faststart',str(output)],check=True)
  temp.unlink();wav.unlink()
  print(f'FINISHED: {output} ({output.stat().st_size/1e6:.1f} MB)',flush=True)
