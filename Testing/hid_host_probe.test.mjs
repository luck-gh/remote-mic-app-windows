import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
const win = '\\\\?\\Synthetic#Selected#{1234}';
const nt = '\\??\\Synthetic#Selected#{1234}';
function fixture() {
  let address = 0, timer, stop, detached = 0, flushed = false, payloadReads = 0;
  const hooks = new Map(), messages = [];
  class Pointer {
    constructor(buffer = Buffer.alloc(64), offset = 0, origin, refs = new Map()) {
      this.buffer = buffer; this.offset = offset; this.origin = origin ?? (address += 8192); this.refs = refs;
    }
    add(n) { return new Pointer(this.buffer, this.offset+n, this.origin, this.refs); }
    isNull() { return false; }
    toString() { return `0x${this.origin.toString(16)}`; }
    compare(other) { return this.origin+this.offset-other.origin-other.offset; }
    readU16() { return this.buffer.readUInt16LE(this.offset); }
    readU32() { return this.buffer.readUInt32LE(this.offset); }
    readPointer() { if (!this.refs.has(this.offset)) throw new Error('unreadable pointer'); return this.refs.get(this.offset); }
    readUtf16String(length) { return this.buffer.toString('utf16le',this.offset,this.offset+length*2); }
    readByteArray() { payloadReads++; throw new Error('payload forbidden'); }
  }
  const handle = number => ({isNull:()=>number===0,toString:()=>number===-1?'0xffffffffffffffff':`0x${number.toString(16)}`});
  const nullPtr=handle(0), result=number=>({toUInt32:()=>number>>>0});
  function wide(value) { return new Pointer(Buffer.from(value+'\0','utf16le')); }
  function unicode(value) { const p=new Pointer(); const data=wide(value);p.buffer.writeUInt16LE(value.length*2);p.buffer.writeUInt16LE(value.length*2,2);p.refs.set(8,data);return p; }
  function attributes(value, root=nullPtr) { const p=new Pointer();p.buffer.writeUInt32LE(48);p.refs.set(8,root);p.refs.set(16,unicode(value));return p; }
  const context={
    Process:{getModuleByName:()=>({getExportByName:name=>name})},
    NativeFunction:function(){return (root,_kind,memory,_capacity,size)=>{
      if (!root.name) return -1;
      const data=Buffer.from(root.name,'utf16le');memory.buffer.writeUInt16LE(data.length);memory.buffer.writeUInt16LE(data.length,2);
      data.copy(memory.buffer,16);memory.refs.set(8,memory.add(16));size.buffer.writeUInt32LE(16+data.length);return 0;
    };},
    Memory:{alloc:size=>new Pointer(Buffer.alloc(size))},
    Interceptor:{attach:(name,callbacks)=>{hooks.set(name,callbacks);return{detach:()=>detached++};},flush:()=>{flushed=true;}},
    send:message=>messages.push(JSON.parse(JSON.stringify(message))),
    setInterval:callback=>{timer=callback;return 1;},clearInterval:()=>{},recv:(_type,callback)=>{stop=callback;}
  };
  const source=readFileSync(new URL('./hid_host_probe.js',import.meta.url),'utf8').replace('__SAYALL_INTERFACES_JSON__',JSON.stringify([win]));
  vm.runInNewContext(source,context);
  function enter(api,args) {const call={threadId:1};hooks.get(api).onEnter.call(call,args);return call;}
  function leave(api,call,value) {hooks.get(api).onLeave?.call(call,value);}
  function open(api='CreateFileW', name=win, h=handle(10), status=0, root=nullPtr) {
    const output=new Pointer();output.refs.set(0,h);
    const args=api==='CreateFileW'?[wide(name)]:[output,null,attributes(name,root)];
    const call=enter(api,args);
    return {finish:()=>leave(api,call,api==='CreateFileW'?h:result(status)),output};
  }
  function ioctl(h=handle(10),status=0) {
    const args = new Proxy([h],{get:(target,key)=>{if(key==='0')return h;if(key==='5')return result(0x80018483);payloadReads++;throw new Error('IOCTL buffer inspected');}});
    const call=enter('NtDeviceIoControlFile',args);leave('NtDeviceIoControlFile',call,result(status));
  }
  return {handle,result,open,ioctl,enter,leave,messages,
    close:(h=handle(10))=>enter('NtClose',[h]),dup:()=>enter('NtDuplicateObject',[]),
    counters:()=>{timer();return messages.at(-1);},
    stop:()=>{stop();return{detached,flushed,last:messages.at(-1)};},reads:()=>payloadReads};
}
test('pre-existing, unrelated, prefix and object-target handles remain unknown; no payload reads',()=>{
  const f=fixture();f.ioctl();
  for(const name of [win+'\\Child','\\Device\\Synthetic',win.replace('Selected','Other')]){f.open('CreateFileW',name).finish();f.ioctl();}
  const c=f.counters();assert.equal(c.ioctlUnknown,4);assert.equal(c.opened,0);assert.equal(f.reads(),0);
});
test('future exact Win32 and literal NT aliases bind only successful synchronous opens',()=>{
  const f=fixture();f.open().finish();f.ioctl();f.close();
  f.open('NtCreateFile',nt).finish();f.ioctl(undefined,0x103);f.close();
  f.open('NtOpenFile',nt).finish();f.ioctl(undefined,0xc0000001);
  const c=f.counters();assert.equal(c.opened,3);assert.equal(c.ioctlKnown,3);assert.equal(c.synchronous,1);assert.equal(c.pending,1);assert.equal(c.failed,1);assert.equal(f.reads(),0);
});
test('failed and pending opens never produce bindings or read pending output HANDLE slots',()=>{
  const f=fixture();f.open('CreateFileW',win,f.handle(-1)).finish();f.open('CreateFileW',win,f.handle(0)).finish();
  f.open('NtOpenFile',nt,undefined,0xc0000001).finish();
  const pending=f.open('NtCreateFile',nt,undefined,0x103);pending.output.refs.clear();pending.finish();f.ioctl();
  const c=f.counters();assert.equal(c.opened,0);assert.equal(c.openPending,1);assert.equal(c.nameRejected,0);assert.equal(c.ioctlUnknown,1);
});
test('nested Win32 to NT open is one handle generation',()=>{
  const f=fixture();const outer=f.open();f.open('NtCreateFile',nt).finish();outer.finish();f.ioctl();
  const c=f.counters();assert.equal(c.opened,1);assert.equal(c.nested,1);assert.equal(c.tracked,1);
});
test('close revokes even if close fails; an unknown reuse never inherits binding',()=>{
  const f=fixture();f.open().finish();f.close();f.ioctl();f.open('CreateFileW',win+'other').finish();f.ioctl();
  f.open().finish();f.ioctl();const c=f.counters();assert.equal(c.ioctlUnknown,2);assert.equal(c.ioctlKnown,1);assert.equal(c.clearedClose,1);assert.equal(c.opened,2);
});
test('any duplicate attempt clears all bindings even on failure; later exact open can recover',()=>{
  const f=fixture();f.open().finish();f.open('CreateFileW',win,f.handle(11)).finish();f.dup();f.ioctl();f.ioctl(f.handle(11));
  f.open().finish();f.ioctl();const c=f.counters();assert.equal(c.clearedDuplicate,2);assert.equal(c.ioctlUnknown,2);assert.equal(c.ioctlKnown,1);
});
test('close or duplicate during open rejects late return instead of rebinding a reused handle',()=>{
  for(const operation of ['close','dup']){const f=fixture();const late=f.open();f[operation]();late.finish();f.ioctl();const c=f.counters();assert.equal(c.openRaced,1);assert.equal(c.opened,0);assert.equal(c.ioctlUnknown,1);}
});
test('close between nested return and outer return cannot resurrect binding',()=>{
  const f=fixture();const outer=f.open();f.open('NtCreateFile',nt).finish();f.close();outer.finish();f.ioctl();
  const c=f.counters();assert.equal(c.tracked,0);assert.equal(c.openRaced,1);assert.equal(c.ioctlUnknown,1);
});
test('RootDirectory metadata must mechanically resolve to the exact absolute NT alias',()=>{
  const f=fixture();const root={...f.handle(20),name:'\\??'};
  f.open('NtOpenFile',nt.slice(4),undefined,0,root).finish();f.ioctl();f.close();
  for(const name of ['..\\'+nt.slice(4),'.\\'+nt.slice(4),nt.slice(4)+'\\Child',nt.slice(4)+'\0x']){f.open('NtOpenFile',name,undefined,0,root).finish();f.ioctl();}
  const c=f.counters();assert.equal(c.opened,1);assert.equal(c.ioctlKnown,1);assert.equal(c.ioctlUnknown,4);
});
test('unresolved or relative roots and malformed names fail closed anonymously',()=>{
  const f=fixture();for(const root of [{...f.handle(1)},{...f.handle(2),name:'relative'}]){f.open('NtOpenFile',nt.slice(4),undefined,0,root).finish();f.ioctl();}
  f.open('CreateFileW','x'.repeat(1025)).finish();f.ioctl();
  const c=f.counters();assert.equal(c.ioctlUnknown,3);assert.equal(c.opened,0);assert.ok(c.queryFailed>0);assert.equal(JSON.stringify(c).includes('Synthetic'),false);
});
test('binding capacity is bounded and stop detaches all hooks and clears all metadata',()=>{
  const f=fixture();for(let i=1;i<=65;i++)f.open('CreateFileW',win,f.handle(i)).finish();const c=f.counters();assert.equal(c.tracked,64);assert.equal(c.capacity,1);
  const done=f.stop();assert.equal(done.detached,6);assert.equal(done.flushed,true);assert.equal(done.last.kind,'stopped');assert.equal(done.last.tracked,0);assert.equal(f.reads(),0);
});
