#define WIN32_LEAN_AND_MEAN
#define SAYALL_THREAD_SELFTEST
#include <assert.h>
#include <stdio.h>
#include "hid_host_probe_thread.h"
static HANDLE expected_process,gate;
static DWORD expected_error=777,worker_code;
static BOOL fake_failure;
static DWORD WINAPI local_worker(void *parameter){assert(parameter==&worker_code);if(gate)WaitForSingleObject(gate,5000);return worker_code;}
static HANDLE WINAPI sayall_fake_remote_thread(HANDLE process,LPSECURITY_ATTRIBUTES attributes,SIZE_T stack,LPTHREAD_START_ROUTINE routine,LPVOID parameter,DWORD flags,LPDWORD thread_id){
  assert(GetLastError()==expected_error);assert(process==expected_process);assert(attributes==NULL&&stack==0&&routine==local_worker&&parameter==&worker_code&&flags==0&&thread_id!=NULL);
  if(fake_failure){SetLastError(ERROR_ACCESS_DENIED);return NULL;}
  HANDLE thread=CreateThread(attributes,stack,routine,parameter,flags,thread_id);SetLastError(1234);return thread;
}
static HANDLE invoke(void){DWORD id;SetLastError(expected_error);HANDLE thread=CreateRemoteThread(expected_process,NULL,0,local_worker,&worker_code,0,&id);assert(GetLastError()==(fake_failure?ERROR_ACCESS_DENIED:1234));return thread;}
int main(void){
  expected_process=GetCurrentProcess();sayall_thread_target_set(expected_process);
  fake_failure=TRUE;assert(invoke()==NULL);assert(!sayall_thread_result().observed);fake_failure=FALSE;
  const DWORD codes[]={0,5,126,1114,STILL_ACTIVE};
  for(unsigned i=0;i<5;i++){sayall_thread_target_set(expected_process);worker_code=codes[i];HANDLE thread=invoke();assert(thread);assert(WaitForSingleObject(thread,5000)==WAIT_OBJECT_0);CloseHandle(thread);SayAllThreadResult result=sayall_thread_result();assert(result.observed&&result.exited&&result.exit_code==codes[i]&&!result.query_error&&result.matched==1);}
  sayall_thread_target_set(expected_process);gate=CreateEventW(NULL,TRUE,FALSE,NULL);HANDLE thread=invoke();SayAllThreadResult pending=sayall_thread_result();assert(pending.observed&&!pending.exited&&pending.wait_result==WAIT_TIMEOUT);SetEvent(gate);assert(WaitForSingleObject(thread,5000)==WAIT_OBJECT_0);CloseHandle(thread);CloseHandle(gate);gate=NULL;
  HANDLE unrelated=CreateEventW(NULL,TRUE,FALSE,NULL);sayall_thread_target_set(unrelated);thread=invoke();assert(WaitForSingleObject(thread,5000)==WAIT_OBJECT_0);CloseHandle(thread);assert(!sayall_thread_result().observed);CloseHandle(unrelated);
  sayall_thread_target_set(NULL);assert(!sayall_thread_result().observed);
  puts("thread_observer_tests=passed cases=9 remote_injection=false");return 0;
}
