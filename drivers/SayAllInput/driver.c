/* SPDX-License-Identifier: GPL-3.0-only
 * HID read completion pattern: QL-4/RemoteMapper, MIT (LICENSES/RemoteMapper.txt).
 * Per-device raw PDO architectural reference: Microsoft kbfiltr (MS-PL),
 * independently implemented here; no Microsoft sample source is copied.
 * See ATTRIBUTION.md for exact source revisions. */
#define POOL_ZERO_DOWN_LEVEL_SUPPORT
#include <ntifs.h>
#pragma warning(push)
#pragma warning(disable:4324) /* KMDF 1.15 request ABI has explicit alignment. */
#include <wdf.h>
#pragma warning(pop)
#include <hidport.h>
#include <hidpddi.h>
#include <ntstrsafe.h>
#include "input_state.h"

/* A private raw PDO, never a keyboard-class filter or global control device. */
static const GUID InputInterface = {0x8ab347da,0x4fcb,0x47db,{0x96,0x55,0x68,0x22,0x45,0x7a,0xd5,0x73}};
static const GUID InputClass = {0xb7b2fc85,0xba63,0x4e8c,{0x9e,0x2f,0x4a,0x0b,0x81,0x24,0xd0,0xe4}};
typedef struct FILTER_CONTEXT {
    WDFSPINLOCK Lock;
    WDFQUEUE Pending;
    WDFTIMER Timer;
    SAYALL_INPUT_STATE State;
    ULONGLONG Renewed;
    BOOLEAN Contract;
    BOOLEAN Online;
    WDFFILEOBJECT Owner;
    WDFREQUEST ReadOwner;
    ULONG OwnerSession;
    WDFFILEOBJECT Maintenance;
} FILTER_CONTEXT;
WDF_DECLARE_CONTEXT_TYPE_WITH_NAME(FILTER_CONTEXT, FilterContext);
typedef struct PDO_CONTEXT { WDFDEVICE Parent; } PDO_CONTEXT;
WDF_DECLARE_CONTEXT_TYPE_WITH_NAME(PDO_CONTEXT, PdoContext);
typedef struct FILE_CONTEXT { BOOLEAN Closing; } FILE_CONTEXT;
WDF_DECLARE_CONTEXT_TYPE_WITH_NAME(FILE_CONTEXT, FileContext);

DRIVER_INITIALIZE DriverEntry;
EVT_WDF_DRIVER_DEVICE_ADD AddDevice;
EVT_WDF_IO_QUEUE_IO_READ ReadReport;
EVT_WDF_REQUEST_COMPLETION_ROUTINE ReportComplete;
EVT_WDF_IO_QUEUE_IO_DEVICE_CONTROL Control;
EVT_WDF_FILE_CLEANUP FileCleanup;
EVT_WDF_TIMER LeaseExpired;
EVT_WDF_DEVICE_SELF_MANAGED_IO_INIT VerifyContract;
EVT_WDF_DEVICE_D0_EXIT PowerDown;
EVT_WDF_DEVICE_D0_ENTRY PowerUp;
EVT_WDF_IO_QUEUE_IO_CANCELED_ON_QUEUE ReadCancelled;

static VOID CompleteRead(FILTER_CONTEXT *c,WDFREQUEST request,NTSTATUS status,ULONG_PTR bytes) {
    WdfSpinLockAcquire(c->Lock);
    if(c->ReadOwner==request) c->ReadOwner=NULL;
    WdfSpinLockRelease(c->Lock);
    WdfRequestCompleteWithInformation(request,status,bytes);
}
VOID ReadCancelled(WDFQUEUE queue,WDFREQUEST request) {
    FILTER_CONTEXT *c=FilterContext(PdoContext(WdfIoQueueGetDevice(queue))->Parent);
    CompleteRead(c,request,STATUS_CANCELLED,0);
}

static VOID Drain(FILTER_CONTEXT *c) {
    WDFREQUEST request;
    SAYALL_INPUT_EVENT event, *out;
    NTSTATUS status;
    if(!c->Pending) return;
    for (;;) {
        WdfSpinLockAcquire(c->Lock);
        if (!c->State.count || !NT_SUCCESS(WdfIoQueueRetrieveNextRequest(c->Pending,&request))) {
            WdfSpinLockRelease(c->Lock); return;
        }
        if(WdfRequestGetFileObject(request)!=c->Owner) {
            WdfSpinLockRelease(c->Lock);
            CompleteRead(c,request,STATUS_CANCELLED,0);
            continue;
        }
        SayAllInputTake(&c->State,&event);
        WdfSpinLockRelease(c->Lock);
        status=WdfRequestRetrieveOutputBuffer(request,sizeof(event),(PVOID*)&out,NULL);
        if (NT_SUCCESS(status)) { *out=event; CompleteRead(c,request,status,sizeof(event)); }
        else CompleteRead(c,request,status,0);
    }
}

VOID FileCleanup(WDFFILEOBJECT file) {
    FILTER_CONTEXT *c=FilterContext(PdoContext(WdfFileObjectGetDevice(file))->Parent);
    WdfSpinLockAcquire(c->Lock);
    FileContext(file)->Closing=TRUE;
    if(c->Maintenance==file) c->Maintenance=NULL;
    if(c->Owner==file) { c->Owner=NULL; SayAllInputCancel(&c->State,SAYALL_CANCEL_CLEANUP); }
    WdfSpinLockRelease(c->Lock);
    Drain(c); /* Framework cancels requests belonging to the closed file. */
}

VOID LeaseExpired(WDFTIMER timer) {
    FILTER_CONTEXT *c=FilterContext(PdoContext((WDFDEVICE)WdfTimerGetParentObject(timer))->Parent);
    WdfSpinLockAcquire(c->Lock);
    if(c->State.claimed && KeQueryInterruptTime()-c->Renewed > (ULONGLONG)SAYALL_INPUT_LEASE_MS*10000) {
        c->Owner=NULL;
        SayAllInputCancel(&c->State,SAYALL_CANCEL_LEASE);
    }
    WdfSpinLockRelease(c->Lock); Drain(c);
}

VOID Control(WDFQUEUE queue,WDFREQUEST request,size_t outputLength,size_t inputLength,ULONG code) {
    FILTER_CONTEXT *c=FilterContext(PdoContext(WdfIoQueueGetDevice(queue))->Parent);
    SAYALL_INPUT_STATUS value,*out;
    NTSTATUS status=STATUS_SUCCESS;
    WDFFILEOBJECT file=WdfRequestGetFileObject(request);
    uint32_t desired=0,*input;
    ULONG session=0;
    if(!file || !NT_SUCCESS(IoGetRequestorSessionId(WdfRequestWdmGetIrp(request),&session))) {
        WdfRequestComplete(request,STATUS_ACCESS_DENIED);return;
    }
    if(code==SAYALL_IOCTL_MAINTENANCE) {
        PEPROCESS process=IoGetRequestorProcess(WdfRequestWdmGetIrp(request));
        PACCESS_TOKEN token;
        BOOLEAN administrator;
        if(!process) {WdfRequestComplete(request,STATUS_ACCESS_DENIED);return;}
        token=PsReferencePrimaryToken(process); administrator=SeTokenIsAdmin(token); PsDereferencePrimaryToken(token);
        if(!administrator) {WdfRequestComplete(request,STATUS_ACCESS_DENIED);return;}
    }
    if(code==SAYALL_IOCTL_CLAIM) {
        if(inputLength!=sizeof(desired) || !NT_SUCCESS(WdfRequestRetrieveInputBuffer(request,sizeof(desired),(PVOID*)&input,NULL)) || *input==0 || (*input&~7u)!=0) {
            WdfRequestComplete(request,STATUS_INVALID_PARAMETER); return;
        }
        desired=*input;
    } else if(inputLength!=0) { WdfRequestComplete(request,STATUS_INVALID_PARAMETER); return; }
    if(code==SAYALL_IOCTL_READ) {
        if(outputLength!=sizeof(SAYALL_INPUT_EVENT)) { WdfRequestComplete(request,STATUS_INVALID_BUFFER_SIZE); return; }
        WdfSpinLockAcquire(c->Lock);
        if(FileContext(file)->Closing || c->Owner!=file || c->OwnerSession!=session) status=STATUS_ACCESS_DENIED;
        else if(c->ReadOwner) status=STATUS_DEVICE_BUSY;
        else {c->ReadOwner=request;c->Renewed=KeQueryInterruptTime();}
        WdfSpinLockRelease(c->Lock);
        if(!NT_SUCCESS(status)) {WdfRequestComplete(request,status);return;}
        status=WdfRequestForwardToIoQueue(request,c->Pending);
        if(!NT_SUCCESS(status)) CompleteRead(c,request,status,0); else Drain(c);
        return;
    }
    if(code!=SAYALL_IOCTL_QUERY && code!=SAYALL_IOCTL_CLAIM && code!=SAYALL_IOCTL_RELEASE && code!=SAYALL_IOCTL_MAINTENANCE) {
        WdfRequestComplete(request,STATUS_INVALID_DEVICE_REQUEST); return;
    }
    if(outputLength!=sizeof(value)) {WdfRequestComplete(request,STATUS_INVALID_BUFFER_SIZE);return;}
    status=WdfRequestRetrieveOutputBuffer(request,sizeof(value),(PVOID*)&out,NULL);
    if(!NT_SUCCESS(status)) { WdfRequestComplete(request,status); return; }
    WdfSpinLockAcquire(c->Lock);
    if(FileContext(file)->Closing) status=STATUS_FILE_CLOSED;
    else if(code==SAYALL_IOCTL_CLAIM) {
        if(!c->Contract || !c->Online) status=STATUS_DEVICE_NOT_READY;
        else if(c->Maintenance) status=STATUS_SHARING_VIOLATION;
        else if((c->Owner && (c->Owner!=file || c->OwnerSession!=session)) || c->ReadOwner) status=STATUS_SHARING_VIOLATION;
        else if(!c->State.claimed) { c->Owner=file;c->OwnerSession=session; c->Renewed=KeQueryInterruptTime(); SayAllInputClaim(&c->State,desired); }
        else if(c->State.enabled!=desired) status=STATUS_INVALID_DEVICE_STATE;
    } else if(code==SAYALL_IOCTL_MAINTENANCE) {
        /* Atomic quiescence reservation: share modes on raw devices alone do
         * not establish this state. QUERY remains passive and cannot reserve. */
        if(c->Owner || c->ReadOwner || (c->Maintenance && c->Maintenance!=file)) status=STATUS_SHARING_VIOLATION;
        else if(!c->Contract || !c->Online || !c->State.seen || c->State.physical || c->State.swallowed) status=STATUS_DEVICE_NOT_READY;
        else c->Maintenance=file;
    } else if(code==SAYALL_IOCTL_RELEASE) {
        if(c->Owner!=file || c->OwnerSession!=session) status=STATUS_ACCESS_DENIED;
        else { SayAllInputCancel(&c->State,SAYALL_CANCEL_RELEASE); c->Owner=NULL; }
    }
    SayAllInputStatus(&c->State,&value);
    value.report_contract=c->Contract?1:0;
    WdfSpinLockRelease(c->Lock);
    if(NT_SUCCESS(status)) { *out=value; WdfRequestCompleteWithInformation(request,status,sizeof(value)); }
    else WdfRequestComplete(request,status);
    Drain(c);
}

/* Compare the OS parser's generated array against the precise layout used by
 * the filter. Shared VID/PID, length, or declared usage ranges alone are not
 * sufficient evidence. This sends no HID report to the physical device. */
NTSTATUS VerifyContract(WDFDEVICE device) {
    FILTER_CONTEXT *c=FilterContext(device);
    HID_COLLECTION_INFORMATION info={0};
    WDF_MEMORY_DESCRIPTOR output;
    WDF_REQUEST_SEND_OPTIONS options;
    PHIDP_PREPARSED_DATA preparsed=NULL;
    HIDP_CAPS caps={0};
    NTSTATUS status;
    UCHAR report[121];
    USAGE usages[3]={0xF1,0x80,0x81};
    ULONG count=3;
    size_t i;
    WDF_REQUEST_SEND_OPTIONS_INIT(&options,WDF_REQUEST_SEND_OPTION_TIMEOUT);
    WDF_REQUEST_SEND_OPTIONS_SET_TIMEOUT(&options,WDF_REL_TIMEOUT_IN_SEC(2));
    WDF_MEMORY_DESCRIPTOR_INIT_BUFFER(&output,&info,sizeof(info));
    status=WdfIoTargetSendIoctlSynchronously(WdfDeviceGetIoTarget(device),NULL,
        IOCTL_HID_GET_COLLECTION_INFORMATION,NULL,&output,&options,NULL);
    if(!NT_SUCCESS(status) || info.DescriptorSize==0 || info.DescriptorSize>65536) return STATUS_SUCCESS;
    preparsed=ExAllocatePoolZero(NonPagedPoolNx,info.DescriptorSize,'IaYS');
    if(!preparsed) return STATUS_SUCCESS;
    WDF_MEMORY_DESCRIPTOR_INIT_BUFFER(&output,preparsed,info.DescriptorSize);
    status=WdfIoTargetSendIoctlSynchronously(WdfDeviceGetIoTarget(device),NULL,
        IOCTL_HID_GET_COLLECTION_DESCRIPTOR,NULL,&output,&options,NULL);
    if(NT_SUCCESS(status)) status=HidP_GetCaps(preparsed,&caps);
    if(status==HIDP_STATUS_SUCCESS && caps.UsagePage==1 && caps.Usage==6 && caps.InputReportByteLength==sizeof(report)) {
        RtlZeroMemory(report,sizeof(report));
        status=HidP_InitializeReportForID(HidP_Input,1,preparsed,(PCHAR)report,sizeof(report));
        if(status==HIDP_STATUS_SUCCESS) status=HidP_SetUsages(HidP_Input,7,0,usages,&count,preparsed,(PCHAR)report,sizeof(report));
        if(status==HIDP_STATUS_SUCCESS && count==3 && report[0]==1) {
            BOOLEAN valid=TRUE;
            for(i=0;i<3;++i) if(report[1+2*i]!=(UCHAR)usages[i] || report[2+2*i]!=0) valid=FALSE;
            for(i=7;i<sizeof(report);++i) if(report[i]!=0) valid=FALSE;
            WdfSpinLockAcquire(c->Lock); c->Contract=valid; WdfSpinLockRelease(c->Lock);
        }
    }
    ExFreePoolWithTag(preparsed,'IaYS');
    return STATUS_SUCCESS; /* Optional enhancement never prevents basic HID. */
}

NTSTATUS PowerUp(WDFDEVICE device,WDF_POWER_DEVICE_STATE previous) {
    FILTER_CONTEXT *c=FilterContext(device); UNREFERENCED_PARAMETER(previous);
    WdfSpinLockAcquire(c->Lock); c->Online=TRUE; WdfSpinLockRelease(c->Lock);
    return STATUS_SUCCESS;
}
NTSTATUS PowerDown(WDFDEVICE device,WDF_POWER_DEVICE_STATE target) {
    FILTER_CONTEXT *c=FilterContext(device); UNREFERENCED_PARAMETER(target);
    WdfSpinLockAcquire(c->Lock); c->Online=FALSE;c->Owner=NULL; c->State.seen=0; SayAllInputCancel(&c->State,SAYALL_CANCEL_POWER); WdfSpinLockRelease(c->Lock);
    Drain(c); return STATUS_SUCCESS;
}

VOID ReadReport(WDFQUEUE queue,WDFREQUEST request,size_t length) {
    WDFDEVICE device=WdfIoQueueGetDevice(queue); UNREFERENCED_PARAMETER(length);
    WdfRequestFormatRequestUsingCurrentType(request);
    WdfRequestSetCompletionRoutine(request,ReportComplete,device);
    if(!WdfRequestSend(request,WdfDeviceGetIoTarget(device),WDF_NO_SEND_OPTIONS))
        WdfRequestComplete(request,WdfRequestGetStatus(request));
}
VOID ReportComplete(WDFREQUEST request,WDFIOTARGET target,PWDF_REQUEST_COMPLETION_PARAMS params,WDFCONTEXT context) {
    FILTER_CONTEXT *c=FilterContext((WDFDEVICE)context);
    PVOID buffer; size_t capacity; ULONG_PTR length=params->IoStatus.Information;
    UNREFERENCED_PARAMETER(target);
    if(NT_SUCCESS(params->IoStatus.Status) && length && NT_SUCCESS(WdfRequestRetrieveOutputBuffer(request,1,&buffer,&capacity)) && length<=capacity) {
        WdfSpinLockAcquire(c->Lock);
        if(c->Contract && c->Online) SayAllInputReport(&c->State,(unsigned char*)buffer,(size_t)length);
        WdfSpinLockRelease(c->Lock);
        Drain(c);
    }
    WdfRequestCompleteWithInformation(request,params->IoStatus.Status,length);
}

static NTSTATUS CreateChannel(WDFDEVICE parent) {
    PWDFDEVICE_INIT init=WdfPdoInitAllocate(parent);
    WDFDEVICE child=NULL;
    WDF_OBJECT_ATTRIBUTES attributes;
    WDF_IO_QUEUE_CONFIG queue;
    WDF_FILEOBJECT_CONFIG files;
    WDF_DEVICE_PNP_CAPABILITIES caps;
    WDF_TIMER_CONFIG timer;
    NTSTATUS status;
    DECLARE_CONST_UNICODE_STRING(id,L"SayAll\\RemoteInput");
    DECLARE_CONST_UNICODE_STRING(instance,L"Input");
    /* Only interactive logons; network/service logons get no user ACE.
     * Query is read-only. Only explicit WRITE-access CLAIM captures keys. */
    DECLARE_CONST_UNICODE_STRING(sddl,L"D:P(D;;GA;;;S-1-5-14)(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)");
    if(!init) return STATUS_INSUFFICIENT_RESOURCES;
    status=WdfPdoInitAssignRawDevice(init,&InputClass); if(!NT_SUCCESS(status)) goto fail;
    status=WdfDeviceInitAssignSDDLString(init,&sddl); if(!NT_SUCCESS(status)) goto fail;
    status=WdfPdoInitAssignDeviceID(init,&id); if(!NT_SUCCESS(status)) goto fail;
    status=WdfPdoInitAssignInstanceID(init,&instance); if(!NT_SUCCESS(status)) goto fail;
    /* Parent-scoped instance identity avoids exposing a hardware identifier. */
    WDF_DEVICE_PNP_CAPABILITIES_INIT(&caps); caps.UniqueID=WdfFalse; caps.Removable=WdfTrue; caps.SurpriseRemovalOK=WdfTrue;
    WDF_FILEOBJECT_CONFIG_INIT(&files,WDF_NO_EVENT_CALLBACK,WDF_NO_EVENT_CALLBACK,FileCleanup);
    WDF_OBJECT_ATTRIBUTES_INIT_CONTEXT_TYPE(&attributes,FILE_CONTEXT);
    WdfDeviceInitSetFileObjectConfig(init,&files,&attributes);
    WDF_OBJECT_ATTRIBUTES_INIT_CONTEXT_TYPE(&attributes,PDO_CONTEXT);
    status=WdfDeviceCreate(&init,&attributes,&child); if(!NT_SUCCESS(status)) goto fail;
    PdoContext(child)->Parent=parent;
    WdfDeviceSetPnpCapabilities(child,&caps);
    WDF_IO_QUEUE_CONFIG_INIT_DEFAULT_QUEUE(&queue,WdfIoQueueDispatchParallel); queue.PowerManaged=WdfFalse; queue.EvtIoDeviceControl=Control;
    /* IoGetRequestorSessionId requires <= APC_LEVEL. Limit this sideband
     * queue only; HID completion and its spin-lock state stay nonpaged. */
    WDF_OBJECT_ATTRIBUTES_INIT(&attributes); attributes.ExecutionLevel=WdfExecutionLevelPassive;
    status=WdfIoQueueCreate(child,&queue,&attributes,NULL); if(!NT_SUCCESS(status)) goto fail;
    WDF_IO_QUEUE_CONFIG_INIT(&queue,WdfIoQueueDispatchManual); queue.PowerManaged=WdfFalse;
    queue.EvtIoCanceledOnQueue=ReadCancelled;
    status=WdfIoQueueCreate(child,&queue,WDF_NO_OBJECT_ATTRIBUTES,&FilterContext(parent)->Pending); if(!NT_SUCCESS(status)) goto fail;
    WDF_OBJECT_ATTRIBUTES_INIT(&attributes); attributes.ParentObject=child;
    WDF_TIMER_CONFIG_INIT_PERIODIC(&timer,LeaseExpired,250);
    status=WdfTimerCreate(&timer,&attributes,&FilterContext(parent)->Timer); if(!NT_SUCCESS(status)) goto fail;
    status=WdfDeviceCreateDeviceInterface(child,&InputInterface,NULL); if(!NT_SUCCESS(status)) goto fail;
    status=WdfFdoAddStaticChild(parent,child); if(!NT_SUCCESS(status)) goto fail;
    WdfTimerStart(FilterContext(parent)->Timer,WDF_REL_TIMEOUT_IN_MS(250));
    return STATUS_SUCCESS;
fail:
    if(init) WdfDeviceInitFree(init);
    if(child) WdfObjectDelete(child);
    return status;
}

NTSTATUS AddDevice(WDFDRIVER driver,PWDFDEVICE_INIT init) {
    WDFDEVICE device;
    FILTER_CONTEXT *c;
    WDF_OBJECT_ATTRIBUTES attributes;
    WDF_IO_QUEUE_CONFIG queue;
    WDF_PNPPOWER_EVENT_CALLBACKS power;
    NTSTATUS status;
    UNREFERENCED_PARAMETER(driver);
    WdfFdoInitSetFilter(init);
    WDF_PNPPOWER_EVENT_CALLBACKS_INIT(&power);
    power.EvtDeviceSelfManagedIoInit=VerifyContract; power.EvtDeviceD0Entry=PowerUp; power.EvtDeviceD0Exit=PowerDown;
    WdfDeviceInitSetPnpPowerEventCallbacks(init,&power);
    WDF_OBJECT_ATTRIBUTES_INIT_CONTEXT_TYPE(&attributes,FILTER_CONTEXT);
    status=WdfDeviceCreate(&init,&attributes,&device); if(!NT_SUCCESS(status)) return status;
    c=FilterContext(device);
    WDF_OBJECT_ATTRIBUTES_INIT(&attributes); attributes.ParentObject=device;
    status=WdfSpinLockCreate(&attributes,&c->Lock); if(!NT_SUCCESS(status)) return status;
    WDF_IO_QUEUE_CONFIG_INIT_DEFAULT_QUEUE(&queue,WdfIoQueueDispatchParallel); queue.EvtIoRead=ReadReport;
    status=WdfIoQueueCreate(device,&queue,WDF_NO_OBJECT_ATTRIBUTES,NULL); if(!NT_SUCCESS(status)) return status;
    status=CreateChannel(device); if(!NT_SUCCESS(status)) return status;
    return STATUS_SUCCESS;
}
NTSTATUS DriverEntry(PDRIVER_OBJECT object,PUNICODE_STRING path) {
    ExInitializeDriverRuntime(DrvRtPoolNxOptIn);
    WDF_DRIVER_CONFIG config; WDF_DRIVER_CONFIG_INIT(&config,AddDevice);
    return WdfDriverCreate(object,path,WDF_NO_OBJECT_ATTRIBUTES,&config,WDF_NO_HANDLE);
}
