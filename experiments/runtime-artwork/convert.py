#!/usr/bin/env python3
"""Original standard-library RGBA-to-PNG converter. Reads capture outputs only."""
import argparse, hashlib, json, os, struct, zlib
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('session',type=Path);p.add_argument('--flip-y',action='store_true');p.add_argument('--fixture',action='store_true');a=p.parse_args()
def atomic(path,data):
    tmp=path.with_suffix(path.suffix+'.tmp')
    with tmp.open('xb') as f:f.write(data);f.flush();os.fsync(f.fileno())
    os.replace(tmp,path)
    fd=os.open(path.parent,os.O_RDONLY);os.fsync(fd);os.close(fd)
def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
items=[];records=[]
for meta in sorted(a.session.glob('event-*.json')):
    v=json.loads(meta.read_text());records.append(v)
    if v['status']!='captured':
        if v['bytes'] or v['raw_file'] is not None:raise ValueError('Failed/skip record has successful payload')
        continue
    name=v['raw_file']
    if Path(name).name!=name:raise ValueError('Nonlocal raw path')
    data=(a.session/name).read_bytes();w=v['width'];h=v['height']
    if not (0<w<=8192 and 0<h<=8192 and len(data)==v['bytes']==w*h*4):raise ValueError('Wrong dimensions or byte count')
    if v['upload_error'] or v['capture_error']:raise ValueError('Captured record has GL error')
    rows=[data[i*w*4:(i+1)*w*4] for i in range(h)]
    if a.flip_y:rows.reverse()
    png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(b''.join(b'\x00'+r for r in rows)))+chunk(b'IEND',b'')
    target=a.session/(Path(name).stem+('-flip-y' if a.flip_y else '')+'.png');atomic(target,png)
    items.append(dict(event=v['event'],raw=name,raw_sha256=hashlib.sha256(data).hexdigest(),png=target.name,png_sha256=hashlib.sha256(png).hexdigest(),flip_y=a.flip_y,width=w,height=h,alpha_min=min(data[3::4]),alpha_max=max(data[3::4])))
if a.fixture:
    expected_rgba=bytes([255,0,0,0,0,255,0,64,0,0,255,128,123,45,67,255])
    expected_rgb=bytes([255,0,0,255,0,255,0,255,0,0,255,255,123,45,67,255])
    statuses=[v['status'] for v in records]
    assert statuses==['captured','captured','skipped_null_allocation','captured']+['captured']*4+['failed_upload'],statuses
    captures=[v for v in records if v['status']=='captured']
    expected_lum=bytes([0,0,0,255,64,64,64,255,128,128,128,255,255,255,255,255])
    expected_la=bytes([17,17,17,0,63,63,63,64,129,129,129,128,251,251,251,255])
    expected=[expected_rgba,expected_rgb,expected_rgba,expected_lum,expected_lum,expected_la,expected_la]
    assert len(captures)==len(expected)
    for v,expected in zip(captures,expected):
        assert (a.session/v['raw_file']).read_bytes()==expected,'Synthetic pixel mismatch'
    assert captures[2]['unpack_buffer']!=0
    assert [v['internal_format'] for v in captures[3:]]==[6409,32832,6410,32837]
    assert all(v['channel_expansion']!='none' for v in captures[3:])
manifest=dict(kind='synthetic_fixture' if a.fixture else 'runtime_texture_capture',artwork_source='OpenGL readback; source files were not opened by this converter',flip_y=a.flip_y,captures=items,records=len(records),fixture_exact_pixel_check='passed' if a.fixture else 'not_requested')
atomic(a.session/('manifest-flip-y.json' if a.flip_y else 'manifest.json'),(json.dumps(manifest,indent=2)+'\n').encode())
print(json.dumps(manifest,indent=2))
