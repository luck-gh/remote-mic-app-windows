/* GPL-3.0-only. One authorized B group, fixed RC003 host and property.
 * Reuses the production module/signature/runtime gates. No arbitrary arguments,
 * report access, mapping, device writes, or production helper replacement. */
#define wmain unused_production_entry
#include "../native/hid-host-helper/main.c"
#undef wmain
static HANDLE source_b_stop;
static gboolean source_b_finished;
static int source_b_code=1;
static volatile LONG source_b_generation=1;
static unsigned source_b_results,source_b_selected,source_b_other,source_b_unknown;
static struct {WCHAR pdo[1024];LONG generation;char signature[600];unsigned requests;} source_b_memos[64];
static unsigned source_b_memo_count,source_b_record_count;
static gboolean source_b_limit;
static HCMNOTIFICATION source_b_notification;
static const GUID source_b_le_device={0x781aee18,0x7733,0x4ce4,{0xad,0xd0,0x91,0xf4,0x1c,0x67,0xb5,0x92}};
typedef enum {B_UNKNOWN,B_EXACT,B_CHILD,B_PARENT,B_ANCESTOR,B_OTHER} BRelation;
static const char *source_b_policy(unsigned matches,BOOL current,BRelation relation,BOOL physical_parent) {
  if(matches!=1)return "unique_unproven";
  if(!current)return "generation_unproven";
  if(relation==B_EXACT)return "selected_function";
  if(relation==B_CHILD)return "selected_child";
  if(relation==B_PARENT&&physical_parent)return "selected_physical";
  if(relation==B_OTHER)return "other_device";
  return "boundary_unproven";
}
static int source_b_self_test(void) {
  struct {unsigned n;BOOL current;BRelation relation;BOOL physical;const char *expected;} cases[]={
    {1,TRUE,B_EXACT,FALSE,"selected_function"},{1,TRUE,B_CHILD,FALSE,"selected_child"},
    {1,TRUE,B_PARENT,TRUE,"selected_physical"},{1,TRUE,B_PARENT,FALSE,"boundary_unproven"},
    {1,TRUE,B_ANCESTOR,TRUE,"boundary_unproven"},{1,TRUE,B_OTHER,TRUE,"other_device"},
    {2,TRUE,B_EXACT,TRUE,"unique_unproven"},{0,TRUE,B_PARENT,TRUE,"unique_unproven"},
    {1,FALSE,B_EXACT,TRUE,"generation_unproven"},{1,FALSE,B_PARENT,TRUE,"generation_unproven"},
    {1,TRUE,B_UNKNOWN,TRUE,"boundary_unproven"}};
  for(unsigned i=0;i<sizeof(cases)/sizeof(cases[0]);i++)if(strcmp(source_b_policy(cases[i].n,cases[i].current,cases[i].relation,cases[i].physical),cases[i].expected))return 1;
  return 0;
}
static CONFIGRET pdo_of(DEVINST node,WCHAR value[1024],ULONG *bytes,ULONG *type) {
  *bytes=2048;*type=0;memset(value,0,2048);
  CONFIGRET status=CM_Get_DevNode_Registry_PropertyW(node,CM_DRP_PHYSICAL_DEVICE_OBJECT_NAME,type,value,bytes,0);
  if(status==CR_SUCCESS&&(*type!=REG_SZ||*bytes<4||*bytes>2048||(*bytes&1)||value[*bytes/2-1]!=0||wcslen(value)!=*bytes/2-1))return CR_INVALID_DATA;
  return status;
}
static BOOL is_ancestor(DEVINST ancestor,DEVINST node) {
  for(unsigned depth=0;depth<12;depth++){DEVINST parent;if(CM_Get_Parent(&parent,node,0)!=CR_SUCCESS)return FALSE;if(parent==ancestor)return TRUE;node=parent;}return FALSE;
}
static BOOL public_physical_device(DEVINST node,const WCHAR *instance,unsigned *interfaces) {
  ULONG chars=0;*interfaces=0;
  if(CM_Get_Device_Interface_List_SizeW(&chars,(LPGUID)&source_b_le_device,(DEVINSTID_W)instance,CM_GET_DEVICE_INTERFACE_LIST_PRESENT)!=CR_SUCCESS||chars<2||chars>65536)return FALSE;
  WCHAR *paths=g_new0(WCHAR,chars);BOOL valid=TRUE;
  if(CM_Get_Device_Interface_ListW((LPGUID)&source_b_le_device,(DEVINSTID_W)instance,paths,chars,CM_GET_DEVICE_INTERFACE_LIST_PRESENT)!=CR_SUCCESS)valid=FALSE;
  if(valid)for(WCHAR *path=paths;*path;path+=wcslen(path)+1){
    WCHAR id[1024]={0};DEVPROPTYPE type=0;ULONG bytes=sizeof(id);DEVINST owner;
    if(CM_Get_Device_Interface_PropertyW(path,&DEVPKEY_Device_InstanceId,&type,(PBYTE)id,&bytes,0)!=CR_SUCCESS||type!=DEVPROP_TYPE_STRING||bytes<4||bytes>sizeof(id)||(bytes&1)||id[bytes/2-1]!=0||
       _wcsicmp(id,instance)||CM_Locate_DevNodeW(&owner,id,CM_LOCATE_DEVNODE_NORMAL)!=CR_SUCCESS||owner!=node){valid=FALSE;break;}(*interfaces)++;
  }
  g_free(paths);return valid&&*interfaces==1;
}
static const char *compare_public_pdo(const WCHAR *value,LONG generation,char proof[300]) {
  DEVINST selected;WCHAR selected_pdo[1024];ULONG bytes=0,type=0,state=0,problem=0;
  CONFIGRET status=CM_Locate_DevNodeW(&selected,selected_instance,CM_LOCATE_DEVNODE_NORMAL);
  if(status==CR_SUCCESS)status=CM_Get_DevNode_Status(&state,&problem,selected,0);
  if(status==CR_SUCCESS&&!(state&DN_STARTED))status=CR_NO_SUCH_DEVNODE;
  if(status==CR_SUCCESS)status=pdo_of(selected,selected_pdo,&bytes,&type);
  snprintf(proof,300,"public_status=0x%08lx public_type=%lu public_bytes=%lu direct_match=%s",status,type,bytes,status==CR_SUCCESS&&!_wcsicmp(value,selected_pdo)?"true":"false");
  if(status!=CR_SUCCESS)return "selected_unavailable";
  // Public present-node reverse lookup establishes uniqueness, not a model/parent heuristic.
  ULONG count=0;status=CM_Get_Device_ID_List_SizeW(&count,NULL,CM_GETIDLIST_FILTER_PRESENT);
  if(status!=CR_SUCCESS||count==0||count>16*1024*1024)return "enumeration_unavailable";
  WCHAR *ids=g_new0(WCHAR,count);status=CM_Get_Device_ID_ListW(NULL,ids,count,CM_GETIDLIST_FILTER_PRESENT);
  unsigned matches=0,other=0;DEVINST matched=0;WCHAR matched_id[1024]={0};
  if(status==CR_SUCCESS)for(WCHAR *id=ids;*id;id+=wcslen(id)+1){
    DEVINST node;WCHAR pdo[1024];ULONG n=0,t=0;
    if(CM_Locate_DevNodeW(&node,id,CM_LOCATE_DEVNODE_NORMAL)!=CR_SUCCESS||pdo_of(node,pdo,&n,&t)!=CR_SUCCESS)continue;
    if(!_wcsicmp(value,pdo)){matches++;matched=node;wcscpy_s(matched_id,1024,id);}else other++;
  }
  g_free(ids);BRelation relation=B_UNKNOWN;unsigned interfaces=0;BOOL physical=FALSE,current=FALSE;DEVINST parent,fresh;
  if(status==CR_SUCCESS&&matches==1){
    if(matched==selected)relation=B_EXACT;
    else if(is_ancestor(selected,matched))relation=B_CHILD;
    else if(CM_Get_Parent(&parent,selected,0)==CR_SUCCESS&&parent==matched){relation=B_PARENT;physical=public_physical_device(matched,matched_id,&interfaces);}
    else if(is_ancestor(matched,selected))relation=B_ANCESTOR;
    else relation=B_OTHER;
    current=generation==InterlockedCompareExchange(&source_b_generation,0,0)&&
      CM_Locate_DevNodeW(&fresh,matched_id,CM_LOCATE_DEVNODE_NORMAL)==CR_SUCCESS&&fresh==matched&&
      CM_Get_DevNode_Status(&state,&problem,fresh,0)==CR_SUCCESS&&(state&DN_STARTED)&&
      CM_Locate_DevNodeW(&fresh,selected_instance,CM_LOCATE_DEVNODE_NORMAL)==CR_SUCCESS&&fresh==selected&&
      CM_Get_DevNode_Status(&state,&problem,fresh,0)==CR_SUCCESS&&(state&DN_STARTED);
  }
  const char *classification=source_b_policy(status==CR_SUCCESS?matches:0,current,relation,physical);
  size_t used=strlen(proof);snprintf(proof+used,300-used," reverse_status=0x%08lx exact_matches=%u relation=%u physical_interfaces=%u current=%s other_pdo_count=%u",status,matches,(unsigned)relation,interfaces,current?"true":"false",other);
  return classification;
}
static void source_b_message(FridaScript *unused,const gchar *text,GBytes *data,gpointer user) {
  (void)unused;(void)data;(void)user;JsonParser *parser=json_parser_new();GError *error=NULL;
  if(!json_parser_load_from_data(parser,text,-1,&error)){g_clear_error(&error);goto failure;}
  JsonNode *root_node=json_parser_get_root(parser);if(!JSON_NODE_HOLDS_OBJECT(root_node))goto failure;
  JsonObject *root=json_node_get_object(root_node);
  if(strcmp(json_object_get_string_member_with_default(root,"type",""),"send"))goto failure;
  JsonNode *payload=json_object_get_member(root,"payload");if(!payload||!JSON_NODE_HOLDS_OBJECT(payload))goto failure;
  JsonObject *object=json_node_get_object(payload);const char *kind=json_object_get_string_member_with_default(object,"kind","");
  if(!strcmp(kind,"bound")){char command[100];snprintf(command,sizeof(command),"{\"type\":\"device_reset\",\"generation\":%ld}",InterlockedCompareExchange(&source_b_generation,0,0));frida_script_post(script,command,NULL);audit("phase=source_b_bound report_access=none suppression=0 mapping=0");g_object_unref(parser);return;}
  if(!strcmp(kind,"stopped")){source_b_finished=TRUE;audit("phase=source_b_script_stopped");if(loop)g_main_loop_quit(loop);g_object_unref(parser);return;}
  if(strcmp(kind,"result"))goto failure;
  const char *reason=json_object_get_string_member_with_default(object,"reason","");
  const char *allowed[]={"queue_missing","device_missing","property_status","property_length","property_termination","property_embedded_null","property_value","contract_exception"};
  gboolean known=FALSE;for(unsigned i=0;i<sizeof(allowed)/sizeof(allowed[0]);i++)if(!strcmp(reason,allowed[i]))known=TRUE;if(!known)goto failure;
  guint32 status=(guint32)json_object_get_int_member_with_default(object,"status",0);
  guint32 bytes=(guint32)json_object_get_int_member_with_default(object,"bytes",0);char line[700],proof[300]="public_query=not_attempted";
  LONG generation=(LONG)json_object_get_int_member_with_default(object,"generation",0);
  gint64 sequence=json_object_get_int_member_with_default(object,"sequence",0);
  const char *classification="property_unproven";source_b_results++;
  gunichar2 *wide=NULL;
  if(!strcmp(reason,"property_value")&&status==0&&bytes>=4&&bytes<=2048){
    const char *pdo=json_object_get_string_member_with_default(object,"pdo","");
    if(strlen(pdo)<=4096){wide=g_utf8_to_utf16(pdo,-1,NULL,NULL,NULL);if(wide)classification=compare_public_pdo((WCHAR*)wide,generation,proof);}
  }
  // Only metadata memoization for log aggregation; public source proof above is recomputed.
  const WCHAR *identity=wide?(WCHAR*)wide:L"";unsigned memo=0;
  for(;memo<source_b_memo_count;memo++)if(source_b_memos[memo].generation==generation&&!_wcsicmp(source_b_memos[memo].pdo,identity))break;
  if(memo==source_b_memo_count&&source_b_memo_count<64){source_b_memo_count++;wcscpy_s(source_b_memos[memo].pdo,1024,identity);source_b_memos[memo].generation=generation;}
  snprintf(line,sizeof(line),"generation=%ld reason=%s status=0x%08x bytes=%u classification=%s %s",generation,reason,status,bytes,classification,proof);
  if(memo>=64){source_b_limit=TRUE;source_b_code=2;audit("phase=source_b_resource_limit experiment=incomplete source_result=unassigned");frida_script_post(script,"{\"type\":\"stop\"}",NULL);}
  else{
    source_b_memos[memo].requests++;
    if(strcmp(source_b_memos[memo].signature,line)){
      if(source_b_record_count>=512){source_b_limit=TRUE;source_b_code=2;audit("phase=source_b_resource_limit experiment=incomplete source_result=unassigned");frida_script_post(script,"{\"type\":\"stop\"}",NULL);}
      else{strcpy_s(source_b_memos[memo].signature,600,line);source_b_record_count++;char record[768];snprintf(record,sizeof(record),"phase=source_b_result sequence=%lld object=%u %s suppression=0 mapping=0",sequence,memo+1,line);audit(record);}
    }
  }
  g_free(wide);
  if(!strncmp(classification,"selected_",9)&&strcmp(classification,"selected_unavailable")){source_b_selected++;if(!source_b_limit)source_b_code=0;}
  else if(!strcmp(classification,"other_device"))source_b_other++;else source_b_unknown++;
  g_object_unref(parser);return;
failure:
  audit("phase=source_b_script_failure raw_details_omitted=true");source_b_finished=TRUE;if(loop)g_main_loop_quit(loop);g_object_unref(parser);
}
static gboolean source_b_control(gpointer unused) {
  (void)unused;
  if(source_b_finished||WaitForSingleObject(source_b_stop,0)==WAIT_OBJECT_0||WaitForSingleObject(target_process,0)!=WAIT_TIMEOUT){
    audit("phase=source_b_normal_stop");if(script&&!session_gone)frida_script_post(script,"{\"type\":\"stop\"}",NULL);else g_main_loop_quit(loop);return G_SOURCE_REMOVE;
  }return G_SOURCE_CONTINUE;
}
static DWORD CALLBACK source_b_changed(HCMNOTIFICATION notification,PVOID context,CM_NOTIFY_ACTION action,PCM_NOTIFY_EVENT_DATA data,DWORD size){
  (void)notification;(void)context;(void)data;(void)size;
  if(action==CM_NOTIFY_ACTION_DEVICEINSTANCESTARTED||action==CM_NOTIFY_ACTION_DEVICEINSTANCEREMOVED){
    LONG generation=InterlockedIncrement(&source_b_generation);char command[100];
    snprintf(command,sizeof(command),"{\"type\":\"device_reset\",\"generation\":%ld}",generation);
    if(script)frida_script_post(script,command,NULL);result("source_b_generation",generation);
  }return ERROR_SUCCESS;
}
int wmain(int argc,WCHAR **argv) {
  if(argc==2&&!wcscmp(argv[1],L"--self-test"))return source_b_self_test();if(argc!=1)return 64;
  DWORD pid=0,length=0,session_id=0;HANDLE mutex=NULL,token=NULL;guint control_timer=0;
  WCHAR image[2048],expected[2048];FridaDeviceManager *manager=NULL;FridaDevice *device=NULL;FridaSession *session=NULL;GError *error=NULL;
  GUID run;CoCreateGuid(&run);const unsigned char *raw=(const unsigned char*)&run;for(unsigned i=0;i<16;i++)sprintf_s(nonce+2*i,3,"%02x",raw[i]);
  audit_handle=RegisterEventSourceA(NULL,"SayAllInput");if(!audit_handle)return 1;
  if(!ProcessIdToSessionId(GetCurrentProcessId(),&session_id)||session_id!=WTSGetActiveConsoleSessionId())goto done;
  if(!OpenProcessToken(GetCurrentProcess(),TOKEN_ADJUST_PRIVILEGES|TOKEN_QUERY,&token))goto done;
  TOKEN_ELEVATION elevated={0};DWORD bytes=0;TOKEN_PRIVILEGES privileges={0};
  if(!GetTokenInformation(token,TokenElevation,&elevated,sizeof(elevated),&bytes)||!elevated.TokenIsElevated)goto done;
  privileges.PrivilegeCount=1;if(!LookupPrivilegeValueW(NULL,L"SeDebugPrivilege",&privileges.Privileges[0].Luid))goto done;
  privileges.Privileges[0].Attributes=SE_PRIVILEGE_ENABLED;SetLastError(0);
  if(!AdjustTokenPrivileges(token,FALSE,&privileges,0,NULL,NULL)||GetLastError()!=0)goto done;
  CloseHandle(token);token=NULL;mutex=CreateMutexW(NULL,TRUE,L"Global\\SayAllHidHostHelper");
  if(!mutex||GetLastError()==ERROR_ALREADY_EXISTS){result("source_b_overlap",1);goto done;}
  source_b_stop=CreateEventW(NULL,TRUE,FALSE,L"Local\\SayAllRc003SourceBStop");if(!source_b_stop||GetLastError()==ERROR_ALREADY_EXISTS)goto done;
  // The static devkit owns GLib allocation; initialize it before lock_hash uses GChecksum.
  frida_init();
  int discovered=host(&pid,selected_instance,1024);if(discovered){result("source_b_discovery",discovered);goto done;}
  CM_NOTIFY_FILTER filter={0};filter.cbSize=sizeof(filter);filter.FilterType=CM_NOTIFY_FILTER_TYPE_DEVICEINSTANCE;
  if(wcslen(selected_instance)>=MAX_DEVICE_ID_LEN)goto done;wcscpy_s(filter.u.DeviceInstance.InstanceId,MAX_DEVICE_ID_LEN,selected_instance);
  if(CM_Register_Notification(&filter,NULL,source_b_changed,&source_b_notification)!=CR_SUCCESS)goto done;
  target_process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION|SYNCHRONIZE,FALSE,pid);length=2048;
  GetSystemDirectoryW(expected,2048);wcscat_s(expected,2048,L"\\WUDFHost.exe");
  if(!target_process||!QueryFullProcessImageNameW(target_process,0,image,&length)||_wcsicmp(image,expected)||!signed_file(expected)||!verify_modules(pid)){audit("phase=source_b_host_rejected");goto done;}
  if(!runtime_prepare(target_process)||!SetEnvironmentVariableW(L"TEMP",runtime_path)||!SetEnvironmentVariableW(L"TMP",runtime_path))goto done;
  loop=g_main_loop_new(NULL,FALSE);manager=frida_device_manager_new();
  if(!begin_operation())goto cleanup;device=frida_device_manager_get_device_by_type_sync(manager,FRIDA_DEVICE_TYPE_LOCAL,5000,operation,&error);end_operation();if(error||!device)goto cleanup;
  DWORD check_pid=0;WCHAR check_instance[1024];
  if(host(&check_pid,check_instance,1024)||check_pid!=pid||_wcsicmp(selected_instance,check_instance)||WaitForSingleObject(target_process,0)!=WAIT_TIMEOUT)goto cleanup;
  if(!begin_operation())goto cleanup;session=frida_device_attach_sync(device,pid,NULL,operation,&error);end_operation();if(error||!session)goto cleanup;
  g_signal_connect(session,"detached",G_CALLBACK(detached),NULL);
  FridaScriptOptions *options=frida_script_options_new();frida_script_options_set_runtime(options,FRIDA_SCRIPT_RUNTIME_QJS);frida_script_options_set_name(options,"sayall-rc003-source-b");
  if(!begin_operation()){g_object_unref(options);goto cleanup;}
  script=frida_session_create_script_sync(session,SAYALL_RUNTIME_SCRIPT,options,operation,&error);end_operation();g_object_unref(options);if(error||!script)goto cleanup;
  g_signal_connect(script,"message",G_CALLBACK(source_b_message),NULL);
  if(!begin_operation())goto cleanup;frida_script_load_sync(script,operation,&error);end_operation();if(error)goto cleanup;
  if(!source_b_finished){control_timer=g_timeout_add(100,source_b_control,NULL);g_main_loop_run(loop);}
cleanup:
  if(source_b_notification){CM_Unregister_Notification(source_b_notification);source_b_notification=NULL;}
  if(control_timer&&g_main_context_find_source_by_id(NULL,control_timer))g_source_remove(control_timer);
  if(error){result("source_b_frida",error->code);g_clear_error(&error);}
  if(script&&begin_operation()){frida_script_unload_sync(script,operation,&error);end_operation();result("source_b_unload",error?error->code:0);g_clear_error(&error);}
  if(session&&begin_operation()){frida_session_detach_sync(session,operation,&error);end_operation();result("source_b_detach",error?error->code:0);g_clear_error(&error);}
  if(script)g_object_unref(script);if(session)g_object_unref(session);if(device)g_object_unref(device);
  if(manager){if(begin_operation()){frida_device_manager_close_sync(manager,operation,&error);end_operation();g_clear_error(&error);}g_object_unref(manager);}
  if(loop){g_main_loop_unref(loop);loop=NULL;}
done:
  if(source_b_notification)CM_Unregister_Notification(source_b_notification);
  if(token)CloseHandle(token);runtime_close();for(unsigned i=0;i<2;i++)if(module_locks[i])CloseHandle(module_locks[i]);
  if(target_process)CloseHandle(target_process);if(source_b_stop)CloseHandle(source_b_stop);if(mutex){ReleaseMutex(mutex);CloseHandle(mutex);}
  for(unsigned i=0;i<source_b_memo_count;i++){char count[140];snprintf(count,sizeof(count),"phase=source_b_object_count object=%u generation=%ld requests=%u",i+1,source_b_memos[i].generation,source_b_memos[i].requests);audit(count);}
  char summary[220];snprintf(summary,sizeof(summary),"phase=source_b_summary requests=%u selected=%u other=%u unknown=%u generation=%ld records=%u incomplete=%s suppression=0 mapping=0",source_b_results,source_b_selected,source_b_other,source_b_unknown,source_b_generation,source_b_record_count,source_b_limit?"true":"false");audit(summary);
  result("source_b_terminal",source_b_code);DeregisterEventSource(audit_handle);return source_b_code;
}
