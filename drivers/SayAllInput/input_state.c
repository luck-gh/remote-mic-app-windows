/* SPDX-License-Identifier: GPL-3.0-only */
#include "input_state.h"

static uint32_t button(uint16_t usage) {
    switch (usage) {
    case 0xF1: return SAYALL_INPUT_BACK;
    case 0x80: return SAYALL_INPUT_VOLUME_UP;
    case 0x81: return SAYALL_INPUT_VOLUME_DOWN;
    default: return 0;
    }
}

static void append(SAYALL_INPUT_STATE *s, uint32_t mask, uint32_t flags, uint32_t reason) {
    unsigned int index = (s->head + s->count) % SAYALL_INPUT_CAPACITY;
    s->events[index].abi = SAYALL_INPUT_ABI;
    s->events[index].sequence = ++s->sequence;
    s->events[index].buttons = mask;
    s->events[index].flags = flags;
    s->events[index].reason = reason;
    s->events[index].reserved = 0;
    ++s->count;
}

void SayAllInputCancel(SAYALL_INPUT_STATE *s, uint32_t reason) {
    /* Keep swallowed until those physical keys release: a client death must
     * not turn the remainder of its hold into a fresh native DOWN. */
    s->claimed = 0;
    s->waiting = 1;
    s->head = s->count = 0;
    ++s->cancellations;
    append(s, 0, SAYALL_INPUT_CANCEL, reason);
}

void SayAllInputClaim(SAYALL_INPUT_STATE *s, uint32_t buttons) {
    s->enabled = buttons & 7u;
    s->claimed = 1;
    s->overflow = 0;
    s->waiting = !s->seen || s->physical != 0 || s->swallowed != 0;
    s->head = s->count = 0;
    append(s, 0, s->waiting ? SAYALL_INPUT_CANCEL : SAYALL_INPUT_READY, s->waiting ? SAYALL_WAIT_RELEASE : 0);
}

unsigned int SayAllInputReport(SAYALL_INPUT_STATE *s, unsigned char *r, size_t n) {
    size_t i;
    uint32_t mask = 0, previous = s->physical;
    unsigned int changed = 0;
    if (!r || n == 0 || r[0] != 1) return 0; /* Voice/vendor reports never touched. */
    if (n != 121) {
        s->seen = 0;
        ++s->rejected;
        if (s->claimed) SayAllInputCancel(s, SAYALL_CANCEL_REPORT);
        return 0;
    }
    /* Reject rollover/error packets and nonzero padding as unknown format. */
    for (i = 7; i < n; ++i) {
        if (r[i] != 0) {
            s->seen = 0;
            ++s->rejected;
            if (s->claimed) SayAllInputCancel(s, SAYALL_CANCEL_REPORT);
            return 0;
        }
    }
    for(i=1;i<7;i+=2) {
        uint16_t usage=(uint16_t)(r[i]|((uint16_t)r[i+1]<<8));
        if(usage>254 || (usage>=1 && usage<=3)) {
            s->seen = 0;
            ++s->rejected;
            if(s->claimed) SayAllInputCancel(s,SAYALL_CANCEL_REPORT);
            return 0;
        }
        mask|=button(usage);
    }
    ++s->reports;
    s->seen = 1;
    s->physical = mask;
    s->swallowed &= mask;
    if (s->waiting && mask == 0) {
        s->waiting = 0;
        previous = mask; /* The pre-claim hold has no owned release edge. */
        if (s->claimed && s->count < SAYALL_INPUT_CAPACITY)
            append(s, 0, SAYALL_INPUT_READY, 0);
    }
    if (s->claimed && !s->waiting && previous != mask) {
        if (s->count == SAYALL_INPUT_CAPACITY) {
            s->overflow = 1;
            SayAllInputCancel(s, SAYALL_CANCEL_OVERFLOW);
        } else {
            s->swallowed |= mask & s->enabled;
            append(s, mask & s->enabled, SAYALL_INPUT_READY, 0);
        }
    }
    for (i = 1; i < 7; i+=2) {
        if ((button((uint16_t)(r[i]|((uint16_t)r[i+1]<<8))) & s->swallowed) != 0) {
            r[i] = r[i+1] = 0;
            changed = 1;
        }
    }
    return changed;
}

unsigned int SayAllInputTake(SAYALL_INPUT_STATE *s, SAYALL_INPUT_EVENT *event) {
    if (!s->count) return 0;
    *event = s->events[s->head];
    s->head = (s->head + 1) % SAYALL_INPUT_CAPACITY;
    --s->count;
    return 1;
}

void SayAllInputStatus(const SAYALL_INPUT_STATE *s, SAYALL_INPUT_STATUS *o) {
    o->abi = SAYALL_INPUT_ABI;
    o->report_contract = 1;
    o->active = s->claimed;
    o->ready = s->claimed && !s->waiting;
    o->sequence = s->sequence;
    o->cancellations = s->cancellations;
    o->report_count = s->reports;
    o->rejected_reports = s->rejected;
    o->physical_buttons = s->physical;
    o->swallowed_buttons = s->swallowed;
    o->observed_report = s->seen;
}
