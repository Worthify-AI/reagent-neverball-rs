'use strict';
miniquad_add_plugin({register_plugin(importObject){importObject.env.worthify_status=(phase,coins,seconds)=>{const s=document.getElementById('game-status');s.textContent=`${['Ready','Playing','Paused','Won','Fell','Time up'][phase]}. ${coins} of 10 coins. ${seconds} seconds remaining.`;Object.assign(s.dataset,{phase:String(phase),coins:String(coins),seconds:String(seconds)});};}});
load('reagent_neverball_rs.wasm');
