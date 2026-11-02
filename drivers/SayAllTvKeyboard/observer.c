#include "observer.h"

int TvObserverConfigure(TV_OBSERVER *observer, unsigned int makeCode, unsigned int flags)
{
    observer->Configured = 0;
    if (makeCode == 0 || makeCode > 0xffff || makeCode == 0xff ||
        (flags != 0 && flags != 2 && flags != 4)) {
        return 0;
    }
    observer->MakeCode = makeCode;
    observer->Flags = flags;
    observer->Configured = 1;
    return 1;
}

void TvObserverStart(TV_OBSERVER *observer)
{
    if (!observer->Active) {
        observer->Active = 1;
        ++observer->Generation;
    }
}

void TvObserverStop(TV_OBSERVER *observer)
{
    if (observer->Active) {
        observer->Active = 0;
        TvObserverReset(observer);
    }
}

void TvObserverReset(TV_OBSERVER *observer)
{
    ++observer->Generation;
    if (observer->Held) {
        observer->Held = 0;
        ++observer->CancelledHold;
    }
}

void TvObserverPacket(TV_OBSERVER *observer, unsigned int makeCode, unsigned int flags)
{
    if (!observer->Active || !observer->Configured ||
        makeCode != observer->MakeCode || (flags & ~1u) != observer->Flags) {
        return;
    }
    if ((flags & 1u) != 0) {
        if (observer->Held) {
            observer->Held = 0;
            ++observer->Up;
        } else {
            ++observer->OrphanUp;
        }
    } else if (observer->Held) {
        ++observer->Repeat;
    } else {
        observer->Held = 1;
        ++observer->Down;
    }
}
