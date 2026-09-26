/* Targeted tests of the production controller, without attaching any process. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <cfgmgr32.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include "frida-core.h"
static unsigned posts, cancels;
static unsigned resets_present, resets_absent, list_calls, size_calls, small_reads, selected_lookups, timer_removals;
static gboolean missing_node, duplicate_pdo, replaced_node;
static GSourceFunc pending_timer;
static CONFIGRET WINAPI test_locate(PDEVINST node,DEVINSTID_W id,ULONG flags) {
  assert(flags==CM_LOCATE_DEVNODE_NORMAL);
  if(!wcscmp(id,L"test-selected")){
    if(missing_node)return CR_NO_SUCH_DEVNODE;
    *node=(replaced_node&&++selected_lookups==3)?43:42;return CR_SUCCESS;
  }
  if(!wcscmp(id,L"test-other")){*node=7;return CR_SUCCESS;}
  return CR_NO_SUCH_DEVNODE;
}
static CONFIGRET WINAPI test_status(PULONG state,PULONG problem,DEVINST node,ULONG flags) {
  (void)node;assert(flags==0);*state=DN_STARTED;*problem=0;return CR_SUCCESS;
}
static CONFIGRET WINAPI test_property(DEVINST node,ULONG property,PULONG type,PVOID buffer,PULONG bytes,ULONG flags) {
  assert(property==CM_DRP_PHYSICAL_DEVICE_OBJECT_NAME&&flags==0);
  const WCHAR *value=(node==42||duplicate_pdo)?L"\\Device\\test-selected":L"\\Device\\test-other";
  ULONG required=(ULONG)(wcslen(value)+1)*sizeof(WCHAR);assert(*bytes>=required);
  memcpy(buffer,value,required);*bytes=required;*type=REG_SZ;return CR_SUCCESS;
}
static CONFIGRET WINAPI test_list_size(PULONG count,PCWSTR filter,ULONG flags) {
  assert(filter==NULL&&flags==CM_GETIDLIST_FILTER_PRESENT);size_calls++;*count=64+size_calls;return CR_SUCCESS;
}
static CONFIGRET WINAPI test_list(PCWSTR filter,PWSTR buffer,ULONG count,ULONG flags) {
  static const WCHAR ids[]=L"test-selected\0test-other\0";
  assert(filter==NULL&&flags==CM_GETIDLIST_FILTER_PRESENT&&count>=64);list_calls++;
  if(small_reads){small_reads--;return CR_BUFFER_SMALL;}
  memcpy(buffer,ids,sizeof(ids));return CR_SUCCESS;
}
static guint test_timeout(guint interval,GSourceFunc function,gpointer data) {
  assert(interval==250&&data==NULL&&!pending_timer);pending_timer=function;return 91;
}
static gboolean test_remove(guint tag) {
  assert(tag==91);timer_removals++;pending_timer=NULL;return TRUE;
}
static char audit_text[768];
static char error_text[768];
static BOOL WINAPI test_event(HANDLE h,WORD t,WORD c,DWORD id,PSID sid,WORD n,DWORD bytes,LPCSTR *strings,LPVOID data) {
  (void)h;(void)t;(void)c;(void)id;(void)sid;(void)n;(void)bytes;(void)data;
  strcpy_s(audit_text,sizeof(audit_text),strings[0]);
  if(strstr(strings[0],"phase=script_error"))strcpy_s(error_text,sizeof(error_text),strings[0]);
  return TRUE;
}
static BOOL WINAPI test_write(HANDLE h,LPCVOID p,DWORD n,LPDWORD written,LPOVERLAPPED o) {
  (void)h;(void)o;assert(n<4096);*written=n;
  if(strstr((const char*)p,"\"kind\":\"cancel\""))cancels++;
  return TRUE;
}
static void test_post(FridaScript *s,const gchar *json,GBytes *data) {
  (void)s;(void)data;
  if(!strcmp(json,"{\"type\":\"stop\"}")){posts++;return;}
  JsonParser *parser=json_parser_new();assert(json_parser_load_from_data(parser,json,-1,NULL));
  JsonObject *object=json_node_get_object(json_parser_get_root(parser));
  assert(!strcmp(json_object_get_string_member(object,"type"),"device_reset"));
  if(json_object_get_boolean_member(object,"present")){
    assert(!strcmp(json_object_get_string_member(object,"pdo"),"\\Device\\test-selected"));resets_present++;
  }else{assert(!strcmp(json_object_get_string_member(object,"pdo"),""));resets_absent++;}
  g_object_unref(parser);
}
#define CM_Locate_DevNodeW test_locate
#define CM_Get_DevNode_Status test_status
#define CM_Get_DevNode_Registry_PropertyW test_property
#define CM_Get_Device_ID_List_SizeW test_list_size
#define CM_Get_Device_ID_ListW test_list
#define g_timeout_add test_timeout
#define g_source_remove test_remove
#define ReportEventA test_event
#define WriteFile test_write
#define frida_script_post test_post
#define wmain unused_product_main
#include "main.c"
#undef wmain
static void source_fixture(void) {
  wcscpy_s(selected_instance,1024,L"test-selected");selected_pdo[0]=0;
  missing_node=duplicate_pdo=replaced_node=FALSE;
  resets_present=resets_absent=list_calls=size_calls=small_reads=selected_lookups=timer_removals=0;
  source_refresh_timer=0;source_refresh_retryable=FALSE;pending_timer=NULL;
  stopping=stopped=session_gone=cleanup_failed=FALSE;script=(FridaScript*)1;
}
int wmain(void) {
  frida_init();
  loop=g_main_loop_new(NULL,FALSE);script=(FridaScript*)1;
  message(NULL,"{\"type\":\"error\",\"description\":\"TypeError: private-device-path\",\"lineNumber\":237,\"columnNumber\":8}",NULL,NULL);
  assert(stopping&&posts==1&&cancels==1);
  assert(strstr(error_text,"type=error class=TypeError line=237 column=8"));
  assert(!strstr(error_text,"private-device-path"));
  begin_stop();assert(posts==1&&cancels==1);
  message(NULL,"{\"type\":\"send\",\"payload\":{\"kind\":\"stopped\",\"held\":0,\"cleanupErrors\":1}}",NULL,NULL);
  assert(stopped&&cleanup_failed);assert(strstr(audit_text,"phase=script_cleanup code=1"));
  stopped=FALSE;cleanup_failed=FALSE;stopping=FALSE;session_gone=FALSE;
  detached(NULL,FRIDA_SESSION_DETACH_REASON_PROCESS_TERMINATED,NULL,NULL);
  assert(session_gone&&!stopping&&cancels==2);
  source_fixture();small_reads=1;
  assert(refresh_selected_pdo()&&size_calls==2&&list_calls==2&&!source_refresh_retryable);
  assert(!wcscmp(selected_pdo,L"\\Device\\test-selected"));
  source_fixture();small_reads=6;device_event(NULL);
  assert(resets_absent==1&&resets_present==0&&pending_timer&&source_refresh_timer==91);
  assert(pending_timer(NULL)==G_SOURCE_CONTINUE&&resets_absent==1&&resets_present==0&&selected_pdo[0]==0);
  assert(pending_timer(NULL)==G_SOURCE_REMOVE&&resets_present==1&&source_refresh_timer==0);
  assert(size_calls==7&&list_calls==7);
  source_fixture();small_reads=3;device_event(NULL);missing_node=TRUE;
  assert(pending_timer(NULL)==G_SOURCE_REMOVE&&resets_present==0&&source_refresh_timer==0&&selected_pdo[0]==0);
  source_fixture();duplicate_pdo=TRUE;
  assert(!refresh_selected_pdo()&&selected_pdo[0]==0&&!source_refresh_retryable);
  source_fixture();replaced_node=TRUE;
  assert(!refresh_selected_pdo()&&selected_pdo[0]==0&&!source_refresh_retryable);
  source_fixture();small_reads=3;device_event(NULL);unsigned reads=list_calls;
  begin_stop();assert(!pending_timer&&source_refresh_timer==0&&timer_removals==1);
  assert(retry_source_refresh(NULL)==G_SOURCE_REMOVE&&list_calls==reads&&resets_present==0);
  source_fixture();small_reads=3;device_event(NULL);reads=list_calls;session_gone=TRUE;
  assert(pending_timer(NULL)==G_SOURCE_REMOVE&&list_calls==reads&&resets_present==0&&source_refresh_timer==0);
  script=NULL;g_main_loop_unref(loop);loop=NULL;
  puts("production_controller_cases=10 passed=10 attachment=none device_calls=none");
  return 0;
}
