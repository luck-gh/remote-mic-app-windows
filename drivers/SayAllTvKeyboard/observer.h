#ifndef SAYALL_TV_OBSERVER_H
#define SAYALL_TV_OBSERVER_H

typedef struct TV_OBSERVER {
    unsigned int MakeCode;
    unsigned int Flags;
    unsigned int Configured;
    unsigned int Active;
    unsigned int Held;
    unsigned int Generation;
    unsigned long long Down;
    unsigned long long Up;
    unsigned long long Repeat;
    unsigned long long OrphanUp;
    unsigned long long CancelledHold;
} TV_OBSERVER;

int TvObserverConfigure(TV_OBSERVER *observer, unsigned int makeCode, unsigned int flags);
void TvObserverStart(TV_OBSERVER *observer);
void TvObserverStop(TV_OBSERVER *observer);
void TvObserverReset(TV_OBSERVER *observer);
void TvObserverPacket(TV_OBSERVER *observer, unsigned int makeCode, unsigned int flags);

#endif
