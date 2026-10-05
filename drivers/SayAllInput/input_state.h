/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stddef.h>
#include "protocol.h"

typedef struct SAYALL_INPUT_STATE {
    uint32_t physical, swallowed, enabled, sequence, cancellations, reports, rejected;
    unsigned int seen, claimed, waiting, overflow, head, count;
    SAYALL_INPUT_EVENT events[SAYALL_INPUT_CAPACITY];
} SAYALL_INPUT_STATE;

void SayAllInputClaim(SAYALL_INPUT_STATE *state, uint32_t buttons);
void SayAllInputCancel(SAYALL_INPUT_STATE *state, uint32_t reason);
/* contract=1 is only the verified ID1 three 16-bit usages at 1/3/5, padded to 121 bytes.
 * Transport/descriptor verification belongs to the driver before this call. */
unsigned int SayAllInputReport(SAYALL_INPUT_STATE *state, unsigned char *report, size_t length);
unsigned int SayAllInputTake(SAYALL_INPUT_STATE *state, SAYALL_INPUT_EVENT *event);
void SayAllInputStatus(const SAYALL_INPUT_STATE *state, SAYALL_INPUT_STATUS *status);
