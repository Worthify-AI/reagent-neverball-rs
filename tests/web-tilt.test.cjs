// SPDX-License-Identifier: GPL-3.0-or-later
const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');const fs=require('node:fs');const path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../web/tilt-controls.js'),'utf8');
class Target {
 constructor(){this.handlers={};this.dataset={};this.attrs={};this.disabled=false;this.textContent='';}
 addEventListener(name,handler){(this.handlers[name]??=new Set()).add(handler);}
 removeEventListener(name,handler){this.handlers[name]?.delete(handler);}
 dispatch(name,event={}){return Promise.all([...this.handlers[name]||[]].map(h=>h(event)));}
 setAttribute(name,value){this.attrs[name]=value;}
 focus(){}
}
function setup(permission='granted'){
 let now=0,calls=0;const timers=new Map(),window=new Target(),document=new Target(),screen={orientation:new Target()};screen.orientation.angle=0;document.hidden=false;
 window.isSecureContext=true;function Orientation(){};Orientation.requestPermission=()=>{calls++;if(permission==='throw')throw new Error('NotAllowed');return Promise.resolve(permission);};window.DeviceOrientationEvent=Orientation;
 vm.runInNewContext(source,{window,document,screen,DeviceOrientationEvent:Orientation,performance:{now:()=>now},setTimeout:(fn)=>{timers.set(fn,fn);return fn;},clearTimeout:id=>timers.delete(id)});
 const enableButton=new Target(),centerButton=new Target(),status=new Target(),canvas=new Target();const controls=window.WorthifyTilt.create({enableButton,centerButton,status,canvas});
 const sample=async(beta,gamma,count=40)=>{for(let i=0;i<count;i++){now+=20;await window.dispatch('deviceorientation',{beta,gamma});}};
 return {window,document,screen,enableButton,centerButton,status,controls,sample,timers,calls:()=>calls};
}
test('motion permission waits for an explicit tap; calibration starts neutral',async()=>{
 const s=setup();assert.equal(s.calls(),0);assert.equal(s.controls.axis().active,0);await s.enableButton.dispatch('click');assert.equal(s.calls(),1);await s.sample(60,0,1);assert.deepEqual({...s.controls.axis()},{x:0,y:0,active:1});assert.equal(s.status.dataset.state,'active');
});
test('relative pitch works from an upright neutral angle and both axes have the expected sign',async()=>{
 const s=setup();await s.enableButton.dispatch('click');await s.sample(60,0,1);await s.sample(67,0);assert.ok(Math.abs(s.controls.axis().x)<0.01);assert.ok(s.controls.axis().y<-.19&&s.controls.axis().y>-.24);await s.centerButton.dispatch('click');await s.sample(60,0,1);await s.sample(60,7);assert.ok(s.controls.axis().x>.19&&s.controls.axis().x<.24);assert.ok(Math.abs(s.controls.axis().y)<.01);
});
test('landscape in either direction rotates steering axes and recenters after rotation',async()=>{
 for(const angle of [90,270]){const s=setup();s.screen.orientation.angle=angle;await s.enableButton.dispatch('click');await s.sample(0,0,1);await s.sample(0,7);const a=s.controls.axis();assert.ok(Math.abs(a.x)<.01);assert.ok(angle===90?a.y>.19:a.y<-.19);await s.screen.orientation.dispatch('change');assert.equal(s.controls.axis().active,0);await s.sample(0,7,1);assert.equal(s.controls.axis().x,0);assert.equal(s.controls.axis().y,0);}
});
test('centering clears steering immediately; disabling removes motion control',async()=>{
 const s=setup();await s.enableButton.dispatch('click');await s.sample(40,0,1);await s.sample(40,35);assert.ok(s.controls.axis().x>.95);await s.centerButton.dispatch('click');assert.equal(s.controls.axis().active,0);await s.sample(40,35,1);assert.equal(s.controls.axis().x,0);await s.enableButton.dispatch('click');await s.sample(0,0);assert.equal(s.controls.axis().active,0);assert.equal(s.enableButton.attrs['aria-pressed'],'false');
});
test('denied and rejected permission keep gameplay on fallback controls',async()=>{
 for(const permission of ['denied','throw']){const s=setup(permission);await s.enableButton.dispatch('click');assert.equal(s.controls.axis().active,0);assert.equal(s.enableButton.disabled,false);assert.match(s.status.textContent,/Touch arrows still work/);}
});
test('missing sensors, insecure pages, and invalid readings do not steer',async()=>{
 for(const kind of ['missing','insecure']){const s=setup();if(kind==='missing')s.window.DeviceOrientationEvent=undefined;else s.window.isSecureContext=false;await s.enableButton.dispatch('click');assert.equal(s.calls(),0);assert.equal(s.controls.axis().active,0);}
 const s=setup();await s.enableButton.dispatch('click');await s.sample(null,NaN,1);assert.equal(s.controls.axis().active,0);for(const fn of [...s.timers.values()])fn();assert.equal(s.enableButton.disabled,false);assert.match(s.status.textContent,/No motion data/);
});
test('hidden and blurred pages drop steering until a fresh neutral sample',async()=>{
 const s=setup();await s.enableButton.dispatch('click');await s.sample(0,0,1);await s.sample(0,14);assert.ok(s.controls.axis().x>.4);s.document.hidden=true;await s.document.dispatch('visibilitychange');assert.equal(s.controls.axis().active,0);s.document.hidden=false;await s.document.dispatch('visibilitychange');await s.sample(0,14,1);assert.equal(s.controls.axis().x,0);await s.window.dispatch('blur');assert.equal(s.controls.axis().active,0);
});
test('browsers without a permission method can opt in; page exit cancels a pending prompt',async()=>{
 const s=setup();delete s.window.DeviceOrientationEvent.requestPermission;await s.enableButton.dispatch('click');await s.sample(0,0,1);assert.equal(s.controls.axis().active,1);await s.window.dispatch('pagehide');assert.equal(s.controls.axis().active,0);
 const p=setup();let resolve;p.window.DeviceOrientationEvent.requestPermission=()=>new Promise(r=>resolve=r);const click=p.enableButton.dispatch('click');await p.window.dispatch('pagehide');resolve('granted');await click;assert.equal(p.controls.axis().active,0);
});
