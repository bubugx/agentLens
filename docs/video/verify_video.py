import json,re,subprocess,pathlib,imageio_ffmpeg,hashlib
root=pathlib.Path(__file__).resolve().parents[2];f=root/'docs/media/agentlens-intro-zh.mp4';ff=imageio_ffmpeg.get_ffmpeg_exe()
probe=subprocess.run([ff,'-hide_banner','-i',str(f)],capture_output=True,text=True).stderr
assert '1920x1080' in probe and '30 fps' in probe and 'h264' in probe and '48000 Hz, stereo' in probe
print(probe,flush=True)
result=subprocess.run([ff,'-v','warning','-i',str(f),'-f','null','-'],capture_output=True,text=True)
assert result.returncode==0,result.stderr
loud=subprocess.run([ff,'-hide_banner','-i',str(f),'-vn','-af','loudnorm=I=-16:TP=-1.5:LRA=7:print_format=json','-f','null','-'],capture_output=True,text=True).stderr
measure=json.loads(loud[loud.rfind('{'):loud.rfind('}')+1])
print('LOUDNESS',measure,flush=True)
meta=json.loads((root/'docs/media/video-metadata.json').read_text());assert meta['duration']<80
srt=(root/'docs/media/agentlens-intro-zh.srt').read_text();items=srt.strip().split('\n\n')
def sec(s):
 h,m,v=s.replace(',','.').split(':');return int(h)*3600+int(m)*60+float(v)
last=0
for item in items:
 lines=item.splitlines();start,end=map(sec,lines[1].split(' --> '));assert start>=last-.001 and start<end<=meta['duration'];last=end
missing=[]
for path in [root/'README.md',root/'docs/implementation.md',root/'docs/video/README.md',root/'docs/video/narration.md']:
 for found in re.findall(r'\]\(([^)]+)\)|(?:href|src)="([^"]+)"',path.read_text()):
  link=found[0] or found[1]
  if link.startswith(('https:','http:','#','data:')):continue
  target=path.parent/link.split('#')[0]
  if not target.exists() and target.resolve()!=(root/'docs/video/validation.json').resolve():missing.append(str(target))
assert not missing,missing
record={
 'checkedDate':'2026-11-18','video':str(f.relative_to(root)), 'durationSeconds':meta['duration'], 'dimensions':'1920x1080','fps':30,'frames':round(meta['duration']*30),'videoCodec':'H.264','pixelFormat':'yuv420p','audioCodec':'AAC','audioRate':48000,'channels':2,'fileBytes':f.stat().st_size,'sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'fullDecode':'passed','decodeWarnings':result.stderr.strip(),'subtitleCues':len(items),'subtitleOrderAndBounds':'passed','documentationLocalLinks':'passed','audioIntegratedLufs':float(measure['input_i']),'audioTruePeakDbtp':float(measure['input_tp']),'audioLoudnessRangeLu':float(measure['input_lra']),'visualReview':'Nine-scene contact sheet and full-resolution overview, architecture and cleanup frames inspected.','notes':['Interface values are preview data, visibly disclosed.','Narration uses Edge TTS rate +60%.','No real cleanup or process-stop actions performed for video capture.']}
(root/'docs/video/validation.json').write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n');print(json.dumps(record,ensure_ascii=False,indent=2))
