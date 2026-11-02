#ifndef SAYALL_TV_KEYBOARD_DRIVER_H
#define SAYALL_TV_KEYBOARD_DRIVER_H

#ifdef SAYALL_TV_OFFLINE_TEST
#include "tests/platform.h"
#else
#include <ntddk.h>
#pragma warning(push)
#pragma warning(disable:4324)
#include <wdf.h>
#pragma warning(pop)
#include <ntddkbd.h>
#include <kbdmou.h>
#endif
#include "observer.h"
#include "trace.h"

typedef struct TV_DEVICE_CONTEXT {
    WDFSPINLOCK Lock;
    CONNECT_DATA Upper;
    PCONNECT_DATA PendingConnect;
    BOOLEAN Connected;
    BOOLEAN Removed;
    ULONG ActiveCallbacks;
    ULONG CallbackEpoch;
    TV_OBSERVER Observer;
    TV_TRACE_SNAPSHOT Counters;
} TV_DEVICE_CONTEXT;

WDF_DECLARE_CONTEXT_TYPE_WITH_NAME(TV_DEVICE_CONTEXT, TvDeviceContext);

DRIVER_INITIALIZE DriverEntry;
EVT_WDF_DRIVER_DEVICE_ADD TvDeviceAdd;
EVT_WDF_DRIVER_UNLOAD TvDriverUnload;
EVT_WDF_IO_QUEUE_IO_INTERNAL_DEVICE_CONTROL TvInternalControl;
EVT_WDF_REQUEST_COMPLETION_ROUTINE TvConnectComplete;
EVT_WDF_DEVICE_D0_ENTRY TvD0Entry;
EVT_WDF_DEVICE_D0_EXIT TvD0Exit;
EVT_WDF_DEVICE_SURPRISE_REMOVAL TvSurpriseRemoval;

VOID TvServiceCallback(PDEVICE_OBJECT deviceObject, PKEYBOARD_INPUT_DATA inputStart,
    PKEYBOARD_INPUT_DATA inputEnd, PULONG inputConsumed);

#endif
