#include "driver.h"

static LONG NextDeviceOrdinal;

static TV_TRACE_SNAPSHOT Snapshot(TV_DEVICE_CONTEXT *context, TV_TRACE_STAGE stage, NTSTATUS status)
{
    TV_TRACE_SNAPSHOT snapshot = context->Counters;
    snapshot.Sequence = ++context->Counters.Sequence;
    snapshot.Stage = stage;
    snapshot.Status = status;
    snapshot.Configured = context->Observer.Configured;
    snapshot.Active = context->Observer.Active;
    snapshot.Removed = context->Removed;
    snapshot.Held = context->Observer.Held;
    snapshot.Generation = context->Observer.Generation;
    snapshot.Down = context->Observer.Down;
    snapshot.Up = context->Observer.Up;
    snapshot.Repeat = context->Observer.Repeat;
    snapshot.OrphanUp = context->Observer.OrphanUp;
    snapshot.CancelledHold = context->Observer.CancelledHold;
    return snapshot;
}

static VOID ReadProfile(WDFDEVICE device)
{
    DECLARE_CONST_UNICODE_STRING(makeName, L"ObserveCandidateMakeCode");
    DECLARE_CONST_UNICODE_STRING(flagsName, L"ObserveCandidateFlags");
    TV_DEVICE_CONTEXT *context = TvDeviceContext(device);
    WDFKEY key;
    ULONG makeCode, flags;
    NTSTATUS status, makeStatus, flagsStatus;
    TV_TRACE_SNAPSHOT snapshot;
    status = WdfDeviceOpenRegistryKey(device, PLUGPLAY_REGKEY_DEVICE, KEY_READ,
        WDF_NO_OBJECT_ATTRIBUTES, &key);
    if (status == STATUS_OBJECT_NAME_NOT_FOUND) {
        snapshot = Snapshot(context, TvProfileAbsent, status);
    } else if (!NT_SUCCESS(status)) {
        snapshot = Snapshot(context, TvProfileRejected, status);
    } else {
        makeStatus = WdfRegistryQueryULong(key, &makeName, &makeCode);
        flagsStatus = WdfRegistryQueryULong(key, &flagsName, &flags);
        WdfRegistryClose(key);
        if (makeStatus == STATUS_OBJECT_NAME_NOT_FOUND && flagsStatus == STATUS_OBJECT_NAME_NOT_FOUND) {
            snapshot = Snapshot(context, TvProfileAbsent, STATUS_OBJECT_NAME_NOT_FOUND);
        } else if (!NT_SUCCESS(makeStatus) || !NT_SUCCESS(flagsStatus)) {
            status = !NT_SUCCESS(makeStatus) ? makeStatus : flagsStatus;
            snapshot = Snapshot(context, TvProfileRejected, status);
        } else if (!TvObserverConfigure(&context->Observer, makeCode, flags)) {
            snapshot = Snapshot(context, TvProfileRejected, STATUS_INVALID_PARAMETER);
        } else {
            snapshot = Snapshot(context, TvProfileAccepted, STATUS_SUCCESS);
        }
    }
    TvTraceWrite(&snapshot);
}

NTSTATUS DriverEntry(PDRIVER_OBJECT driverObject, PUNICODE_STRING registryPath)
{
    WDF_DRIVER_CONFIG config;
    NTSTATUS status = TvTraceRegister();
    if (!NT_SUCCESS(status)) {
        return status;
    }
    WDF_DRIVER_CONFIG_INIT(&config, TvDeviceAdd);
    config.EvtDriverUnload = TvDriverUnload;
    status = WdfDriverCreate(driverObject, registryPath, WDF_NO_OBJECT_ATTRIBUTES,
        &config, WDF_NO_HANDLE);
    if (!NT_SUCCESS(status)) {
        TvTraceUnregister();
    }
    return status;
}

VOID TvDriverUnload(WDFDRIVER driver)
{
    UNREFERENCED_PARAMETER(driver);
    TvTraceUnregister();
}

NTSTATUS TvDeviceAdd(WDFDRIVER driver, PWDFDEVICE_INIT deviceInit)
{
    WDF_OBJECT_ATTRIBUTES attributes, lockAttributes;
    WDF_PNPPOWER_EVENT_CALLBACKS power;
    WDF_IO_QUEUE_CONFIG queue;
    WDFDEVICE device;
    TV_DEVICE_CONTEXT *context;
    TV_TRACE_SNAPSHOT snapshot = {0};
    NTSTATUS status;
    UNREFERENCED_PARAMETER(driver);
    WdfFdoInitSetFilter(deviceInit);
    WdfDeviceInitSetDeviceType(deviceInit, FILE_DEVICE_KEYBOARD);
    WDF_PNPPOWER_EVENT_CALLBACKS_INIT(&power);
    power.EvtDeviceD0Entry = TvD0Entry;
    power.EvtDeviceD0Exit = TvD0Exit;
    power.EvtDeviceSurpriseRemoval = TvSurpriseRemoval;
    WdfDeviceInitSetPnpPowerEventCallbacks(deviceInit, &power);
    WDF_OBJECT_ATTRIBUTES_INIT_CONTEXT_TYPE(&attributes, TV_DEVICE_CONTEXT);
    status = WdfDeviceCreate(&deviceInit, &attributes, &device);
    if (!NT_SUCCESS(status)) {
        snapshot.Stage = TvDeviceRejected;
        snapshot.Status = status;
        TvTraceWrite(&snapshot);
        return status;
    }
    context = TvDeviceContext(device);
    context->Counters.DeviceOrdinal = (ULONG)InterlockedIncrement(&NextDeviceOrdinal);
    WDF_OBJECT_ATTRIBUTES_INIT(&lockAttributes);
    lockAttributes.ParentObject = device;
    status = WdfSpinLockCreate(&lockAttributes, &context->Lock);
    if (!NT_SUCCESS(status)) {
        snapshot = Snapshot(context, TvDeviceRejected, status);
        TvTraceWrite(&snapshot);
        return status;
    }
    ReadProfile(device);
    WDF_IO_QUEUE_CONFIG_INIT_DEFAULT_QUEUE(&queue, WdfIoQueueDispatchParallel);
    queue.PowerManaged = WdfFalse;
    queue.EvtIoInternalDeviceControl = TvInternalControl;
    status = WdfIoQueueCreate(device, &queue, WDF_NO_OBJECT_ATTRIBUTES, WDF_NO_HANDLE);
    if (!NT_SUCCESS(status)) {
        snapshot = Snapshot(context, TvDeviceRejected, status);
        TvTraceWrite(&snapshot);
    }
    return status;
}

static VOID FinishConnect(TV_DEVICE_CONTEXT *context, WDFREQUEST request, NTSTATUS status)
{
    TV_TRACE_SNAPSHOT snapshot;
    WdfSpinLockAcquire(context->Lock);
    if (NT_SUCCESS(status)) {
        context->Connected = TRUE;
    } else {
        *context->PendingConnect = context->Upper;
        RtlZeroMemory(&context->Upper, sizeof(context->Upper));
    }
    context->PendingConnect = NULL;
    snapshot = Snapshot(context, NT_SUCCESS(status) ? TvConnectAccepted : TvConnectRejected, status);
    WdfSpinLockRelease(context->Lock);
    TvTraceWrite(&snapshot);
    WdfRequestComplete(request, status);
}

VOID TvConnectComplete(WDFREQUEST request, WDFIOTARGET target,
    PWDF_REQUEST_COMPLETION_PARAMS parameters, WDFCONTEXT completionContext)
{
    UNREFERENCED_PARAMETER(target);
    FinishConnect((TV_DEVICE_CONTEXT *)completionContext, request, parameters->IoStatus.Status);
}

VOID TvInternalControl(WDFQUEUE queue, WDFREQUEST request, size_t outputLength,
    size_t inputLength, ULONG controlCode)
{
    WDFDEVICE device = WdfIoQueueGetDevice(queue);
    TV_DEVICE_CONTEXT *context = TvDeviceContext(device);
    PCONNECT_DATA connect;
    WDF_REQUEST_SEND_OPTIONS sendOptions;
    TV_TRACE_SNAPSHOT snapshot;
    NTSTATUS status;
    UNREFERENCED_PARAMETER(outputLength);
    UNREFERENCED_PARAMETER(inputLength);
    if (controlCode == IOCTL_INTERNAL_KEYBOARD_DISCONNECT) {
        status = STATUS_NOT_IMPLEMENTED;
    } else if (controlCode != IOCTL_INTERNAL_KEYBOARD_CONNECT) {
        WdfRequestFormatRequestUsingCurrentType(request);
        WDF_REQUEST_SEND_OPTIONS_INIT(&sendOptions, WDF_REQUEST_SEND_OPTION_SEND_AND_FORGET);
        if (!WdfRequestSend(request, WdfDeviceGetIoTarget(device), &sendOptions)) {
            status = WdfRequestGetStatus(request);
            WdfSpinLockAcquire(context->Lock);
            snapshot = Snapshot(context, TvRequestRejected, status);
            WdfSpinLockRelease(context->Lock);
            TvTraceWrite(&snapshot);
            WdfRequestComplete(request, status);
        }
        return;
    } else {
        status = WdfRequestRetrieveInputBuffer(request, sizeof(CONNECT_DATA), (PVOID *)&connect, NULL);
        if (NT_SUCCESS(status) && (!connect->ClassDeviceObject || !connect->ClassService)) {
            status = STATUS_INVALID_PARAMETER;
        }
        if (NT_SUCCESS(status)) {
            WdfSpinLockAcquire(context->Lock);
            if (context->Removed) {
                status = STATUS_DELETE_PENDING;
            } else if (context->Connected || context->PendingConnect) {
                status = STATUS_SHARING_VIOLATION;
            } else {
                context->Upper = *connect;
                context->PendingConnect = connect;
                connect->ClassDeviceObject = WdfDeviceWdmGetDeviceObject(device);
                connect->ClassService = (PVOID)TvServiceCallback;
            }
            snapshot = Snapshot(context, NT_SUCCESS(status) ? TvConnectPending : TvConnectRejected, status);
            WdfSpinLockRelease(context->Lock);
            TvTraceWrite(&snapshot);
            if (NT_SUCCESS(status)) {
                WdfRequestFormatRequestUsingCurrentType(request);
                WdfRequestSetCompletionRoutine(request, TvConnectComplete, context);
                if (!WdfRequestSend(request, WdfDeviceGetIoTarget(device), WDF_NO_SEND_OPTIONS)) {
                    FinishConnect(context, request, WdfRequestGetStatus(request));
                }
                return;
            }
        }
    }
    WdfSpinLockAcquire(context->Lock);
    snapshot = Snapshot(context, TvRequestRejected, status);
    WdfSpinLockRelease(context->Lock);
    TvTraceWrite(&snapshot);
    WdfRequestComplete(request, status);
}

VOID TvServiceCallback(PDEVICE_OBJECT deviceObject, PKEYBOARD_INPUT_DATA inputStart,
    PKEYBOARD_INPUT_DATA inputEnd, PULONG inputConsumed)
{
    TV_DEVICE_CONTEXT *context = TvDeviceContext(WdfWdmDeviceGetWdfDeviceHandle(deviceObject));
    CONNECT_DATA upper;
    ULONG epoch, generation;
    BOOLEAN overlap;
    size_t offered = (size_t)(inputEnd - inputStart);
    ULONG packetIndex;
    TV_TRACE_STAGE stage = TvPacketsObserved;
    TV_TRACE_SNAPSHOT snapshot;
    NT_ASSERT(KeGetCurrentIrql() == DISPATCH_LEVEL);
    WdfSpinLockAcquire(context->Lock);
    upper = context->Upper;
    overlap = ++context->ActiveCallbacks > 1;
    if (overlap) {
        ++context->CallbackEpoch;
        ++context->Counters.Overlapped;
        TvObserverReset(&context->Observer);
    }
    epoch = context->CallbackEpoch;
    generation = context->Observer.Generation;
    WdfSpinLockRelease(context->Lock);
    ((PSERVICE_CALLBACK_ROUTINE)upper.ClassService)(upper.ClassDeviceObject,
        inputStart, inputEnd, inputConsumed);
    WdfSpinLockAcquire(context->Lock);
    --context->ActiveCallbacks;
    context->Counters.Offered += offered;
    if (*inputConsumed > offered) {
        ++context->Counters.InvalidConsumed;
        TvObserverReset(&context->Observer);
        stage = TvConsumptionRejected;
    } else {
        context->Counters.Consumed += *inputConsumed;
        if (*inputConsumed < offered) {
            ++context->Counters.Partial;
        }
        if (*inputConsumed == 0) {
            ++context->Counters.ZeroConsumed;
        }
        if (overlap || epoch != context->CallbackEpoch || generation != context->Observer.Generation) {
            ++context->Counters.Skipped;
            stage = TvObservationSkipped;
        } else {
            for (packetIndex = 0; packetIndex < *inputConsumed; ++packetIndex) {
                TvObserverPacket(&context->Observer, inputStart[packetIndex].MakeCode,
                    inputStart[packetIndex].Flags);
            }
        }
    }
    snapshot = Snapshot(context, stage, stage == TvConsumptionRejected ? STATUS_INVALID_PARAMETER : STATUS_SUCCESS);
    WdfSpinLockRelease(context->Lock);
    if (snapshot.Configured || stage == TvConsumptionRejected) {
        TvTraceWrite(&snapshot);
    }
}

NTSTATUS TvD0Entry(WDFDEVICE device, WDF_POWER_DEVICE_STATE previousState)
{
    TV_DEVICE_CONTEXT *context = TvDeviceContext(device);
    TV_TRACE_SNAPSHOT snapshot;
    UNREFERENCED_PARAMETER(previousState);
    WdfSpinLockAcquire(context->Lock);
    if (!context->Removed) {
        TvObserverStart(&context->Observer);
    }
    snapshot = Snapshot(context, TvPowerStarted, STATUS_SUCCESS);
    WdfSpinLockRelease(context->Lock);
    TvTraceWrite(&snapshot);
    return STATUS_SUCCESS;
}

static VOID StopObservation(WDFDEVICE device, TV_TRACE_STAGE stage, BOOLEAN removed)
{
    TV_DEVICE_CONTEXT *context = TvDeviceContext(device);
    TV_TRACE_SNAPSHOT snapshot;
    WdfSpinLockAcquire(context->Lock);
    context->Removed |= removed;
    TvObserverStop(&context->Observer);
    snapshot = Snapshot(context, stage, STATUS_SUCCESS);
    WdfSpinLockRelease(context->Lock);
    TvTraceWrite(&snapshot);
}

NTSTATUS TvD0Exit(WDFDEVICE device, WDF_POWER_DEVICE_STATE targetState)
{
    UNREFERENCED_PARAMETER(targetState);
    StopObservation(device, TvPowerStopped, FALSE);
    return STATUS_SUCCESS;
}

VOID TvSurpriseRemoval(WDFDEVICE device)
{
    StopObservation(device, TvSurpriseRemoved, TRUE);
}
