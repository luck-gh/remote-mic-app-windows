const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs'), vm = require('node:vm'), path = require('node:path');
const context = { module: { exports: {} } };
vm.runInNewContext(fs.readFileSync(path.join(__dirname, 'runtime.js'), 'utf8'), context);
const { Ownership, decodeInput } = context.module.exports;
function report(...usages) {
  const result = new Uint8Array(7); result[0] = 1;
  usages.forEach((usage, i) => { result[i * 2 + 1] = usage & 255; result[i * 2 + 2] = usage >>> 8; });
  return result;
}
test('existing native hold passes both edges before ownership', () => {
  const owner = new Ownership(); owner.configure(31);
  assert.deepEqual([...owner.accept(report(0x35)).output], [...report(0x35)]);
  assert.equal(owner.ready, false);
  assert.equal(owner.accept(report()).ready, true);
});
test('each configured key is captured once in a mixed report; voice and others pass', () => {
  for (const [i, usage] of [0xf1, 0x80, 0x81, 0x35, 0x4a].entries()) {
    const owner = new Ownership(); owner.configure(1 << i); owner.accept(report());
    const state = owner.accept(report(usage, 0x3e, 0x28));
    assert.equal(state.buttons, 1 << i);
    assert.deepEqual([...state.output], [...report(0, 0x3e, 0x28)]);
    assert.equal(owner.accept(report(usage, 0x3e, 0x28)).buttons, 1 << i);
    assert.equal(owner.accept(report()).buttons, 0);
  }
});
test('reconfigure or exit during hold drains the old native pair without new mapping', () => {
  const owner = new Ownership(); owner.configure(31); owner.accept(report());
  owner.accept(report(0xf1, 0x80));
  owner.configure(0);
  const held = owner.accept(report(0xf1, 0x80));
  assert.equal(held.buttons, 0); assert.equal(held.suppressed, 3);
  assert.deepEqual([...held.output], [...report()]);
  owner.configure(31);
  assert.equal(owner.accept(report(0xf1, 0x80)).buttons, 0);
  assert.equal(owner.accept(report()).suppressed, 0);
  assert.equal(owner.accept(report(0xf1)).buttons, 1);
});
test('unknown contract cannot mutate input or claim ownership', () => {
  const owner = new Ownership(); owner.configure(31); owner.accept(report());
  for (const bytes of [new Uint8Array(6), report(1), report(256), new Uint8Array([1,0,0,0,0,0,0,1])]) {
    const before = [...bytes];
    assert.throws(() => owner.accept(bytes));
    assert.deepEqual([...bytes], before); assert.equal(owner.owned, 0);
  }
});
test('collection padding is accepted only when zero, all five values share the declared slots', () => {
  const bytes = new Uint8Array(121); bytes.set(report(0x35, 0x4a, 0xf1));
  assert.equal(decodeInput(bytes).physical, 25);
  bytes[120] = 1; assert.throws(() => decodeInput(bytes));
});

// Execute the real adapter against an in-memory WDF contract, without a process,
// device, native interception, or Windows API. Assert writes as well as events.
function adapter() {
  const blocks=[], functions=new Map(), hooks=new Map(), messages=[];
  let next=0x100000;
  const receivers=new Map();
  class P {
    constructor(n){this.n=Number(n);}
    add(n){return new P(this.n+Number(n));} isNull(){return this.n===0;}
    equals(p){return this.n===p.n;} compare(p){return Math.sign(this.n-Number(p instanceof P?p.n:p));}
    toString(){return '0x'+this.n.toString(16);} toInt32(){return this.n|0;}
    at(){const b=blocks.find(b=>this.n>=b.start&&this.n<b.start+b.data.length);if(!b)throw Error('invalid memory');return [b.data,this.n-b.start];}
    readU8(){const[b,o]=this.at();return b[o];} readU16(){const[b,o]=this.at();return b.readUInt16LE(o);}
    readU32(){const[b,o]=this.at();return b.readUInt32LE(o);} writeU16(v){const[b,o]=this.at();b.writeUInt16LE(v,o);}
    writeU32(v){const[b,o]=this.at();b.writeUInt32LE(v,o);}
    readU64(){const[b,o]=this.at();const n=Number(b.readBigUInt64LE(o));return {compare:v=>Math.sign(n-Number(v)),toNumber:()=>n};}
    readPointer(){const[b,o]=this.at();return new P(b.readBigUInt64LE(o));}
    writePointer(v){const[b,o]=this.at();b.writeBigUInt64LE(BigInt(v.n),o);}
    writeByteArray(v){const[b,o]=this.at();Buffer.from(v).copy(b,o);}
    readByteArray(n){const[b,o]=this.at();return Uint8Array.from(b.subarray(o,o+n)).buffer;}
    readUtf16String(n){const[b,o]=this.at();return b.subarray(o,o+n*2).toString('utf16le');}
  }
  function alloc(n, address){const start=address??next;next+=n+16;blocks.push({start,data:Buffer.alloc(n)});return new P(start);}
  const host=alloc(0x33000,0x200000), framework=new P(0x300000), table=alloc(2048), globals=alloc(8);
  host.add(0x32dc0).writePointer(table);host.add(0x32dc8).writePointer(globals);
  const request=alloc(8), queue=alloc(8), device=alloc(8);
  const state={source:'selected',queue:true,status:0,ioctl:0xb000b,type:14,refs:0};
  let destination, length;
  const bind=(slot,f)=>{const address=framework.add(slot*16);table.add(slot*8).writePointer(address);functions.set(address.n,f);};
  bind(175,()=>state.queue?queue:new P(0));bind(90,()=>device.add(state.deviceShift??0));
  bind(31,(g,d,p,capacity,out,required)=>{
    assert.equal(g.n,globals.n);assert.equal(d.n,device.add(state.deviceShift??0).n);
    assert.equal(p,11);assert.equal(capacity,2048);assert.equal(required.readU32(),0);
    if(state.source==='error')return -1;
    const b=Buffer.from(state.source+'\0','utf16le');out.writeByteArray(b);
    if(state.unterminated)out.add(b.length-2).writeU16(65);
    required.writeU32(state.propertyBytes??b.length);
    return state.propertyStatus??0;
  });
  bind(165,(_g,_r,p)=>{p.add(4).writeU32(state.type);p.add(24).writeU32(state.ioctl);});
  bind(169,(_g,_r,_min,out,cap)=>{out.writePointer(destination);cap.writePointer(new P(length));return 0;});
  bind(171,()=>({toNumber:()=>length}));bind(126,()=>{state.cleaned=false;state.refs++;});
  bind(127,()=>{
    state.dereferences=(state.dereferences??0)+1;
    if(state.cleaned && !state.inReleaseHardware)throw Error('object used after framework cleanup');
    if(state.dereferenceFailure)throw Object.assign(new Error('access violation accessing private-address'),{lineNumber:77});
    state.refs--;
  });
  bind(163,()=>{});bind(164,()=>{});
  const env={Uint8Array,Number,Set,ptr:n=>new P(n),send:x=>messages.push(x),recv:(type,f)=>receivers.set(type,f),
    Memory:{alloc,allocUtf8String:s=>alloc(s.length+1)},
    Process:{pointerSize:8,getModuleByName:name=>name==='WUDFx02000.dll'?{base:framework,size:0x10000}:{base:host},findRangeByAddress:()=>({protection:'r-x'})},
    NativeFunction:function(address,result,args,options){
      if(address.n===framework.add(126*16).n||address.n===framework.add(127*16).n)assert.equal(options.scheduling,'exclusive');
      return functions.get(address.n);},
    Interceptor:{attach(address,callbacks){hooks.set(address.n,callbacks);return {detach:()=>hooks.delete(address.n)};}}};
  vm.runInNewContext(fs.readFileSync(path.join(__dirname,'runtime.js'),'utf8').replace('__SAYALL_CONFIG__',JSON.stringify({pdo:'selected',contract:1,maxLength:121})),env);
  const api={state,messages,receivers,hooks,releaseHardware(shift=0){
    const hook=hooks.get(host.add(0xce60).n);assert.ok(hook,'fixed ReleaseHardware hook');
    state.inReleaseHardware=true;hook.onEnter([device.add(shift),new P(0)]);state.inReleaseHardware=false;
    if(shift===(state.deviceShift??0))state.cleaned=true;
  },remove(){this.releaseHardware(state.deviceShift??0);this.command({type:'device_reset',present:false,pdo:''});},command:x=>{
    const handler=receivers.get(x.type);assert.ok(handler,'registered '+x.type+' receiver');
    receivers.delete(x.type);handler(x);
  },read(bytes,withInformation=false){
    length=bytes.length;destination=alloc(length);destination.writeByteArray(bytes);
    const hook=hooks.get(framework.add((withInformation?164:163)*16).n),context={threadId:1};
    hook.onEnter.call(context,[globals,request,new P(state.status),new P(length)]);hook.onLeave.call(context);
    return [...new Uint8Array(destination.readByteArray(length))];
  }};return api;
}
test('actual completion adapter requires queue and exact per-request identity before every write',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1});a.read(report());
  for(const source of ['another-device','error']){a.state.source=source;assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);}
  a.state.source='selected';a.state.queue=false;assert.deepEqual(a.read(report(0x35)),[...report(0x35)]);
  a.state.queue=true;a.state.ioctl=0x80018483;assert.deepEqual(a.read(report(0x4a)),[...report(0x4a)]);
  a.state.ioctl=0xb000b;assert.deepEqual(a.read(report(0xf1,0x80,0x35),true),[...report()]);
  assert.equal(a.messages.at(-1).buttons,11);assert.equal(a.messages.at(-1).configuration,1);
  assert.equal(a.state.refs,1);
  const cancels=a.messages.filter(x=>x.kind==='cancel').length;
  a.state.source='other-pooled-device';assert.deepEqual(a.read(report(0x4a)),[...report(0x4a)]);
  assert.equal(a.messages.filter(x=>x.kind==='cancel').length,cancels);
  assert.equal(a.state.refs,1);a.state.source='selected';
  assert.deepEqual(a.read(report(0xf1,0x80,0x35)),[...report()]);
});
test('normal exit holds suppression through release and detaches with paired object reference',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:0});a.read(report());a.read(report(0x4a));
  a.command({type:'stop'});assert.notEqual(a.messages.at(-1).kind,'stopped');
  assert.deepEqual(a.read(report(0x4a)),[...report()]);a.read(report());
  assert.equal(a.messages.at(-1).kind,'stopped');assert.equal(a.state.refs,0);
});
test('property failure reports exact safe status/type/length and never modifies an unverified report',()=>{
  for(const [details,reason] of [
    [{propertyStatus:0xc0000023|0,propertyBytes:4096},'source_property_buffer_small'],
    [{propertyStatus:0x80000005|0},'source_property_buffer_overflow'],
    [{propertyStatus:0xc000000d|0},'source_property_status'],
    [{propertyBytes:3},'source_property_length'],
    [{propertyBytes:4096},'source_property_length'],
    [{unterminated:true},'source_property_termination'],
    [{source:'selected\0hidden'},'source_property_embedded_null'],
    [{source:'other-instance'},'source_mismatch'],
  ]){
    const a=adapter();a.command({type:'configure',mask:31,configuration:1});
    Object.assign(a.state,details);
    assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);
    const event=a.messages.at(-1);assert.equal(event.reason,reason);
    assert.equal(event.propertyStatus,details.propertyStatus??0);
    assert.equal(event.propertyType,0);
    assert.ok(Number.isInteger(event.propertyBytes));
    assert.equal('source' in event,false);assert.equal(a.state.refs,0);
    const count=a.messages.length;a.read(report());assert.equal(a.messages.length,count);
    a.command({type:'configure',mask:31,configuration:2});a.read(report(0xf1));
    assert.equal(a.messages.at(-1).reason,reason);assert.equal(a.messages.at(-1).configuration,2);
    assert.equal(a.messages.at(-1).propertyStatus,details.propertyStatus??0);
  }
});
test('object replacement and public PnP epochs cannot reuse a previous PDO binding',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1});a.read(report());a.read(report(0xf1));
  a.state.deviceShift=16;assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);
  assert.equal(a.messages.at(-1).reason,'source_device_changed');assert.equal(a.state.refs,1);
  a.releaseHardware(0);a.command({type:'device_reset',present:false,pdo:''});assert.equal(a.state.refs,0);
  assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);
  a.command({type:'device_reset',present:true,pdo:'new-selected'});
  assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);assert.equal(a.messages.at(-1).reason,'source_mismatch');
  a.state.source='new-selected';assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);
  a.read(report());assert.deepEqual(a.read(report(0xf1)),[...report()]);assert.equal(a.state.refs,1);
});

test('configuration barrier reuses verified released state and captures every first key after profile changes',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});
  assert.equal(a.messages.at(-1).kind,'configured'); // Initial physical state is unknown.
  a.read(report());
  let configuration=1;
  for(const usage of [0xf1,0x80,0x81,0x35,0x4a]){
    for(const mask of [0,31,1,31]){
      a.command({type:'configure',mask,configuration:++configuration,preserveReleased:true});
      const event=a.messages.at(-1);
      assert.equal(event.kind,'state');assert.equal(event.basis,'configuration_reuse');
      assert.equal(event.configuration,configuration);assert.equal(event.allUp,true);
      assert.equal(event.buttons,0);assert.equal(event.ready,mask!==0);
    }
    assert.deepEqual(a.read(report(usage)),[...report()]);
    assert.notEqual(a.messages.at(-1).buttons,0);assert.equal(a.messages.at(-1).basis,'request');
    a.read(report());assert.equal(a.messages.at(-1).buttons,0);
  }
});

test('mask zero continuously tracks physical target and non-target holds without new ownership',()=>{
  for(const usage of [0xf1,0x4a,0x3e]){
    const a=adapter();a.command({type:'configure',mask:31,configuration:1});a.read(report());
    a.command({type:'configure',mask:0,configuration:2,preserveReleased:true});
    assert.deepEqual(a.read(report(usage)),[...report(usage)]);
    assert.equal(a.messages.at(-1).buttons,0);assert.equal(a.messages.at(-1).suppressed,0);
    a.command({type:'configure',mask:31,configuration:3,preserveReleased:true});
    assert.equal(a.messages.at(-1).kind,'configured');
    assert.deepEqual(a.read(report(usage)),[...report(usage)]);
    assert.equal(a.messages.at(-1).ready,false);
    a.read(report());assert.equal(a.messages.at(-1).ready,true);
    assert.deepEqual(a.read(report(0xf1)),[...report()]);
  }
});

test('configuration during an owned hold drains it without reusing release or creating a new action',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1});a.read(report());a.read(report(0x4a));
  for(const [configuration,mask] of [[2,0],[3,31]]){
    a.command({type:'configure',mask,configuration,preserveReleased:true});
    assert.equal(a.messages.at(-1).kind,'configured');
    assert.deepEqual(a.read(report(0x4a)),[...report()]);
    assert.equal(a.messages.at(-1).buttons,0);assert.equal(a.messages.at(-1).ready,false);
  }
  a.read(report());assert.equal(a.messages.at(-1).ready,true);
  assert.deepEqual(a.read(report(0x4a)),[...report()]);assert.equal(a.messages.at(-1).buttons,16);
});

test('source resets, changed objects, rejected contracts and uncleared raw input never reuse release',()=>{
  for(const cause of ['raw','object','disconnect','contract']){
    const a=adapter();a.command({type:'configure',mask:31,configuration:1});a.read(report());
    if(cause==='object'){a.state.deviceShift=16;a.read(report());}
    if(cause==='disconnect'){
      a.remove();
      a.command({type:'device_reset',present:true,pdo:'selected'});
    }
    if(cause==='contract')a.read(report(1));
    a.command({type:'configure',mask:31,configuration:2,preserveReleased:cause!=='raw'});
    assert.equal(a.messages.at(-1).kind,'configured');
  }
  const a=adapter();a.command({type:'configure',mask:31,configuration:1});a.read(report());
  a.state.source='other-device';assert.deepEqual(a.read(report(0x4a)),[...report(0x4a)]);
  a.command({type:'configure',mask:31,configuration:2,preserveReleased:true});
  assert.equal(a.messages.at(-1).basis,'configuration_reuse');
  a.state.source='selected';assert.deepEqual(a.read(report(0x4a)),[...report()]);
});

test('released reconnect freshly proves its new PDO and owns the first DOWN without fabricated all-up',()=>{
  for(const usage of [0xf1,0x80,0x81,0x35,0x4a]){
    const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());
    a.remove();assert.equal(a.state.refs,0);
    a.state.deviceShift=16;a.state.source='new-selected';
    a.command({type:'device_reset',present:true,pdo:'new-selected'});
    for(const [configuration,mask] of [[2,0],[3,31]])a.command({type:'configure',mask,configuration,preserveReleased:true});
    assert.deepEqual(a.read(report(usage)),[...report()]);
    const down=a.messages.at(-1);
    assert.equal(down.reconnectedFirst,true);assert.equal(down.allUp,false);
    assert.equal(down.ready,true);assert.equal(down.buttons,down.suppressed);assert.notEqual(down.buttons,0);
    assert.equal(a.state.refs,1);a.read(report());assert.equal(a.messages.at(-1).buttons,0);
  }
});

test('a held key, unknown baseline, uncleared raw source or missed identity interval cannot resume a DOWN',()=>{
  for(const cause of ['held','initial','raw','gap','pre-notify-gap','missing-queue','contract']){
    const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});
    if(cause!=='initial')a.read(report());
    if(cause==='held')a.read(report(0xf1));
    if(cause==='contract')a.read(report(1));
    if(cause==='pre-notify-gap'){a.state.deviceShift=16;a.state.source='error';a.read(report(0xf1));a.state.deviceShift=0;}
    a.remove();
    a.state.deviceShift=16;
    if(cause==='gap')assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);
    if(cause==='missing-queue'){a.state.queue=false;a.read(report(0xf1));a.state.queue=true;}
    a.state.source='new-selected';a.command({type:'device_reset',present:true,pdo:'new-selected'});
    a.command({type:'configure',mask:31,configuration:2,preserveReleased:cause!=='raw'});
    assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)],cause);
    assert.equal(a.messages.at(-1).ready,false,cause);assert.equal(a.messages.at(-1).buttons,0,cause);
    a.read(report());assert.deepEqual(a.read(report(0xf1)),[...report()]);
  }
});

test('pooled other devices neither inherit fresh source nor cancel its reconnect eligibility or hold',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());
  a.remove();
  a.state.deviceShift=32;assert.deepEqual(a.read(report(0x4a)),[...report(0x4a)]);
  a.command({type:'device_reset',present:true,pdo:'new-selected'});
  a.state.source='other';assert.deepEqual(a.read(report(0x4a)),[...report(0x4a)]);
  a.state.deviceShift=16;a.state.source='new-selected';assert.deepEqual(a.read(report(0xf1)),[...report()]);
  assert.equal(a.messages.at(-1).reconnectedFirst,true);
  const cancels=a.messages.filter(x=>x.kind==='cancel').length;
  a.state.deviceShift=32;a.state.source='other';a.read(report());
  assert.equal(a.messages.filter(x=>x.kind==='cancel').length,cancels);
  a.state.deviceShift=16;a.state.source='new-selected';assert.deepEqual(a.read(report(0xf1)),[...report()]);
  a.read(report());assert.equal(a.messages.at(-1).buttons,0);
});

test('mask-zero report after reconnect prevents configuration from taking over an already native hold',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());
  a.remove();a.state.deviceShift=16;
  a.command({type:'configure',mask:0,configuration:2,preserveReleased:true});
  a.command({type:'device_reset',present:true,pdo:'selected'});
  assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);
  a.command({type:'configure',mask:31,configuration:3,preserveReleased:true});
  assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);assert.equal(a.messages.at(-1).ready,false);
  a.read(report());assert.deepEqual(a.read(report(0xf1)),[...report()]);
});

test('bounded unknown-object capacity fails closed and a real selected release restores continuity',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());
  a.remove();
  for(let i=1;i<=65;i++){a.state.deviceShift=i*16;a.read(report(0xf1));}
  a.state.deviceShift=2048;a.command({type:'device_reset',present:true,pdo:'selected'});
  assert.deepEqual(a.read(report(0xf1)),[...report(0xf1)]);a.read(report());
  a.remove();
  a.state.deviceShift=4096;a.command({type:'device_reset',present:true,pdo:'selected'});
  assert.deepEqual(a.read(report(0xf1)),[...report()]);
});

test('hardware release reference exception records only safe context and completes normal teardown without double dereference',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());
  a.state.dereferenceFailure=true;
  a.releaseHardware();
  const failure=a.messages.find(x=>x.kind==='rejected'&&x.reason==='control_failure');
  assert.equal(failure.operation,'hardware_reference_release');assert.equal(failure.errorClass,'access_violation');
  assert.equal(failure.codeLine,77);assert.equal(JSON.stringify(failure).includes('private-address'),false);
  assert.equal(a.messages.at(-1).kind,'stopped');assert.equal(a.messages.at(-1).cleanupErrors,1);
  assert.equal(a.state.dereferences,1);assert.equal(a.hooks.size,0);
});

test('independent stop survives a failed configure while owned input drains its paired release',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());a.read(report(0x4a));
  a.command({type:'configure',mask:32,configuration:2,preserveReleased:true});
  assert.equal(a.messages.at(-1).reason,'control_failure');
  assert.ok(a.receivers.has('configure'));assert.ok(a.receivers.has('device_reset'));assert.ok(a.receivers.has('stop'));
  a.command({type:'stop'});assert.notEqual(a.messages.at(-1).kind,'stopped');
  assert.deepEqual(a.read(report(0x4a)),[...report()]);assert.equal(a.messages.at(-1).buttons,0);
  a.read(report());assert.equal(a.messages.at(-1).kind,'stopped');assert.equal(a.messages.at(-1).cleanupErrors,0);
  assert.equal(a.state.refs,0);assert.equal(a.state.dereferences,1);assert.equal(a.hooks.size,0);
});

test('normal stop reference failure still removes interceptors and reports unsuccessful reference cleanup',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1});a.read(report());
  a.state.dereferenceFailure=true;a.command({type:'stop'});
  assert.equal(a.messages.at(-1).kind,'stopped');assert.equal(a.messages.at(-1).cleanupErrors,1);
  assert.equal(a.messages.find(x=>x.reason==='cleanup_failure').operation,'reference_release');
  assert.equal(a.state.dereferences,1);assert.equal(a.hooks.size,0);
});


test('public removal before or after framework release never dereferences a cleaned object',()=>{
  for(const order of ['hardware-first','notify-first','new-present-first']){
    const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());
    if(order!=='hardware-first'){
      a.command({type:'device_reset',present:false,pdo:''});
      assert.equal(a.state.refs,1);assert.equal(a.state.dereferences,undefined);
    }
    if(order==='new-present-first'){
      a.command({type:'device_reset',present:true,pdo:'new-selected'});
      a.command({type:'configure',mask:31,configuration:2,preserveReleased:true});
    }
    a.releaseHardware();assert.equal(a.state.refs,0);assert.equal(a.state.dereferences,1);
    if(order==='hardware-first')a.command({type:'device_reset',present:false,pdo:''});
    if(order!=='new-present-first')a.command({type:'device_reset',present:true,pdo:'new-selected'});
    // Reuse exactly the same handle address after deletion: fresh PDO/epoch and
    // the paired old release, not pointer equality, authorize the new instance.
    a.state.source='new-selected';assert.deepEqual(a.read(report(0xf1)),[...report()],order);
    assert.equal(a.messages.at(-1).reconnectedFirst,true,order);assert.equal(a.state.refs,1);
    a.read(report());a.command({type:'stop'});assert.equal(a.state.refs,0);
    assert.equal(a.state.dereferences,2);assert.equal(a.messages.at(-1).cleanupErrors,0);
  }
});

test('held removal cancels ownership before cleanup and never resurrects the old hold',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());a.read(report(0xf1,0x80));
  const count=a.messages.length;a.releaseHardware(16);
  assert.equal(a.messages.length,count);assert.equal(a.state.refs,1); // other device
  a.releaseHardware();assert.equal(a.state.refs,0);
  assert.ok(a.messages.slice(count).some(x=>x.kind==='cancel'));
  a.command({type:'device_reset',present:false,pdo:''});a.command({type:'device_reset',present:true,pdo:'selected'});
  assert.deepEqual(a.read(report(0xf1,0x80)),[...report(0xf1,0x80)]);
  assert.equal(a.messages.at(-1).ready,false);a.read(report());
  assert.deepEqual(a.read(report(0xf1)),[...report()]);
});

test('stop during retiring reference waits for framework release and then unloads exactly once',()=>{
  const a=adapter();a.command({type:'configure',mask:31,configuration:1,preserveReleased:true});a.read(report());a.read(report(0x4a));
  a.command({type:'device_reset',present:false,pdo:''});a.command({type:'stop'});
  assert.notEqual(a.messages.at(-1).kind,'stopped');assert.equal(a.state.refs,1);assert.equal(a.hooks.size,3);
  a.releaseHardware();assert.equal(a.state.refs,0);assert.equal(a.state.dereferences,1);
  assert.equal(a.messages.filter(x=>x.kind==='stopped').length,1);assert.equal(a.messages.at(-1).cleanupErrors,0);
  assert.equal(a.hooks.size,0);
});
