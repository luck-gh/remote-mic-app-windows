/* GPL-3.0-only. Bounded, target-specific diagnostic, not a generic injector.
 * Uses the official Frida 17.15.3 devkit attach/script/unload/detach API.
 * No caller PID, script, output path, report mutation or process termination. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <cfgmgr32.h>
#include <initguid.h>
#include <devpkey.h>
#include <wintrust.h>
#include <softpub.h>
#include <tlhelp32.h>
#include <aclapi.h>
#include <stdio.h>
#include <stdint.h>
#include "frida-core.h"
#include "hid_host_probe_script.h"
#include "hid_host_probe_thread.h"

static GMainLoop *loop;
static FridaScript *script;
static gboolean stopped;
static HANDLE audit_handle;
static char run_nonce[33];
static GCancellable *operation;
static HANDLE operation_timer;
static const char *operation_stage="none";
static ULONGLONG operation_started;
static void CALLBACK cancel_operation(PVOID context,BOOLEAN fired){(void)fired;g_cancellable_cancel((GCancellable*)context);}
static GCancellable *begin_operation(void){operation=g_cancellable_new();operation_timer=NULL;if(!CreateTimerQueueTimer(&operation_timer,NULL,cancel_operation,operation,10000,0,WT_EXECUTEONLYONCE)){g_object_unref(operation);operation=NULL;return NULL;}return operation;}
static void end_operation(void){if(operation_timer)DeleteTimerQueueTimer(NULL,operation_timer,INVALID_HANDLE_VALUE);if(operation)g_object_unref(operation);operation=NULL;operation_timer=NULL;}
static void audit(const char *text) {
  char line[768];snprintf(line,sizeof(line),"hid_host_probe run_id=%s probe_pid=%lu %s",run_nonce,GetCurrentProcessId(),strncmp(text,"hid_host_probe ",15)==0?text+15:text);
  const char *strings[] = {line};
  ReportEventA(audit_handle, EVENTLOG_INFORMATION_TYPE, 0, 1002, NULL, 1, 0, strings, NULL);
  puts(line); fflush(stdout);
}
static void result(const char *phase, int code) {
  char line[180]; snprintf(line,sizeof(line),"hid_host_probe phase=%s code=%d",phase,code);audit(line);
}
#include "hid_host_probe_runtime.h"
static void stage(const char *name){operation_stage=name;operation_started=GetTickCount64();char line[150];snprintf(line,sizeof(line),"phase=operation_begin stage=%s",name);audit(line);}
static void operation_result(GError *error){
  const char *domain="none",*category="success";
  if(error){domain=error->domain==FRIDA_ERROR?"frida":error->domain==G_IO_ERROR?"gio":"other";category="unclassified";
    if(strstr(error->message,"refused to load frida-agent"))category="agent_load_or_early_exit";
    else if(strstr(error->message,"connection is closed")||strstr(error->message,"Connection closed"))category="connection_closed";
    else if(strstr(error->message,"timed out"))category="timeout";
    else if(strstr(error->message,"cancelled"))category="cancelled";
    else if(strstr(error->message,"OpenProcess"))category="open_process";
    else if(strstr(error->message,"VirtualAllocEx"))category="remote_allocation";
    else if(strstr(error->message,"WriteProcessMemory"))category="remote_write";
    else if(strstr(error->message,"VirtualProtectEx"))category="remote_protect";
    else if(strstr(error->message,"CreateRemoteThread"))category="remote_thread";
  }
  char line[260];snprintf(line,sizeof(line),"phase=operation_result stage=%s domain=%s code=%d category=%s elapsed_ms=%llu",operation_stage,domain,error?error->code:0,category,(unsigned long long)(GetTickCount64()-operation_started));audit(line);
}
static void thread_result(const char *stage_name){SayAllThreadResult observed=sayall_thread_result();char line[300];snprintf(line,sizeof(line),"phase=worker_thread stage=%s matched=%lu observed=%d duplicate_error=%lu exited=%d wait=%lu query_error=%lu exit_code=%lu",stage_name,observed.matched,observed.observed,observed.duplicate_error,observed.exited,observed.wait_result,observed.query_error,observed.exit_code);audit(line);}
/* Only public metadata for the already identity-verified process. No memory contents. */
static void host_facts(HANDLE process,DWORD pid){
  char line[300];PROCESS_PROTECTION_LEVEL_INFORMATION protection={0};
  if(GetProcessInformation(process,ProcessProtectionLevelInfo,&protection,sizeof(protection)))result("host_protection_level",(int)protection.ProtectionLevel);else result("host_protection_query_failed",GetLastError());
  HANDLE query=OpenProcess(PROCESS_QUERY_INFORMATION,FALSE,pid);
  if(query){PROCESS_MITIGATION_DYNAMIC_CODE_POLICY dynamic={0};PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY signature={0};
    if(GetProcessMitigationPolicy(query,ProcessDynamicCodePolicy,&dynamic,sizeof(dynamic)))result("host_dynamic_code_flags",dynamic.Flags);else result("host_dynamic_code_query_failed",GetLastError());
    if(GetProcessMitigationPolicy(query,ProcessSignaturePolicy,&signature,sizeof(signature)))result("host_signature_policy_flags",signature.Flags);else result("host_signature_policy_query_failed",GetLastError());CloseHandle(query);
  }else result("host_mitigation_open_failed",GetLastError());
  HANDLE token=NULL,impersonation=NULL;
  if(OpenProcessToken(process,TOKEN_QUERY,&token)){
    DWORD size=0,appcontainer=0;BYTE user_buffer[512];
    BOOL user_ok=GetTokenInformation(token,TokenUser,user_buffer,sizeof(user_buffer),&size);
    BOOL app_ok=GetTokenInformation(token,TokenIsAppContainer,&appcontainer,sizeof(appcontainer),&size);
    snprintf(line,sizeof(line),"phase=host_token user_known=%d local_service=%d system=%d appcontainer_known=%d appcontainer=%lu restricted=%d",user_ok,user_ok&&IsWellKnownSid(((TOKEN_USER*)user_buffer)->User.Sid,WinLocalServiceSid),user_ok&&IsWellKnownSid(((TOKEN_USER*)user_buffer)->User.Sid,WinLocalSystemSid),app_ok,appcontainer,IsTokenRestricted(token));audit(line);
    WCHAR temp[MAX_PATH];PSECURITY_DESCRIPTOR descriptor=NULL;
    HANDLE duplicate_source=NULL;
    if(GetTempPathW(MAX_PATH,temp)&&OpenProcessToken(process,TOKEN_QUERY|TOKEN_DUPLICATE,&duplicate_source)&&DuplicateToken(duplicate_source,SecurityImpersonation,&impersonation)){
      DWORD status=GetNamedSecurityInfoW(temp,SE_FILE_OBJECT,DACL_SECURITY_INFORMATION|OWNER_SECURITY_INFORMATION|GROUP_SECURITY_INFORMATION,NULL,NULL,NULL,NULL,&descriptor);
      if(status==ERROR_SUCCESS){GENERIC_MAPPING mapping={FILE_GENERIC_READ,FILE_GENERIC_WRITE,FILE_GENERIC_EXECUTE,FILE_ALL_ACCESS};DWORD desired=FILE_GENERIC_READ|FILE_GENERIC_EXECUTE,granted=0,privilege_size=sizeof(PRIVILEGE_SET)+1024;BYTE privilege_buffer[sizeof(PRIVILEGE_SET)+1024];BOOL allowed=FALSE;
        BOOL checked=AccessCheck(descriptor,impersonation,desired,&mapping,(PPRIVILEGE_SET)privilege_buffer,&privilege_size,&granted,&allowed);
        snprintf(line,sizeof(line),"phase=caller_temp_host_access checked=%d read_execute=%d error=%lu actual_agent_path_observed=false",checked,allowed,checked?0:GetLastError());audit(line);LocalFree(descriptor);
      }else result("caller_temp_acl_query_failed",status);CloseHandle(impersonation);
    }else result("caller_temp_access_check_unavailable",GetLastError());if(duplicate_source)CloseHandle(duplicate_source);CloseHandle(token);
  }else result("host_token_query_failed",GetLastError());
  HANDLE snapshot=CreateToolhelp32Snapshot(TH32CS_SNAPMODULE|TH32CS_SNAPMODULE32,pid);
  if(snapshot!=INVALID_HANDLE_VALUE){MODULEENTRY32W module={sizeof(module)};DWORD count=0,frida=0;BOOL ok=Module32FirstW(snapshot,&module);
    if(ok)do{count++;WCHAR folded[MAX_MODULE_NAME32+1];wcscpy_s(folded,MAX_MODULE_NAME32+1,module.szModule);CharLowerBuffW(folded,(DWORD)wcslen(folded));if(wcsstr(folded,L"frida"))frida++;}while(Module32NextW(snapshot,&module));
    snprintf(line,sizeof(line),"phase=host_module_snapshot query_ok=%d modules=%lu frida_named_modules=%lu physical_unload_proven=false",ok,count,frida);audit(line);CloseHandle(snapshot);
  }else result("host_module_snapshot_failed",GetLastError());
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
  RegCloseKey(root);char discovery[250];snprintf(discovery,sizeof(discovery),"hid_host_probe phase=discovery_counts hid_services=%lu pid_values=%lu pid_rejected=%lu process_open_failed=%lu process_open_error=%lu process_not_live=%lu target_count=%lu",hid_services,pid_values,pid_rejected,open_failed,open_error,not_live,targets);audit(discovery);if(targets!=1){audit(targets==0?"hid_host_probe phase=discovery_rejected reason=no_valid_target":"hid_host_probe phase=discovery_rejected reason=multiple_targets");return 3;}
  DEVINST devnode;
  if(CM_Locate_DevNodeW(&devnode,selected,CM_LOCATE_DEVNODE_NORMAL)!=CR_SUCCESS)return 4;
  if(wcscpy_s(instance_id,instance_chars,selected)!=0)return 5;
  return 0;
}
/* CM lists only public interfaces belonging to this exact selected devnode.
 * Literal \??\ aliases are generated in JS. Never whitelist a resolved PDO or
 * QueryDosDevice target: several interfaces may share such an object. */
static gchar *interface_names(const WCHAR *instance_id) {
  JsonBuilder *builder=json_builder_new();json_builder_begin_array(builder);
  DWORD count=0,classes=0;BOOL complete=FALSE;gchar *json=NULL;
  WCHAR list[16384];
  for(ULONG index=0;index<4096;index++) {
    GUID guid;CONFIGRET cr=CM_Enumerate_Classes(index,&guid,CM_ENUMERATE_CLASSES_INTERFACE);
    if(cr==CR_NO_SUCH_VALUE){complete=TRUE;break;}
    if(cr==CR_INVALID_DATA)continue;
    if(cr!=CR_SUCCESS){result("interface_class_failed",cr);goto end;}
    classes++;BOOL read=FALSE;ULONG chars=0;
    for(int retry=0;retry<2;retry++) {
      cr=CM_Get_Device_Interface_List_SizeW(&chars,&guid,(DEVINSTID_W)instance_id,CM_GET_DEVICE_INTERFACE_LIST_PRESENT);
      if(cr!=CR_SUCCESS||chars<1||chars>16384){result("interface_size_failed",cr);goto end;}
      ZeroMemory(list,sizeof(list));
      cr=CM_Get_Device_Interface_ListW(&guid,(DEVINSTID_W)instance_id,list,chars,CM_GET_DEVICE_INTERFACE_LIST_PRESENT);
      if(cr==CR_BUFFER_SMALL)continue;
      if(cr!=CR_SUCCESS){result("interface_list_failed",cr);goto end;}
      read=TRUE;break;
    }
    if(!read){result("interface_list_changed",1);goto end;}
    for(ULONG offset=0;offset<chars&&list[offset];) {
      size_t length=wcsnlen_s(list+offset,chars-offset);
      if(length==0||length>1024||length>=chars-offset||wcsncmp(list+offset,L"\\\\?\\",4)!=0||++count>64){result("interface_shape_rejected",1);goto end;}
      gchar *name=g_utf16_to_utf8((const gunichar2*)(list+offset),(glong)length,NULL,NULL,NULL);
      if(!name){result("interface_encoding_rejected",1);goto end;}
      json_builder_add_string_value(builder,name);g_free(name);offset+=(ULONG)length+1;
    }
  }
  if(!complete||!count){result(complete?"interface_none":"interface_class_limit",1);goto end;}
  json_builder_end_array(builder);JsonNode *node=json_builder_get_root(builder);json=json_to_string(node,FALSE);json_node_free(node);
  char line[180];snprintf(line,sizeof(line),"phase=interface_inventory classes=%lu interfaces=%lu selected_devnode_only=true resolved_object_authorized=false",classes,count);audit(line);
end:
  g_object_unref(builder);return json;
}
static gboolean force_stop(gpointer data){(void)data;g_main_loop_quit(loop);return G_SOURCE_REMOVE;}
static gboolean stop(gpointer data){(void)data;if(script)frida_script_post(script,"{\"type\":\"stop\"}",NULL);g_timeout_add_seconds(5,force_stop,NULL);return G_SOURCE_REMOVE;}
static void detached(FridaSession *session,FridaSessionDetachReason reason,FridaCrash *crash,gpointer data){(void)session;(void)crash;(void)data;result("session_detached",reason);g_main_loop_quit(loop);}
static gint64 integer(JsonObject *object,const char *name){return json_object_has_member(object,name)?json_object_get_int_member(object,name):0;}
static void message(FridaScript *unused,const gchar *text,GBytes *data,gpointer user) {
  (void)unused;(void)data;(void)user;JsonParser *parser=json_parser_new();GError *error=NULL;
  if(!json_parser_load_from_data(parser,text,-1,&error)){g_clear_error(&error);g_object_unref(parser);return;}
  JsonObject *root=json_node_get_object(json_parser_get_root(parser));
  const char *type=json_object_get_string_member(root,"type");
  if(strcmp(type,"send")!=0){audit("hid_host_probe phase=script_error");g_main_loop_quit(loop);g_object_unref(parser);return;}
  JsonObject *payload=json_object_get_object_member(root,"payload");const char *kind=json_object_get_string_member(payload,"kind");char line[600];
  if(strcmp(kind,"ready")==0) audit("hid_host_probe phase=hook_ready metadata_only=true physical_report_verified=false");
  else if(strcmp(kind,"counters")==0||strcmp(kind,"stopped")==0) {
    snprintf(line,sizeof(line),"phase=%s metadata_only=true interfaces=%lld tracked=%lld open_calls=%lld exact=%lld unknown=%lld open_failed=%lld open_pending=%lld raced=%lld opened=%lld nested=%lld capacity=%lld close=%lld revoked=%lld cleared_close=%lld cleared_dup_all=%lld duplicate=%lld query_failed=%lld name_rejected=%lld ioctl_known=%lld ioctl_unknown=%lld synchronous=%lld pending=%lld failed=%lld",kind,
      (long long)integer(payload,"interfaces"),(long long)integer(payload,"tracked"),(long long)integer(payload,"openCalls"),(long long)integer(payload,"openExact"),(long long)integer(payload,"openUnknown"),(long long)integer(payload,"openFailed"),(long long)integer(payload,"openPending"),(long long)integer(payload,"openRaced"),(long long)integer(payload,"opened"),(long long)integer(payload,"nested"),(long long)integer(payload,"capacity"),(long long)integer(payload,"closeCalls"),(long long)integer(payload,"revoked"),(long long)integer(payload,"clearedClose"),(long long)integer(payload,"clearedDuplicate"),(long long)integer(payload,"duplicateCalls"),(long long)integer(payload,"queryFailed"),(long long)integer(payload,"nameRejected"),(long long)integer(payload,"ioctlKnown"),(long long)integer(payload,"ioctlUnknown"),(long long)integer(payload,"synchronous"),(long long)integer(payload,"pending"),(long long)integer(payload,"failed"));audit(line);
    if(strcmp(kind,"stopped")==0){stopped=TRUE;g_main_loop_quit(loop);}
  }
  g_object_unref(parser);
}
int main(int argc,char **argv) {
  int code=1;DWORD pid=0,length,session_id=0;WCHAR instance_id[1024]={0},path[1024],expected[1024];HANDLE process=NULL,mutex=NULL;
  FILETIME created,exit_time,kernel,user;FridaDeviceManager *manager=NULL;FridaDevice *device=NULL;FridaSession *session=NULL;GError *error=NULL;gchar *source=NULL,*interfaces_json=NULL;
  if(argc!=3||(strcmp(argv[1],"--inspect")!=0&&strcmp(argv[1],"--prepare")!=0&&strcmp(argv[1],"--capture")!=0)||strlen(argv[2])!=32)return 64;
  for(int i=0;i<32;i++)if(!((argv[2][i]>='0'&&argv[2][i]<='9')||(argv[2][i]>='a'&&argv[2][i]<='f')))return 64;
  strcpy_s(run_nonce,sizeof(run_nonce),argv[2]);
  audit_handle=RegisterEventSourceA(NULL,"SayAllInput");if(!audit_handle)return 1;
  mutex=CreateMutexW(NULL,TRUE,L"Global\\SayAllHidHostProbe");if(!mutex||GetLastError()==ERROR_ALREADY_EXISTS){result("mutex_rejected",1);goto done;}
  if(!ProcessIdToSessionId(GetCurrentProcessId(),&session_id)||session_id!=WTSGetActiveConsoleSessionId()){result("session_rejected",1);goto done;}
  HANDLE token;TOKEN_PRIVILEGES privileges={0};
  if(!OpenProcessToken(GetCurrentProcess(),TOKEN_ADJUST_PRIVILEGES|TOKEN_QUERY,&token)){result("privilege_open_failed",GetLastError());code=1;goto done;}
  TOKEN_ELEVATION elevation={0};DWORD elevation_size=0;if(!GetTokenInformation(token,TokenElevation,&elevation,sizeof(elevation),&elevation_size)||!elevation.TokenIsElevated){CloseHandle(token);result("elevated_token_required",1);code=1;goto done;}
  privileges.PrivilegeCount=1;if(!LookupPrivilegeValueW(NULL,L"SeDebugPrivilege",&privileges.Privileges[0].Luid)){CloseHandle(token);result("privilege_lookup_failed",GetLastError());code=1;goto done;}privileges.Privileges[0].Attributes=SE_PRIVILEGE_ENABLED;SetLastError(0);AdjustTokenPrivileges(token,FALSE,&privileges,0,NULL,NULL);DWORD privilege_error=GetLastError();CloseHandle(token);
  if(privilege_error){result("privilege_failed",privilege_error);code=1;goto done;}
  audit("hid_host_probe phase=privilege_ready elevated=true debug_enabled=true");
  code=host(&pid,instance_id,1024);if(code){result("host_discovery",code);goto done;}
  process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,pid);length=1024;
  if(!process||!QueryFullProcessImageNameW(process,0,path,&length)||!GetProcessTimes(process,&created,&exit_time,&kernel,&user)){result("host_identity",GetLastError());code=1;goto done;}
  GetSystemDirectoryW(expected,1024);wcscat_s(expected,1024,L"\\WUDFHost.exe");
  if(_wcsicmp(path,expected)!=0){result("host_path_rejected",1);code=1;goto done;}
  WINTRUST_FILE_INFO file={sizeof(file)};file.pcwszFilePath=expected;
  WINTRUST_DATA trust={sizeof(trust)};GUID action=WINTRUST_ACTION_GENERIC_VERIFY_V2;
  trust.dwUIChoice=WTD_UI_NONE;trust.fdwRevocationChecks=WTD_REVOKE_WHOLECHAIN;trust.dwUnionChoice=WTD_CHOICE_FILE;trust.pFile=&file;trust.dwStateAction=WTD_STATEACTION_VERIFY;trust.dwProvFlags=WTD_CACHE_ONLY_URL_RETRIEVAL;
  LONG trusted=WinVerifyTrust(NULL,&action,&trust);trust.dwStateAction=WTD_STATEACTION_CLOSE;WinVerifyTrust(NULL,&action,&trust);
  if(trusted!=ERROR_SUCCESS){result("host_signature_rejected",trusted);code=1;goto done;}
  char identity[180];snprintf(identity,sizeof(identity),"hid_host_probe phase=identity_verified target=rc003 pid=%lu caller_session=%lu system_path=true signed=true creation_high=%lu creation_low=%lu",pid,session_id,created.dwHighDateTime,created.dwLowDateTime);audit(identity);
  host_facts(process,pid);
  // The static devkit initializes GLib allocation/type dispatch in frida_init.
  // Inspect uses JsonBuilder too, but never creates a device/session or attaches.
  frida_init();
  if(strcmp(argv[1],"--inspect")==0){interfaces_json=interface_names(instance_id);code=interfaces_json?0:1;goto done;}
  if(!runtime_prepare(process)){code=1;goto done;}
  if(strcmp(argv[1],"--prepare")==0){code=0;goto done;}
  if(!SetEnvironmentVariableW(L"TEMP",runtime_path)||!SetEnvironmentVariableW(L"TMP",runtime_path)){result("runtime_process_environment_failed",GetLastError());code=1;goto done;}
  WCHAR temp_check[MAX_PATH];if(!GetTempPathW(MAX_PATH,temp_check)){result("runtime_process_temp_query_failed",GetLastError());code=1;goto done;}
  size_t temp_length=wcslen(temp_check);if(temp_length&&temp_check[temp_length-1]==L'\\')temp_check[temp_length-1]=0;
  if(_wcsicmp(temp_check,runtime_path)!=0){result("runtime_process_temp_mismatch",1);code=1;goto done;}
  audit("phase=runtime_process_temp_ready scope=helper_only persistent_environment_changed=false");
  sayall_thread_target_set(process);
  code=1;
  loop=g_main_loop_new(NULL,FALSE);manager=frida_device_manager_new();
  if(!begin_operation()){result("timer_failed",1);goto cleanup;}
  stage("local_device");device=frida_device_manager_get_device_by_type_sync(manager,FRIDA_DEVICE_TYPE_LOCAL,5000,operation,&error);end_operation();operation_result(error);
  if(error)goto frida_error;
  // The retained process handle prevents confusing an exit with a live target.
  if(WaitForSingleObject(process,0)!=WAIT_TIMEOUT){result("host_exited",1);code=1;goto cleanup;}
  if(!begin_operation()){result("timer_failed",1);goto cleanup;}
  DWORD current_pid=0;WCHAR current_instance[1024]={0};
  if(host(&current_pid,current_instance,1024)!=0||current_pid!=pid||_wcsicmp(current_instance,instance_id)!=0||WaitForSingleObject(process,0)!=WAIT_TIMEOUT){end_operation();result("pre_attach_identity_changed",1);goto cleanup;}
  interfaces_json=interface_names(instance_id);if(!interfaces_json){end_operation();goto cleanup;}
  stage("attach");session=frida_device_attach_sync(device,pid,NULL,operation,&error);end_operation();operation_result(error);thread_result("attach_return");if(error)goto frida_error;
  g_signal_connect(session,"detached",G_CALLBACK(detached),NULL);
  gchar **pieces=g_strsplit(SAYALL_PROBE_SCRIPT,"__SAYALL_INTERFACES_JSON__",2);source=g_strconcat(pieces[0],interfaces_json,pieces[1],NULL);g_strfreev(pieces);
  FridaScriptOptions *options=frida_script_options_new();frida_script_options_set_runtime(options,FRIDA_SCRIPT_RUNTIME_QJS);frida_script_options_set_name(options,"sayall-rc003-read-only-probe");
  if(!begin_operation()){g_object_unref(options);result("timer_failed",1);goto cleanup;}
  stage("create_script");script=frida_session_create_script_sync(session,source,options,operation,&error);end_operation();operation_result(error);g_object_unref(options);if(error)goto frida_error;
  g_signal_connect(script,"message",G_CALLBACK(message),NULL);if(!begin_operation()){result("timer_failed",1);goto cleanup;}stage("load_script");frida_script_load_sync(script,operation,&error);end_operation();operation_result(error);if(error)goto frida_error;
  audit("hid_host_probe phase=capture_started duration_seconds=120 metadata_only=true modifies_report=false");g_timeout_add_seconds(120,stop,NULL);g_main_loop_run(loop);code=stopped?0:1;
cleanup:
  if(script){if(begin_operation()){frida_script_unload_sync(script,operation,&error);end_operation();result("script_unload",error?error->code:0);if(error){code=1;g_clear_error(&error);}}else{result("script_cleanup_unconfirmed_timer_failed",1);code=1;}g_object_unref(script);script=NULL;}
  if(session){if(begin_operation()){frida_session_detach_sync(session,operation,&error);end_operation();result("session_detach",error?error->code:0);if(error){code=1;g_clear_error(&error);}}else{result("session_cleanup_unconfirmed_timer_failed",1);code=1;}g_object_unref(session);}
  if(device)g_object_unref(device);if(manager){if(begin_operation()){frida_device_manager_close_sync(manager,operation,&error);end_operation();result("manager_close",error?error->code:0);if(error){code=1;g_clear_error(&error);}}else{result("manager_cleanup_unconfirmed_timer_failed",1);code=1;}g_object_unref(manager);}if(loop)g_main_loop_unref(loop);g_free(source);
  audit("hid_host_probe phase=cleanup module_residency=unknown host_terminated=false");goto done;
frida_error: result("frida_failure",error?error->code:-1);g_clear_error(&error);code=1;goto cleanup;
done:
  g_free(interfaces_json);
  thread_result("terminal");sayall_thread_target_set(NULL);
  runtime_close();
  if(process)CloseHandle(process);if(mutex){ReleaseMutex(mutex);CloseHandle(mutex);}result("terminal",code);DeregisterEventSource(audit_handle);return code;
}
