#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <initguid.h>
#include <stdio.h>
#include <assert.h>
static void result(const char *phase,int code){(void)phase;(void)code;}
static void audit(const char *message){(void)message;}
#include "hid_host_probe_runtime.h"
static void check(const WCHAR *sddl,BOOL expected){PSECURITY_DESCRIPTOR descriptor=NULL;assert(ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl,SDDL_REVISION_1,&descriptor,NULL));assert(runtime_exact_acl(descriptor,TRUE)==expected);LocalFree(descriptor);}
int main(void){
  check(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;LS)",TRUE);
  check(L"O:BAG:BAD:(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;LS)",FALSE);
  check(L"O:LSG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;LS)",FALSE);
  check(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;LS)",FALSE);
  check(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;WD)",FALSE);
  check(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;LS)(A;OICI;GRGX;;;LS)",FALSE);
  puts("runtime_acl_tests=passed cases=6 filesystem_mutated=false");return 0;
}
