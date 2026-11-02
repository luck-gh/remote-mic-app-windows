#include <stdio.h>
#include "../driver.h"

struct TEST_DEVICE *TestCreatedDevice;
unsigned int TestProfileMode;
ULONG TestMakeCode = 0x29, TestFlags;
NTSTATUS TestDriverCreateStatus;
WDF_PNPPOWER_EVENT_CALLBACKS TestPowerCallbacks;
static TV_TRACE_SNAPSHOT LastTrace;
static unsigned int TraceRegistered, TraceUnregistered, TraceCount;
static ULONG ConsumeLimit;
static PDEVICE_OBJECT SeenUpper;
static PKEYBOARD_INPUT_DATA SeenStart, SeenEnd;
static PULONG SeenConsumed;
static WDFDEVICE LifecycleDuringUpper, NestedDuringUpper;

long TvTraceRegister(void) { ++TraceRegistered; return STATUS_SUCCESS; }
void TvTraceUnregister(void) { ++TraceUnregistered; }
void TvTraceWrite(const TV_TRACE_SNAPSHOT *snapshot) { LastTrace = *snapshot; ++TraceCount; }

static VOID Upper(PDEVICE_OBJECT object, PKEYBOARD_INPUT_DATA start,
    PKEYBOARD_INPUT_DATA end, PULONG consumed)
{
    SeenUpper = object; SeenStart = start; SeenEnd = end; SeenConsumed = consumed;
    if (LifecycleDuringUpper) {
        WDFDEVICE deviceHandle = LifecycleDuringUpper;
        LifecycleDuringUpper = NULL;
        TvD0Exit(deviceHandle, 0);
        TvD0Entry(deviceHandle, 0);
    }
    if (NestedDuringUpper) {
        WDFDEVICE deviceHandle = NestedDuringUpper;
        ULONG nestedConsumed = 0;
        NestedDuringUpper = NULL;
        TvServiceCallback(&deviceHandle->Object, start, end, &nestedConsumed);
        assert(nestedConsumed == ConsumeLimit);
    }
    *consumed = ConsumeLimit;
}

static WDFDEVICE device(unsigned int profileMode)
{
    TestProfileMode = profileMode;
    assert(TvDeviceAdd(NULL, NULL) == STATUS_SUCCESS);
    assert(TestCreatedDevice->Power.EvtDeviceD0Entry == TvD0Entry);
    assert(TestCreatedDevice->Power.EvtDeviceD0Exit == TvD0Exit);
    assert(TestCreatedDevice->Power.EvtDeviceSurpriseRemoval == TvSurpriseRemoval);
    assert(TestCreatedDevice->Queue.PowerManaged == WdfFalse);
    assert(TestCreatedDevice->Queue.EvtIoInternalDeviceControl == TvInternalControl);
    return TestCreatedDevice;
}

static void dispose(WDFDEVICE deviceHandle)
{
    free(TvDeviceContext(deviceHandle)->Lock);
    free(deviceHandle->Context);
    free(deviceHandle);
}

static void connect(WDFDEVICE deviceHandle, CONNECT_DATA *data, struct TEST_REQUEST *request)
{
    request->Buffer = data;
    request->Length = sizeof(*data);
    TvInternalControl(deviceHandle, request, 0, sizeof(*data), IOCTL_INTERNAL_KEYBOARD_CONNECT);
}

static void test_callback_and_retry(void)
{
    WDFDEVICE remote = device(1);
    WDFDEVICE keyboard = device(0);
    DEVICE_OBJECT upperDevice = {0};
    CONNECT_DATA binding = {&upperDevice, (PVOID)Upper};
    struct TEST_REQUEST request = {0};
    KEYBOARD_INPUT_DATA input[5] = {{0, 0x29, 0, 0, 77}, {0, 0x2a, 0, 0, 88},
        {0, 0x29, 1, 0, 99}, {0, 0x29, 2, 0, 100}, {0, 0x2a, 1, 0, 101}};
    KEYBOARD_INPUT_DATA original[5];
    ULONG consumed = 17;
    memcpy(original, input, sizeof(input));
    request.SendAccepted = 1; request.CompleteInline = 1;
    connect(remote, &binding, &request);
    assert(request.Completed && request.Status == STATUS_SUCCESS);
    assert(!request.Forgotten && request.CompletionInvoked == 1 && request.FilterCompleted == 1);
    assert(request.LowerCompleted == 1 && request.CompleteCount == 1);
    assert(binding.ClassDeviceObject == &remote->Object);
    assert(binding.ClassService == (PVOID)TvServiceCallback);
    assert(TvDeviceContext(remote)->Connected);
    TvD0Entry(remote, 0);
    ConsumeLimit = 2;
    TvServiceCallback(&remote->Object, input, input + 5, &consumed);
    assert(SeenUpper == &upperDevice && SeenStart == input && SeenEnd == input + 5);
    assert(SeenConsumed == &consumed && consumed == 2);
    assert(memcmp(original, input, sizeof(input)) == 0);
    assert(LastTrace.Down == 1 && LastTrace.Up == 0 && LastTrace.Held);
    assert(LastTrace.Consumed == 2 && LastTrace.Partial == 1);
    ConsumeLimit = 0;
    TvServiceCallback(&remote->Object, input + 2, input + 5, &consumed);
    assert(LastTrace.Down == 1 && LastTrace.Up == 0 && LastTrace.ZeroConsumed == 1);
    ConsumeLimit = 3;
    TvServiceCallback(&remote->Object, input + 2, input + 5, &consumed);
    assert(LastTrace.Down == 1 && LastTrace.Up == 1 && !LastTrace.Held);
    assert(LastTrace.Consumed == 5 && memcmp(original, input, sizeof(input)) == 0);
    assert(TvDeviceContext(keyboard)->Observer.Down == 0);
    {
        CONNECT_DATA physicalBinding = {&upperDevice, (PVOID)Upper};
        struct TEST_REQUEST physicalRequest = {0};
        physicalRequest.SendAccepted = 1; physicalRequest.CompleteInline = 1;
        connect(keyboard, &physicalBinding, &physicalRequest);
        TvD0Entry(keyboard, 0);
        ConsumeLimit = 5;
        TvServiceCallback(&keyboard->Object, input, input + 5, &consumed);
        assert(consumed == 5 && memcmp(original, input, sizeof(input)) == 0);
        assert(!TvDeviceContext(keyboard)->Observer.Down && !TvDeviceContext(keyboard)->Observer.Up);
    }
    ConsumeLimit = 6;
    TvServiceCallback(&remote->Object, input, input + 5, &consumed);
    assert(consumed == 6 && LastTrace.Stage == TvConsumptionRejected);
    assert(LastTrace.InvalidConsumed == 1 && LastTrace.Down == 1);
    ConsumeLimit = 0;
    TvServiceCallback(&remote->Object, input, input, &consumed);
    assert(consumed == 0);
    dispose(remote); dispose(keyboard);
}

static void test_connect_failures(void)
{
    WDFDEVICE remote = device(1);
    DEVICE_OBJECT upperDevice = {0};
    CONNECT_DATA original = {&upperDevice, (PVOID)Upper};
    CONNECT_DATA binding = original;
    struct TEST_REQUEST request = {0};
    request.Status = STATUS_ACCESS_DENIED;
    connect(remote, &binding, &request);
    assert(request.Completed && request.Status == STATUS_ACCESS_DENIED);
    assert(memcmp(&binding, &original, sizeof(binding)) == 0);
    assert(!TvDeviceContext(remote)->PendingConnect && !TvDeviceContext(remote)->Connected);
    memset(&request, 0, sizeof(request));
    request.SendAccepted = 1; request.CompleteInline = 1; request.Status = STATUS_CANCELLED;
    connect(remote, &binding, &request);
    assert(request.Completed && request.Status == STATUS_CANCELLED);
    assert(memcmp(&binding, &original, sizeof(binding)) == 0);
    memset(&request, 0, sizeof(request));
    request.SendAccepted = 1;
    connect(remote, &binding, &request);
    assert(!request.Completed && TvDeviceContext(remote)->PendingConnect);
    {
        CONNECT_DATA duplicate = original;
        struct TEST_REQUEST second = {0};
        connect(remote, &duplicate, &second);
        assert(second.Completed && second.Status == STATUS_SHARING_VIOLATION && !second.Sent);
        assert(memcmp(&duplicate, &original, sizeof(duplicate)) == 0);
    }
    TestLowerComplete(&request, remote, STATUS_SUCCESS);
    assert(request.Completed && TvDeviceContext(remote)->Connected);
    assert(!request.Forgotten && request.CompletionInvoked == 1 && request.CompleteCount == 1);
    {
        CONNECT_DATA duplicate = original;
        struct TEST_REQUEST second = {0};
        connect(remote, &duplicate, &second);
        assert(second.Status == STATUS_SHARING_VIOLATION);
    }
    {
        struct TEST_REQUEST shortRequest = {0};
        TvInternalControl(remote, &shortRequest, 0, 0, IOCTL_INTERNAL_KEYBOARD_CONNECT);
        assert(shortRequest.Status == STATUS_BUFFER_TOO_SMALL && !shortRequest.Sent);
    }
    TvSurpriseRemoval(remote);
    dispose(remote);
    remote = device(0);
    binding.ClassDeviceObject = NULL;
    binding.ClassService = (PVOID)Upper;
    memset(&request, 0, sizeof(request));
    connect(remote, &binding, &request);
    assert(request.Status == STATUS_INVALID_PARAMETER && !request.Sent);
    binding.ClassDeviceObject = &upperDevice;
    binding.ClassService = NULL;
    memset(&request, 0, sizeof(request));
    connect(remote, &binding, &request);
    assert(request.Status == STATUS_INVALID_PARAMETER && !request.Sent);
    TvSurpriseRemoval(remote);
    binding = original;
    memset(&request, 0, sizeof(request));
    connect(remote, &binding, &request);
    assert(request.Status == STATUS_DELETE_PENDING && !request.Sent);
    assert(memcmp(&binding, &original, sizeof(binding)) == 0);
    dispose(remote);
}

static void test_profiles_and_lifecycle(void)
{
    unsigned int mode;
    for (mode = 0; mode < 6; ++mode) {
        WDFDEVICE remote = device(mode);
        TV_DEVICE_CONTEXT *context = TvDeviceContext(remote);
        DEVICE_OBJECT upperDevice = {0};
        CONNECT_DATA binding = {&upperDevice, (PVOID)Upper};
        struct TEST_REQUEST request = {0};
        KEYBOARD_INPUT_DATA input = {0, 0x29, 0, 0, 0};
        ULONG consumed = 0;
        assert(LastTrace.Stage == (unsigned int)(mode == 0 || mode == 5 ? TvProfileAbsent : mode == 1 ? TvProfileAccepted : TvProfileRejected));
        request.SendAccepted = 1; request.CompleteInline = 1;
        connect(remote, &binding, &request);
        TvD0Entry(remote, 0);
        ConsumeLimit = 1;
        TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
        assert(consumed == 1 && input.MakeCode == 0x29 && input.Flags == 0);
        assert(context->Observer.Down == (mode == 1 ? 1u : 0u));
        TvD0Exit(remote, 0); TvD0Exit(remote, 0);
        assert(!context->Observer.Active && !context->Observer.Held);
        assert(context->Observer.CancelledHold == (mode == 1 ? 1u : 0u));
        TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
        assert(consumed == 1);
        TvD0Entry(remote, 0); input.Flags = 1;
        TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
        assert(context->Observer.OrphanUp == (mode == 1 ? 1u : 0u));
        TvSurpriseRemoval(remote); TvSurpriseRemoval(remote); TvD0Entry(remote, 0);
        assert(context->Removed && !context->Observer.Active);
        TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
        assert(consumed == 1);
        dispose(remote);
    }
    TestMakeCode = 0xff;
    {
        WDFDEVICE remote = device(1);
        assert(LastTrace.Stage == TvProfileRejected && LastTrace.Status == STATUS_INVALID_PARAMETER);
        assert(!TvDeviceContext(remote)->Observer.Configured);
        dispose(remote);
    }
    TestMakeCode = 0x29;
    TestFlags = 6;
    {
        WDFDEVICE remote = device(1);
        assert(LastTrace.Stage == TvProfileRejected && LastTrace.Status == STATUS_INVALID_PARAMETER);
        dispose(remote);
    }
    TestFlags = 0;
}

static void test_overlapping_and_late_callbacks(void)
{
    WDFDEVICE remote = device(1);
    DEVICE_OBJECT upperDevice = {0};
    CONNECT_DATA binding = {&upperDevice, (PVOID)Upper};
    struct TEST_REQUEST request = {0};
    KEYBOARD_INPUT_DATA input = {0, 0x29, 0, 0, 0};
    ULONG consumed = 0;
    TV_DEVICE_CONTEXT *context = TvDeviceContext(remote);
    request.SendAccepted = 1; request.CompleteInline = 1;
    connect(remote, &binding, &request);
    TvD0Entry(remote, 0);
    ConsumeLimit = 1;
    TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
    assert(context->Observer.Down == 1 && context->Observer.Held);
    NestedDuringUpper = remote;
    TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
    assert(context->Counters.Overlapped == 1 && context->Counters.Skipped == 2);
    assert(context->Observer.Down == 1 && !context->Observer.Held);
    assert(context->Observer.CancelledHold == 1 && !context->ActiveCallbacks);
    LifecycleDuringUpper = remote;
    TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
    assert(context->Counters.Skipped == 3 && context->Observer.Down == 1);
    assert(LastTrace.Stage == TvObservationSkipped);
    TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
    assert(context->Observer.Down == 2 && context->Observer.Held);
    ConsumeLimit = 2;
    TvServiceCallback(&remote->Object, &input, &input + 1, &consumed);
    assert(consumed == 2 && context->Counters.InvalidConsumed == 1);
    assert(!context->Observer.Held && context->Observer.CancelledHold == 2);
    dispose(remote);
}

static void test_entry_and_other_controls(void)
{
    WDFDEVICE remote;
    struct TEST_REQUEST request = {0};
    assert(DriverEntry(NULL, NULL) == STATUS_SUCCESS);
    TvDriverUnload(NULL);
    assert(TraceRegistered == 1 && TraceUnregistered == 1);
    TestDriverCreateStatus = STATUS_ACCESS_DENIED;
    assert(DriverEntry(NULL, NULL) == STATUS_ACCESS_DENIED);
    assert(TraceRegistered == 2 && TraceUnregistered == 2);
    remote = device(0);
    request.Status = STATUS_ACCESS_DENIED;
    TvInternalControl(remote, &request, 0, 0, 123);
    assert(request.Sent == 1 && request.Completed && request.Status == STATUS_ACCESS_DENIED);
    assert(request.FilterCompleted == 1 && !request.LowerCompleted && request.CompleteCount == 1);
    {
        unsigned int iteration;
        for (iteration = 0; iteration < 4; ++iteration) {
            NTSTATUS lowerStatus = iteration < 2 ? STATUS_SUCCESS : STATUS_CANCELLED;
            memset(&request, 0, sizeof(request));
            request.SendAccepted = 1;
            request.CompleteInline = iteration % 2;
            request.Status = lowerStatus;
            TvInternalControl(remote, &request, 0, 0, 123);
            if (!request.CompleteInline) {
                assert(!request.Completed && request.LowerPending);
                TestLowerComplete(&request, remote, lowerStatus);
            }
            assert(request.Completed && request.Status == lowerStatus);
            assert(request.Forgotten && request.LowerCompleted == 1 && request.CompleteCount == 1);
            assert(!request.FilterCompleted && !request.CompletionInvoked && !request.LowerPending);
        }
    }
    memset(&request, 0, sizeof(request));
    TvInternalControl(remote, &request, 0, 0, IOCTL_INTERNAL_KEYBOARD_DISCONNECT);
    assert(request.Completed && request.Status == STATUS_NOT_IMPLEMENTED);
    dispose(remote);
}

int main(void)
{
    test_callback_and_retry();
    test_connect_failures();
    test_profiles_and_lifecycle();
    test_overlapping_and_late_callbacks();
    test_entry_and_other_controls();
    assert(TraceCount > 20);
    puts("passed: same driver.c CONNECT/rollback/callback/partial retry/profile/lifecycle/provider harness");
    return 0;
}
