/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>

#define SAYALL_INPUT_ABI 3u
#define SAYALL_INPUT_CAPACITY 64u
#define SAYALL_INPUT_BACK 1u
#define SAYALL_INPUT_VOLUME_UP 2u
#define SAYALL_INPUT_VOLUME_DOWN 4u
#define SAYALL_INPUT_CANCEL 1u
#define SAYALL_INPUT_READY 2u
#define SAYALL_INPUT_LEASE_MS 2000u
#define SAYALL_CANCEL_RELEASE 1u
#define SAYALL_CANCEL_CLEANUP 2u
#define SAYALL_CANCEL_LEASE 3u
#define SAYALL_CANCEL_OVERFLOW 4u
#define SAYALL_CANCEL_REPORT 5u
#define SAYALL_CANCEL_POWER 6u
#define SAYALL_WAIT_RELEASE 7u
/* FILE_DEVICE_UNKNOWN, METHOD_BUFFERED; query needs READ, claim needs WRITE. */
#define SAYALL_IOCTL_QUERY 0x00226000u
#define SAYALL_IOCTL_CLAIM 0x0022A004u
#define SAYALL_IOCTL_READ  0x00226008u
#define SAYALL_IOCTL_RELEASE 0x0022A00Cu
#define SAYALL_IOCTL_MAINTENANCE 0x0022A010u

typedef struct SAYALL_INPUT_EVENT {
    uint32_t abi;
    uint32_t sequence;
    uint32_t buttons;
    uint32_t flags;
    uint32_t reason;
    uint32_t reserved;
} SAYALL_INPUT_EVENT;

typedef struct SAYALL_INPUT_STATUS {
    uint32_t abi;
    uint32_t report_contract;
    uint32_t active;
    uint32_t ready;
    uint32_t sequence;
    uint32_t cancellations;
    uint32_t report_count;
    uint32_t rejected_reports;
    uint32_t physical_buttons;
    uint32_t swallowed_buttons;
    uint32_t observed_report;
} SAYALL_INPUT_STATUS;
