#ifndef SAYALL_TV_TEST_PLATFORM_H
#define SAYALL_TV_TEST_PLATFORM_H
#include <assert.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>

typedef void VOID;
typedef void *PVOID;
typedef void *WDFCONTEXT;
typedef long NTSTATUS;
typedef long LONG;
typedef unsigned long ULONG;
typedef ULONG *PULONG;
typedef unsigned char BOOLEAN;
typedef unsigned short USHORT;
typedef int WDF_POWER_DEVICE_STATE;
typedef struct TEST_DEVICE *WDFDEVICE, *WDFQUEUE, *WDFIOTARGET, *WDFKEY;
typedef struct TEST_REQUEST *WDFREQUEST;
typedef void *WDFDRIVER;
typedef void *PDRIVER_OBJECT;
typedef void *PWDFDEVICE_INIT;
typedef struct { const wchar_t *Buffer; } UNICODE_STRING, *PUNICODE_STRING;
typedef struct { WDFDEVICE Device; } DEVICE_OBJECT, *PDEVICE_OBJECT;
typedef struct { PDEVICE_OBJECT ClassDeviceObject; PVOID ClassService; } CONNECT_DATA, *PCONNECT_DATA;
typedef struct {
    USHORT UnitId, MakeCode, Flags, Reserved;
    ULONG ExtraInformation;
} KEYBOARD_INPUT_DATA, *PKEYBOARD_INPUT_DATA;
typedef VOID (*PSERVICE_CALLBACK_ROUTINE)(PVOID, PVOID, PVOID, PVOID);
typedef struct { struct { NTSTATUS Status; } IoStatus; } WDF_REQUEST_COMPLETION_PARAMS, *PWDF_REQUEST_COMPLETION_PARAMS;
typedef NTSTATUS DRIVER_INITIALIZE(PDRIVER_OBJECT, PUNICODE_STRING);
typedef NTSTATUS EVT_WDF_DRIVER_DEVICE_ADD(WDFDRIVER, PWDFDEVICE_INIT);
typedef VOID EVT_WDF_DRIVER_UNLOAD(WDFDRIVER);
typedef VOID EVT_WDF_IO_QUEUE_IO_INTERNAL_DEVICE_CONTROL(WDFQUEUE, WDFREQUEST, size_t, size_t, ULONG);
typedef VOID EVT_WDF_REQUEST_COMPLETION_ROUTINE(WDFREQUEST, WDFIOTARGET, PWDF_REQUEST_COMPLETION_PARAMS, WDFCONTEXT);
typedef NTSTATUS EVT_WDF_DEVICE_D0_ENTRY(WDFDEVICE, WDF_POWER_DEVICE_STATE);
typedef NTSTATUS EVT_WDF_DEVICE_D0_EXIT(WDFDEVICE, WDF_POWER_DEVICE_STATE);
typedef VOID EVT_WDF_DEVICE_SURPRISE_REMOVAL(WDFDEVICE);
typedef struct { EVT_WDF_DRIVER_DEVICE_ADD *EvtDeviceAdd; EVT_WDF_DRIVER_UNLOAD *EvtDriverUnload; } WDF_DRIVER_CONFIG;
typedef struct {
    EVT_WDF_DEVICE_D0_ENTRY *EvtDeviceD0Entry;
    EVT_WDF_DEVICE_D0_EXIT *EvtDeviceD0Exit;
    EVT_WDF_DEVICE_SURPRISE_REMOVAL *EvtDeviceSurpriseRemoval;
} WDF_PNPPOWER_EVENT_CALLBACKS;
typedef struct { size_t ContextSize; WDFDEVICE ParentObject; } WDF_OBJECT_ATTRIBUTES;
typedef struct { int PowerManaged; EVT_WDF_IO_QUEUE_IO_INTERNAL_DEVICE_CONTROL *EvtIoInternalDeviceControl; } WDF_IO_QUEUE_CONFIG;
typedef struct { ULONG Flags; } WDF_REQUEST_SEND_OPTIONS;
typedef struct TEST_LOCK { int Held; } *WDFSPINLOCK;

struct TEST_DEVICE {
    void *Context;
    DEVICE_OBJECT Object;
    unsigned int ProfileMode;
    ULONG MakeCode, Flags;
    WDF_PNPPOWER_EVENT_CALLBACKS Power;
    WDF_IO_QUEUE_CONFIG Queue;
};
struct TEST_REQUEST {
    void *Buffer;
    size_t Length;
    int Sent, Completed, SendAccepted, CompleteInline;
    int Forgotten, LowerPending, LowerCompleted, FilterCompleted, CompletionInvoked, CompleteCount;
    NTSTATUS Status;
    EVT_WDF_REQUEST_COMPLETION_ROUTINE *Completion;
    WDFCONTEXT CompletionContext;
};

#define TRUE 1
#define FALSE 0
#define STATUS_SUCCESS ((NTSTATUS)0)
#define STATUS_OBJECT_NAME_NOT_FOUND ((NTSTATUS)0xc0000034L)
#define STATUS_INVALID_PARAMETER ((NTSTATUS)0xc000000dL)
#define STATUS_OBJECT_TYPE_MISMATCH ((NTSTATUS)0xc0000024L)
#define STATUS_ACCESS_DENIED ((NTSTATUS)0xc0000022L)
#define STATUS_BUFFER_TOO_SMALL ((NTSTATUS)0xc0000023L)
#define STATUS_NOT_IMPLEMENTED ((NTSTATUS)0xc0000002L)
#define STATUS_DELETE_PENDING ((NTSTATUS)0xc0000056L)
#define STATUS_SHARING_VIOLATION ((NTSTATUS)0xc0000043L)
#define STATUS_CANCELLED ((NTSTATUS)0xc0000120L)
#define NT_SUCCESS(status) (((NTSTATUS)(status)) >= 0)
#define UNREFERENCED_PARAMETER(parameter) ((void)(parameter))
#define RtlZeroMemory(destination, size) memset(destination, 0, size)
#define WDF_NO_OBJECT_ATTRIBUTES NULL
#define WDF_NO_HANDLE NULL
#define WDF_NO_SEND_OPTIONS NULL
#define WDF_REQUEST_SEND_OPTION_SEND_AND_FORGET 0x00000008
#define WDF_REQUEST_SEND_OPTIONS_INIT(options, flags) ((options)->Flags = (flags))
#define WdfFalse 0
#define WdfIoQueueDispatchParallel 1
#define FILE_DEVICE_KEYBOARD 11
#define PLUGPLAY_REGKEY_DEVICE 1
#define KEY_READ 0x20019
#define IOCTL_INTERNAL_KEYBOARD_CONNECT 0x0b0203
#define IOCTL_INTERNAL_KEYBOARD_DISCONNECT 0x0b0207
#define DISPATCH_LEVEL 2
#define NT_ASSERT(condition) assert(condition)
#define DECLARE_CONST_UNICODE_STRING(name, value) const UNICODE_STRING name = {value}
#define WDF_DECLARE_CONTEXT_TYPE_WITH_NAME(type, name) static inline type *name(WDFDEVICE device) { return (type *)device->Context; }
#define WDF_OBJECT_ATTRIBUTES_INIT(attributes) memset(attributes, 0, sizeof(*(attributes)))
#define WDF_OBJECT_ATTRIBUTES_INIT_CONTEXT_TYPE(attributes, type) do { WDF_OBJECT_ATTRIBUTES_INIT(attributes); (attributes)->ContextSize = sizeof(type); } while (0)
#define WDF_DRIVER_CONFIG_INIT(config, callback) do { memset(config, 0, sizeof(*(config))); (config)->EvtDeviceAdd = callback; } while (0)
#define WDF_PNPPOWER_EVENT_CALLBACKS_INIT(power) memset(power, 0, sizeof(*(power)))
#define WDF_IO_QUEUE_CONFIG_INIT_DEFAULT_QUEUE(queue, dispatch) do { (void)(dispatch); memset(queue, 0, sizeof(*(queue))); } while (0)

extern struct TEST_DEVICE *TestCreatedDevice;
extern unsigned int TestProfileMode;
extern ULONG TestMakeCode, TestFlags;
extern NTSTATUS TestDriverCreateStatus;
extern WDF_PNPPOWER_EVENT_CALLBACKS TestPowerCallbacks;

static inline LONG InterlockedIncrement(LONG *value) { return ++*value; }
static inline int KeGetCurrentIrql(void) { return DISPATCH_LEVEL; }
static inline VOID WdfFdoInitSetFilter(PWDFDEVICE_INIT init) { (void)init; }
static inline VOID WdfDeviceInitSetDeviceType(PWDFDEVICE_INIT init, int type) { (void)init; assert(type == FILE_DEVICE_KEYBOARD); }
static inline VOID WdfDeviceInitSetPnpPowerEventCallbacks(PWDFDEVICE_INIT init, WDF_PNPPOWER_EVENT_CALLBACKS *power) { (void)init; TestPowerCallbacks = *power; }
static inline NTSTATUS WdfDeviceCreate(PWDFDEVICE_INIT *init, WDF_OBJECT_ATTRIBUTES *attributes, WDFDEVICE *device)
{
    (void)init;
    *device = (WDFDEVICE)calloc(1, sizeof(**device));
    assert(*device);
    (*device)->Context = calloc(1, attributes->ContextSize);
    assert((*device)->Context);
    (*device)->Object.Device = *device;
    (*device)->ProfileMode = TestProfileMode;
    (*device)->MakeCode = TestMakeCode;
    (*device)->Flags = TestFlags;
    (*device)->Power = TestPowerCallbacks;
    TestCreatedDevice = *device;
    return STATUS_SUCCESS;
}
static inline NTSTATUS WdfSpinLockCreate(WDF_OBJECT_ATTRIBUTES *attributes, WDFSPINLOCK *lock)
{
    assert(attributes->ParentObject);
    *lock = (WDFSPINLOCK)calloc(1, sizeof(**lock));
    assert(*lock);
    return STATUS_SUCCESS;
}
static inline VOID WdfSpinLockAcquire(WDFSPINLOCK lock) { assert(!lock->Held); lock->Held = 1; }
static inline VOID WdfSpinLockRelease(WDFSPINLOCK lock) { assert(lock->Held); lock->Held = 0; }
static inline NTSTATUS WdfIoQueueCreate(WDFDEVICE device, WDF_IO_QUEUE_CONFIG *config, void *attributes, void *handle)
{
    (void)attributes; (void)handle; device->Queue = *config; return STATUS_SUCCESS;
}
static inline WDFDEVICE WdfIoQueueGetDevice(WDFQUEUE queue) { return queue; }
static inline WDFIOTARGET WdfDeviceGetIoTarget(WDFDEVICE device) { return device; }
static inline PDEVICE_OBJECT WdfDeviceWdmGetDeviceObject(WDFDEVICE device) { return &device->Object; }
static inline WDFDEVICE WdfWdmDeviceGetWdfDeviceHandle(PDEVICE_OBJECT object) { return object->Device; }
static inline NTSTATUS WdfDeviceOpenRegistryKey(WDFDEVICE device, int keyType, int access, void *attributes, WDFKEY *key)
{
    (void)attributes;
    assert(keyType == PLUGPLAY_REGKEY_DEVICE && access == KEY_READ);
    *key = device;
    return device->ProfileMode == 0 ? STATUS_OBJECT_NAME_NOT_FOUND : device->ProfileMode == 4 ? STATUS_ACCESS_DENIED : STATUS_SUCCESS;
}
static inline NTSTATUS WdfRegistryQueryULong(WDFKEY key, const UNICODE_STRING *name, ULONG *value)
{
    if (key->ProfileMode == 5) return STATUS_OBJECT_NAME_NOT_FOUND;
    if (key->ProfileMode == 2) return STATUS_OBJECT_TYPE_MISMATCH;
    if (wcscmp(name->Buffer, L"ObserveCandidateMakeCode") == 0) *value = key->MakeCode;
    else { if (key->ProfileMode == 3) return STATUS_OBJECT_NAME_NOT_FOUND; *value = key->Flags; }
    return STATUS_SUCCESS;
}
static inline VOID WdfRegistryClose(WDFKEY key) { (void)key; }
static inline NTSTATUS WdfDriverCreate(PDRIVER_OBJECT object, PUNICODE_STRING path, void *attributes, WDF_DRIVER_CONFIG *config, void *handle)
{
    (void)object; (void)path; (void)attributes; (void)handle;
    assert(config->EvtDeviceAdd && config->EvtDriverUnload);
    return TestDriverCreateStatus;
}
static inline NTSTATUS WdfRequestRetrieveInputBuffer(WDFREQUEST request, size_t minimum, void **buffer, void *length)
{
    (void)length;
    if (request->Length < minimum) return STATUS_BUFFER_TOO_SMALL;
    *buffer = request->Buffer;
    return STATUS_SUCCESS;
}
static inline VOID WdfRequestFormatRequestUsingCurrentType(WDFREQUEST request) { (void)request; }
static inline VOID WdfRequestSetCompletionRoutine(WDFREQUEST request, EVT_WDF_REQUEST_COMPLETION_ROUTINE *completion, WDFCONTEXT context)
{
    request->Completion = completion; request->CompletionContext = context;
}
static inline VOID TestLowerComplete(WDFREQUEST request, WDFIOTARGET target, NTSTATUS status)
{
    WDF_REQUEST_COMPLETION_PARAMS parameters;
    assert(request->LowerPending && !request->Completed);
    request->LowerPending = 0;
    ++request->LowerCompleted;
    request->Status = status;
    if (request->Completion) {
        ++request->CompletionInvoked;
        parameters.IoStatus.Status = status;
        request->Completion(request, target, &parameters, request->CompletionContext);
    } else if (request->Forgotten) {
        request->Completed = 1;
        ++request->CompleteCount;
    }
}
static inline int WdfRequestSend(WDFREQUEST request, WDFIOTARGET target, WDF_REQUEST_SEND_OPTIONS *options)
{
    assert(!request->Completed && !request->Forgotten);
    ++request->Sent;
    if (options) {
        assert(options->Flags == WDF_REQUEST_SEND_OPTION_SEND_AND_FORGET && !request->Completion);
    }
    if (request->SendAccepted) {
        request->Forgotten = options != NULL;
        request->LowerPending = 1;
        if (request->CompleteInline) {
            TestLowerComplete(request, target, request->Status);
        }
    }
    return request->SendAccepted;
}
static inline NTSTATUS WdfRequestGetStatus(WDFREQUEST request) { assert(!request->Forgotten); return request->Status; }
static inline VOID WdfRequestComplete(WDFREQUEST request, NTSTATUS status)
{
    assert(!request->Completed && !request->Forgotten);
    request->Completed = 1; request->Status = status;
    ++request->FilterCompleted; ++request->CompleteCount;
}
#endif
