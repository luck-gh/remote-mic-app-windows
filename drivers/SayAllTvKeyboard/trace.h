#ifndef SAYALL_TV_TRACE_H
#define SAYALL_TV_TRACE_H

typedef enum TV_TRACE_STAGE {
    TvProfileAbsent = 1,
    TvProfileAccepted,
    TvProfileRejected,
    TvConnectPending,
    TvConnectAccepted,
    TvConnectRejected,
    TvPowerStarted,
    TvPowerStopped,
    TvSurpriseRemoved,
    TvPacketsObserved,
    TvConsumptionRejected,
    TvRequestRejected,
    TvDeviceRejected,
    TvObservationSkipped
} TV_TRACE_STAGE;

typedef struct TV_TRACE_SNAPSHOT {
    unsigned int DeviceOrdinal;
    unsigned long long Sequence;
    unsigned int Stage;
    long Status;
    unsigned int Configured;
    unsigned int Active;
    unsigned int Removed;
    unsigned int Held;
    unsigned int Generation;
    unsigned long long Down;
    unsigned long long Up;
    unsigned long long Repeat;
    unsigned long long OrphanUp;
    unsigned long long CancelledHold;
    unsigned long long Offered;
    unsigned long long Consumed;
    unsigned long long Partial;
    unsigned long long ZeroConsumed;
    unsigned long long InvalidConsumed;
    unsigned long long Skipped;
    unsigned long long Overlapped;
} TV_TRACE_SNAPSHOT;

void TvTraceWrite(const TV_TRACE_SNAPSHOT *snapshot);
long TvTraceRegister(void);
void TvTraceUnregister(void);

#endif
