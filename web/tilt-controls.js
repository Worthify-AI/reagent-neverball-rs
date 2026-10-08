'use strict';
// Motion stays on this device. Only relative screen tilt is used; no compass heading.
(() => {
 const clamp=v=>Math.max(-1,Math.min(1,v));
 window.WorthifyTilt={create({enableButton,centerButton,status,canvas}){
  let enabled=false,waiting=false,epoch=0,neutral=null,last=null,filtered={x:0,y:0},timer;
  const reset=()=>{neutral=null;last=null;filtered={x:0,y:0};centerButton.disabled=true;};
  const angle=()=>Number.isFinite(screen.orientation?.angle)?screen.orientation.angle:(Number(window.orientation)||0);
  const attitude=event=>{
   if(!Number.isFinite(event.beta)||!Number.isFinite(event.gamma))return null;
   const b=event.beta*Math.PI/180,g=event.gamma*Math.PI/180;
   // Columns of Rx(beta) Ry(gamma). Compass/yaw is deliberately unused.
   return [[Math.cos(g),Math.sin(b)*Math.sin(g),-Math.cos(b)*Math.sin(g)],
           [0,Math.cos(b),Math.sin(b)],
           [Math.sin(g),-Math.sin(b)*Math.cos(g),Math.cos(b)*Math.cos(g)]];
  };
  const relative=current=>{
   const normal=neutral[2],dot=v=>normal.reduce((sum,n,i)=>sum+n*v[i],0);
   const z=dot(current[2]);
   const right=Math.atan2(-dot(current[0]),z)/(28*Math.PI/180);
   const forward=-Math.atan2(dot(current[1]),z)/(28*Math.PI/180);
   const a=angle()*Math.PI/180;
   return {x:clamp(right*Math.cos(a)-forward*Math.sin(a)),y:clamp(right*Math.sin(a)+forward*Math.cos(a))};
  };
  const stop=message=>{
   ++epoch;enabled=false;waiting=false;clearTimeout(timer);reset();
   window.removeEventListener('deviceorientation',onOrientation);
   enableButton.textContent='Enable tilt';enableButton.disabled=false;enableButton.setAttribute('aria-pressed','false');
   status.textContent=message||'Tilt off. Touch arrows still work.';status.dataset.state='off';
  };
  function onOrientation(event){
   if(!enabled||document.hidden)return;
   const value=attitude(event);if(!value)return;
   const now=performance.now();clearTimeout(timer);
   if(!neutral){neutral=value;filtered={x:0,y:0};last=now;centerButton.disabled=false;status.textContent='Tilt on. Center tilt resets your neutral angle.';status.dataset.state='active';return;}
   const valueTilt=relative(value),dead=v=>Math.abs(v)<0.055?0:Math.sign(v)*(Math.abs(v)-0.055)/0.945;
   const target={x:dead(valueTilt.x),y:dead(valueTilt.y)};
   const dt=Math.max(0.001,Math.min(0.1,(now-last)/1000));const blend=1-Math.exp(-dt/0.08);last=now;
   filtered.x+=blend*(target.x-filtered.x);filtered.y+=blend*(target.y-filtered.y);
   if(target.x===0&&Math.abs(filtered.x)<0.01)filtered.x=0;
   if(target.y===0&&Math.abs(filtered.y)<0.01)filtered.y=0;
  }
  enableButton.addEventListener('click',async()=>{
   if(enabled){stop();canvas.focus();return;}if(waiting)return;
   if(!window.isSecureContext||!window.DeviceOrientationEvent){stop('Tilt is unavailable here. Use touch arrows.');return;}
   waiting=true;const attempt=++epoch;enableButton.disabled=true;status.dataset.state='permission';status.textContent='Waiting for motion permission…';
   try{
    // Invoke inside the tap handler, before any await: Safari requires user activation.
    const request=typeof DeviceOrientationEvent.requestPermission==='function'?DeviceOrientationEvent.requestPermission():Promise.resolve('granted');
    const permission=await request;if(attempt!==epoch)return;
    if(permission!=='granted'){stop('Motion access was declined. Touch arrows still work.');return;}
    waiting=false;enabled=true;reset();enableButton.disabled=false;enableButton.textContent='Disable tilt';enableButton.setAttribute('aria-pressed','true');
    status.dataset.state='waiting';status.textContent='Hold the device comfortably and still to set the neutral angle…';
    window.addEventListener('deviceorientation',onOrientation,{passive:true});canvas.focus();
    timer=setTimeout(()=>{if(enabled&&!neutral)stop('No motion data received. Use touch arrows, or open the game in its own tab.');},5000);
   }catch(_){if(attempt===epoch)stop('Could not enable motion access. Touch arrows still work.');}
  });
  centerButton.addEventListener('click',()=>{if(enabled){reset();status.textContent='Hold still to center tilt…';status.dataset.state='waiting';canvas.focus();}});
  const recenter=()=>{if(enabled){reset();status.textContent='Hold still to center tilt…';status.dataset.state='waiting';}};
  window.addEventListener('orientationchange',recenter);screen.orientation?.addEventListener('change',recenter);
  document.addEventListener('visibilitychange',recenter);window.addEventListener('blur',recenter);window.addEventListener('pagehide',()=>stop());
  return {axis(){return enabled&&neutral&&!document.hidden?{x:filtered.x,y:filtered.y,active:1}:{x:0,y:0,active:0};}};
 }};
})();
