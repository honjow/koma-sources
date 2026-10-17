from pathlib import Path
import sys, tempfile, subprocess, json, shutil, importlib.util, os, hashlib
from PIL import Image
repo=Path(__file__).resolve().parents[1];ev=Path(os.environ.get('KOMA_ICON_TEST_ARTIFACT_DIR',tempfile.mkdtemp(prefix='koma-icon-contract-report-')));ev.mkdir(parents=True,exist_ok=True);spec=importlib.util.spec_from_file_location('icons',repo/'scripts/validate-source-icon.py');icons=importlib.util.module_from_spec(spec);spec.loader.exec_module(icons);results=[]
def run(name,fn):
 try:fn();results.append({'case':name,'state':'PASS'})
 except Exception as e:results.append({'case':name,'state':'FAIL','error':repr(e)})
def rejects(fn):
 try:fn()
 except icons.ValidationError:return
 raise AssertionError('unexpected acceptance')
with tempfile.TemporaryDirectory(prefix='koma-icon-contract-') as tmp:
 t=Path(tmp)
 def formats():
  for ext,fmt in icons.FORMATS.items():
   p=t/('icon'+ext);im=Image.new('RGB',(16,16),(20,120,200));im.save(p,format=fmt);icons.validate_icon(p);p.unlink()
 run('six_actual_decodable_formats_synthetic_pixels',formats)
 def required():
  rejects(lambda:icons.source_icon(t));(t/'icon.png').touch();rejects(lambda:icons.source_icon(t));(t/'icon.png').unlink();Image.new('RGB',(16,16)).save(t/'icon.png');(t/'manifest.json').write_text(json.dumps({'icon':'icon.gif'}));rejects(lambda:icons.source_icon(t,True));(t/'manifest.json').write_text(json.dumps({'icon':'icon.png'}));icons.source_icon(t,True);shutil.copyfile(t/'icon.png',t/'icon.jpg');rejects(lambda:icons.source_icon(t));(t/'icon.jpg').unlink();(t/'icon.png').rename(t/'real.png');(t/'icon.png').symlink_to(t/'real.png');rejects(lambda:icons.source_icon(t));(t/'icon.png').unlink();(t/'real.png').unlink()
 run('missing_empty_mismatched_duplicate_symlink_rejected',required)
 def budgets():
  p=t/'icon.gif';Image.new('RGB',(1026,102)).save(p);icons.validate_icon(p);p.unlink();p=t/'icon.png';Image.new('RGB',(1025,1024)).save(p);rejects(lambda:icons.validate_icon(p));p.unlink();p=t/'icon.jpeg';p.write_bytes(b'not an image');rejects(lambda:icons.validate_icon(p));p.unlink()
 run('wide_original_compatible_total_pixel_budget_and_bad_bytes',budgets)
 # Exercise the actual build.sh path without compiling/rebuilding any parser.
 def packaging():
  fixture=t/'production-fixture';fixture.mkdir();shutil.copyfile(repo/'build.sh',fixture/'build.sh');(fixture/'scripts').mkdir();shutil.copyfile(repo/'scripts/validate-source-icon.py',fixture/'scripts/validate-source-icon.py');shutil.copyfile(repo/'scripts/read-source-localized-metadata.py',fixture/'scripts/read-source-localized-metadata.py');src=fixture/'sources/mangabz';src.mkdir(parents=True);(src/'Cargo.toml').write_text('[package]\nname="koma_mangabz_source"\n');target=fixture/'target/wasm32-unknown-unknown/release';target.mkdir(parents=True);(target/'koma_mangabz_source.wasm').write_bytes(b'\0asm\1\0\0\0');bin=t/'bin';bin.mkdir();cargo=bin/'cargo';cargo.write_text('#!/bin/sh\nexit 0\n');cargo.chmod(0o755);runner=bin/'runner';runner.write_text('#!/bin/sh\nprintf \'%s\\n\' \'{"data":{"sourceInfo":{"id":"test.icon.fixture","name":"Icon fixture","version":"0.1.0","language":"zh","author":"test","description":"synthetic packaging fixture","contentRating":"unknown"}}}\'\n');runner.chmod(0o755);env=os.environ.copy();env.update(REAL_HOME=str(t/'no-rustup'),PATH=str(bin)+':'+env['PATH'],DEV_RUNNER=str(runner));cmd=['bash',str(fixture/'build.sh'),'--source','mangabz'];p=subprocess.run(cmd,env=env,capture_output=True,text=True);assert p.returncode!=0 and 'exactly one' in p.stderr
  sample=t/'wide-original-fixture.gif';Image.new('RGB',(1026,102),(20,120,200)).save(sample);original=sample.read_bytes();(src/'icon.gif').write_bytes(original)
  import zipfile
  for tag in [None,'v-test-icons']:
   p=subprocess.run(cmd+(['--tag',tag] if tag else []),env=env,capture_output=True,text=True);(ev/('release-build-fixture.log' if tag else 'static-build-fixture.log')).write_text(p.stdout+p.stderr);assert p.returncode==0,p.stderr;idx=json.loads((fixture/'dist/index.json').read_text())[0];assert idx['icon']==('mangabz-icon.gif' if tag else 'sources/mangabz/icon.gif');published=fixture/'dist'/('release-icons' if tag else '')/idx['icon'];assert published.read_bytes()==original
   with zipfile.ZipFile(fixture/'dist/sources/mangabz/mangabz-0.1.0.koma') as z:
    assert set(z.namelist())=={'manifest.json','source.wasm','icon.gif'};assert z.read('icon.gif')==original;assert json.loads(z.read('manifest.json'))['icon']=='icon.gif'
  # Invoke the actual index validator with an empty and a missing resource.
  script=(repo/'build.sh').read_text();func=script[script.index('validate_index_icons() {'):script.index('\n# Main')];harness='set -euo pipefail\nlog(){ echo "$@" >&2; }\nOUTPUT_DIR="$1"\nRELEASE_ICON_DIR="$1/release-icons"\nVERSION_TAG=""\nICON_VALIDATOR="$2"\n'+func+'\nvalidate_index_icons\n';test=fixture/'index-test.sh';test.write_text(harness)
  for icon in ['', 'sources/mangabz/icon.png']:
   (fixture/'dist/index.json').write_text(json.dumps([{'icon':icon}]));p=subprocess.run(['bash',str(test),str(fixture/'dist'),str(repo/'scripts/validate-source-icon.py')],capture_output=True,text=True);assert p.returncode!=0
 run('actual_build_static_release_manifest_zip_index_bytes_and_missing_gate',packaging)
(ev/'verification.json').write_text(json.dumps({'scope':'Production packaging/validator only; synthetic WASM/runner used explicitly; no source parser compile/runtime acceptance','results':results},ensure_ascii=False,indent=2)+'\n');print(json.dumps(results,ensure_ascii=False,indent=2));sys.exit(any(x['state']!='PASS' for x in results))
