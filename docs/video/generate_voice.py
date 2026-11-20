import argparse, asyncio, json, edge_tts, pathlib, subprocess, re, imageio_ffmpeg
root=pathlib.Path(__file__).resolve().parents[2]
p=root/'docs/video/storyboard.json'
j=json.loads(p.read_text())
async def render(s):
 out=root/'docs/video/assets'/f"voice-{s['id']}.mp3"
 c=edge_tts.Communicate(s['narration'],j['voice'],rate=j['rate'],boundary='WordBoundary')
 boundaries=[]
 with out.open('wb') as f:
  async for item in c.stream():
   if item['type']=='audio':f.write(item['data'])
   elif item['type']=='WordBoundary':boundaries.append({k:item[k] for k in ['offset','duration','text']})
 (out.with_suffix('.json')).write_text(json.dumps(boundaries,ensure_ascii=False,indent=2))
 probe=subprocess.run([imageio_ffmpeg.get_ffmpeg_exe(),'-hide_banner','-i',str(out)],capture_output=True,text=True).stderr
 m=re.search(r'Duration: (\d+):(\d+):(\d+\.\d+)',probe)
 duration=int(m[1])*3600+int(m[2])*60+float(m[3])
 print(s['id'],len(s['narration']),duration,flush=True)
 return duration
async def main():
 parser=argparse.ArgumentParser()
 parser.add_argument('--scene',choices=[s['id'] for s in j['scenes']])
 args=parser.parse_args()
 for s in j['scenes']:
  if args.scene is None or s['id']==args.scene:s['voiceDuration']=await render(s)
 p.write_text(json.dumps(j,ensure_ascii=False,indent=2)+'\n')
 print('TOTAL',sum(s['voiceDuration'] for s in j['scenes']),flush=True)
asyncio.run(main())
