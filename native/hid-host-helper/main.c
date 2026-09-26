/* GPL-3.0-only. Fixed RC003 input enhancement Helper. Explicit elevation only.
 * No caller-selected process, script, hook address, output file, or device write. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <cfgmgr32.h>
#include <initguid.h>
#include <devpkey.h>
#include <wintrust.h>
#include <softpub.h>
#include <mscat.h>
#include <tlhelp32.h>
#include <hidsdi.h>
#include <hidpi.h>
#include <stdio.h>
#include <stdint.h>
#include "frida-core.h"
#include "runtime_script.h"
#define HOGP_SHA256 "372c3628e3366c18152199eb8fb554d27bb4f02d1356b1f514f711328df4a8d6"
#define FRAMEWORK_SHA256 "e97a8bbbda4dd4989d9b3063cbfea05df9f60cdc7758fd21b163ebe3ce6786e0"
typedef char hogp_sha256_must_be_64_hex_chars[sizeof(HOGP_SHA256)==65?1:-1];
typedef char framework_sha256_must_be_64_hex_chars[sizeof(FRAMEWORK_SHA256)==65?1:-1];
static GMainLoop *loop;
static FridaScript *script;
static HANDLE pipe_handle=INVALID_HANDLE_VALUE, audit_handle, parent_process, target_process;
static HANDLE module_locks[2];
static char nonce[33];
static gboolean stopping, stopped, session_gone;
static gboolean cleanup_failed;
static guint requested_mask;
static gint64 configuration;
static HCMNOTIFICATION device_notification;
static guint source_refresh_timer;
static gboolean source_refresh_retryable;
static WCHAR selected_instance[1024];
static WCHAR selected_pdo[1024];
static GCancellable *operation;
static HANDLE operation_timer;
static void audit(const char *message) {
  char line[768];snprintf(line,sizeof(line),"hid_host run_id=%s %s",nonce,message);
  const char *strings[]={line};ReportEventA(audit_handle,EVENTLOG_INFORMATION_TYPE,0,1002,NULL,1,0,strings,NULL);
}
static void result(const char *phase,int code) {
  char line[160];snprintf(line,sizeof(line),"phase=%s code=%d",phase,code);audit(line);
}
#include "runtime_security.h"
static void CALLBACK cancel_operation(PVOID context,BOOLEAN fired){(void)fired;g_cancellable_cancel(context);}
static gboolean begin_operation(void) {
  operation=g_cancellable_new();operation_timer=NULL;
  if(!CreateTimerQueueTimer(&operation_timer,NULL,cancel_operation,operation,10000,0,WT_EXECUTEONLYONCE)){
    g_object_unref(operation);operation=NULL;return FALSE;
  }return TRUE;
}
static void end_operation(void) {
  if(operation_timer)DeleteTimerQueueTimer(NULL,operation_timer,INVALID_HANDLE_VALUE);
  if(operation)g_object_unref(operation);operation=NULL;operation_timer=NULL;
}
static gboolean send_packet(const char *text) {
  DWORD written=0;size_t length=strlen(text);
  return length<=4096&&WriteFile(pipe_handle,text,(DWORD)length,&written,NULL)&&written==length;
}
static gboolean read_packet(char *text,DWORD capacity) {
  DWORD got=0;
  if(!ReadFile(pipe_handle,text,capacity-1,&got,NULL)||got==0)return FALSE;
  text[got]=0;return TRUE;
}
static gboolean verify_parent(DWORD pid) {
  DWORD actual_pid=0,session=0,length=2048;
  WCHAR path[2048],expected[2048];
  if(!GetNamedPipeServerProcessId(pipe_handle,&actual_pid)||actual_pid!=pid||
     !ProcessIdToSessionId(pid,&session)||session!=WTSGetActiveConsoleSessionId())return FALSE;
  parent_process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,pid);
  if(!parent_process||!QueryFullProcessImageNameW(parent_process,0,path,&length)||
     !GetModuleFileNameW(NULL,expected,2048))return FALSE;
  WCHAR *name=wcsrchr(expected,L'\\');if(!name)return FALSE;
  wcscpy_s(name+1,2048-(name+1-expected),L"sayall-windows-app.exe");
  return _wcsicmp(path,expected)==0&&WaitForSingleObject(parent_process,0)==WAIT_TIMEOUT;
}
static gboolean signed_file(const WCHAR *path) {
  WINTRUST_FILE_INFO file={sizeof(file)};file.pcwszFilePath=path;
  WINTRUST_DATA trust={sizeof(trust)};GUID action=WINTRUST_ACTION_GENERIC_VERIFY_V2;
  trust.dwUIChoice=WTD_UI_NONE;trust.fdwRevocationChecks=WTD_REVOKE_WHOLECHAIN;
  trust.dwUnionChoice=WTD_CHOICE_FILE;trust.pFile=&file;trust.dwStateAction=WTD_STATEACTION_VERIFY;
  trust.dwProvFlags=WTD_CACHE_ONLY_URL_RETRIEVAL;
  LONG status=WinVerifyTrust(NULL,&action,&trust);
  trust.dwStateAction=WTD_STATEACTION_CLOSE;WinVerifyTrust(NULL,&action,&trust);
  if(status==ERROR_SUCCESS)return TRUE;
  result("signature_embedded",status);
  // Windows system modules can be signed as catalog members. Verify the
  // calculated member hash AND the catalog trust chain; a catalog's presence
  // alone never grants trust. This is the same WinTrust policy as above.
  HCATADMIN admin=NULL;HCATINFO catalog=NULL;gboolean trusted=FALSE;
  HANDLE member=CreateFileW(path,GENERIC_READ,FILE_SHARE_READ,NULL,OPEN_EXISTING,0,NULL);
  if(member==INVALID_HANDLE_VALUE){result("signature_member_open",GetLastError());return FALSE;}
  if(!CryptCATAdminAcquireContext2(&admin,NULL,L"SHA256",NULL,0)){result("signature_catalog_context",GetLastError());goto catalog_done;}
  BYTE hash[64];DWORD size=sizeof(hash);
  if(!CryptCATAdminCalcHashFromFileHandle2(admin,member,&size,hash,0)||size!=32){result("signature_catalog_hash",GetLastError());goto catalog_done;}
  WCHAR tag[129];for(DWORD i=0;i<size;i++)swprintf_s(tag+i*2,129-i*2,L"%02X",hash[i]);
  for(unsigned attempt=0;attempt<16;attempt++){
    catalog=CryptCATAdminEnumCatalogFromHash(admin,hash,size,0,&catalog);
    if(!catalog)break;
    CATALOG_INFO information={sizeof(information)};
    if(!CryptCATCatalogInfoFromContext(catalog,&information,0))continue;
    WINTRUST_CATALOG_INFO item={sizeof(item)};
    item.pcwszCatalogFilePath=information.wszCatalogFile;item.pcwszMemberTag=tag;
    item.pcwszMemberFilePath=path;item.hMemberFile=member;
    item.pbCalculatedFileHash=hash;item.cbCalculatedFileHash=size;item.hCatAdmin=admin;
    trust.dwUnionChoice=WTD_CHOICE_CATALOG;trust.pCatalog=&item;
    trust.dwStateAction=WTD_STATEACTION_VERIFY;trust.hWVTStateData=NULL;
    status=WinVerifyTrust(NULL,&action,&trust);
    trust.dwStateAction=WTD_STATEACTION_CLOSE;WinVerifyTrust(NULL,&action,&trust);
    if(status==ERROR_SUCCESS){trusted=TRUE;break;}
    result("signature_catalog_trust",status);
  }
  audit(trusted?"phase=signature_verified type=catalog":"phase=signature_rejected type=catalog");
catalog_done:
  if(catalog)CryptCATAdminReleaseCatalogContext(admin,catalog,0);
  if(admin)CryptCATAdminReleaseContext(admin,0);CloseHandle(member);return trusted;
}
static gboolean lock_hash(const WCHAR *path,const char *expected,HANDLE *lock) {
  *lock=CreateFileW(path,GENERIC_READ,FILE_SHARE_READ,NULL,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,NULL);
  if(*lock==INVALID_HANDLE_VALUE){*lock=NULL;return FALSE;}
  FILE_ATTRIBUTE_TAG_INFO info;
  if(!GetFileInformationByHandleEx(*lock,FileAttributeTagInfo,&info,sizeof(info))||
     (info.FileAttributes&(FILE_ATTRIBUTE_REPARSE_POINT|FILE_ATTRIBUTE_DIRECTORY)))return FALSE;
  GChecksum *sum=g_checksum_new(G_CHECKSUM_SHA256);BYTE bytes[65536];DWORD got;gboolean ok=FALSE;
  while(ReadFile(*lock,bytes,sizeof(bytes),&got,NULL)){if(!got){ok=TRUE;break;}g_checksum_update(sum,bytes,got);}
  ok=ok&&strcmp(g_checksum_get_string(sum),expected)==0;g_checksum_free(sum);return ok;
}
static int host(DWORD *pid, WCHAR *instance_id, size_t instance_chars) {
  HKEY root; DWORD index=0, targets=0, hid_services=0, pid_values=0, pid_rejected=0, open_failed=0, open_error=0, not_live=0; WCHAR service[512], instance[512], selected[1024]={0};
  if(RegOpenKeyExW(HKEY_LOCAL_MACHINE,L"SYSTEM\\CurrentControlSet\\Enum\\BTHLEDevice",0,KEY_READ,&root)!=ERROR_SUCCESS)return 1;
  while(1) {
    DWORD length=512; HKEY service_key; LONG status=RegEnumKeyExW(root,index++,service,&length,NULL,NULL,NULL,NULL);
    if(status==ERROR_NO_MORE_ITEMS)break;if(status!=ERROR_SUCCESS){RegCloseKey(root);return 2;}
    if(_wcsnicmp(service,L"{00001812-",10)!=0)continue;hid_services++;
    if(RegOpenKeyExW(root,service,0,KEY_READ,&service_key)!=ERROR_SUCCESS)continue;
    DWORD instance_index=0;
    while(1) {
      length=512; status=RegEnumKeyExW(service_key,instance_index++,instance,&length,NULL,NULL,NULL,NULL);
      if(status==ERROR_NO_MORE_ITEMS)break;if(status!=ERROR_SUCCESS)break;
      WCHAR key[1024]; HKEY diagnostic;swprintf_s(key,1024,L"%s\\Device Parameters\\WUDFDiagnosticInfo",instance);
      if(RegOpenKeyExW(service_key,key,0,KEY_READ,&diagnostic)!=ERROR_SUCCESS)continue;
      ULONGLONG stored_pid=0;DWORD size=sizeof(stored_pid),type=0;
      status=RegQueryValueExW(diagnostic,L"HostPid",NULL,&type,(BYTE*)&stored_pid,&size);RegCloseKey(diagnostic);
      if(status!=ERROR_SUCCESS||!((type==REG_DWORD&&size==4)||(type==REG_QWORD&&size==8))||stored_pid==0||stored_pid>MAXDWORD){pid_rejected++;continue;}pid_values++;
      DWORD value=(DWORD)stored_pid;
      HANDLE candidate=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,value);
      if(!candidate){open_error=GetLastError();open_failed++;continue;}DWORD exit_code=0;GetExitCodeProcess(candidate,&exit_code);CloseHandle(candidate);
      if(exit_code!=STILL_ACTIVE){not_live++;continue;}
      WCHAR folded[512];wcscpy_s(folded,512,service);CharLowerBuffW(folded,(DWORD)wcslen(folded));
      if(wcsstr(folded,L"dev_vid&012717_pid&32b8_rev&00a4")){
        targets++;*pid=value;swprintf_s(selected,1024,L"BTHLEDevice\\%s\\%s",service,instance);
      }
    }
    RegCloseKey(service_key);
  }
  RegCloseKey(root);char discovery[250];snprintf(discovery,sizeof(discovery),"hid_host phase=discovery_counts hid_services=%lu pid_values=%lu pid_rejected=%lu process_open_failed=%lu process_open_error=%lu process_not_live=%lu target_count=%lu",hid_services,pid_values,pid_rejected,open_failed,open_error,not_live,targets);audit(discovery);if(targets!=1){audit(targets==0?"hid_host phase=discovery_rejected reason=no_valid_target":"hid_host phase=discovery_rejected reason=multiple_targets");return 3;}
  DEVINST devnode;
  if(CM_Locate_DevNodeW(&devnode,selected,CM_LOCATE_DEVNODE_NORMAL)!=CR_SUCCESS)return 4;
  if(wcscpy_s(instance_id,instance_chars,selected)!=0)return 5;
  return 0;
}

/* Exact current function-node proof. No parent/model/container fallback. */
static CONFIGRET node_pdo(DEVINST node,WCHAR value[1024],ULONG *bytes,ULONG *type) {
  *bytes=2048;*type=0;memset(value,0,2048);
  CONFIGRET status=CM_Get_DevNode_Registry_PropertyW(node,CM_DRP_PHYSICAL_DEVICE_OBJECT_NAME,type,value,bytes,0);
  if(status==CR_SUCCESS&&(*type!=REG_SZ||*bytes<4||*bytes>2048||(*bytes&1)||value[*bytes/2-1]!=0||wcslen(value)!=*bytes/2-1))return CR_INVALID_DATA;
  return status;
}
static gboolean refresh_selected_pdo(void) {
  selected_pdo[0]=0;source_refresh_retryable=FALSE;DEVINST selected;ULONG state=0,problem=0,bytes=0,type=0,count=0;WCHAR value[1024];
  CONFIGRET status=CM_Locate_DevNodeW(&selected,selected_instance,CM_LOCATE_DEVNODE_NORMAL);
  if(status==CR_SUCCESS)status=CM_Get_DevNode_Status(&state,&problem,selected,0);
  if(status==CR_SUCCESS&&!(state&DN_STARTED))status=CR_NO_SUCH_DEVNODE;
  if(status==CR_SUCCESS)status=node_pdo(selected,value,&bytes,&type);
  char detail[200];snprintf(detail,sizeof(detail),"phase=source_public_property status=0x%08lx type=%lu bytes=%lu",status,type,bytes);audit(detail);
  if(status!=CR_SUCCESS)return FALSE;
  WCHAR *ids=NULL;
  /* The present tree can grow between Size and List (CR_BUFFER_SMALL).
     Never interpret that incomplete snapshot as an empty authoritative list. */
  for(unsigned attempt=1;attempt<=3;attempt++) {
    status=CM_Get_Device_ID_List_SizeW(&count,NULL,CM_GETIDLIST_FILTER_PRESENT);
    if(status!=CR_SUCCESS||count==0||count>16*1024*1024){result("source_public_enumeration",status);return FALSE;}
    ids=g_new0(WCHAR,count);
    status=CM_Get_Device_ID_ListW(NULL,ids,count,CM_GETIDLIST_FILTER_PRESENT);
    if(status!=CR_BUFFER_SMALL)break;
    g_free(ids);ids=NULL;
    snprintf(detail,sizeof(detail),"phase=source_public_resize status=0x%08lx attempt=%u",status,attempt);audit(detail);
  }
  if(status==CR_BUFFER_SMALL){source_refresh_retryable=TRUE;return FALSE;}
  unsigned matches=0;gboolean exact=FALSE;
  if(status==CR_SUCCESS)for(WCHAR *id=ids;*id;id+=wcslen(id)+1){
    DEVINST node;WCHAR pdo[1024];ULONG n=0,t=0;
    if(CM_Locate_DevNodeW(&node,id,CM_LOCATE_DEVNODE_NORMAL)!=CR_SUCCESS||node_pdo(node,pdo,&n,&t)!=CR_SUCCESS)continue;
    if(!_wcsicmp(value,pdo)){matches++;if(node==selected&&!_wcsicmp(id,selected_instance))exact=TRUE;}
  }
  g_free(ids);DEVINST current;WCHAR final_value[1024];ULONG final_bytes=0,final_type=0;
  gboolean current_ok=status==CR_SUCCESS&&matches==1&&exact&&
    CM_Locate_DevNodeW(&current,selected_instance,CM_LOCATE_DEVNODE_NORMAL)==CR_SUCCESS&&current==selected&&
    CM_Get_DevNode_Status(&state,&problem,current,0)==CR_SUCCESS&&(state&DN_STARTED)&&
    node_pdo(current,final_value,&final_bytes,&final_type)==CR_SUCCESS&&!_wcsicmp(final_value,value);
  snprintf(detail,sizeof(detail),"phase=source_public_binding status=0x%08lx exact_matches=%u selected_function=%s current=%s",status,matches,exact?"true":"false",current_ok?"true":"false");audit(detail);
  if(current_ok)wcscpy_s(selected_pdo,1024,value);return current_ok;
}


static gboolean descriptor(const WCHAR *hid_path,const WCHAR *service) {
  WCHAR instance[1024];ULONG size=sizeof(instance);DEVPROPTYPE type=0;DEVINST node=0;
  if(CM_Get_Device_Interface_PropertyW(hid_path,&DEVPKEY_Device_InstanceId,&type,
      (PBYTE)instance,&size,0)!=CR_SUCCESS||type!=DEVPROP_TYPE_STRING||
      size<2||size>sizeof(instance)||instance[size/2-1]!=0||
      CM_Locate_DevNodeW(&node,instance,CM_LOCATE_DEVNODE_NORMAL)!=CR_SUCCESS)return FALSE;
  gboolean matched=FALSE;
  for(unsigned depth=0;depth<8;depth++){
    if(CM_Get_Device_IDW(node,instance,1024,0)!=CR_SUCCESS)break;
    if(_wcsicmp(instance,service)==0){matched=TRUE;break;}
    DEVINST parent;if(CM_Get_Parent(&parent,node,0)!=CR_SUCCESS)break;node=parent;
  }
  if(!matched){audit("phase=descriptor_rejected reason=selected_ancestor");return FALSE;}
  HANDLE file=CreateFileW(hid_path,0,FILE_SHARE_READ|FILE_SHARE_WRITE,NULL,OPEN_EXISTING,0,NULL);
  if(file==INVALID_HANDLE_VALUE)return FALSE;
  PHIDP_PREPARSED_DATA data=NULL;HIDP_CAPS caps={0};gboolean valid=FALSE;
  if(!HidD_GetPreparsedData(file,&data))goto done;
  if(HidP_GetCaps(data,&caps)!=HIDP_STATUS_SUCCESS||caps.UsagePage!=1||caps.Usage!=6||
     caps.InputReportByteLength!=121||HidP_MaxUsageListLength(HidP_Input,7,data)!=3)goto done;
  HIDP_BUTTON_CAPS buttons[32];USHORT count=32;
  if(caps.NumberInputButtonCaps>32||HidP_GetButtonCaps(HidP_Input,buttons,&count,data)!=HIDP_STATUS_SUCCESS)goto done;
  unsigned relevant=0;
  for(unsigned i=0;i<count;i++)if(buttons[i].ReportID==1){
    if(buttons[i].UsagePage!=7||!buttons[i].IsRange||
       buttons[i].Range.UsageMin!=0||buttons[i].Range.UsageMax!=254)goto done;
    relevant++;
  }
  if(relevant!=1)goto done;
  HIDP_VALUE_CAPS values[32];count=32;
  if(caps.NumberInputValueCaps>32)goto done;
  if(caps.NumberInputValueCaps){
    if(HidP_GetValueCaps(HidP_Input,values,&count,data)!=HIDP_STATUS_SUCCESS)goto done;
    for(unsigned i=0;i<count;i++)if(values[i].ReportID==1)goto done;
  }
  const USAGE patterns[2][3]={{0xf1,0x80,0x81},{0x35,0x4a,0xf1}};
  for(unsigned p=0;p<2;p++){
    UCHAR report[121]={0};ULONG usages=3;
    if(HidP_InitializeReportForID(HidP_Input,1,data,(PCHAR)report,sizeof(report))!=HIDP_STATUS_SUCCESS||
       HidP_SetUsages(HidP_Input,7,0,(PUSAGE)patterns[p],&usages,data,(PCHAR)report,sizeof(report))!=HIDP_STATUS_SUCCESS||
       usages!=3||report[0]!=1)goto done;
    for(unsigned i=0;i<3;i++)if(report[1+2*i]!=(UCHAR)patterns[p][i]||report[2+2*i]!=0)goto done;
    for(unsigned i=7;i<121;i++)if(report[i])goto done;
    usages=3;
    if(HidP_UnsetUsages(HidP_Input,7,0,(PUSAGE)patterns[p],&usages,data,(PCHAR)report,sizeof(report))!=HIDP_STATUS_SUCCESS)goto done;
    for(unsigned i=1;i<121;i++)if(report[i])goto done;
  }
  valid=TRUE;
done:
  {char detail[200];snprintf(detail,sizeof(detail),"phase=descriptor_gate valid=%s page=%u usage=%u max_input=%u button_caps=%u value_caps=%u",valid?"true":"false",caps.UsagePage,caps.Usage,caps.InputReportByteLength,caps.NumberInputButtonCaps,caps.NumberInputValueCaps);audit(detail);}
  if(data)HidD_FreePreparsedData(data);CloseHandle(file);
  audit(valid?"phase=descriptor_verified contract=1 five_keys=true physical_report_verified=false":"phase=descriptor_rejected");
  return valid;
}
static gboolean verify_modules(DWORD pid) {
  WCHAR system[1024],paths[2][1200];GetSystemDirectoryW(system,1024);
  swprintf_s(paths[0],1200,L"%s\\drivers\\UMDF\\Microsoft.Bluetooth.Profiles.HidOverGatt.dll",system);
  swprintf_s(paths[1],1200,L"%s\\WUDFx02000.dll",system);
  const char *hashes[]={HOGP_SHA256,FRAMEWORK_SHA256};
  for(unsigned i=0;i<2;i++){
    if(!lock_hash(paths[i],hashes[i],&module_locks[i])){result("module_hash_rejected",i);return FALSE;}
    if(!signed_file(paths[i])){result("module_signature_rejected",i);return FALSE;}
  }
  HANDLE snapshot=CreateToolhelp32Snapshot(TH32CS_SNAPMODULE|TH32CS_SNAPMODULE32,pid);
  if(snapshot==INVALID_HANDLE_VALUE)return FALSE;
  MODULEENTRY32W module={sizeof(module)};unsigned found=0;
  if(Module32FirstW(snapshot,&module))do{
    for(unsigned i=0;i<2;i++)if(_wcsicmp(module.szExePath,paths[i])==0)found|=1u<<i;
  }while(Module32NextW(snapshot,&module));
  CloseHandle(snapshot);if(found!=3)result("module_loaded_paths_rejected",found);return found==3;
}
static void cancel_source_refresh(void) {
  if(source_refresh_timer){g_source_remove(source_refresh_timer);source_refresh_timer=0;}
}
static void begin_stop(void) {
  cancel_source_refresh();
  if(stopping)return;stopping=TRUE;
  send_packet("{\"kind\":\"cancel\",\"reason\":\"helper_stopping\"}");
  if(script)frida_script_post(script,"{\"type\":\"stop\"}",NULL);
  else if(loop)g_main_loop_quit(loop);
  audit("phase=stopping policy=release_mapping_then_drain_owned_holds");
}
static gboolean control(gpointer unused) {
  (void)unused;
  if(WaitForSingleObject(parent_process,0)!=WAIT_TIMEOUT){begin_stop();return G_SOURCE_CONTINUE;}
  DWORD available=0;
  if(!PeekNamedPipe(pipe_handle,NULL,0,NULL,&available,NULL)){begin_stop();return G_SOURCE_CONTINUE;}
  if(available){
    char packet[4096];
    if(!read_packet(packet,sizeof(packet))){begin_stop();return G_SOURCE_CONTINUE;}
    JsonParser *parser=json_parser_new();GError *error=NULL;
    if(!json_parser_load_from_data(parser,packet,-1,&error)){g_clear_error(&error);g_object_unref(parser);begin_stop();return G_SOURCE_CONTINUE;}
    if(!JSON_NODE_HOLDS_OBJECT(json_parser_get_root(parser))){g_object_unref(parser);begin_stop();return G_SOURCE_CONTINUE;}
    JsonObject *object=json_node_get_object(json_parser_get_root(parser));
    const char *kind=json_object_get_string_member_with_default(object,"kind","");
    if(strcmp(kind,"stop")==0)begin_stop();
    else if(strcmp(kind,"configure")==0&&!stopping){
      gint64 mask=json_object_get_int_member_with_default(object,"mask",-1);
      gint64 revision=json_object_get_int_member_with_default(object,"configuration",-1);
      if(mask<0||mask>31||revision<configuration||revision>9007199254740991LL)begin_stop();
      else {
        requested_mask=(guint)mask;configuration=revision;char command[192];
        gboolean preserve_released=json_object_get_boolean_member_with_default(object,"preserveReleased",FALSE);
        snprintf(command,sizeof(command),"{\"type\":\"configure\",\"mask\":%u,\"configuration\":%lld,\"preserveReleased\":%s}",requested_mask,configuration,preserve_released?"true":"false");
        if(script)frida_script_post(script,command,NULL);
      }
    }else if(strcmp(kind,"heartbeat")!=0)begin_stop();
    g_object_unref(parser);
  }
  return G_SOURCE_CONTINUE;
}
static void publish_device_presence(gboolean present) {
  JsonObject *object=json_object_new();json_object_set_string_member(object,"type","device_reset");json_object_set_boolean_member(object,"present",present);
  gchar *pdo=g_utf16_to_utf8((gunichar2*)selected_pdo,-1,NULL,NULL,NULL);json_object_set_string_member(object,"pdo",pdo);g_free(pdo);
  JsonNode *node=json_node_new(JSON_NODE_OBJECT);json_node_take_object(node,object);gchar *command=json_to_string(node,FALSE);json_node_free(node);
  if(script)frida_script_post(script,command,NULL);g_free(command);
  audit(present?"phase=device_generation present=true":"phase=device_generation present=false");
}
static gboolean retry_source_refresh(gpointer unused) {
  (void)unused;
  if(stopping||session_gone||!script){source_refresh_timer=0;return G_SOURCE_REMOVE;}
  gboolean present=refresh_selected_pdo();
  if(present){
    publish_device_presence(TRUE);source_refresh_timer=0;
    audit("phase=source_public_retry result=recovered proof=exact_current_unique");
    return G_SOURCE_REMOVE;
  }
  if(source_refresh_retryable)return G_SOURCE_CONTINUE;
  source_refresh_timer=0;audit("phase=source_public_retry result=unverified native=passthrough");
  return G_SOURCE_REMOVE;
}
static gboolean device_event(gpointer unused) {
  (void)unused;
  if(stopping||session_gone||!script)return G_SOURCE_REMOVE;
  cancel_source_refresh();
  gboolean present=refresh_selected_pdo();publish_device_presence(present);
  if(!present&&source_refresh_retryable){
    source_refresh_timer=g_timeout_add(250,retry_source_refresh,NULL);
    audit("phase=source_public_retry result=scheduled reason=buffer_small native=passthrough");
  }
  return G_SOURCE_REMOVE;
}
static DWORD CALLBACK device_changed(HCMNOTIFICATION notification,PVOID context,CM_NOTIFY_ACTION action,PCM_NOTIFY_EVENT_DATA data,DWORD size) {
  (void)notification;(void)context;(void)data;(void)size;
  if(action==CM_NOTIFY_ACTION_DEVICEINSTANCESTARTED||action==CM_NOTIFY_ACTION_DEVICEINSTANCEREMOVED)
    g_idle_add(device_event,NULL);
  return ERROR_SUCCESS;
}
static void detached(FridaSession *session,FridaSessionDetachReason reason,FridaCrash *crash,gpointer unused) {
  (void)session;(void)crash;(void)unused;session_gone=TRUE;
  result("session_detached",reason);send_packet("{\"kind\":\"cancel\",\"reason\":\"session_detached\"}");
  if(loop)g_main_loop_quit(loop);
}
static void message(FridaScript *unused,const gchar *text,GBytes *data,gpointer user) {
  (void)unused;(void)data;(void)user;
  JsonParser *parser=json_parser_new();GError *error=NULL;
  if(!json_parser_load_from_data(parser,text,-1,&error)){g_clear_error(&error);g_object_unref(parser);begin_stop();return;}
  JsonObject *root=json_node_get_object(json_parser_get_root(parser));
  const char *type=json_object_get_string_member_with_default(root,"type","");
  if(strcmp(type,"send")!=0){
    const char *description=json_object_get_string_member_with_default(root,"description","");
    const char *category=strstr(description,"access violation")?"access_violation":
      g_str_has_prefix(description,"TypeError")?"TypeError":
      g_str_has_prefix(description,"ReferenceError")?"ReferenceError":
      g_str_has_prefix(description,"RangeError")?"RangeError":
      g_str_has_prefix(description,"SyntaxError")?"SyntaxError":"runtime_error";
    gint64 line=json_object_get_int_member_with_default(root,"lineNumber",0);
    gint64 column=json_object_get_int_member_with_default(root,"columnNumber",0);
    char summary[180];snprintf(summary,sizeof(summary),"phase=script_error type=%s class=%s line=%lld column=%lld",
      !strcmp(type,"error")?"error":!strcmp(type,"log")?"log":"unknown",category,
      line>0&&line<10000?line:0,column>0&&column<10000?column:0);audit(summary);
    begin_stop();g_object_unref(parser);return;
  }
  JsonNode *payload=json_object_get_member(root,"payload");
  if(!payload||!JSON_NODE_HOLDS_OBJECT(payload)){begin_stop();g_object_unref(parser);return;}
  JsonObject *object=json_node_get_object(payload);
  const char *kind=json_object_get_string_member_with_default(object,"kind","");
  gchar *encoded=json_to_string(payload,FALSE);
  if(!send_packet(encoded))begin_stop();
  g_free(encoded);
  if(strcmp(kind,"bound")==0){
    char command[160];snprintf(command,sizeof(command),"{\"type\":\"configure\",\"mask\":%u,\"configuration\":%lld}",requested_mask,configuration);
    frida_script_post(script,command,NULL);audit("phase=bound source=pending_per_request contract=1");
  }else if(strcmp(kind,"stopped")==0){
    cleanup_failed=json_object_get_int_member_with_default(object,"cleanupErrors",0)!=0;
    result("script_cleanup",cleanup_failed?1:0);stopped=TRUE;if(loop)g_main_loop_quit(loop);
  }
  g_object_unref(parser);
}
int wmain(int argc,WCHAR **argv) {
  int code=1;DWORD pid=0,parent_pid=0,length=0,session_id=0;WCHAR instance[1024],image[2048],expected[2048];
  HANDLE mutex=NULL,token=NULL;gchar *source=NULL,*config_json=NULL;
  FridaDeviceManager *manager=NULL;FridaDevice *device=NULL;FridaSession *session=NULL;GError *error=NULL;
  gboolean recover=FALSE;guint control_timer=0;gunichar2 *hid_path=NULL;
  if(argc!=3||wcslen(argv[2])!=32)return 64;
  WCHAR *end;parent_pid=wcstoul(argv[1],&end,10);if(!parent_pid||*end)return 64;
  for(unsigned i=0;i<32;i++){WCHAR c=argv[2][i];if(!((c>=L'0'&&c<=L'9')||(c>=L'a'&&c<=L'f')))return 64;nonce[i]=(char)c;}nonce[32]=0;
  audit_handle=RegisterEventSourceA(NULL,"SayAllInput");if(!audit_handle)return 1;
  if(!ProcessIdToSessionId(GetCurrentProcessId(),&session_id)||session_id!=WTSGetActiveConsoleSessionId())goto done;
  if(!OpenProcessToken(GetCurrentProcess(),TOKEN_ADJUST_PRIVILEGES|TOKEN_QUERY,&token))goto done;
  TOKEN_ELEVATION elevated={0};DWORD bytes=0;TOKEN_PRIVILEGES privileges={0};
  if(!GetTokenInformation(token,TokenElevation,&elevated,sizeof(elevated),&bytes)||!elevated.TokenIsElevated)goto done;
  privileges.PrivilegeCount=1;
  if(!LookupPrivilegeValueW(NULL,L"SeDebugPrivilege",&privileges.Privileges[0].Luid))goto done;
  privileges.Privileges[0].Attributes=SE_PRIVILEGE_ENABLED;SetLastError(0);
  if(!AdjustTokenPrivileges(token,FALSE,&privileges,0,NULL,NULL)||GetLastError()!=0)goto done;
  CloseHandle(token);token=NULL;
  mutex=CreateMutexW(NULL,TRUE,L"Global\\SayAllHidHostHelper");
  if(!mutex||GetLastError()==ERROR_ALREADY_EXISTS){result("helper_already_running",1);goto done;}
  WCHAR pipe_name[160];swprintf_s(pipe_name,160,L"\\\\.\\pipe\\SayAllHidHost-%lu-%s",parent_pid,argv[2]);
  pipe_handle=CreateFileW(pipe_name,GENERIC_READ|GENERIC_WRITE,0,NULL,OPEN_EXISTING,0,NULL);
  if(pipe_handle==INVALID_HANDLE_VALUE||!verify_parent(parent_pid))goto done;
  DWORD mode=PIPE_READMODE_MESSAGE;if(!SetNamedPipeHandleState(pipe_handle,&mode,NULL,NULL))goto done;
  frida_init();
  char packet[4096];if(!read_packet(packet,sizeof(packet)))goto done;
  JsonParser *parser=json_parser_new();
  if(!json_parser_load_from_data(parser,packet,-1,&error)){g_clear_error(&error);g_object_unref(parser);goto done;}
  if(!JSON_NODE_HOLDS_OBJECT(json_parser_get_root(parser))){g_object_unref(parser);goto done;}
  JsonObject *init=json_node_get_object(json_parser_get_root(parser));
  const char *kind=json_object_get_string_member_with_default(init,"kind","");
  const char *selected=json_object_get_string_member_with_default(init,"selected","");
  gint64 mask=json_object_get_int_member_with_default(init,"mask",-1);
  if(strcmp(kind,"init")||mask<0||mask>31||strlen(selected)>3072){g_object_unref(parser);goto done;}
  hid_path=g_utf8_to_utf16(selected,-1,NULL,NULL,NULL);requested_mask=(guint)mask;
  g_object_unref(parser);
reconnect_target:
  session_gone=FALSE;recover=FALSE;
  int discovered=host(&pid,instance,1024);
  gboolean contract=discovered==0&&hid_path&&descriptor((WCHAR*)hid_path,instance);
  if(!contract){result("activation_rejected",discovered);goto done;}
  wcscpy_s(selected_instance,1024,instance);
  if(!refresh_selected_pdo()){audit("phase=activation_rejected reason=source_public_binding");goto done;}
  CM_NOTIFY_FILTER filter={0};filter.cbSize=sizeof(filter);filter.FilterType=CM_NOTIFY_FILTER_TYPE_DEVICEINSTANCE;
  if(wcslen(instance)>=MAX_DEVICE_ID_LEN){result("notification_identity_size",1);goto done;}
  wcscpy_s(filter.u.DeviceInstance.InstanceId,MAX_DEVICE_ID_LEN,instance);
  if(CM_Register_Notification(&filter,NULL,device_changed,&device_notification)!=CR_SUCCESS){result("notification_unavailable",1);goto done;}
  target_process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,pid);length=2048;
  GetSystemDirectoryW(expected,2048);wcscat_s(expected,2048,L"\\WUDFHost.exe");
  if(!target_process||!QueryFullProcessImageNameW(target_process,0,image,&length)){result("host_process_rejected",GetLastError());goto done;}
  if(_wcsicmp(image,expected)){result("host_path_rejected",1);goto done;}
  if(!signed_file(expected)){result("host_signature_rejected",1);goto done;}
  if(!verify_modules(pid)){result("host_modules_rejected",1);goto done;}
  if(!runtime_prepare(target_process))goto done;
  if(!SetEnvironmentVariableW(L"TEMP",runtime_path)||!SetEnvironmentVariableW(L"TMP",runtime_path))goto done;
  loop=g_main_loop_new(NULL,FALSE);manager=frida_device_manager_new();
  if(!begin_operation())goto cleanup;
  device=frida_device_manager_get_device_by_type_sync(manager,FRIDA_DEVICE_TYPE_LOCAL,5000,operation,&error);end_operation();
  if(error||!device)goto cleanup;
  DWORD check_pid=0;WCHAR check_instance[1024];
  if(host(&check_pid,check_instance,1024)||check_pid!=pid||_wcsicmp(instance,check_instance)||
     WaitForSingleObject(target_process,0)!=WAIT_TIMEOUT)goto cleanup;
  if(!begin_operation())goto cleanup;
  session=frida_device_attach_sync(device,pid,NULL,operation,&error);end_operation();if(error||!session)goto cleanup;
  g_signal_connect(session,"detached",G_CALLBACK(detached),NULL);
  gchar *utf8=g_utf16_to_utf8((gunichar2*)selected_pdo,-1,NULL,NULL,NULL);
  JsonObject *config=json_object_new();json_object_set_string_member(config,"pdo",utf8);g_free(utf8);
  json_object_set_int_member(config,"contract",1);json_object_set_int_member(config,"maxLength",121);
  JsonNode *node=json_node_new(JSON_NODE_OBJECT);json_node_take_object(node,config);config_json=json_to_string(node,FALSE);json_node_free(node);
  gchar **parts=g_strsplit(SAYALL_RUNTIME_SCRIPT,"__SAYALL_CONFIG__",2);
  source=g_strconcat(parts[0],config_json,parts[1],NULL);g_strfreev(parts);
  FridaScriptOptions *options=frida_script_options_new();frida_script_options_set_runtime(options,FRIDA_SCRIPT_RUNTIME_QJS);
  frida_script_options_set_name(options,"sayall-rc003-input");
  if(!begin_operation()){g_object_unref(options);goto cleanup;}
  script=frida_session_create_script_sync(session,source,options,operation,&error);end_operation();g_object_unref(options);
  if(error||!script)goto cleanup;
  g_signal_connect(script,"message",G_CALLBACK(message),NULL);
  if(!begin_operation())goto cleanup;frida_script_load_sync(script,operation,&error);end_operation();if(error)goto cleanup;
  control_timer=g_timeout_add(10,control,NULL);g_main_loop_run(loop);code=(stopped||session_gone)&&!cleanup_failed?0:1;
  recover=session_gone&&!stopping;
cleanup:
  cancel_source_refresh();
  if(control_timer){g_source_remove(control_timer);control_timer=0;}
  if(device_notification){CM_Unregister_Notification(device_notification);device_notification=NULL;}
  if(error){result("frida_operation_failed",error->code);g_clear_error(&error);}
  if(script&&begin_operation()){frida_script_unload_sync(script,operation,&error);end_operation();result("script_unload",error?error->code:0);g_clear_error(&error);}
  if(session&&begin_operation()){frida_session_detach_sync(session,operation,&error);end_operation();result("session_detach",error?error->code:0);g_clear_error(&error);}
  if(script){g_object_unref(script);script=NULL;}if(session){g_object_unref(session);session=NULL;}if(device){g_object_unref(device);device=NULL;}
  if(manager){if(begin_operation()){frida_device_manager_close_sync(manager,operation,&error);end_operation();g_clear_error(&error);}g_object_unref(manager);}
  manager=NULL;if(loop){g_main_loop_unref(loop);loop=NULL;}g_free(source);source=NULL;g_free(config_json);config_json=NULL;
  if(recover){
    runtime_close();for(unsigned i=0;i<2;i++)if(module_locks[i]){CloseHandle(module_locks[i]);module_locks[i]=NULL;}
    if(target_process){CloseHandle(target_process);target_process=NULL;}
    audit("phase=reconnecting same_elevated_helper=true");
    while(!stopping){
      control(NULL);if(stopping)break;
      DEVINST node;ULONG state=0,problem=0;
      if(requested_mask&&CM_Locate_DevNodeW(&node,selected_instance,CM_LOCATE_DEVNODE_NORMAL)==CR_SUCCESS&&
         CM_Get_DevNode_Status(&state,&problem,node,0)==CR_SUCCESS&&(state&DN_STARTED))goto reconnect_target;
      Sleep(100);
    }
  }
done:
  if(device_notification)CM_Unregister_Notification(device_notification);
  g_free(hid_path);
  if(token)CloseHandle(token);runtime_close();
  for(unsigned i=0;i<2;i++)if(module_locks[i])CloseHandle(module_locks[i]);
  if(target_process)CloseHandle(target_process);if(parent_process)CloseHandle(parent_process);
  if(pipe_handle!=INVALID_HANDLE_VALUE)CloseHandle(pipe_handle);if(mutex){ReleaseMutex(mutex);CloseHandle(mutex);}
  result("terminal",code);DeregisterEventSource(audit_handle);return code;
}
