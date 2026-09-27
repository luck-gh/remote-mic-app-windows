// GPL-3.0-only. Authorized source-B group: fresh property per request, no report reads/writes.
const host = Process.getModuleByName('Microsoft.Bluetooth.Profiles.HidOverGatt.dll');
const framework = Process.getModuleByName('WUDFx02000.dll');
if (Process.pointerSize !== 8) throw Error('contract');
const table = host.base.add(0x32dc0).readPointer();
const globals = host.base.add(0x32dc8).readPointer();
if (table.isNull() || globals.isNull()) throw Error('binding');
function address(slot) {
  const p = table.add(slot * 8).readPointer();
  if (p.compare(framework.base) < 0 || p.compare(framework.base.add(framework.size)) >= 0 ||
      !Process.findRangeByAddress(p)?.protection.includes('x')) throw Error('binding');
  return p;
}
const queueOf = new NativeFunction(address(175), 'pointer', ['pointer', 'pointer']);
const deviceOf = new NativeFunction(address(90), 'pointer', ['pointer', 'pointer']);
const parameters = new NativeFunction(address(165), 'void', ['pointer', 'pointer', 'pointer']);
const property = new NativeFunction(address(31), 'int', ['pointer', 'pointer', 'uint', 'uint', 'pointer', 'pointer']);
let finished = false, generation = 1, sequence = 0;
const listeners = [];
function finish(reason) {
  if (finished) return;
  finished = true; listeners.forEach(x => x.detach()); send({ kind: 'stopped', reason, sequence, generation });
}
function publish(value) {
  send({ kind: 'result', generation, sequence: ++sequence, ...value });
}
function complete(args) {
  if (finished || !args[0].equals(globals) || args[2].toInt32() < 0) return;
  try {
    const params = Memory.alloc(40); params.writeByteArray(new Uint8Array(40)); params.writeU16(40);
    parameters(globals, args[1], params);
    const type = params.add(4).readU32();
    if ((type !== 14 && type !== 15) || params.add(24).readU32() !== 0xb000b) return;
    const queue = queueOf(globals, args[1]);
    if (queue.isNull()) { publish({ reason: 'queue_missing' }); return; }
    const device = deviceOf(globals, queue);
    if (device.isNull()) { publish({ reason: 'device_missing' }); return; }
    // Both handles are consumed synchronously while the original completion owns the request.
    // No identity is cached; no device handle survives this callback.
    const value = Memory.alloc(2048), required = Memory.alloc(4); required.writeU32(0);
    const status = property(globals, device, 11, 2048, value, required);
    const bytes = required.readU32();
    if (status !== 0) { publish({ reason: 'property_status', status, bytes }); return; }
    if (bytes < 4 || bytes > 2048 || (bytes & 1)) { publish({ reason: 'property_length', status, bytes }); return; }
    const chars = bytes / 2;
    if (value.add((chars - 1) * 2).readU16() !== 0) { publish({ reason: 'property_termination', status, bytes }); return; }
    const pdo = value.readUtf16String(chars - 1);
    if (pdo.includes('\0')) { publish({ reason: 'property_embedded_null', status, bytes }); return; }
    // Private in-memory message only: native receiver compares public PnP and never logs this name.
    publish({ reason: 'property_value', status, bytes, pdo });
  } catch (_) { publish({ reason: 'contract_exception' }); }
}
const completing = new Set();
function hooks() { return {
  onEnter(args) { this.owner = !completing.has(this.threadId); if (this.owner) { completing.add(this.threadId); complete(args); } },
  onLeave() { if (this.owner) completing.delete(this.threadId); }
}; }
listeners.push(Interceptor.attach(address(163), hooks()));
listeners.push(Interceptor.attach(address(164), hooks()));
function control(message) {
  if (message.type === 'stop') finish('normal_stop');
  else if (message.type === 'device_reset' && Number.isSafeInteger(message.generation)) generation = message.generation;
  if (!finished) recv(control);
}
recv(control);
send({ kind: 'bound' });
