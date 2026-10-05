/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "../input_state.h"
static void report(SAYALL_INPUT_STATE *s, unsigned char a, unsigned char b, unsigned char c) {
    unsigned char r[121] = {1,0,0}; r[1]=a; r[3]=b; r[5]=c;
    SayAllInputReport(s,r,sizeof(r));
}
static SAYALL_INPUT_EVENT take(SAYALL_INPUT_STATE *s) {
    SAYALL_INPUT_EVENT e; assert(SayAllInputTake(s,&e)); return e;
}
int main(void) {
    SAYALL_INPUT_STATE s={0}; SAYALL_INPUT_EVENT e; unsigned int i;
    unsigned char r[121]={1,0,0};
    /* Passive query/inspection never captures; claim while held waits all-up. */
    r[3]=0xF1; assert(!SayAllInputReport(&s,r,sizeof(r))); assert(r[3]==0xF1);
    SayAllInputClaim(&s,7); assert(take(&s).flags==SAYALL_INPUT_CANCEL);
    report(&s,0xF1,0,0); assert(!SayAllInputTake(&s,&e));
    report(&s,0,0,0); assert(take(&s).flags==SAYALL_INPUT_READY);
    /* Three independent slots, duplicate reports, voice and ordinary keys. */
    r[1]=0xF1;r[3]=0x80;r[5]=0x81;
    assert(SayAllInputReport(&s,r,sizeof(r))); assert(!r[1]&&!r[3]&&!r[5]);
    assert(take(&s).buttons==7);
    report(&s,0xF1,0x80,0x81); assert(!SayAllInputTake(&s,&e));
    report(&s,0,0x80,0); assert(take(&s).buttons==2);
    report(&s,0,0,0); assert(take(&s).buttons==0);
    /* Fast down/up is retained separately, no latest-state coalescing. */
    for(i=0;i<10;i++){ report(&s,0xF1,0,0); report(&s,0,0,0); }
    for(i=0;i<20;i++) assert(take(&s).buttons==(i%2?0u:1u));
    /* Exit, lease expiry, disconnect and unload use the same cancellation. */
    for(i=0;i<4;i++) {
        report(&s,0x80,0,0); assert(take(&s).buttons==2); SayAllInputCancel(&s,i+1);
        e=take(&s);assert(e.flags==SAYALL_INPUT_CANCEL&&e.reason==i+1);
        memset(r,0,sizeof(r));r[0]=1;r[3]=0x80;
        assert(SayAllInputReport(&s,r,sizeof(r))); assert(!r[3]);
        SayAllInputClaim(&s,7); assert(take(&s).flags==SAYALL_INPUT_CANCEL);
        report(&s,0,0,0); assert(take(&s).flags==SAYALL_INPUT_READY);
    }
    /* Saturation cancels, retains suppressed held keys and cannot auto rearm. */
    for(i=0;i<70;i++) report(&s,i%2?0:0x81,0,0);
    assert(s.overflow && !s.claimed); assert(take(&s).flags==SAYALL_INPUT_CANCEL);
    /* Non-keyboard reports and unknown shapes are never rewritten. */
    memset(r,0,sizeof(r));r[0]=6;r[3]=0x80; assert(!SayAllInputReport(&s,r,sizeof(r)));assert(r[3]==0x80);
    r[0]=1;r[120]=9; SayAllInputClaim(&s,7); assert(!SayAllInputReport(&s,r,sizeof(r)));assert(r[3]==0x80&&!s.claimed);
    memset(r,0,sizeof(r));r[0]=1;r[3]=1; assert(!SayAllInputReport(&s,r,sizeof(r)));
    assert(!SayAllInputReport(&s,r,4));
    /* Selecting Back never steals an unconfigured native volume hold. */
    memset(&s,0,sizeof(s));report(&s,0,0,0);SayAllInputClaim(&s,1);take(&s);
    memset(r,0,sizeof(r));r[0]=1;r[1]=0xF1;r[3]=0x80;r[5]=0x3E;
    assert(SayAllInputReport(&s,r,sizeof(r)));assert(r[1]==0&&r[3]==0x80&&r[5]==0x3E);
    assert(take(&s).buttons==1);
    report(&s,0,0,0);take(&s);r[1]=0xF1;r[2]=1;
    assert(!SayAllInputReport(&s,r,sizeof(r)));assert(r[1]==0xF1&&r[2]==1);
    { SAYALL_INPUT_STATUS status;
      SayAllInputStatus(&s,&status); assert(!status.observed_report);
      report(&s,0,0,0); SayAllInputStatus(&s,&status);
      assert(status.observed_report && !status.physical_buttons && !status.swallowed_buttons);
      report(&s,0x80,0,0); SayAllInputStatus(&s,&status); assert(status.physical_buttons==2);
      for(i=0;i<3;i++) {
        report(&s,0,0,0); memset(r,0,sizeof(r));r[0]=1;
        if(i==1)r[120]=9; if(i==2)r[2]=1;
        assert(!SayAllInputReport(&s,r,i==0?7:sizeof(r)));
        SayAllInputStatus(&s,&status);assert(!status.observed_report);
        report(&s,0,0,0);SayAllInputStatus(&s,&status);assert(status.observed_report&&!status.physical_buttons&&!status.swallowed_buttons);
        r[0]=6;SayAllInputReport(&s,r,sizeof(r));SayAllInputStatus(&s,&status);assert(status.observed_report);
      }
    }
    puts("passed: passive/claim, multi-slot, duplicate, fast edges, voice isolation, lifecycle cancellation, overflow, unknown shapes");
    return 0;
}
