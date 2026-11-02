#include "driver.h"
#include <TraceLoggingProvider.h>
#include "build_identity.h"

TRACELOGGING_DEFINE_PROVIDER(TvKeyboardProvider, "WirelessMic.TvKeyboard",
    (0x20b52f98, 0x976a, 0x4d84, 0x9a, 0xc6, 0x55, 0xd9, 0xaf, 0x7c, 0x70, 0xc2));

long TvTraceRegister(void)
{
    return TraceLoggingRegister(TvKeyboardProvider);
}

void TvTraceUnregister(void)
{
    TraceLoggingUnregister(TvKeyboardProvider);
}

void TvTraceWrite(const TV_TRACE_SNAPSHOT *snapshot)
{
    TraceLoggingWrite(TvKeyboardProvider, "Observer",
        TraceLoggingLevel(4),
        TraceLoggingUInt32(1, "schema_version"),
        TraceLoggingString(TV_SOURCE_FINGERPRINT, "source_fingerprint"),
        TraceLoggingUInt32(snapshot->DeviceOrdinal, "device_ordinal"),
        TraceLoggingUInt64(snapshot->Sequence, "sequence"),
        TraceLoggingUInt32(snapshot->Stage, "stage"),
        TraceLoggingInt32(snapshot->Status, "status"),
        TraceLoggingUInt32(snapshot->Configured, "configured"),
        TraceLoggingUInt32(snapshot->Active, "active"),
        TraceLoggingUInt32(snapshot->Removed, "removed"),
        TraceLoggingUInt32(snapshot->Held, "held"),
        TraceLoggingUInt32(snapshot->Generation, "generation"),
        TraceLoggingUInt64(snapshot->Down, "down"),
        TraceLoggingUInt64(snapshot->Up, "up"),
        TraceLoggingUInt64(snapshot->Repeat, "repeat"),
        TraceLoggingUInt64(snapshot->OrphanUp, "orphan_up"),
        TraceLoggingUInt64(snapshot->CancelledHold, "cancelled_hold"),
        TraceLoggingUInt64(snapshot->Offered, "offered"),
        TraceLoggingUInt64(snapshot->Consumed, "consumed"),
        TraceLoggingUInt64(snapshot->Partial, "partial"),
        TraceLoggingUInt64(snapshot->ZeroConsumed, "zero_consumed"),
        TraceLoggingUInt64(snapshot->InvalidConsumed, "invalid_consumed"),
        TraceLoggingUInt64(snapshot->Skipped, "skipped"),
        TraceLoggingUInt64(snapshot->Overlapped, "overlapped"));
}
