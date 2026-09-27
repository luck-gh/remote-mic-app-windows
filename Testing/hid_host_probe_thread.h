/* GPL-3.0-only. Diagnostic-only link-time observation in our own probe.
 * No target hooks, memory access, thread suspension or altered arguments. */
#pragma once
#include <windows.h>

typedef HANDLE (WINAPI *SayAllCreateRemoteThread)(HANDLE,LPSECURITY_ATTRIBUTES,SIZE_T,LPTHREAD_START_ROUTINE,LPVOID,DWORD,LPDWORD);
typedef struct {DWORD matched,duplicate_error,wait_result,query_error,exit_code;BOOL observed,exited;} SayAllThreadResult;
static SRWLOCK sayall_thread_lock=SRWLOCK_INIT;
static HANDLE sayall_thread_target,sayall_observed_thread;
static DWORD sayall_thread_matched,sayall_thread_duplicate_error;

static void sayall_thread_target_set(HANDLE target){
  AcquireSRWLockExclusive(&sayall_thread_lock);
  if(sayall_observed_thread)CloseHandle(sayall_observed_thread);
  sayall_observed_thread=NULL;sayall_thread_target=target;sayall_thread_matched=0;sayall_thread_duplicate_error=0;
  ReleaseSRWLockExclusive(&sayall_thread_lock);
}
static SayAllThreadResult sayall_thread_result(void){
  SayAllThreadResult result={0};
  AcquireSRWLockShared(&sayall_thread_lock);
  result.matched=sayall_thread_matched;result.duplicate_error=sayall_thread_duplicate_error;
  if(sayall_observed_thread){
    result.observed=TRUE;result.wait_result=WaitForSingleObject(sayall_observed_thread,0);
    if(result.wait_result==WAIT_OBJECT_0){result.exited=TRUE;if(!GetExitCodeThread(sayall_observed_thread,&result.exit_code))result.query_error=GetLastError();}
    else if(result.wait_result==WAIT_FAILED)result.query_error=GetLastError();
  }
  ReleaseSRWLockShared(&sayall_thread_lock);return result;
}
#ifdef SAYALL_THREAD_SELFTEST
static HANDLE WINAPI sayall_fake_remote_thread(HANDLE,LPSECURITY_ATTRIBUTES,SIZE_T,LPTHREAD_START_ROUTINE,LPVOID,DWORD,LPDWORD);
#endif
static HANDLE WINAPI sayall_observe_remote_thread(HANDLE process,LPSECURITY_ATTRIBUTES attributes,SIZE_T stack,LPTHREAD_START_ROUTINE routine,LPVOID parameter,DWORD flags,LPDWORD thread_id){
  DWORD initial_error=GetLastError();
#ifdef SAYALL_THREAD_SELFTEST
  SayAllCreateRemoteThread real=sayall_fake_remote_thread;
#else
  SayAllCreateRemoteThread real=(SayAllCreateRemoteThread)GetProcAddress(GetModuleHandleW(L"kernel32.dll"),"CreateRemoteThread");
#endif
  if(!real){SetLastError(ERROR_PROC_NOT_FOUND);return NULL;}
  SetLastError(initial_error);
  HANDLE result=real(process,attributes,stack,routine,parameter,flags,thread_id);
  DWORD result_error=GetLastError();
  if(result){
    AcquireSRWLockExclusive(&sayall_thread_lock);
    if(sayall_thread_target&&CompareObjectHandles(process,sayall_thread_target)){
      sayall_thread_matched++;
      if(!sayall_observed_thread&&!DuplicateHandle(GetCurrentProcess(),result,GetCurrentProcess(),&sayall_observed_thread,THREAD_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,0))sayall_thread_duplicate_error=GetLastError();
    }
    ReleaseSRWLockExclusive(&sayall_thread_lock);
  }
  SetLastError(result_error);return result;
}
/* Satisfies only this executable's static-library import at link time. */
SayAllCreateRemoteThread __imp_CreateRemoteThread=sayall_observe_remote_thread;
