/* GPL-3.0-only. Fixed private runtime for the explicit elevated HID Helper.
 * Reuses the already verified LocalService read/execute-only runtime policy. */
#pragma once
#include <windows.h>
#include <shlobj.h>
#include <knownfolders.h>
#include <sddl.h>
#include <authz.h>
#include <aclapi.h>
static HANDLE runtime_locks[8];
static unsigned runtime_lock_count;
static WCHAR runtime_path[MAX_PATH];
static const DWORD runtime_replace_rights=DELETE|FILE_DELETE_CHILD|WRITE_DAC|WRITE_OWNER;
static void runtime_close(void){while(runtime_lock_count)CloseHandle(runtime_locks[--runtime_lock_count]);}
static BOOL runtime_access(AUTHZ_CLIENT_CONTEXT_HANDLE context,PSECURITY_DESCRIPTOR descriptor,DWORD *granted){
  BOOL present,defaulted;PACL acl;
  if(!GetSecurityDescriptorDacl(descriptor,&present,&acl,&defaulted)||!present||!acl){SetLastError(ERROR_INVALID_SECURITY_DESCR);return FALSE;}
  GENERIC_MAPPING mapping={FILE_GENERIC_READ,FILE_GENERIC_WRITE,FILE_GENERIC_EXECUTE,FILE_ALL_ACCESS};
  for(DWORD i=0;i<acl->AceCount;i++){ACCESS_ALLOWED_ACE *ace;if(!GetAce(acl,i,(void**)&ace))return FALSE;if(ace->Header.AceType!=ACCESS_ALLOWED_ACE_TYPE&&ace->Header.AceType!=ACCESS_DENIED_ACE_TYPE){SetLastError(ERROR_NOT_SUPPORTED);return FALSE;}MapGenericMask(&ace->Mask,&mapping);}
  AUTHZ_ACCESS_REQUEST request={0};request.DesiredAccess=MAXIMUM_ALLOWED;
  DWORD status=0,sacl=0;AUTHZ_ACCESS_REPLY reply={0};reply.ResultListLength=1;reply.GrantedAccessMask=granted;reply.Error=&status;reply.SaclEvaluationResults=&sacl;
  if(!AuthzAccessCheck(0,context,&request,NULL,descriptor,NULL,0,&reply,NULL))return FALSE;
  if(status!=ERROR_SUCCESS&&status!=ERROR_ACCESS_DENIED){SetLastError(status);return FALSE;}return TRUE;
}
static BOOL runtime_exact_acl(PSECURITY_DESCRIPTOR descriptor,BOOL directory){
  PSID owner;BOOL defaulted,present;PACL acl;SECURITY_DESCRIPTOR_CONTROL control;DWORD revision;
  if(!GetSecurityDescriptorOwner(descriptor,&owner,&defaulted)||!IsWellKnownSid(owner,WinBuiltinAdministratorsSid))return FALSE;
  if(!GetSecurityDescriptorControl(descriptor,&control,&revision)||(directory&&!(control&SE_DACL_PROTECTED)))return FALSE;
  if(!GetSecurityDescriptorDacl(descriptor,&present,&acl,&defaulted)||!present||!acl||acl->AceCount!=3)return FALSE;
  GENERIC_MAPPING mapping={FILE_GENERIC_READ,FILE_GENERIC_WRITE,FILE_GENERIC_EXECUTE,FILE_ALL_ACCESS};DWORD seen=0;
  for(DWORD i=0;i<acl->AceCount;i++){
    ACCESS_ALLOWED_ACE *ace;if(!GetAce(acl,i,(void**)&ace)||ace->Header.AceType!=ACCESS_ALLOWED_ACE_TYPE)return FALSE;
    if(directory&&ace->Header.AceFlags!=(OBJECT_INHERIT_ACE|CONTAINER_INHERIT_ACE))return FALSE;
    if(!directory&&(ace->Header.AceFlags&INHERIT_ONLY_ACE))return FALSE;
    DWORD mask=ace->Mask;MapGenericMask(&mask,&mapping);PSID sid=&ace->SidStart;DWORD bit,expected;
    if(IsWellKnownSid(sid,WinLocalSystemSid)){bit=1;expected=FILE_ALL_ACCESS;}
    else if(IsWellKnownSid(sid,WinBuiltinAdministratorsSid)){bit=2;expected=FILE_ALL_ACCESS;}
    else if(IsWellKnownSid(sid,WinLocalServiceSid)){bit=4;expected=FILE_GENERIC_READ|FILE_GENERIC_EXECUTE;}
    else return FALSE;
    if(mask!=expected||(seen&bit))return FALSE;seen|=bit;
  }return seen==7;
}
static BOOL runtime_lock_directory(const WCHAR *path,BOOL exact,BOOL volume_root,AUTHZ_CLIENT_CONTEXT_HANDLE user_context){
  if(volume_root&&(exact||wcslen(path)!=3||path[1]!=L':'||path[2]!=L'\\')){SetLastError(ERROR_BAD_PATHNAME);return FALSE;}
  if(runtime_lock_count>=8){SetLastError(ERROR_BUFFER_OVERFLOW);return FALSE;}
  HANDLE directory=CreateFileW(path,FILE_READ_ATTRIBUTES|READ_CONTROL,FILE_SHARE_READ,NULL,OPEN_EXISTING,FILE_FLAG_BACKUP_SEMANTICS|FILE_FLAG_OPEN_REPARSE_POINT,NULL);
  if(directory==INVALID_HANDLE_VALUE){result("runtime_directory_open_failed",GetLastError());return FALSE;}
  FILE_ATTRIBUTE_TAG_INFO info;PSECURITY_DESCRIPTOR descriptor=NULL;BOOL valid=FALSE;DWORD error=ERROR_ACCESS_DENIED;
  if(!GetFileInformationByHandleEx(directory,FileAttributeTagInfo,&info,sizeof(info))){error=GetLastError();goto done;}
  if(!(info.FileAttributes&FILE_ATTRIBUTE_DIRECTORY)||(info.FileAttributes&FILE_ATTRIBUTE_REPARSE_POINT)){error=ERROR_REPARSE_TAG_INVALID;goto done;}
  error=GetSecurityInfo(directory,SE_FILE_OBJECT,OWNER_SECURITY_INFORMATION|GROUP_SECURITY_INFORMATION|DACL_SECURITY_INFORMATION,NULL,NULL,NULL,NULL,&descriptor);if(error)goto done;
  if(exact&&!runtime_exact_acl(descriptor,TRUE)){error=ERROR_INVALID_ACL;goto done;}
  DWORD granted=0;if(!runtime_access(user_context,descriptor,&granted)){error=GetLastError();goto done;}
  if(!volume_root&&(granted&runtime_replace_rights)){char line[160];snprintf(line,sizeof(line),"phase=runtime_ordinary_replacement_rejected exact_product=%d granted_mask=%lu replacement_mask=%lu",exact,granted,granted&runtime_replace_rights);audit(line);error=ERROR_ACCESS_DENIED;goto done;}
  if(volume_root){char line[180];snprintf(line,sizeof(line),"phase=runtime_volume_object_locked volume_acl_safe_claim=false ordinary_replacement_mask=%lu child_ancestor_checks_required=true",granted&runtime_replace_rights);audit(line);}
  valid=TRUE;runtime_locks[runtime_lock_count++]=directory;
done:
  if(descriptor)LocalFree(descriptor);if(!valid)CloseHandle(directory);SetLastError(error);return valid;
}
static BOOL runtime_prepare(HANDLE host_process){
  BOOL success=FALSE;DWORD error=0,size=0,created=0;HANDLE self_token=NULL,host_token=NULL,fixture=INVALID_HANDLE_VALUE;TOKEN_LINKED_TOKEN linked={0};PWSTR common=NULL;PSECURITY_DESCRIPTOR desired=NULL,actual=NULL;
  AUTHZ_RESOURCE_MANAGER_HANDLE manager=NULL;AUTHZ_CLIENT_CONTEXT_HANDLE user_context=NULL,host_context=NULL;LUID luid={0};
  if(!OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&self_token)||!GetTokenInformation(self_token,TokenLinkedToken,&linked,sizeof(linked),&size)||!OpenProcessToken(host_process,TOKEN_QUERY,&host_token)){error=GetLastError();goto done;}
  BYTE token_user[512];if(!GetTokenInformation(host_token,TokenUser,token_user,sizeof(token_user),&size)||!IsWellKnownSid(((TOKEN_USER*)token_user)->User.Sid,WinLocalServiceSid)){error=ERROR_ACCESS_DENIED;goto done;}
  if(!AuthzInitializeResourceManager(AUTHZ_RM_FLAG_NO_AUDIT,NULL,NULL,NULL,L"SayAllRuntimeAccess",&manager)||!AuthzInitializeContextFromToken(0,linked.LinkedToken,manager,NULL,luid,NULL,&user_context)||!AuthzInitializeContextFromToken(0,host_token,manager,NULL,luid,NULL,&host_context)){error=GetLastError();goto done;}
  HRESULT hr=SHGetKnownFolderPath(&FOLDERID_ProgramData,0,NULL,&common);if(FAILED(hr)){error=(DWORD)hr;goto done;}
  if(wcslen(common)<3||common[1]!=L':'||common[2]!=L'\\'||wcslen(common)+60>=MAX_PATH){error=ERROR_BAD_PATHNAME;goto done;}
  WCHAR path[MAX_PATH];wcscpy_s(path,MAX_PATH,common);
  WCHAR volume[4]={path[0],L':',L'\\',0};
  if(!runtime_lock_directory(volume,FALSE,TRUE,user_context)){error=GetLastError();result("runtime_volume_rejected",error);goto done;}
  /* Lock each ancestor from volume to ProgramData, rejecting any reparse point. */
  for(WCHAR *cursor=path+3;;cursor++){if(*cursor==L'\\'||*cursor==0){WCHAR saved=*cursor;*cursor=0;BOOL locked=runtime_lock_directory(path,FALSE,FALSE,user_context);*cursor=saved;if(!locked){error=GetLastError();result("runtime_ancestor_rejected",error);goto done;}if(!saved)break;}}
  if(!ConvertStringSecurityDescriptorToSecurityDescriptorW(L"O:BAG:BAD:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;LS)",SDDL_REVISION_1,&desired,NULL)){error=GetLastError();goto done;}
  SECURITY_ATTRIBUTES attributes={sizeof(attributes),desired,FALSE};const WCHAR *parts[]={L"\\SayAll",L"\\HidHostRuntime",L"\\17.15.3"};
  for(unsigned i=0;i<3;i++){
    wcscat_s(path,MAX_PATH,parts[i]);BOOL made=CreateDirectoryW(path,&attributes);DWORD create_error=made?0:GetLastError();
    if(!made&&create_error!=ERROR_ALREADY_EXISTS){error=create_error;result("runtime_create_failed",error);goto done;}
    if(!runtime_lock_directory(path,TRUE,FALSE,user_context)){error=GetLastError();result("runtime_existing_or_created_acl_rejected",error);goto done;}
    created+=made?1:0;result(made?"runtime_directory_created":"runtime_directory_reused",i);
  }
  wcscpy_s(runtime_path,MAX_PATH,path);wcscat_s(path,MAX_PATH,L"\\permission-check.bin");
  fixture=CreateFileW(path,GENERIC_READ|GENERIC_WRITE|READ_CONTROL,FILE_SHARE_READ,NULL,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,NULL);
  if(fixture==INVALID_HANDLE_VALUE&&GetLastError()==ERROR_FILE_EXISTS)fixture=CreateFileW(path,GENERIC_READ|READ_CONTROL,FILE_SHARE_READ,NULL,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,NULL);
  if(fixture==INVALID_HANDLE_VALUE){error=GetLastError();goto done;}
  FILE_ATTRIBUTE_TAG_INFO file_info;LARGE_INTEGER file_size;
  if(!GetFileInformationByHandleEx(fixture,FileAttributeTagInfo,&file_info,sizeof(file_info))||(file_info.FileAttributes&(FILE_ATTRIBUTE_REPARSE_POINT|FILE_ATTRIBUTE_DIRECTORY))||!GetFileSizeEx(fixture,&file_size)||file_size.QuadPart!=0){error=ERROR_INVALID_DATA;goto done;}
  error=GetSecurityInfo(fixture,SE_FILE_OBJECT,OWNER_SECURITY_INFORMATION|GROUP_SECURITY_INFORMATION|DACL_SECURITY_INFORMATION,NULL,NULL,NULL,NULL,&actual);if(error)goto done;
  if(!runtime_exact_acl(actual,FALSE)){error=ERROR_INVALID_ACL;result("runtime_generated_file_acl_rejected",error);goto done;}
  DWORD granted=0;if(!runtime_access(host_context,actual,&granted)){error=GetLastError();goto done;}
  if((granted&(FILE_GENERIC_READ|FILE_GENERIC_EXECUTE))!=(FILE_GENERIC_READ|FILE_GENERIC_EXECUTE)||(granted&(FILE_WRITE_DATA|FILE_APPEND_DATA|FILE_WRITE_ATTRIBUTES|FILE_WRITE_EA|DELETE|WRITE_DAC|WRITE_OWNER))){error=ERROR_ACCESS_DENIED;goto done;}
  audit("phase=runtime_access_verified host_token=LocalService host_read_execute=true host_write=false ordinary_user_replace=false fixture=empty_preserved no_security_product_change=true");success=TRUE;
done:
  if(actual)LocalFree(actual);if(desired)LocalFree(desired);if(common)CoTaskMemFree(common);if(fixture!=INVALID_HANDLE_VALUE)CloseHandle(fixture);
  if(host_context)AuthzFreeContext(host_context);if(user_context)AuthzFreeContext(user_context);if(manager)AuthzFreeResourceManager(manager);
  if(host_token)CloseHandle(host_token);if(linked.LinkedToken)CloseHandle(linked.LinkedToken);if(self_token)CloseHandle(self_token);
  if(!success){runtime_close();result("runtime_prepare_failed",error);}result("runtime_created_directory_count",created);return success;
}
