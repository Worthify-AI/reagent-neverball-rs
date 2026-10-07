# SPDX-License-Identifier: GPL-3.0-or-later
"""Convert licensed SOL v8 visual data, without game implementation source."""
import json, struct, sys
from pathlib import Path

def decode(path):
    b=Path(path).read_bytes(); c=struct.unpack_from('<23i', b)
    assert c[:2] == (1280267183,8)
    o=92+c[2]+8*c[3]; materials=[]
    for _ in range(c[4]):
        v=struct.unpack_from('<17fi',b,o)
        materials.append(dict(name=b[o+72:o+136].split(b'\0')[0].decode(),diffuse=v[:4],flags=v[17]));o+=136
    def rows(n,fmt):
        nonlocal o
        size=struct.calcsize(fmt);r=[struct.unpack_from(fmt,b,o+i*size) for i in range(n)];o+=size*n;return r
    vertices=rows(c[5],'<3f');rows(c[6],'<2i');normals=rows(c[7],'<4f');uvs=rows(c[8],'<2f');offsets=rows(c[9],'<3i');geoms=rows(c[10],'<4i')
    triangles=[]
    for m,*offs in geoms:
        pts=[];ns=[];ts=[]
        for i in offs:
            t,s,v=offsets[i];pts.append(vertices[v]);ns.append(normals[s][:3]);ts.append(uvs[t])
        triangles.append(dict(m=m,p=pts,n=ns,uv=ts))
    return dict(materials=materials,triangles=triangles)
if __name__=='__main__':
    p=Path(sys.argv[1]); a=p/'assets/neverball'
    for name,src in [('course',a/'map-easy/easy.sol'),('ball',a/'ball/basic-ball/basic-ball-solid.sol'),('coin',a/'item/coin/coin.sol')]:
        obj=decode(src)
        if name=='course':
            old=json.loads((p/'fixtures/course.json').read_text())
            assert json.loads(json.dumps([t['p'] for t in obj['triangles']]))==[t['p'] for t in old['mesh']], 'Geometry must remain identical'
        (p/f'fixtures/{name}-visual.json').write_text(json.dumps(obj,separators=(',',':'))+'\n')
    b=(a/'map-back/clouds.sol').read_bytes();bill=[]
    for i in range(76):
        v=struct.unpack_from('<2i20f',b,1092+i*88)
        bill.append(dict(m=v[1],distance=v[3],width=v[4],height=v[7],rx=v[10],ry=v[13],rz=v[16]))
    (p/'fixtures/background-visual.json').write_text(json.dumps(bill,separators=(',',':'))+'\n')
