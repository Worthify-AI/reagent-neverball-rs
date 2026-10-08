'use strict';
miniquad_add_plugin({name:"worthify_status",version:1,register_plugin(importObject){
 importObject.env.glPolygonOffset=(factor,units)=>gl.polygonOffset(factor,units);
 importObject.env.worthify_status=(phase,coins,seconds,target)=>{
  const s=document.getElementById('game-status');
  const name=['Level introduction','Playing','Paused','Won','Fell','Time up','Menu','Recorded replay'][phase]||'Game';
  const text=phase===6?'Menu. Choose Play, Replay, Help or Options.':`${name}. ${coins} of ${target} coins. ${seconds} seconds remaining.`;
  if(s.textContent!==text)s.textContent=text;
  Object.assign(s.dataset,{phase:String(phase),coins:String(coins),seconds:String(seconds),target:String(target)});
 };
}});
load('reagent_neverball_rs.wasm');
