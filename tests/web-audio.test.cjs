// SPDX-License-Identifier: GPL-3.0-or-later
const {test}=require('node:test');const assert=require('node:assert/strict');const vm=require('node:vm');const fs=require('node:fs');const path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../web/quad-snd.js'),'utf8');
function setup(fail){
 const warnings=[],notice={textContent:''};class Audio {constructor(){this.listener={};this.sampleRate=48000;}decodeAudioData(){return fail?Promise.reject(new Error('EncodingError')):Promise.resolve({length:1000});}createBuffer(channels,length,sampleRate){return {channels,length,sampleRate};}}
 const ctx=vm.createContext({window:{AudioContext:Audio},document:{addEventListener(){},querySelector(){return notice;}},console:{warn:(...args)=>warnings.push(args),log(){}},wasm_memory:{buffer:new ArrayBuffer(8)},miniquad_add_plugin(){}});vm.runInContext(source,ctx);vm.runInContext('audio_init()',ctx);return {ctx,warnings,notice};
}
test('a rejected audio decoder completes a silent cue rather than hanging loading',async()=>{
 const s=setup(true);const id=vm.runInContext('audio_add_buffer(0,8)',s.ctx);await new Promise(r=>setImmediate(r));assert.equal(vm.runInContext(`audio_source_is_loaded(${id})`,s.ctx),true);assert.equal(vm.runInContext(`sounds.get(${id}).length`,s.ctx),1);assert.equal(s.warnings.length,1);assert.match(s.notice.textContent,/Gameplay still works/);
});
test('supported audio decodes normally without fallback or warnings',async()=>{
 const s=setup(false);const id=vm.runInContext('audio_add_buffer(0,8)',s.ctx);await new Promise(r=>setImmediate(r));assert.equal(vm.runInContext(`audio_source_is_loaded(${id})`,s.ctx),true);assert.equal(vm.runInContext(`sounds.get(${id}).length`,s.ctx),1000);assert.equal(s.warnings.length,0);
});
