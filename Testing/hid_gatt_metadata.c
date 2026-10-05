// GPL-3.0-only. One exact public service interface; metadata only, ordinary user.
#define UNICODE
#define _UNICODE
#include <windows.h>
#include <setupapi.h>
#include <bluetoothleapis.h>
#include <stdio.h>
#include <wchar.h>
#include <io.h>
#include <fcntl.h>

static const GUID service_class = {0x6e3bb679,0x4372,0x40c8,{0x9e,0xaa,0x45,0x09,0xdf,0x26,0x0c,0xd8}};
static ULONGLONG started;
static void note(const char *stage, HRESULT hr, unsigned count) {
    printf("{\"stage\":\"%s\",\"hresult\":\"0x%08lx\",\"count\":%u,\"elapsedMs\":%llu}\n",stage,(unsigned long)hr,count,GetTickCount64()-started);
    fflush(stdout);
}
static BOOL budget(void) { return GetTickCount64()-started < 30000; }
static BOOL uuid(const BTH_LE_UUID *u, USHORT short_id) {
    GUID full={short_id,0,0x1000,{0x80,0,0,0x80,0x5f,0x9b,0x34,0xfb}};
    return u->IsShortUuid ? u->Value.ShortUuid==short_id : IsEqualGUID(&u->Value.LongUuid,&full);
}
int wmain(void) {
    WCHAR path[4096], instance[4096]; DWORD needed=0; HANDLE file=INVALID_HANDLE_VALUE, token=NULL;
    HDEVINFO set=INVALID_HANDLE_VALUE; SP_DEVICE_INTERFACE_DATA iface={sizeof(iface)};
    SP_DEVINFO_DATA dev={sizeof(dev)}; PSP_DEVICE_INTERFACE_DETAIL_DATA_W detail=NULL;
    BTH_LE_GATT_CHARACTERISTIC *chars=NULL; BTH_LE_GATT_DESCRIPTOR *descs=NULL;
    TOKEN_ELEVATION elevation; DWORD bytes; USHORT count=0,actual=0; HRESULT hr; int result=1;
    BOOL iface_open=FALSE; started=GetTickCount64();
    if(!OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&token))goto done;
    if(!GetTokenInformation(token,TokenElevation,&elevation,sizeof(elevation),&bytes)||elevation.TokenIsElevated){note("ordinary_user_gate",E_ACCESSDENIED,0);goto done;}
    // Parent sends exactly one UTF-16 line over stdin, never command-line arguments.
    _setmode(_fileno(stdin),_O_U16TEXT);
    if(!fgetws(path,4096,stdin)) {note("stdin",E_INVALIDARG,0);goto done;}
    size_t n=wcslen(path); if(n<5||path[n-1]!=L'\n'){note("path_length",E_INVALIDARG,0);goto done;}
    path[--n]=0;if(n&&path[n-1]==L'\r')path[--n]=0;
    if(wcsncmp(path,L"\\\\?\\",4)!=0||!wcsstr(path,L"1812")){note("path_shape",E_INVALIDARG,0);goto done;}
    if(!budget())goto timed_out;
    set=SetupDiCreateDeviceInfoList(NULL,NULL);if(set==INVALID_HANDLE_VALUE){note("setup_create",HRESULT_FROM_WIN32(GetLastError()),0);goto done;}
    if(!budget())goto timed_out;
    if(!SetupDiOpenDeviceInterfaceW(set,path,0,&iface)){note("setup_open",HRESULT_FROM_WIN32(GetLastError()),0);goto done;}iface_open=TRUE;
    if(!IsEqualGUID(&iface.InterfaceClassGuid,&service_class)){note("service_class",E_INVALIDARG,0);goto done;}
    if(!budget())goto timed_out;
    if(SetupDiGetDeviceInterfaceDetailW(set,&iface,NULL,0,&needed,NULL)||GetLastError()!=ERROR_INSUFFICIENT_BUFFER||needed>65536||needed<sizeof(*detail)){note("detail_size",E_INVALIDARG,0);goto done;}
    detail=(PSP_DEVICE_INTERFACE_DETAIL_DATA_W)calloc(1,needed);if(!detail)goto done;detail->cbSize=sizeof(*detail);
    if(!budget())goto timed_out;
    if(!SetupDiGetDeviceInterfaceDetailW(set,&iface,detail,needed,NULL,&dev)){note("detail",HRESULT_FROM_WIN32(GetLastError()),0);goto done;}
    if(_wcsicmp(detail->DevicePath,path)!=0){note("exact_interface",E_INVALIDARG,0);goto done;}
    if(!budget())goto timed_out;
    if(!SetupDiGetDeviceInstanceIdW(set,&dev,instance,4096,NULL)){note("instance",HRESULT_FROM_WIN32(GetLastError()),0);goto done;}
    _wcslwr_s(instance,4096);
    if(!wcsstr(instance,L"dev_vid&012717_pid&32b8_rev&00a4")||!wcsstr(instance,L"1812")){note("instance_revision",E_INVALIDARG,0);goto done;}
    note("identity_exact_service",S_OK,1);
    if(!budget()){note("budget",HRESULT_FROM_WIN32(ERROR_TIMEOUT),0);goto done;}
    file=CreateFileW(detail->DevicePath,GENERIC_READ,FILE_SHARE_READ|FILE_SHARE_WRITE|FILE_SHARE_DELETE,NULL,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,NULL);
    if(file==INVALID_HANDLE_VALUE){note("create_file_readonly",HRESULT_FROM_WIN32(GetLastError()),0);goto done;}
    note("create_file_readonly",S_OK,1);
    if(!budget())goto timed_out;
    hr=BluetoothGATTGetCharacteristics(file,NULL,0,NULL,&count,BLUETOOTH_GATT_FLAG_NONE);note("characteristics_size",hr,count);
    if(hr==S_OK&&count==0){result=0;goto done;}
    if(hr!=HRESULT_FROM_WIN32(ERROR_MORE_DATA)||count==0||count>64)goto done;
    chars=(BTH_LE_GATT_CHARACTERISTIC*)calloc(count,sizeof(*chars));if(!chars)goto done;
    if(!budget())goto timed_out;
    hr=BluetoothGATTGetCharacteristics(file,NULL,count,chars,&actual,BLUETOOTH_GATT_FLAG_NONE);note("characteristics",hr,actual);
    if(hr!=S_OK||actual>count)goto done;
    for(USHORT i=0;i<actual;i++) {
        const BOOL report=uuid(&chars[i].CharacteristicUuid,0x2a4d),map=uuid(&chars[i].CharacteristicUuid,0x2a4b);
        if(!report&&!map)continue;
        note(report?"report_characteristic":"report_map_characteristic",S_OK,(chars[i].IsReadable?1:0)|(chars[i].IsNotifiable?2:0));
        USHORT dc=0,da=0;if(!budget())goto timed_out;
        hr=BluetoothGATTGetDescriptors(file,&chars[i],0,NULL,&dc,BLUETOOTH_GATT_FLAG_NONE);note("descriptors_size",hr,dc);
        if(hr==S_OK&&dc==0)continue;
        if(hr!=HRESULT_FROM_WIN32(ERROR_MORE_DATA)||dc==0||dc>32)goto done;
        descs=(BTH_LE_GATT_DESCRIPTOR*)calloc(dc,sizeof(*descs));if(!descs)goto done;
        if(!budget())goto timed_out;
        hr=BluetoothGATTGetDescriptors(file,&chars[i],dc,descs,&da,BLUETOOTH_GATT_FLAG_NONE);note("descriptors",hr,da);
        if(hr!=S_OK||da>dc)goto done;
        unsigned refs=0;for(USHORT j=0;j<da;j++)if(uuid(&descs[j].DescriptorUuid,0x2908))refs++;
        note("report_reference_count",S_OK,refs);free(descs);descs=NULL;
    }
    result=0;goto done;
timed_out: note("budget",HRESULT_FROM_WIN32(ERROR_TIMEOUT),0);
done:
    free(descs);free(chars);free(detail);
    if(file!=INVALID_HANDLE_VALUE&&!CloseHandle(file)){note("close_failed",HRESULT_FROM_WIN32(GetLastError()),0);result=1;}
    if(iface_open&&!SetupDiDeleteDeviceInterfaceData(set,&iface)){note("interface_cleanup_failed",HRESULT_FROM_WIN32(GetLastError()),0);result=1;}
    if(set!=INVALID_HANDLE_VALUE&&!SetupDiDestroyDeviceInfoList(set)){note("setup_cleanup_failed",HRESULT_FROM_WIN32(GetLastError()),0);result=1;}
    if(token)CloseHandle(token);note("terminal",result?E_FAIL:S_OK,result);return result;
}
