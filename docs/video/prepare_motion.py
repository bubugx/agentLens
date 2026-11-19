"""Extract short UI interaction sequences; source recordings remain local."""
from pathlib import Path
import os,re,subprocess
import imageio_ffmpeg
HERE=Path(__file__).resolve().parent
ff=os.environ.get('FFMPEG_BINARY',imageio_ffmpeg.get_ffmpeg_exe())
for name in ['overview','processes','adoption','cleanup']:
 source=HERE/'assets/motion'/f'{name}.webm'
 if not source.exists():raise FileNotFoundError(f'{source}: run node docs/video/capture_motion.mjs first')
 log=subprocess.run([ff,'-hide_banner','-i',str(source)],capture_output=True,text=True).stderr
 match=re.search(r'Duration: (\d+):(\d+):(\d+\.\d+)',log)
 duration=int(match[1])*3600+int(match[2])*60+float(match[3])
 out=source.with_suffix('');out.mkdir(exist_ok=True)
 for p in out.glob('*.jpg'):p.unlink()
 subprocess.run([ff,'-y','-v','error','-ss',str(max(0,duration-4.8)),'-i',str(source),'-t','4.8','-vf','fps=12,scale=1280:-2','-q:v','3',str(out/'%04d.jpg')],check=True)
 print(name,len(list(out.glob('*.jpg'))),flush=True)
