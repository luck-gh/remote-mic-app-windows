// GPL-3.0-only. Bounded future-open metadata diagnostic.
// Public PnP interface names are injected by the identity-checked native helper.
// No IOCTL input/output buffers, reports, keyboard events or sockets are read.
const interfaces = __SAYALL_INTERFACES_JSON__;
const names = new Set(interfaces.map(name => name.toLowerCase()));
const ntNames = new Set(interfaces.map(name => ('\\??\\' + name.slice(4)).toLowerCase()));
const ntdll = Process.getModuleByName('ntdll.dll');
const kernel = Process.getModuleByName('KernelBase.dll');
const queryObject = new NativeFunction(ntdll.getExportByName('NtQueryObject'),
  'int', ['pointer', 'uint', 'pointer', 'uint', 'pointer']);
const bindings = new Map(), stacks = new Map(), listeners = [];
let serial = 0, mutation = 0, stopped = false;
const stats = { openCalls:0, openExact:0, openUnknown:0, openFailed:0,
  openPending:0, openRaced:0, opened:0, nested:0, capacity:0,
  closeCalls:0, revoked:0, clearedClose:0, clearedDuplicate:0, duplicateCalls:0, queryFailed:0, nameRejected:0,
  ioctlKnown:0, ioctlUnknown:0, synchronous:0, pending:0, failed:0 };
function invalidHandle(handle) { return handle.isNull() || handle.toString() === '0xffffffffffffffff'; }
function cleanName(name) {
  return typeof name === 'string' && name.length > 0 && name.length <= 1024 &&
    !name.includes('\0') && !name.split('\\').some(part => part === '.' || part === '..');
}
function winName(pointer) {
  if (pointer.isNull()) throw new Error('null');
  let name = '';
  for (let i = 0; i <= 1024; i++) {
    const value = pointer.add(i * 2).readU16();
    if (value === 0) return name;
    name += String.fromCharCode(value);
  }
  throw new Error('unterminated');
}
function unicode(pointer) {
  if (pointer.isNull()) throw new Error('null');
  const length = pointer.readU16(), maximum = pointer.add(2).readU16();
  const buffer = pointer.add(8).readPointer();
  if (!length || (length & 1) || length > 2048 || length > maximum || buffer.isNull()) throw new Error('shape');
  const value = buffer.readUtf16String(length / 2);
  if (!cleanName(value)) throw new Error('name');
  return value;
}
function rootName(handle) {
  const memory = Memory.alloc(4096), size = Memory.alloc(4);
  if (queryObject(handle, 1, memory, 4096, size) !== 0) { stats.queryFailed++; throw new Error('query'); }
  const returned = size.readU32(), length = memory.readU16(), buffer = memory.add(8).readPointer();
  if (returned < 16 || returned > 4096 || buffer.isNull() ||
      buffer.compare(memory.add(16)) < 0 || buffer.add(length).compare(memory.add(returned)) > 0) throw new Error('bounds');
  return unicode(memory);
}
function ntName(attributes) {
  // x64 OBJECT_ATTRIBUTES: Length0, RootDirectory8, ObjectName16, Attributes24.
  if (attributes.isNull() || attributes.readU32() !== 48) throw new Error('attributes');
  const root = attributes.add(8).readPointer();
  const name = unicode(attributes.add(16).readPointer());
  if (root.isNull()) return name.startsWith('\\') ? name : '';
  if (name.startsWith('\\')) return ''; // Do not guess mixed absolute/root semantics.
  if (name.split('\\').some(part => !part)) return '';
  const directory = rootName(root);
  if (!directory.startsWith('\\')) return '';
  return directory + '\\' + name;
}
function beginOpen(context, name) {
  stats.openCalls++;
  const thread = context.threadId;
  let stack = stacks.get(thread);
  if (!stack) { stack = []; stacks.set(thread, stack); }
  const scope = stack.length ? stack[0].scope : ++serial;
  const call = { scope, mutation, exact:name, thread };
  stack.push(call); context.open = call;
  if (name) stats.openExact++; else stats.openUnknown++;
}
function finishOpen(context, success, handle) {
  const call = context.open, stack = stacks.get(call.thread);
  if (stack) { stack.pop(); if (!stack.length) stacks.delete(call.thread); }
  if (stopped || !success || !handle || invalidHandle(handle)) { stats.openFailed++; return; }
  // Any intervening close/duplication makes an in-flight result ambiguous.
  // Conservative global epoch avoids retaining unbounded handle tombstones.
  if (call.mutation !== mutation) { stats.openRaced++; return; }
  const key = handle.toString(), old = bindings.get(key);
  if (old && old.scope === call.scope) { stats.nested++; return; }
  bindings.delete(key); // A successful unknown open cannot inherit a reused handle.
  if (!call.exact) return;
  if (bindings.size >= 64) { stats.capacity++; return; }
  bindings.set(key, { scope:call.scope, generation:++serial }); stats.opened++;
}
listeners.push(Interceptor.attach(kernel.getExportByName('CreateFileW'), {
  onEnter(args) {
    let exact = false;
    try { const name = winName(args[0]); exact = cleanName(name) && names.has(name.toLowerCase()); }
    catch (_) { stats.nameRejected++; }
    beginOpen(this, exact);
  },
  onLeave(result) { finishOpen(this, !invalidHandle(result), result); }
}));
for (const api of ['NtCreateFile', 'NtOpenFile']) listeners.push(Interceptor.attach(ntdll.getExportByName(api), {
  onEnter(args) {
    let exact = false;
    try { exact = ntNames.has(ntName(args[2]).toLowerCase()); } catch (_) { stats.nameRejected++; }
    beginOpen(this, exact); this.outputHandle = args[0];
  },
  onLeave(result) {
    const status = result.toUInt32();
    if (status === 0x103) stats.openPending++;
    // Only synchronous STATUS_SUCCESS permits reading the returned HANDLE slot.
    let handle = null;
    if (status === 0) { try { handle = this.outputHandle.readPointer(); } catch (_) { stats.nameRejected++; } }
    finishOpen(this, status === 0, handle);
  }
}));
listeners.push(Interceptor.attach(ntdll.getExportByName('NtClose'), {
  onEnter(args) {
    mutation++; stats.closeCalls++;
    if (bindings.delete(args[0].toString())) { stats.revoked++; stats.clearedClose++; }
    // Even a failed close revokes conservatively; no stale reactivation.
  }
}));
listeners.push(Interceptor.attach(ntdll.getExportByName('NtDuplicateObject'), {
  onEnter() {
    // Do not inherit any duplicate or infer target process identity. Invalidating
    // all tracked handles is intentionally conservative for this diagnostic.
    mutation++; stats.duplicateCalls++; stats.revoked += bindings.size; stats.clearedDuplicate += bindings.size; bindings.clear();
  }
}));
listeners.push(Interceptor.attach(ntdll.getExportByName('NtDeviceIoControlFile'), {
  onEnter(args) {
    this.known = false;
    if (args[5].toUInt32() !== 0x80018483) return;
    this.known = bindings.has(args[0].toString());
    if (this.known) stats.ioctlKnown++; else stats.ioctlUnknown++;
    // Deliberately do not inspect/store args[4], [6..9] or read their buffers.
  },
  onLeave(result) {
    if (!this.known) return;
    const status = result.toUInt32();
    if (status === 0x103) stats.pending++;
    else if (status === 0) stats.synchronous++;
    else stats.failed++;
  }
}));
function counters(kind) { send({kind, metadataOnly:true, interfaces:interfaces.length, tracked:bindings.size, ...stats}); }
const timer = setInterval(() => counters('counters'), 5000);
recv('stop', () => {
  stopped = true;
  for (const listener of listeners) listener.detach();
  Interceptor.flush(); clearInterval(timer); bindings.clear(); stacks.clear(); counters('stopped');
});
// Discard anything observed while the hook set was only partially installed.
mutation++; bindings.clear(); stacks.clear();
send({kind:'ready', metadataOnly:true, interfaces:interfaces.length});
