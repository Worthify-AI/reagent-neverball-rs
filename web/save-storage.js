'use strict';
// Local profile only: game progress/settings/replays stay in this browser.
miniquad_add_plugin({name:"worthify_storage",version:1,register_plugin(importObject){
  const key='neverball-rust-save-v1';
  importObject.env.worthify_save=(ptr,len)=>{
    try {localStorage.setItem(key,new TextDecoder().decode(new Uint8Array(wasm_memory.buffer,ptr,len)));return 1;} catch (_) {return 0;}
  };
  importObject.env.worthify_load=(ptr,capacity)=>{
    try {const bytes=new TextEncoder().encode(localStorage.getItem(key)||'');if(capacity>=bytes.length&&ptr!==0)new Uint8Array(wasm_memory.buffer,ptr,bytes.length).set(bytes);return bytes.length;}catch(_){return 0;}
  };
}});
