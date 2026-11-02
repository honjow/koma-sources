#!/usr/bin/env python3
"""Build unpublished schema-2 plugin resources from independent definitions."""
import argparse, hashlib, json, math
from pathlib import Path
import generate

def plugin_meta(value):
    generate.require(isinstance(value,dict),'meta must be object')
    result={key:generate.text(value,key) for key in ('name','description')}
    for key in ('author','website'):
        if key in value:
            generate.require(isinstance(value[key],str),f'invalid meta {key}')
            result[key]=value[key]
    return result

def validate_settings(settings):
    generate.require(isinstance(settings,list) and len(settings)<=64,'invalid settings')
    ids=set()
    for field in settings:
        generate.require(isinstance(field,dict),'invalid setting')
        ident=generate.text(field,'id'); generate.text(field,'label');kind=field.get('kind')
        generate.require(ident not in ids and kind in ('string','boolean','select','multiselect','range') and not field.get('sensitive') and not field.get('loginRefKey') and 'validation' not in field,'invalid setting')
        ids.add(ident);options=[]
        if kind in ('select','multiselect'):
            opts=field.get('options');generate.require(isinstance(opts,list),'invalid options')
            for opt in opts:
                generate.require(isinstance(opt,dict) and isinstance(opt.get('id'),str),'invalid option');generate.text(opt,'label');options.append(opt['id'])
            generate.require(len(set(options))==len(options),'duplicate option id')
        if kind=='range':
            def finite(v): return type(v) in (int,float) and math.isfinite(v)
            lo,hi,step=(field.get(k) for k in ('minValue','maxValue','step'))
            generate.require(finite(lo) and finite(hi) and finite(step) and hi>lo and step>0,'invalid range')
        if 'defaultValue' in field:
            default=field['defaultValue']
            valid=(kind=='string' and isinstance(default,str) or kind=='boolean' and type(default) is bool or kind=='range' and type(default) in (int,float) and math.isfinite(default) or kind=='select' and isinstance(default,str) and default in options or kind=='multiselect' and isinstance(default,list))
            generate.require(valid,'invalid defaultValue')

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--definitions',type=Path,default=generate.ROOT/'plugin-definitions')
    p.add_argument('--catalog',type=Path,default=generate.ROOT/'plugin-catalog.json')
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--processor',action='append',default=[],help='override module path as plugin-id=path')
    a=p.parse_args(); overrides=dict(x.split('=',1) for x in a.processor); files=sorted(a.definitions.glob('*.json'))
    generate.require(bool(files),'no plugin definitions'); catalog=generate.meta(json.loads(a.catalog.read_text())); entries=[];assets={};ids=set()
    for file in files:
        src=json.loads(file.read_text());pid=generate.text(src,'id');version=src.get('version');generate.require(pid not in ids and type(version) is int and version>0,'invalid plugin id/version');ids.add(pid)
        proc=src.get('processor',{});generate.require(proc.get('abi')=='koma-image-v1','unsupported ABI');wasm=Path(overrides[pid]) if pid in overrides else file.parent/generate.text(proc,'wasm');raw=wasm.read_bytes();generate.require(raw[:4]==b'\0asm' and len(raw)<=2*1024*1024,'invalid or oversized WASM');wd=hashlib.sha256(raw).hexdigest();assets[wd+'.wasm']=raw
        params=src.get('parameters',{})
        if 'parametersFromRule' in src:
            rule=generate.build_pack(file.parent/src['parametersFromRule'])['rules'][0];params={k:rule[k] for k in ('signatures','top','bottom')}
        generate.require(isinstance(params,dict),'parameters must be object');application=src.get('application');generate.require(isinstance(application,dict) and type(application.get('automatic')) is bool and isinstance(application.get('sourceHints'),list),'invalid application')
        pack={'schemaVersion':2,'id':pid,'version':version,'meta':plugin_meta(src.get('meta')),'processor':{'abi':'koma-image-v1','path':wd+'.wasm','sha256':wd},'parameters':params,'application':application}
        if 'output' in src:
            output=src['output'];generate.require(isinstance(output,dict) and output.get('format') in ('image/jpeg','image/png','image/webp'),'invalid output format')
            generate.require(type(output.get('quality')) is int and 0<=output['quality']<=100,'invalid output quality')
            pack['output']=output
        if 'settings' in src:
            settings=src['settings'];validate_settings(settings)
            pack['settings']=settings
        b=generate.encode(pack);pd=hashlib.sha256(b).hexdigest();assets[pd+'.json']=b;entries.append({'id':pid,'version':version,'path':pd+'.json','sha256':pd})
    revision=hashlib.sha256(generate.encode({'meta':catalog,'packs':entries})).hexdigest();index={'schemaVersion':2,'kind':'image_plugins','revision':revision,'meta':catalog,'packs':entries}
    a.output.mkdir(parents=True,exist_ok=True)
    for name,raw in assets.items():(a.output/name).write_bytes(raw)
    (a.output/'index.json').write_bytes(generate.encode(index));print('plugins',len(entries),'revision',revision)
if __name__=='__main__':main()
