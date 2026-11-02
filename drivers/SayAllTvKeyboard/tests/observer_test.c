#include <assert.h>
#include <stdio.h>
#include "../observer.h"

static void test_profile_and_edges(void)
{
    TV_OBSERVER remote = {0};
    TV_OBSERVER keyboard = {0};
    TvObserverStart(&remote);
    TvObserverStart(&keyboard);
    TvObserverPacket(&remote, 0x29, 0);
    assert(remote.Down == 0);
    assert(TvObserverConfigure(&remote, 0x29, 0));
    TvObserverPacket(&remote, 0x29, 0);
    TvObserverPacket(&remote, 0x29, 0);
    TvObserverPacket(&remote, 0x29, 1);
    TvObserverPacket(&remote, 0x29, 1);
    TvObserverPacket(&keyboard, 0x29, 0);
    TvObserverPacket(&keyboard, 0x2a, 0);
    TvObserverPacket(&keyboard, 0x29, 1);
    assert(remote.Down == 1 && remote.Up == 1);
    assert(remote.Repeat == 1 && remote.OrphanUp == 1);
    assert(keyboard.Down == 0 && keyboard.Up == 0);
    TvObserverPacket(&remote, 0x2a, 0);
    TvObserverPacket(&remote, 0x29, 2);
    TvObserverPacket(&remote, 0x29, 8);
    assert(remote.Down == 1 && !remote.Held);
    assert(!TvObserverConfigure(&remote, 0xff, 0));
    assert(!remote.Configured);
    assert(!TvObserverConfigure(&remote, 0x29, 6));
    assert(!TvObserverConfigure(&remote, 0x10000, 0));
    assert(!TvObserverConfigure(&remote, 0, 0));
    assert(!TvObserverConfigure(&remote, 0x29, 1));
}

static void test_lifecycle(void)
{
    TV_OBSERVER observer = {0};
    unsigned int iteration;
    assert(TvObserverConfigure(&observer, 0x29, 0));
    TvObserverPacket(&observer, 0x29, 0);
    assert(observer.Down == 0);
    for (iteration = 0; iteration < 4; ++iteration) {
        TvObserverStart(&observer);
        TvObserverPacket(&observer, 0x29, 0);
        TvObserverStop(&observer);
        TvObserverStop(&observer);
        assert(!observer.Held && !observer.Active);
        TvObserverPacket(&observer, 0x29, 1);
    }
    assert(observer.Down == 4 && observer.Up == 0);
    assert(observer.CancelledHold == 4 && observer.Generation == 8);
    TvObserverStart(&observer);
    TvObserverStart(&observer);
    TvObserverPacket(&observer, 0x29, 1);
    assert(observer.OrphanUp == 1 && observer.Generation == 9);
    assert(TvObserverConfigure(&observer, 0x29, 2));
    TvObserverPacket(&observer, 0x29, 0);
    TvObserverPacket(&observer, 0x29, 2);
    TvObserverPacket(&observer, 0x29, 3);
    assert(observer.Down == 5 && observer.Up == 1);
}

int main(void)
{
    test_profile_and_edges();
    test_lifecycle();
    puts("passed: observer profile/edges/device isolation/flags/lifecycle (synthetic scan 0x29)");
    return 0;
}
