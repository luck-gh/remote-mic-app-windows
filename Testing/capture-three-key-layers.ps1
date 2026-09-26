param(
    [Parameter(Mandatory = $true)][string]$OutFile,
    [ValidateRange(0, 600)][int]$Seconds = 60
)
$ErrorActionPreference = 'Stop'
# Passive, separate-process probe. No SendInput, suppression, BLE writes or
# configuration access. Device paths remain in memory and are never logged.
Add-Type @'
using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using Microsoft.Win32.SafeHandles;
public static class ThreeKeyLayers {
 [StructLayout(LayoutKind.Sequential)] struct Device { public IntPtr Handle; public uint Type; }
 [StructLayout(LayoutKind.Sequential)] struct Registration { public ushort Page,Usage; public uint Flags; public IntPtr Window; }
 [StructLayout(LayoutKind.Sequential)] struct Header { public uint Type,Size; public IntPtr Device,Param; }
 [StructLayout(LayoutKind.Sequential)] struct Keyboard { public ushort Make,Flags,Reserved,Vk; public uint Message,Extra; }
 [StructLayout(LayoutKind.Sequential)] struct HookKey { public uint Vk,Scan,Flags,Time; public UIntPtr Extra; }
 [StructLayout(LayoutKind.Sequential)] struct WindowClass { public uint Style; public IntPtr Proc; public int ClassExtra,WindowExtra; public IntPtr Instance,Icon,Cursor,Background,Menu,Name; }
 [StructLayout(LayoutKind.Sequential)] struct Message { public IntPtr Window; public uint Id; public UIntPtr WParam; public IntPtr LParam; public uint Time; public int X,Y; public uint Private; }
 delegate IntPtr WindowProc(IntPtr w,uint m,UIntPtr p,IntPtr l);
 delegate IntPtr HookProc(int c,UIntPtr p,IntPtr l);
 static WindowProc windowProc = Window;
 static HookProc hookProc = Hook;
 static StreamWriter output;
 static Stopwatch clock;
 static int rawCount,hookCount;
 [DllImport("user32.dll", SetLastError=true)] static extern uint GetRawInputDeviceList(IntPtr p,ref uint n,uint size);
 [DllImport("user32.dll", SetLastError=true)] static extern uint GetRawInputDeviceInfoW(IntPtr h,uint cmd,IntPtr p,ref uint n);
 [DllImport("user32.dll", SetLastError=true)] static extern uint GetRawInputData(IntPtr h,uint cmd,IntPtr p,ref uint n,uint size);
 [DllImport("user32.dll", SetLastError=true)] static extern bool RegisterRawInputDevices(Registration[] r,uint n,uint size);
 [DllImport("user32.dll", SetLastError=true)] static extern ushort RegisterClassW(ref WindowClass c);
 [DllImport("user32.dll", SetLastError=true)] static extern IntPtr CreateWindowExW(uint ex,IntPtr c,IntPtr title,uint style,int x,int y,int width,int height,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr param);
 [DllImport("user32.dll")] static extern IntPtr DefWindowProcW(IntPtr w,uint m,UIntPtr p,IntPtr l);
 [DllImport("user32.dll")] static extern bool PeekMessageW(out Message m,IntPtr w,uint min,uint max,uint remove);
 [DllImport("user32.dll")] static extern IntPtr DispatchMessageW(ref Message m);
 [DllImport("user32.dll")] static extern bool DestroyWindow(IntPtr w);
 [DllImport("user32.dll")] static extern bool UnregisterClassW(IntPtr c,IntPtr i);
 [DllImport("user32.dll", SetLastError=true)] static extern IntPtr SetWindowsHookExW(int id,HookProc p,IntPtr module,uint thread);
 [DllImport("user32.dll")] static extern IntPtr CallNextHookEx(IntPtr h,int c,UIntPtr p,IntPtr l);
 [DllImport("user32.dll")] static extern bool UnhookWindowsHookEx(IntPtr h);
 [DllImport("kernel32.dll")] static extern IntPtr GetModuleHandleW(IntPtr n);
 [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern SafeFileHandle CreateFileW(string p,uint access,uint share,IntPtr security,uint disposition,uint flags,IntPtr template);
 [DllImport("hid.dll")] static extern bool HidD_GetPreparsedData(SafeFileHandle h,out IntPtr p);
 [DllImport("hid.dll")] static extern bool HidD_FreePreparsedData(IntPtr p);
 [DllImport("hid.dll")] static extern int HidP_GetCaps(IntPtr p,IntPtr caps);
 [DllImport("hid.dll")] static extern int HidP_GetButtonCaps(int type,IntPtr caps,ref ushort count,IntPtr p);
 [DllImport("hid.dll")] static extern int HidP_InitializeReportForID(int type,byte id,IntPtr p,byte[] report,uint length);
 [DllImport("hid.dll")] static extern int HidP_SetUsages(int type,ushort page,ushort link,ushort[] usages,ref uint count,IntPtr p,byte[] report,uint length);
 static void Log(string s) { output.WriteLine("utc="+DateTime.UtcNow.ToString("o")+" elapsed_ms="+clock.ElapsedMilliseconds+" "+s); output.Flush(); }
 static string Name(IntPtr h) {
   uint n=0; GetRawInputDeviceInfoW(h,0x20000007,IntPtr.Zero,ref n);
   if(n==0 || n>32768) return "";
   IntPtr p=Marshal.AllocHGlobal(checked((int)(n+1)*2));
   try { return GetRawInputDeviceInfoW(h,0x20000007,p,ref n)==uint.MaxValue ? "" : Marshal.PtrToStringUni(p).ToLowerInvariant(); }
   finally { Marshal.FreeHGlobal(p); }
 }
 static bool IsRemote(string p) { return ((p.Contains("vid_2717")&&p.Contains("pid_32b8")) || ((p.Contains("dev_vid&012717")||p.Contains("dev_vid&002717"))&&p.Contains("pid&32b8"))); }
 static bool Relevant(uint vk) { return vk==0xFF || vk==0xA6 || vk==0xAE || vk==0xAF || vk==0x1B || (vk>=0x7C&&vk<=0x87); }
 static IntPtr Hook(int c,UIntPtr p,IntPtr l) {
   if(c>=0) { var k=(HookKey)Marshal.PtrToStructure(l,typeof(HookKey));
     if(Relevant(k.Vk)) { hookCount++; Log("layer=keyboard_hook source=unattributed vk="+k.Vk+" scan="+k.Scan+" down="+((k.Flags&0x80)==0)+" injected="+((k.Flags&0x10)!=0)+" verdict=pass"); }
   }
   return CallNextHookEx(IntPtr.Zero,c,p,l);
 }
 static IntPtr Window(IntPtr w,uint m,UIntPtr p,IntPtr l) {
   if(m==0xFF) {
     uint n=0,hs=(uint)Marshal.SizeOf(typeof(Header));
     if(GetRawInputData(l,0x10000003,IntPtr.Zero,ref n,hs)!=uint.MaxValue && n>=hs && n<=65536) {
       IntPtr b=Marshal.AllocHGlobal((int)n);
       try { if(GetRawInputData(l,0x10000003,b,ref n,hs)==n) {
         var h=(Header)Marshal.PtrToStructure(b,typeof(Header));
         if(IsRemote(Name(h.Device))) {
           rawCount++;
           if(h.Type==1 && n>=hs+16) { var k=(Keyboard)Marshal.PtrToStructure(IntPtr.Add(b,(int)hs),typeof(Keyboard)); Log("layer=raw_keyboard source=xiaomi vk="+k.Vk+" scan="+k.Make+" flags="+k.Flags+" down="+((k.Flags&1)==0)); }
           else if(h.Type==2 && n>=hs+8) { uint len=(uint)Marshal.ReadInt32(b,(int)hs),count=(uint)Marshal.ReadInt32(b,(int)hs+4); Log("layer=raw_hid source=xiaomi report_bytes="+len+" report_count="+count+" body_bytes="+(n-hs-8)); }
         }
       }} finally { Marshal.FreeHGlobal(b); }
     }
   }
   return DefWindowProcW(w,m,p,l);
 }
 static void Inspect() {
   uint n=0,size=(uint)Marshal.SizeOf(typeof(Device));
   if(GetRawInputDeviceList(IntPtr.Zero,ref n,size)==uint.MaxValue) throw new InvalidOperationException("enumeration_failed");
   IntPtr p=Marshal.AllocHGlobal(checked((int)(n*size))); int matched=0;
   try {
     uint got=GetRawInputDeviceList(p,ref n,size); if(got==uint.MaxValue) throw new InvalidOperationException("enumeration_failed");
     for(int i=0;i<got;i++) {
       var d=(Device)Marshal.PtrToStructure(IntPtr.Add(p,i*(int)size),typeof(Device)); string path=Name(d.Handle); if(!IsRemote(path)) continue;
       matched++; Log("layer=enumeration source=xiaomi slot="+matched+" raw_type="+d.Type+" model=not_inferred_from_shared_vid_pid");
       using(var f=CreateFileW(path,0,3,IntPtr.Zero,3,0,IntPtr.Zero)) {
         IntPtr pp;
         if(f.IsInvalid || !HidD_GetPreparsedData(f,out pp)) { Log("layer=descriptor result=unavailable error="+Marshal.GetLastWin32Error()); continue; }
         try {
           IntPtr caps=Marshal.AllocHGlobal(64);
           try {
             int status=HidP_GetCaps(pp,caps); Log("layer=descriptor result="+status+" usage="+(ushort)Marshal.ReadInt16(caps,0)+" page="+(ushort)Marshal.ReadInt16(caps,2)+" input_report_bytes="+(ushort)Marshal.ReadInt16(caps,4));
             if(status==0x110000) {
               uint length=(ushort)Marshal.ReadInt16(caps,4);
               if(length>0 && length<=4096) {
                 var report=new byte[length]; var usages=new ushort[]{0xF1,0x80,0x81,0x52,0x3E,0x28}; uint usageCount=6;
                 int init=HidP_InitializeReportForID(0,1,pp,report,length);
                 int set=init==0x110000?HidP_SetUsages(0,7,0,usages,ref usageCount,pp,report,length):init;
                 bool exact=set==0x110000&&usageCount==6&&length==121&&report[0]==1&&report[1]==0&&report[2]==0;
                 for(int k=0;k<6&&exact;k++) exact=report[3+k]==usages[k];
                 for(int k=9;k<length&&exact;k++) exact=report[k]==0;
                 Log("layer=descriptor synthetic_parser_only=true physical_reports=0 init_status="+init+" set_status="+set+" usages="+usageCount+" six_byte_array_contract="+exact);
                 report=new byte[length];usages=new ushort[]{0xF1,0x80,0x81};usageCount=3;
                 init=HidP_InitializeReportForID(0,1,pp,report,length);
                 set=init==0x110000?HidP_SetUsages(0,7,0,usages,ref usageCount,pp,report,length):init;
                 exact=set==0x110000&&usageCount==3&&length==121&&report[0]==1;
                 for(int k=0;k<3&&exact;k++) exact=report[1+k*2]==usages[k]&&report[2+k*2]==0;
                 for(int k=7;k<length&&exact;k++) exact=report[k]==0;
                 Log("layer=descriptor synthetic_parser_only=true physical_reports=0 init_status="+init+" set_status="+set+" usages="+usageCount+" three_word_array_contract="+exact);
                 Log("layer=descriptor synthetic_only_prefix="+BitConverter.ToString(report,0,Math.Min(12,report.Length)));
               }
               ushort count=(ushort)Marshal.ReadInt16(caps,46);
               if(count>0 && count<128) { IntPtr bc=Marshal.AllocHGlobal(count*72);
                 try { if(HidP_GetButtonCaps(0,bc,ref count,pp)==0x110000) for(int j=0;j<count;j++) {
                   IntPtr c=IntPtr.Add(bc,j*72); bool range=Marshal.ReadByte(c,12)!=0;
                   Log("layer=descriptor button_cap=true page="+(ushort)Marshal.ReadInt16(c,0)+" report_id="+Marshal.ReadByte(c,2)+" range="+range+" usage_min="+(ushort)Marshal.ReadInt16(c,56)+" usage_max="+(range?(ushort)Marshal.ReadInt16(c,58):(ushort)Marshal.ReadInt16(c,56)));
                 }} finally {Marshal.FreeHGlobal(bc);}
               }
             }
           } finally {Marshal.FreeHGlobal(caps);}
         } finally {HidD_FreePreparsedData(pp);}
       }
       using(var f=CreateFileW(path,0x80000000,3,IntPtr.Zero,3,0x40000000,IntPtr.Zero)) Log("layer=direct_read_open result="+(f.IsInvalid?"denied":"available")+" error="+(f.IsInvalid?Marshal.GetLastWin32Error():0)+" reads_issued=0");
     }
   } finally {Marshal.FreeHGlobal(p);}
   Log("layer=enumeration matched_collections="+matched);
 }
 public static void Run(string file,int seconds) {
   using(output=new StreamWriter(file,false,new System.Text.UTF8Encoding(false))) {
     clock=Stopwatch.StartNew(); Log("probe=three_key_layers mode=passive configuration_writes=0 injection=0 suppression=0"); Inspect(); if(seconds==0) return;
     IntPtr instance=GetModuleHandleW(IntPtr.Zero),cls=Marshal.StringToHGlobalUni("SayAllThreeKeyLayers"),hwnd=IntPtr.Zero,hook=IntPtr.Zero;
     var wc=new WindowClass { Proc=Marshal.GetFunctionPointerForDelegate(windowProc),Instance=instance,Name=cls };
     if(RegisterClassW(ref wc)==0) throw new InvalidOperationException("window_class_failed");
     try {
       hwnd=CreateWindowExW(0,cls,IntPtr.Zero,0,0,0,0,0,new IntPtr(-3),IntPtr.Zero,instance,IntPtr.Zero);
       if(hwnd==IntPtr.Zero) throw new InvalidOperationException("window_failed");
       var r=new[]{new Registration { Page=1,Usage=6,Flags=0x100,Window=hwnd },new Registration { Page=12,Usage=1,Flags=0x100,Window=hwnd }};
       if(!RegisterRawInputDevices(r,2,(uint)Marshal.SizeOf(typeof(Registration)))) throw new InvalidOperationException("registration_failed");
       hook=SetWindowsHookExW(13,hookProc,instance,0); if(hook==IntPtr.Zero) throw new InvalidOperationException("hook_failed");
       Log("phase=ready seconds="+seconds+" app_hook_may_hide_raw_keyboard=true"); long end=clock.ElapsedMilliseconds+seconds*1000L; Message msg;
       while(clock.ElapsedMilliseconds<end) { while(PeekMessageW(out msg,IntPtr.Zero,0,0,1)) DispatchMessageW(ref msg); Thread.Sleep(5); }
     } finally {
       if(hook!=IntPtr.Zero) UnhookWindowsHookEx(hook);
       RegisterRawInputDevices(new[]{new Registration {Page=1,Usage=6,Flags=1},new Registration {Page=12,Usage=1,Flags=1}},2,(uint)Marshal.SizeOf(typeof(Registration)));
       if(hwnd!=IntPtr.Zero) DestroyWindow(hwnd); UnregisterClassW(cls,instance); Marshal.FreeHGlobal(cls);
       Log("phase=completed raw_remote_events="+rawCount+" hook_candidate_events="+hookCount+" evidence_boundary=delivery_not_device_report_or_action_success");
     }
   }
 }
}
'@
[ThreeKeyLayers]::Run($OutFile, $Seconds)
