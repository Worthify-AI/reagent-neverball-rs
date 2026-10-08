'use strict';
// Local controls only. No inputs, replay files or device information are sent anywhere.
(() => {
 const canvas=document.getElementById('glcanvas');
 let touchX=0,touchY=0,pressed=new Map(),pendingReplay=new Uint8Array(0);
 const controls=document.createElement('div');controls.className='game-controls';
 controls.innerHTML='<div class="touch-controls" aria-label="Touch game controls"><div class="direction-pad"><button data-axis="0,1" aria-label="Tilt forward">↑</button><button data-axis="-1,0" aria-label="Tilt left">←</button><button data-axis="0,-1" aria-label="Tilt backward">↓</button><button data-axis="1,0" aria-label="Tilt right">→</button></div><div class="action-buttons"><button data-key="Enter">Select / Start</button><button data-key="Escape">Pause</button><button data-key="KeyR">Restart</button><button data-key="KeyE">Camera</button></div></div><div class="tilt-controls" aria-label="Device tilt controls"><div class="tilt-buttons"><button class="tilt-enable" aria-pressed="false">Enable tilt</button><button class="tilt-center" disabled>Center tilt</button><a href="index.html" target="_blank" rel="noopener">Open game tab</a></div><span class="tilt-status" role="status" aria-live="polite">Tilt your phone or tablet to steer. Touch arrows also work.</span></div><label class="replay-import">Open replay <input type="file" accept=".nbr,.json" aria-label="Open a Neverball replay file"></label><span class="control-notice" role="status"></span>';
 document.querySelector('footer').append(controls);
 const tilt=WorthifyTilt.create({enableButton:controls.querySelector('.tilt-enable'),centerButton:controls.querySelector('.tilt-center'),status:controls.querySelector('.tilt-status'),canvas});
 const recalc=()=>{touchX=0;touchY=0;for(const v of pressed.values()){touchX+=v[0];touchY+=v[1];}touchX=Math.max(-1,Math.min(1,touchX));touchY=Math.max(-1,Math.min(1,touchY));};
 for(const button of controls.querySelectorAll('[data-axis]')){
  button.addEventListener('pointerdown',e=>{e.preventDefault();button.setPointerCapture(e.pointerId);pressed.set(e.pointerId,button.dataset.axis.split(',').map(Number));button.classList.add('held');recalc();canvas.focus();});
  const release=e=>{pressed.delete(e.pointerId);button.classList.remove('held');recalc();};
  button.addEventListener('pointerup',release);button.addEventListener('pointercancel',release);button.addEventListener('lostpointercapture',release);
 }
 for(const button of controls.querySelectorAll('[data-key]'))button.addEventListener('click',()=>{canvas.focus();canvas.dispatchEvent(new KeyboardEvent('keydown',{code:button.dataset.key,bubbles:true}));setTimeout(()=>canvas.dispatchEvent(new KeyboardEvent('keyup',{code:button.dataset.key,bubbles:true})),100);});
 window.addEventListener('keydown',event=>{if(event.code==='F12'&&document.activeElement===canvas)event.preventDefault();},true);
 window.addEventListener('blur',()=>{pressed.clear();recalc();});
 const notice=controls.querySelector('.control-notice');
 controls.querySelector('input').addEventListener('change',async e=>{
  const file=e.target.files[0];if(!file)return;
  if(file.size>16*1024*1024){notice.textContent='Replay is too large (16 MB maximum).';return;}
  try{pendingReplay=new Uint8Array(await file.arrayBuffer());notice.textContent='Opening replay…';canvas.focus();}catch(_){notice.textContent='Could not read this replay.';}
  e.target.value='';
 });
 let padPressed=new Set();
 const buttonMap={0:'Enter',1:'Escape',2:'KeyR',3:'KeyE',4:'KeyS',5:'KeyD'};
 const axis=()=>{
  if(pressed.size)return {x:touchX,y:touchY,active:1};
  if(document.activeElement!==canvas)return {x:0,y:0,active:false};
  const motion=tilt.axis();if(motion.active)return motion;
  if(!navigator.getGamepads)return {x:0,y:0,active:false};
  const pad=Array.from(navigator.getGamepads()).find(Boolean);
  if(!pad)return {x:0,y:0,active:false};
  const dead=v=>Math.abs(v)<0.12?0:Math.max(-1,Math.min(1,v));
  return {x:dead(pad.axes[0]||0),y:-dead(pad.axes[1]||0),active:2};
 };
 const pollButtons=()=>{
  const pad=document.activeElement===canvas&&navigator.getGamepads?Array.from(navigator.getGamepads()).find(Boolean):null;
  for(const [index,code] of Object.entries(buttonMap)){
   const down=!!pad?.buttons[index]?.pressed;
   if(down!==padPressed.has(code)){canvas.dispatchEvent(new KeyboardEvent(down?'keydown':'keyup',{code,bubbles:true}));if(down)padPressed.add(code);else padPressed.delete(code);}
  }
  requestAnimationFrame(pollButtons);
 };requestAnimationFrame(pollButtons);
 miniquad_add_plugin({name:"worthify_controls",version:1,register_plugin(importObject){
  importObject.env.worthify_axis_active=()=>Number(axis().active)||0;
  importObject.env.worthify_screenshot=()=>{
   canvas.toBlob(blob=>{if(!blob){notice.textContent='Screenshot could not be saved.';return;}const url=URL.createObjectURL(blob);const link=document.createElement('a');link.href=url;link.download='neverball-rust-'+new Date().toISOString().replace(/[:.]/g,'-')+'.png';link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);},'image/png');
  };
  importObject.env.worthify_axis_x=()=>axis().x;
  importObject.env.worthify_axis_y=()=>axis().y;
  importObject.env.worthify_replay_load=(ptr,capacity)=>{
   const size=pendingReplay.length;
   if(ptr!==0&&capacity>=size){new Uint8Array(wasm_memory.buffer,ptr,size).set(pendingReplay);pendingReplay=new Uint8Array(0);notice.textContent='';}
   return size;
  };
 }});
})();
