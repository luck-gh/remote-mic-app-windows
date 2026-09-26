// GPL-3.0-only. Fixed RC003 WDF input completion adapter; no arbitrary hooks.
// Config is compiled by the native Helper after its device/descriptor gates.
function decodeInput(bytes) {
  if (bytes.length < 7 || bytes.length > 121 || bytes[0] !== 1) throw Error('report_shape');
  for (let i = 7; i < bytes.length; ++i) if (bytes[i] !== 0) throw Error('report_tail');
  let physical = 0, allUp = true;
  const usages = [];
  for (let i = 1; i < 7; i += 2) {
    const usage = bytes[i] | (bytes[i + 1] << 8);
    if (usage > 254 || (usage >= 1 && usage <= 3)) throw Error('report_usage');
    allUp = allUp && usage === 0; usages.push(usage);
    const bit = [0xf1, 0x80, 0x81, 0x35, 0x4a].indexOf(usage);
    if (bit >= 0) physical |= 1 << bit;
  }
  return { physical, allUp, usages };
}
class Ownership {
  constructor() { this.mask = 0; this.owned = 0; this.ready = false; this.physical = 0; this.allUp = false; }
  configure(mask, preserveReleased = false) {
    if (!Number.isInteger(mask) || mask < 0 || mask > 31) throw Error('mask');
    this.mask = mask;
    this.ready = preserveReleased && this.ready && this.allUp && this.owned === 0;
    // Already suppressed holds remain suppressed until their physical release.
  }
  accept(bytes, reconnectReleased = false) {
    const report = decodeInput(bytes);
    // Only the adapter may supply this after fresh source proof and continuous
    // completion coverage. It is not a synthetic all-up report.
    if (reconnectReleased) this.ready = true;
    this.physical = report.physical;
    this.allUp = report.allUp;
    this.owned &= report.physical;
    if (!this.ready && report.allUp) this.ready = true;
    if (this.ready) this.owned |= report.physical & this.mask;
    const output = Uint8Array.from(bytes);
    report.usages.forEach((usage, i) => {
      const bit = [0xf1, 0x80, 0x81, 0x35, 0x4a].indexOf(usage);
      if (bit >= 0 && (this.owned & (1 << bit))) {
        output[1 + i * 2] = 0; output[2 + i * 2] = 0;
      }
    });
    return { output, buttons: this.ready ? report.physical & this.mask : 0,
      ready: this.ready && this.mask !== 0, physical: report.physical,
      suppressed: this.owned, allUp: report.allUp };
  }
}
if (typeof module !== 'undefined') module.exports = { decodeInput, Ownership };
else {
  const config = __SAYALL_CONFIG__;
  const host = Process.getModuleByName('Microsoft.Bluetooth.Profiles.HidOverGatt.dll');
  const framework = Process.getModuleByName('WUDFx02000.dll');
  if (Process.pointerSize !== 8 || config.contract !== 1 || config.maxLength !== 121 ||
      typeof config.pdo !== 'string' || !config.pdo.length) throw Error('contract');
  const table = host.base.add(0x32dc0).readPointer();
  const globals = host.base.add(0x32dc8).readPointer();
  if (table.isNull() || globals.isNull()) throw Error('wdf_binding');
  function address(slot) {
    const p = table.add(slot * 8).readPointer();
    if (p.compare(framework.base) < 0 || p.compare(framework.base.add(framework.size)) >= 0 ||
        !Process.findRangeByAddress(p)?.protection.includes('x')) throw Error('wdf_function');
    return p;
  }
  function fn(slot, result, args) { return new NativeFunction(address(slot), result, args); }
  const getQueue = fn(175, 'pointer', ['pointer', 'pointer']);
  const getDevice = fn(90, 'pointer', ['pointer', 'pointer']);
  const property = fn(31, 'int', ['pointer', 'pointer', 'uint', 'uint', 'pointer', 'pointer']);
  const parameters = fn(165, 'void', ['pointer', 'pointer', 'pointer']);
  const buffer = fn(169, 'int', ['pointer', 'pointer', 'uint64', 'pointer', 'pointer']);
  const information = fn(171, 'uint64', ['pointer', 'pointer']);
  // Keep JS ownership transitions atomic with these short reference operations.
  const reference = new NativeFunction(address(126), 'void', ['pointer', 'pointer', 'pointer', 'int', 'pointer'], { scheduling: 'exclusive' });
  const dereference = new NativeFunction(address(127), 'void', ['pointer', 'pointer', 'pointer', 'int', 'pointer'], { scheduling: 'exclusive' });
  const referenceFile = Memory.allocUtf8String('sayall_hid_host');
  let sourceReference = null, present = true, referenceReleaseFailed = false;
  function releaseSource() {
    if (sourceReference !== null) {
      // A native exception can occur after an unknown amount of work. Never
      // issue a second dereference against the same reference in that case.
      const held = sourceReference;
      sourceReference = null;
      try { dereference(globals, held, ptr(0), 0, referenceFile); }
      catch (error) { referenceReleaseFailed = true; throw error; }
    }
  }
  const state = new Ownership(), listeners = [];
  let generation = 1, sequence = 0, source = null, stopping = false, stopped = false, desiredMask = 0;
  let lastFailure = '', lastState = '', configuration = 0, lastLength = 0;
  let expectedPdo = config.pdo.toLowerCase(), deviceChanged = false;
  let pendingPdo = '';
  let disconnected = false, reconnectReleased = false, rawReleased = false, coverageKnown = true;
  // Negative evidence only: a request from this object was passed natively
  // while its identity was unavailable. Pointer reuse can only deny takeover.
  const passedObjects = new Set();
  function passedUnknown(identity) {
    if (identity === null || passedObjects.size >= 64) {
      coverageKnown = false; reconnectReleased = false;
    } else passedObjects.add(identity);
  }
  function publish(kind, fields) { send(Object.assign({ kind, generation, sequence: ++sequence, configuration }, fields)); }
  function cancel(reason) {
    if (reason !== 'device_generation') reconnectReleased = false;
    state.configure(stopping ? 0 : desiredMask); generation++; sequence = 0; lastState = ''; lastFailure = '';
    publish('cancel', { reason });
  }
  function reject(reason, details = {}) {
    const failure = JSON.stringify({ reason, ...details });
    if (failure !== lastFailure) { publish('rejected', { reason, ...details }); lastFailure = failure; }
  }
  function errorDetails(error, operation) {
    const message = String(error?.message ?? '');
    const errorClass = message.includes('access violation') ? 'access_violation' :
      ['TypeError', 'ReferenceError', 'RangeError', 'SyntaxError'].includes(error?.name) ? error.name : 'runtime_error';
    const match = String(error?.stack ?? '').match(/sayall-rc003-input\.js:(\d+)/);
    const line = Number(error?.lineNumber ?? match?.[1] ?? 0);
    return { operation, errorClass, codeLine: Number.isInteger(line) && line > 0 && line < 10000 ? line : 0 };
  }
  function finish() {
    if (!stopping || stopped || state.owned !== 0) return;
    // A public removal notification can precede ReleaseHardware. Only the
    // framework callback may release that retiring object's reference.
    if (!present && sourceReference !== null) return;
    stopped = true;
    let cleanupErrors = 0;
    // Keep the framework lifetime hook installed until the live reference is
    // released; removing it first opens a removal/dereference race on exit.
    try { releaseSource(); }
    catch (error) { reject('cleanup_failure', errorDetails(error, 'reference_release')); }
    for (const listener of listeners) {
      try { listener.detach(); }
      catch (error) { cleanupErrors++; reject('cleanup_failure', errorDetails(error, 'listener_detach')); }
    }
    if (referenceReleaseFailed) cleanupErrors++;
    // The native controller still performs its normal Frida unload/detach.
    // A failed WDF release is explicitly failed cleanup, never a successful release.
    publish('stopped', { held: 0, cleanupErrors });
  }
  function complete(args, withInformation) {
    if (stopped || !args[0].equals(globals)) return;
    if (args[2].toInt32() < 0) return;
    let requestIdentity = null;
    try {
      const request = args[1], params = Memory.alloc(40);
      params.writeByteArray(new Uint8Array(40)); params.writeU16(40);
      parameters(globals, request, params);
      const type = params.add(4).readU32();
      if ((type !== 14 && type !== 15) || params.add(24).readU32() !== 0xb000b) return;
      const queue = getQueue(globals, request);
      if (queue.isNull()) { passedUnknown(null); reject('queue_missing'); return; }
      const device = getDevice(globals, queue);
      if (device.isNull()) { passedUnknown(null); reject('device_missing'); return; }
      requestIdentity = device.toString();
      if (!present) { passedUnknown(requestIdentity); return; }
      const value = Memory.alloc(2048), required = Memory.alloc(4); required.writeU32(0);
      const propertyStatus = property(globals, device, 11, 2048, value, required);
      // QueryProperty has no type output; 0 means unavailable, not DEVPROPTYPE_STRING.
      const details = { propertyStatus, propertyType: 0, propertyBytes: required.readU32() };
      if (propertyStatus !== 0) {
        // Never parse a partial buffer or substitute another identity property.
        const reason = (propertyStatus >>> 0) === 0xc0000023 ? 'source_property_buffer_small' :
          (propertyStatus >>> 0) === 0x80000005 ? 'source_property_buffer_overflow' : 'source_property_status';
        passedUnknown(requestIdentity); reject(reason, details); return;
      }
      if (details.propertyBytes < 2 || details.propertyBytes > 2048 || (details.propertyBytes & 1)) {
        passedUnknown(requestIdentity); reject('source_property_length', details); return;
      }
      const chars = details.propertyBytes / 2;
      if (value.add((chars - 1) * 2).readU16() !== 0) {
        passedUnknown(requestIdentity); reject('source_property_termination', details); return;
      }
      const pdo = value.readUtf16String(chars - 1).toLowerCase();
      if (pdo.includes('\0')) { passedUnknown(requestIdentity); reject('source_property_embedded_null', details); return; }
      if (pdo !== expectedPdo) {
        passedUnknown(requestIdentity); reject('source_mismatch', details); return;
      }
      const identity = device.toString();
      if (deviceChanged || (source !== null && source !== identity)) {
        // A different WDF object cannot inherit a PDO association in the same epoch.
        // Keep the old reference/owned holds until public PnP resets the source.
        if (!deviceChanged) { deviceChanged = true; cancel('source_device_changed'); }
        passedUnknown(requestIdentity); reject('source_device_changed', details); return;
      }
      if (source !== identity) {
        if (sourceReference !== null) {
          passedUnknown(requestIdentity); reject('source_retiring'); return;
        }
        reference(globals, device, ptr(0), 0, referenceFile);
        sourceReference = device;
      }
      source = identity;
      const length = withInformation ? Number(args[3].toString()) : information(globals, request).toNumber();
      if (!Number.isSafeInteger(length) || length < 1 || length > config.maxLength) { passedUnknown(requestIdentity); reconnectReleased = false; reject('report_length'); return; }
      const out = Memory.alloc(8), capacity = Memory.alloc(8);
      if (buffer(globals, request, length, out, capacity) < 0 ||
          capacity.readU64().compare(length) < 0 || out.readPointer().isNull()) { passedUnknown(requestIdentity); reconnectReleased = false; reject('output_buffer'); return; }
      const destination = out.readPointer();
      if (destination.readU8() !== 1) return;
      const raw = new Uint8Array(destination.readByteArray(length));
      const priorReleased = reconnectReleased, nativeGap = passedObjects.has(identity);
      const reconnectedFirst = priorReleased && rawReleased && coverageKnown && !nativeGap;
      const next = state.accept(raw, reconnectedFirst);
      reconnectReleased = false;
      if (next.allUp) { passedObjects.delete(identity); coverageKnown = true; }
      lastLength = length;
      // The only mutation in this adapter: the exact verified request, before completion.
      if (next.suppressed) destination.writeByteArray(next.output);
      lastFailure = '';
      const stateKey = [next.ready, next.buttons, next.physical, next.suppressed].join(':');
      if (stateKey !== lastState) {
        lastState = stateKey;
        publish('state', { ready: next.ready, buttons: next.buttons, physical: next.physical,
          suppressed: next.suppressed, source: 'selected_instance', length, allUp: next.allUp,
          basis: 'request', reconnectedFirst, priorReleased, nativeGap, coverageKnown, rawReleased });
      }
      finish();
    } catch (_) { passedUnknown(requestIdentity); cancel('report_contract'); reject('report_contract'); }
  }
  const completing = new Set();
  function hooks(withInformation) {
    return {
      onEnter(args) {
        this.owner = !completing.has(this.threadId);
        if (this.owner) { completing.add(this.threadId); complete(args, withInformation); }
      },
      onLeave() { if (this.owner) completing.delete(this.threadId); }
    };
  }
  listeners.push(Interceptor.attach(address(163), hooks(false)));
  listeners.push(Interceptor.attach(address(164), hooks(true)));
  function disconnectSource() {
    if (!disconnected) {
      reconnectReleased = present && source !== null && !deviceChanged && coverageKnown &&
        rawReleased && state.ready && state.allUp && state.owned === 0;
      disconnected = true;
    }
    present = false; expectedPdo = ''; source = null; state.owned = 0;
  }
  // Fixed driver C928..C95F registers CE60 as EvtDeviceReleaseHardware
  // (WDF_PNPPOWER_EVENT_CALLBACKS +48). Its Device argument is still valid;
  // a later CM removal message is not a WDF object lifetime callback.
  listeners.push(Interceptor.attach(host.base.add(0xce60), {
    onEnter(args) {
      if (stopped || sourceReference === null || !args[0].equals(sourceReference)) return;
      disconnectSource();
      try {
        releaseSource();
        if (pendingPdo) {
          present = true; expectedPdo = pendingPdo; pendingPdo = ''; disconnected = false;
        }
        cancel('device_generation');
        publish('reference_released', { basis: 'release_hardware', held: 0 });
      } catch (error) {
        stopping = true; reconnectReleased = false; coverageKnown = false;
        cancel('control_failure');
        reject('control_failure', errorDetails(error, 'hardware_reference_release'));
      }
      finish();
    }
  }));
  function control(message) {
    if (stopped) return;
    let operation = 'command';
    try {
    if (message.type === 'configure' && !stopping) {
      if (!Number.isSafeInteger(message.configuration) || message.configuration < 0) throw Error('configuration');
      configuration = message.configuration;
      generation++; sequence = 0; lastState = ''; lastFailure = '';
      desiredMask = message.mask;
      rawReleased = message.preserveReleased === true;
      if (!rawReleased) reconnectReleased = false;
      // Configuration changes do not change device identity. Reuse only an
      // already verified all-up state in this device epoch; held/unknown states
      // still wait for a new physical release. Cancellation never uses this path.
      state.configure(message.mask, message.preserveReleased === true && present && source !== null && !deviceChanged);
      publish('configured', { mask: message.mask });
      if (state.ready) {
        const ready = state.mask !== 0;
        lastState = [ready, 0, 0, 0].join(':');
        // Prime the client's edge decoder after its configuration barrier. This
        // is a cached proof, not a new physical report or an injected key edge.
        publish('state', { ready, buttons: 0, physical: 0, suppressed: 0,
          source: 'selected_instance', length: lastLength, allUp: true, basis: 'configuration_reuse' });
      }
    } else if (message.type === 'device_reset') {
      const nextPresent = message.present === true && typeof message.pdo === 'string' && message.pdo.length > 0;
      if (!nextPresent) {
        pendingPdo = '';
        disconnectSource();
      } else {
        if (!disconnected) reconnectReleased = false;
        if (sourceReference === null) disconnected = false;
      }
      if (nextPresent && sourceReference !== null) {
        pendingPdo = message.pdo.toLowerCase(); present = false; expectedPdo = '';
      } else {
        present = nextPresent;
        expectedPdo = present ? message.pdo.toLowerCase() : '';
      }
      deviceChanged = false;
      state.owned = 0; source = null;
      operation = 'device_cancel';
      cancel('device_generation'); finish();
    } else if (message.type === 'stop') {
      stopping = true; cancel('normal_stop'); finish();
    }
    } catch (error) {
      // Stop accepting new ownership, retain already suppressed physical holds,
      // and leave the independent stop receiver reachable after any command error.
      stopping = true; reconnectReleased = false; coverageKnown = false;
      cancel('control_failure');
      reject('control_failure', errorDetails(error, operation));
      finish();
    } finally {
      if (!stopped) recv(message.type, control);
    }
  }
  // Frida receivers are one-shot. Stop must not depend on re-registering a
  // configure/device callback that may have just thrown.
  for (const type of ['configure', 'device_reset', 'stop']) recv(type, control);
  publish('bound', { contract: 1, source: 'pending_per_request' });
}
