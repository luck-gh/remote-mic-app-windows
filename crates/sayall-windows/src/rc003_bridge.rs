//! RC003 全按键「助手 ↔ 主程序」桥接：捕获链的**第 ② 段（传输与所有权）**。
//!
//! ## 三段链条与本模块的位置
//!
//! RC003 的十三个普通按键共用报告层捕获，语音仍独立走 ATVV + SendInput。
//! 当前模板与本应用菜单所需的按键组成动态目标集；回执授予映射执行能力。
//!
//! 1. **捕获**（`hardware/RC003/helper` + Gadget 侧 agent）：在承载该设备的
//!    用户态 `WUDFHost.exe` 内、于报告层把动态目标 usage 拦下（选择性清空），
//!    并把按键状态上报给提权助手；当前组合版本的真机结果另行验收。
//! 2. **传输**（本模块）：把助手手里的边沿送进主程序。← **这里**
//! 3. **重映射**（`button_mapping` 引擎）：边沿驱动手势识别与动作注入。
//!    本地模板、菜单及短/双/长按继续共用原引擎。
//!
//! 捕获边沿使用独立的 [`EngineMessage::DriverEdge`] 来源，避免 Raw Input 的迟到
//! UP 释放捕获通道的按住状态。`R` 报告观察只驱动界面，不保证物理来源，零映射不会执行动作。
//! 没有当前所有权时暂停 RC003 自定义映射，保持原生输入，不启用键盘吞键兜底。
//!
//! ## 方向与发现机制：为什么是「主程序监听、助手连接」
//!
//! - 助手是**提权**进程，其运行时目录在 `%ProgramData%\SayAllPersonal\rc003-helper`；
//!   主程序是普通权限，默认**读不到**该目录的写入（也不该去猜）。
//! - 反过来则权限确定成立：主程序在 `%LOCALAPPDATA%\SayAllPersonal\` 下写桥接描述文件，
//!   提权助手读取用户目录**没有障碍**。
//! - 因此：**主程序创建命名管道并写出「管道 + 诊断端口 + 令牌」描述文件，助手
//!   读取后回连。** 命名管道是 Windows 产品主路径，不经过会改写 loopback 的
//!   Winsock/WFP/TUN；随机 TCP 端口用于同协议的本地测试与诊断。
//!
//! ## 威胁模型（必须如实理解，别把它当成安全边界）
//!
//! 令牌的作用是**防误连、防混淆**（例如另一个程序恰好占了端口），
//! **不是**防同用户恶意进程——同用户权限的进程本来就能读该描述文件、也能注入
//! 主进程，Windows 用户态没有能挡住它的边界。真正需要防的是"助手没在跑时，
//! 有别的进程占住端口往主程序喂伪造边沿"，那已由**方向选择**天然消解：
//! 主程序是监听方，且**只接受出示正确令牌的连接**。
//!
//! 纵深防御在**白名单 + 动态目标集**：桥接只接受 13 个已知语义按键，且只有
//! 当前启用并已配置映射的动态目标可以进入映射引擎；语音键始终排除。
//!
//! ## 协议（ASCII 行，`\n` 结尾；助手 → 主程序）
//!
//! | 行 | 方向 | 含义 |
//! | --- | --- | --- |
//! | `HELLO <ver> <token> <helper_pid>` | 助手 → app | 鉴权；必须首行 |
//! | `OK <ver> <gen> <usages>` / `DENY <reason>` | app → 助手 | 鉴权结果与初始目标 |
//! | `T <gen> <usages>` | app → 助手 | 热更新报告层捕获目标 |
//! | `O <conn> <epoch> <seq> <sample_ms> <gen> <state> <usages>` | 助手 → app | 同机单调时钟采样的接管状态 |
//! | `E <conn> <epoch> <seq> <gen> <usages>` | 助手 → app | 当前代次按下的 usage 集合；`-` 为全部释放 |
//! | `R <conn> <epoch> <usages>` | 助手 → app | 仅供界面的报告观察，不保证物理来源 |
//! | `S -` / `A -` | app → 助手 / 助手 → app | 固定关闭报告语音合成及回执 |
//! | `P <t_ms>` | 助手 → app | 心跳（每 1s） |
//! | `R <1|0>` | 助手 → app | 设备节点恢复进行中/结束（绝对状态；期间暂缓 capture_recover） |
//! | `BYE <reason>` | 助手 → app | 助手收尾，按键即将释放 |
//!
//! 边沿是**绝对状态**（与 agent 侧一致，只在变化时才发），主程序侧做差分转成
//! 逐按钮边沿。这样即使某一行丢失，下一行也能自愈（不会被"丢了释放"卡住），
//! 而看门狗负责兜住"连行都不再来"的情况。

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddrV4, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::fs::File;
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle};
#[cfg(windows)]
use windows::core::{HRESULT, PCWSTR};
#[cfg(windows)]
use windows::Win32::Foundation::{ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, HANDLE};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
#[cfg(windows)]
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_NOWAIT, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
};

use serde::Serialize;

use crate::button_mapping::EngineMessage;
use crate::raw_input::{button_for_usage, ButtonEdge, ENHANCED_CAPTURE_BUTTON_USAGES};

/// 语音键的 HID 键盘 usage (F5)。产品报告目标始终排除它;
/// 语音继续由 ATVV + 本地成对 SendInput 管理, 不扩大报告层吞键范围。
pub const VOICE_KEY_HID_USAGE: u16 = 0x003E;

/// 桥接描述文件名。约定路径见 [`default_bridge_dir`]。
pub const BRIDGE_FILE_NAME: &str = "rc003-bridge.ini";

/// 协议版本。助手与主程序不一致时**拒绝连接**（宁可不可用，也不要半懂不懂地跑）。
pub const BRIDGE_PROTOCOL_VERSION: u32 = 3;

/// 允许通过桥接的 usage 白名单。实际还必须命中当前动态目标集。
///
/// 与 agent 侧的全量语义按键白名单必须一致；助手侧自检会核对内嵌 agent 与助手
/// 的常量一致性，本模块由单测核对。运行时实际集合仍由主程序动态下发。
pub const BRIDGE_ALLOWED_USAGES: [u16; 13] = [
    0x00F1, 0x0028, 0x0035, 0x004A, 0x004F, 0x0050, 0x0051, 0x0052, 0x0065, 0x0066, 0x007F, 0x0080,
    0x0081,
];

/// 等待 `HELLO` 的上限。超时即断开——避免连接被空占。
const HELLO_TIMEOUT: Duration = Duration::from_millis(5_000);

/// 静默看门狗上限。助手每 1s 发一次 `P`，超过该上限没有任何行即认为链路已死，
/// **必须**释放全部按下状态：否则助手被强杀时，引擎会永远以为按键还按着
/// （进而触发长按/连发语义）。
const SILENCE_TIMEOUT: Duration = Duration::from_millis(3_000);

/// 单次读等待。决定看门狗与停止标志的响应粒度。
const READ_POLL: Duration = Duration::from_millis(250);

/// 命名管道空闲读退避。`PIPE_NOWAIT` 连接在无数据时读会**立即**返回
/// `ERROR_NO_DATA`，退避缺失会让连接线程变成忙等（上游 2026-10-02 实验：
/// Helper 一连上 `sayall-rc003-bridge-conn` 就吃满一个核，应用 UI 被饿死、
/// 点不动也关不掉）。取 2ms：边沿投递的额外上界延迟 ≤2ms（在本链路噪声内），
/// 空闲轮询 500Hz 的 CPU 成本可忽略；TCP 路径由 `READ_POLL` 读超时提供
/// 同等的"非忙等"语义。
const PIPE_IDLE_BACKOFF: Duration = Duration::from_millis(2);

/// agent 租约为 2s；所有权心跳提前失效，暂停自定义映射并保留原生输入。
const OWNERSHIP_TIMEOUT: Duration = Duration::from_millis(1_500);

/// 未通过鉴权的连接允许的错误次数，超过即断开（与助手侧 REJECT 折叠计数同旨）。
const MAX_DENY: u32 = 3;

/// 单行最大字节数。超长一律断开：读缓冲不能由对端无限撑大。
const MAX_LINE_BYTES: usize = 4_096;

/// Windows 命名管道绕开 Winsock/WFP。现场已确认 Clash/Meta TUN 会把计划任务
/// Helper 发往当前 loopback 端口的 SYN 改送到另一个端口，导致 TCP 永久超时；
/// 命名管道不经过 IP 栈；TCP 使用相同协议，用于本地测试与诊断。
const PIPE_BUFFER_BYTES: u32 = 4_096;

/// 桥接阶段（诊断用）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgePhase {
    /// 尚未开始（非 Windows 或已停止）。
    #[default]
    Stopped,
    /// 正在监听，尚无助手连接。
    Listening,
    /// 助手已连接且通过鉴权。
    Connected,
    /// 监听失败：暂停自定义映射，保留原生输入。
    Failed,
}

/// 桥接诊断快照。
///
/// `Default` 是"桥接不存在"的意思（非 Windows 平台恒为该值）——
/// 这样才能有一个**跨平台**的访问器，调用方不必自己 cfg 分支。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSnapshot {
    pub phase: BridgePhase,
    pub port: u16,
    /// 最近一次通过鉴权的助手进程号（0 = 无）。
    pub helper_pid: u32,
    /// 累计接受的连接数。
    pub accepted_total: u64,
    /// 累计鉴权被拒的**次数**（不是连接数）。
    pub denied_total: u64,
    /// 累计"后来的助手顶掉旧连接"的次数。
    pub replaced_total: u64,
    /// 累计投递给映射引擎的按键边沿数。
    pub edges_applied: u64,
    /// 白名单之外被丢弃的 usage 数（正常恒为 0）。
    pub usages_dropped: u64,
    /// 无法解析的行数（正常恒为 0）。
    pub malformed_total: u64,
    /// 因静默超时或断线而强制释放全部按键的次数。
    pub watchdog_release_total: u64,
    /// 当前被桥接认为按下的 usage。
    pub pressed_usages: Vec<u16>,
    /// 距最近一次收到助手数据的毫秒数（None = 从未收到）。
    pub last_rx_age_ms: Option<u64>,
    /// 当前下发给 Helper 的动态捕获代次与目标集合。
    pub target_generation: u64,
    pub target_usages: Vec<u16>,
    /// 已由 agent ACK 且仍在所有权租约内的集合。
    pub owned_usages: Vec<u16>,
    pub diagnostics: BridgeDiagnosticSnapshot,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeDiagnosticSnapshot {
    pub queue_full: u64,
    pub queue_closed: u64,
    pub start_failed: u64,
    pub write_failed: u64,
}

#[derive(Default)]
struct BridgeDiagnosticHealth {
    queue_full: AtomicU64,
    queue_closed: AtomicU64,
    start_failed: AtomicU64,
    write_failed: AtomicU64,
}

impl BridgeDiagnosticHealth {
    fn snapshot(&self) -> BridgeDiagnosticSnapshot {
        BridgeDiagnosticSnapshot {
            queue_full: self.queue_full.load(Ordering::Relaxed),
            queue_closed: self.queue_closed.load(Ordering::Relaxed),
            start_failed: self.start_failed.load(Ordering::Relaxed),
            write_failed: self.write_failed.load(Ordering::Relaxed),
        }
    }
}

const DIAGNOSTIC_QUEUE_CAPACITY: usize = 256;
static DIAGNOSTIC_HEALTH: BridgeDiagnosticHealth = BridgeDiagnosticHealth {
    queue_full: AtomicU64::new(0),
    queue_closed: AtomicU64::new(0),
    start_failed: AtomicU64::new(0),
    write_failed: AtomicU64::new(0),
};
static DIAGNOSTIC_WORKER: OnceLock<Result<SyncSender<String>, ()>> = OnceLock::new();
#[cfg(test)]
static DIAGNOSTIC_WORKER_STARTS: AtomicU64 = AtomicU64::new(0);

/// 默认桥接描述文件目录：`%LOCALAPPDATA%\SayAll`。
///
/// 与主程序诊断日志同一根目录，便于用户一并排查；助手侧读该目录无权限障碍。
pub fn default_bridge_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        format!("{home}\\AppData\\Local")
    });
    PathBuf::from(base).join("SayAllPersonal")
}

/// 解析结果：一行协议消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeLine {
    Hello {
        version: u32,
        token: String,
        helper_pid: u32,
    },
    /// 绝对状态边沿。`usages` 为空表示"全部释放"。
    Edges {
        conn_gen: u64,
        epoch: u64,
        observation_seq: u64,
        generation: u64,
        usages: Vec<u16>,
    },
    /// UI report observation, independent of capture targets; never executes actions.
    Observed {
        conn_gen: u64,
        epoch: u64,
        usages: Vec<u16>,
    },
    Ping,
    /// agent 已应用该代目标集合；后续心跳重复上报同一所有权。
    Ownership {
        conn_gen: u64,
        epoch: u64,
        observation_seq: u64,
        sampled_tick_ms: u64,
        generation: u64,
        state: CaptureState,
        usages: Vec<u16>,
    },
    /// agent 已把语音键报告层合成配置应用到位（助手转发的 agent 回执）。
    ///
    /// 绝对状态：`Some(usage)` = 生效中，`None` = 已关闭。门禁只认它，
    /// 不再认"S 行写进 socket"（2026-10-03 加固：写出成功 ≠ 报告层已生效）。
    SynthAck {
        to: Option<u16>,
    },
    Bye {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Active,
    Inactive,
    Closed,
}

fn parse_capture_number(part: Option<&str>, field: &str) -> Result<u64, String> {
    part.ok_or_else(|| format!("capture 缺少 {field}"))?
        .parse()
        .map_err(|_| format!("capture {field} 不是整数"))
}

fn parse_usage_payload(payload: &str) -> Result<Vec<u16>, String> {
    if payload == "-" {
        return Ok(Vec::new());
    }
    let mut usages = Vec::new();
    for raw in payload.split(',') {
        let raw = raw.trim();
        let token = if raw.len() > 2 && (raw.starts_with("0x") || raw.starts_with("0X")) {
            &raw[2..]
        } else {
            raw
        };
        if token.is_empty() {
            continue;
        }
        let usage =
            u16::from_str_radix(token, 16).map_err(|_| format!("{token:?} 不是十六进制 usage"))?;
        if !usages.contains(&usage) {
            usages.push(usage);
        }
    }
    Ok(usages)
}

/// 解析一行（已去掉行尾换行）。空行返回 `None`。
///
/// `E` 行的 usage 是**十六进制**（`f1` / `80` / `81`），与日志里的 `0x00F1`
/// 形态同源，避免十进制/十六进制在排查时来回换算。
pub fn parse_bridge_line(line: &str) -> Result<Option<BridgeLine>, String> {
    let line = line.trim_end_matches('\r').trim();
    if line.is_empty() {
        return Ok(None);
    }
    let mut parts = line.split(' ');
    let head = parts.next().unwrap_or_default();
    match head {
        "HELLO" => {
            let version = parts
                .next()
                .ok_or_else(|| "HELLO 缺少协议版本".to_string())?
                .parse::<u32>()
                .map_err(|_| "HELLO 协议版本不是整数".to_string())?;
            let token = parts
                .next()
                .ok_or_else(|| "HELLO 缺少令牌".to_string())?
                .to_string();
            if token.is_empty() {
                return Err("HELLO 令牌为空".to_string());
            }
            let helper_pid = parts
                .next()
                .ok_or_else(|| "HELLO 缺少进程号".to_string())?
                .parse::<u32>()
                .map_err(|_| "HELLO 进程号不是整数".to_string())?;
            Ok(Some(BridgeLine::Hello {
                version,
                token,
                helper_pid,
            }))
        }
        "E" | "R" => {
            let conn_gen = parse_capture_number(parts.next(), "conn_gen")?;
            let epoch = parse_capture_number(parts.next(), "epoch")?;
            let edge = if head == "E" {
                Some((
                    parse_capture_number(parts.next(), "observation_seq")?,
                    parse_capture_number(parts.next(), "generation")?,
                ))
            } else {
                None
            };
            let usages = parse_usage_payload(parts.next().ok_or("capture 缺少 usages")?)?;
            if parts.next().is_some() {
                return Err("capture 存在多余字段".to_owned());
            }
            Ok(Some(match edge {
                Some((observation_seq, generation)) => BridgeLine::Edges {
                    conn_gen,
                    epoch,
                    observation_seq,
                    generation,
                    usages,
                },
                None => BridgeLine::Observed {
                    conn_gen,
                    epoch,
                    usages,
                },
            }))
        }
        "P" => Ok(Some(BridgeLine::Ping)),
        "A" => {
            // agent 回执（助手转发）：绝对状态，`-` = 关闭。
            let payload = parts.next().unwrap_or("-");
            let to = if payload == "-" {
                None
            } else {
                let token = payload.trim_start_matches("0x").trim_start_matches("0X");
                let usage = u16::from_str_radix(token, 16)
                    .map_err(|_| format!("{token:?} 不是十六进制 usage"))?;
                (usage != 0).then_some(usage)
            };
            Ok(Some(BridgeLine::SynthAck { to }))
        }
        "O" => {
            let conn_gen = parse_capture_number(parts.next(), "conn_gen")?;
            let epoch = parse_capture_number(parts.next(), "epoch")?;
            let observation_seq = parse_capture_number(parts.next(), "observation_seq")?;
            let sampled_tick_ms = parse_capture_number(parts.next(), "sampled_tick_ms")?;
            let generation = parse_capture_number(parts.next(), "generation")?;
            let state = match parts.next() {
                Some("active") => CaptureState::Active,
                Some("inactive") => CaptureState::Inactive,
                Some("closed") => CaptureState::Closed,
                _ => return Err("capture state 无效".to_owned()),
            };
            let usages = parse_usage_payload(parts.next().ok_or("capture 缺少 usages")?)?;
            if parts.next().is_some() {
                return Err("capture 存在多余字段".to_owned());
            }
            Ok(Some(BridgeLine::Ownership {
                conn_gen,
                epoch,
                observation_seq,
                sampled_tick_ms,
                generation,
                state,
                usages,
            }))
        }
        "BYE" => Ok(Some(BridgeLine::Bye {
            reason: parts.collect::<Vec<_>>().join(" "),
        })),
        other => Err(format!("未知命令 {other:?}")),
    }
}

/// 常数时间比较令牌。
///
/// 长度不同直接返回 false（长度本身不是秘密：令牌是定长十六进制）。
fn token_matches(expected: &str, got: &str) -> bool {
    if expected.len() != got.len() || expected.is_empty() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in expected.bytes().zip(got.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// 生成桥接令牌（32 位十六进制）。
///
/// **不是密码学强度**：由时间、进程号与栈地址混合出的 xorshift 序列，
/// 目的只是"每次启动都不同、不可预测到同一个值"，够用于**防误连**。
/// 它是信息面很小的本地握手指令，不承担安全边界职责（见模块头威胁模型）。
fn generate_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    let stack = &nanos as *const u64 as u64;
    let mut state = nanos ^ (pid << 32) ^ stack.rotate_left(17);
    if state == 0 {
        state = 0x9E37_79B9_7F4A_7C15;
    }
    let mut out = String::with_capacity(32);
    for _ in 0..4 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        out.push_str(&format!("{:08x}", (state & 0xFFFF_FFFF) as u32));
    }
    out
}

#[cfg(windows)]
fn named_pipe_name() -> &'static str {
    r"\\.\pipe\SayAllPersonal.Rc003Bridge"
}

/// 跨线程共享的桥接状态。
#[derive(Debug)]
struct BridgeShared {
    phase: BridgePhase,
    helper_pid: u32,
    accepted_total: u64,
    denied_total: u64,
    replaced_total: u64,
    edges_applied: u64,
    usages_dropped: u64,
    malformed_total: u64,
    watchdog_release_total: u64,
    pressed: BTreeSet<u16>,
    last_rx: Option<Instant>,
    owned: BTreeSet<u16>,
    ownership_sampled_tick_ms: Option<u64>,
    mapping: std::sync::Weak<crate::button_mapping::ButtonMappingRuntime>,
}

impl Default for BridgeShared {
    fn default() -> Self {
        Self {
            phase: BridgePhase::Stopped,
            helper_pid: 0,
            accepted_total: 0,
            denied_total: 0,
            replaced_total: 0,
            edges_applied: 0,
            usages_dropped: 0,
            malformed_total: 0,
            watchdog_release_total: 0,
            pressed: BTreeSet::new(),
            last_rx: None,
            owned: BTreeSet::new(),
            ownership_sampled_tick_ms: None,
            mapping: std::sync::Weak::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct CaptureTargets {
    generation: u64,
    usages: BTreeSet<u16>,
}

impl Default for CaptureTargets {
    fn default() -> Self {
        Self {
            generation: 1,
            usages: BTreeSet::new(),
        }
    }
}

fn format_usage_payload(usages: &BTreeSet<u16>) -> String {
    if usages.is_empty() {
        "-".to_owned()
    } else {
        usages
            .iter()
            .map(|usage| format!("{usage:x}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn usage_mask(usages: &BTreeSet<u16>) -> u64 {
    usages.iter().fold(0u64, |mask, usage| {
        button_for_usage(*usage)
            .map(|button| mask | (1u64 << button.ordinal()))
            .unwrap_or(mask)
    })
}

fn clear_ownership(shared: &Arc<Mutex<BridgeShared>>, notes: &mut Vec<String>) {
    let (had_ownership, mapping) = {
        let mut state = lock(shared);
        let had = !state.owned.is_empty();
        state.owned.clear();
        state.ownership_sampled_tick_ms = None;
        (had, state.mapping.upgrade())
    };
    crate::key_gate::set_enhanced_owned_mask(0);
    if let Some(mapping) = mapping {
        if let Some(mask) = mapping.set_capture_owned(0) {
            notes.push(format!(
                "enhanced_capture event=engine_ownership mask={mask} available={}",
                mask != 0
            ));
        }
    }
    // 门禁必须跟所有权一起落下：所有权是"报告层还归我们"的唯一证据，租约一丢
    // 报告层合成就不可能再生效。少这一行，按下的路径选择会继续指向已经死掉的
    // 报告层（issue #264 现场形态）。
    if had_ownership {
        notes.push(
            "enhanced_capture event=ownership_released scope=local mapping=revoked".to_owned(),
        );
    }
}

#[cfg(windows)]
fn capture_tick_ms() -> u64 {
    unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
}

#[cfg(not(windows))]
fn capture_tick_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock precedes epoch")
        .as_millis() as u64
}

fn capture_sample_fresh(sampled_tick_ms: u64) -> bool {
    capture_sample_age_fresh(capture_tick_ms().checked_sub(sampled_tick_ms))
}

fn capture_sample_age_fresh(age_ms: Option<u64>) -> bool {
    age_ms.is_some_and(|age| age < OWNERSHIP_TIMEOUT.as_millis() as u64)
}

fn revoke_capture(
    shared: &Arc<Mutex<BridgeShared>>,
    targets: &Arc<Mutex<CaptureTargets>>,
    sender: &Sender<EngineMessage>,
    notes: &mut Vec<String>,
) -> usize {
    clear_ownership(shared, notes);
    release_capture_usages(shared, targets, sender)
}

fn release_capture_usages(
    shared: &Arc<Mutex<BridgeShared>>,
    targets: &Arc<Mutex<CaptureTargets>>,
    sender: &Sender<EngineMessage>,
) -> usize {
    let released = apply_usages(shared, targets, &BTreeSet::new());
    let count = released.len();
    for edge in released {
        let _ = sender.send(EngineMessage::DriverEdge(edge));
    }
    let _ = sender.send(EngineMessage::HidObservation(0));
    count
}

#[derive(Debug)]
struct CaptureObservation {
    conn_gen: u64,
    epoch: u64,
    observation_seq: u64,
    min_edge_seq: u64,
    generation: u64,
    state: CaptureState,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn note(message: String) {
    #[cfg(test)]
    let test_sink = tests::diagnostic_fixture_enabled();
    #[cfg(not(test))]
    let test_sink = false;
    #[cfg(windows)]
    if !test_sink && crate::ble::diagnostic_log_path().is_none() {
        return;
    }
    #[cfg(not(windows))]
    if !test_sink {
        return;
    }
    let record = diagnostic_record(&message);
    let worker = DIAGNOSTIC_WORKER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel(DIAGNOSTIC_QUEUE_CAPACITY);
        match std::thread::Builder::new()
            .name("sayall-bridge-diagnostic".into())
            .spawn(move || diagnostic_writer(receiver))
        {
            Ok(_) => {
                #[cfg(test)]
                DIAGNOSTIC_WORKER_STARTS.fetch_add(1, Ordering::Relaxed);
                Ok(sender)
            }
            Err(_) => {
                DIAGNOSTIC_HEALTH
                    .start_failed
                    .fetch_add(1, Ordering::Relaxed);
                Err(())
            }
        }
    });
    if let Ok(sender) = worker {
        enqueue_diagnostic(sender, record, &DIAGNOSTIC_HEALTH);
    }
}

fn diagnostic_record(message: &str) -> String {
    #[cfg(windows)]
    return crate::ble::diagnostic_note_record(message);
    #[cfg(not(windows))]
    message.to_owned()
}

fn enqueue_diagnostic(
    sender: &SyncSender<String>,
    record: String,
    health: &BridgeDiagnosticHealth,
) {
    match sender.try_send(record) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            health.queue_full.fetch_add(1, Ordering::Relaxed);
        }
        Err(TrySendError::Disconnected(_)) => {
            health.queue_closed.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn write_diagnostic(record: &str) -> std::io::Result<bool> {
    #[cfg(test)]
    {
        tests::block_diagnostic_if_selected(record);
        if let Some(result) = tests::write_test_diagnostic(record) {
            return result;
        }
    }
    #[cfg(windows)]
    {
        let Some(path) = crate::ble::diagnostic_log_path() else {
            return Ok(false);
        };
        crate::diagnostic_log::append_line_checked(&path, record)?;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        let _ = record;
        Ok(false)
    }
}

fn diagnostic_writer(receiver: mpsc::Receiver<String>) {
    let mut reported = BridgeDiagnosticSnapshot::default();
    for record in receiver {
        match write_diagnostic(&record) {
            Err(_) => {
                DIAGNOSTIC_HEALTH
                    .write_failed
                    .fetch_add(1, Ordering::Relaxed);
            }
            Ok(false) => {}
            Ok(true) => {
                let health = DIAGNOSTIC_HEALTH.snapshot();
                if health != reported {
                    let summary = diagnostic_record(&format!(
                        "capture event=diagnostic_recovered queue_full={} queue_closed={} start_failed={} write_failed={}",
                        health.queue_full, health.queue_closed, health.start_failed, health.write_failed
                    ));
                    match write_diagnostic(&summary) {
                        Ok(true) => reported = health,
                        Ok(false) => {}
                        Err(_) => {
                            DIAGNOSTIC_HEALTH
                                .write_failed
                                .fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Default)]
struct CaptureNotes(Vec<String>);

impl Drop for CaptureNotes {
    fn drop(&mut self) {
        for message in self.0.drain(..) {
            note(message);
        }
    }
}

/// 当前助手连接：自增编号 + 连接克隆。
///
/// **编号是必需的，不能用 socket 地址判断"我还是不是当前连接"**：连接被
/// `shutdown` 之后 `local_addr()` 会失败，地址比较会退化成"谁都不是当前"，
/// 于是刚接手的新连接立刻以为自己被替换、主动让位，桥接就此哑掉。
/// 这个缺陷是在自测里被构造出来的（见
/// `replacement_takes_over_and_survivor_keeps_working`），不是纸面推演。
#[derive(Debug)]
enum BridgeIo {
    Tcp(TcpStream),
    #[cfg(windows)]
    Pipe(File),
}

impl BridgeIo {
    fn transport(&self) -> &'static str {
        match self {
            Self::Tcp(_) => "tcp_loopback",
            #[cfg(windows)]
            Self::Pipe(_) => "named_pipe",
        }
    }

    fn uses_os_identity(&self) -> bool {
        match self {
            Self::Tcp(_) => false,
            #[cfg(windows)]
            Self::Pipe(_) => true,
        }
    }

    fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            Self::Tcp(stream) => stream.try_clone().map(Self::Tcp),
            #[cfg(windows)]
            Self::Pipe(file) => file.try_clone().map(Self::Pipe),
        }
    }

    fn disconnect(&self) {
        match self {
            Self::Tcp(stream) => {
                let _ = stream.shutdown(Shutdown::Both);
            }
            #[cfg(windows)]
            Self::Pipe(file) => {
                let _ = unsafe { DisconnectNamedPipe(HANDLE(file.as_raw_handle())) };
            }
        }
    }
}

impl Read for BridgeIo {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.read(buffer),
            #[cfg(windows)]
            Self::Pipe(file) => match file.read(buffer) {
                // PIPE_NOWAIT 在连接仍然有效、只是暂时无数据时可能成功返回 0；
                // 命名管道真正断开会返回 ERROR_BROKEN_PIPE。不能套用 TCP 的 EOF 语义。
                Ok(0) => Err(std::io::Error::from(ErrorKind::WouldBlock)),
                result => result,
            },
        }
    }
}

impl Write for BridgeIo {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(stream) => stream.write(buffer),
            #[cfg(windows)]
            Self::Pipe(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(stream) => stream.flush(),
            #[cfg(windows)]
            Self::Pipe(file) => file.flush(),
        }
    }
}

fn read_available_capture_input(
    reader: &mut BufReader<BridgeIo>,
    pending: &mut Vec<u8>,
) -> std::io::Result<usize> {
    let budget = MAX_LINE_BYTES - pending.len();
    let buffered = reader.buffer().len().min(budget);
    pending.extend_from_slice(&reader.buffer()[..buffered]);
    reader.consume(buffered);
    #[cfg(windows)]
    if buffered < budget {
        if let BridgeIo::Pipe(pipe) = reader.get_mut() {
            let mut bytes = [0u8; MAX_LINE_BYTES];
            match pipe.read(&mut bytes[..budget - buffered]) {
                Ok(read) => {
                    pending.extend_from_slice(&bytes[..read]);
                    return Ok(buffered + read);
                }
                Err(error) if retryable_bridge_read(&error) => {}
                Err(error) => return Err(error),
            }
        }
    }
    Ok(buffered)
}

#[derive(Debug)]
struct CurrentConn {
    id: u64,
    stream: BridgeIo,
}

/// RC003 报告层按键传输桥接。随平台生命周期存活（`Drop` 即停止监听并清理描述文件）。
pub struct Rc003Bridge {
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<BridgeShared>>,
    current: Arc<Mutex<Option<CurrentConn>>>,
    targets: Arc<Mutex<CaptureTargets>>,
    sender: Sender<EngineMessage>,
    worker: Mutex<Option<JoinHandle<()>>>,
    file: Option<PathBuf>,
    port: u16,
}

impl Rc003Bridge {
    /// 按约定目录启动（生产路径）。
    pub fn start(sender: Sender<EngineMessage>) -> Arc<Self> {
        Self::start_in(default_bridge_dir(), sender)
    }

    /// 在指定目录启动：创建命名管道与诊断 loopback 端口 → 写出描述文件 → 等待助手回连。
    ///
    /// 端口与令牌由主程序决定，助手只读不改；监听失败**不是**致命错误
    /// （桥接不可用时暂停 RC003 自定义映射，保留原生输入），
    /// 但会在诊断日志里留下 `phase=failed` 的记录。
    pub fn start_in(dir: PathBuf, sender: Sender<EngineMessage>) -> Arc<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Mutex::new(BridgeShared::default()));
        let current: Arc<Mutex<Option<CurrentConn>>> = Arc::new(Mutex::new(None));
        let targets = Arc::new(Mutex::new(CaptureTargets::default()));
        let next_id = Arc::new(AtomicU64::new(1));

        let listener = match TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)) {
            Ok(listener) => listener,
            Err(error) => {
                lock(&shared).phase = BridgePhase::Failed;
                note(format!(
                    "rc003_bridge phase=failed reason=bind_error detail={error}"
                ));
                return Arc::new(Self {
                    stop,
                    shared,
                    current,
                    targets,
                    sender,
                    worker: Mutex::new(None),
                    file: None,
                    port: 0,
                });
            }
        };
        let port = listener.local_addr().map(|addr| addr.port()).unwrap_or(0);
        let token = generate_token();
        let token_for_worker = token.clone();

        #[cfg(windows)]
        let pipe_name = Some(if dir == default_bridge_dir() {
            // 产品单实例使用固定本机名：不依赖计划任务会读到旧值的描述字段。
            named_pipe_name().to_owned()
        } else {
            // 测试会并行启动多个 bridge，也可能与正在运行的安装版共存；每个实例
            // 必须独占名字，否则客户端会被 Windows 分配到另一个同名 pipe 实例。
            format!("{}.Test.{}.{}", named_pipe_name(), std::process::id(), port)
        });
        #[cfg(not(windows))]
        let pipe_name: Option<String> = None;

        let file = match write_bridge_file(
            &dir,
            port,
            pipe_name.as_deref(),
            &token,
            crate::ble::diagnostic_log_path().as_deref(),
        ) {
            Ok(path) => Some(path),
            Err(error) => {
                // 监听已成功但描述文件写不出：助手将无法发现我们 → 相当于不可用。
                // 如实记录，不把"监听成功"当成"桥接可用"。
                note(format!(
                    "rc003_bridge phase=degraded reason=descriptor_write_failed detail={error}"
                ));
                None
            }
        };

        {
            let mut state = lock(&shared);
            state.phase = BridgePhase::Listening;
        }
        note(format!(
            "rc003_bridge phase=listening port={port} descriptor={}",
            file.as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "none".to_string())
        ));

        let bridge_sender = sender.clone();
        let worker = {
            let stop = Arc::clone(&stop);
            let shared = Arc::clone(&shared);
            let current = Arc::clone(&current);
            let targets = Arc::clone(&targets);
            let next_id = Arc::clone(&next_id);
            std::thread::Builder::new()
                .name("sayall-rc003-bridge".to_owned())
                .spawn(move || {
                    accept_loop(
                        listener,
                        stop,
                        shared,
                        current,
                        targets,
                        next_id,
                        pipe_name,
                        &token_for_worker,
                        bridge_sender,
                    )
                })
                .ok()
        };

        Arc::new(Self {
            stop,
            shared,
            current,
            targets,
            sender,
            worker: Mutex::new(worker),
            file,
            port,
        })
    }

    /// 监听到的端口（0 = 未监听）。
    pub(crate) fn attach_mapping_runtime(
        &self,
        mapping: &Arc<crate::button_mapping::ButtonMappingRuntime>,
    ) {
        lock(&self.shared).mapping = Arc::downgrade(mapping);
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn snapshot(&self) -> BridgeSnapshot {
        let state = lock(&self.shared);
        let targets = lock(&self.targets);
        BridgeSnapshot {
            phase: state.phase,
            port: self.port,
            helper_pid: state.helper_pid,
            accepted_total: state.accepted_total,
            denied_total: state.denied_total,
            replaced_total: state.replaced_total,
            edges_applied: state.edges_applied,
            usages_dropped: state.usages_dropped,
            malformed_total: state.malformed_total,
            watchdog_release_total: state.watchdog_release_total,
            pressed_usages: state.pressed.iter().copied().collect(),
            last_rx_age_ms: state.last_rx.map(|t| t.elapsed().as_millis() as u64),
            target_generation: targets.generation,
            target_usages: targets.usages.iter().copied().collect(),
            owned_usages: state.owned.iter().copied().collect(),
            diagnostics: DIAGNOSTIC_HEALTH.snapshot(),
        }
    }

    /// 更新增强捕获目标。只有功能开启且已配置映射的按键才被报告层接管；
    /// 关闭功能或删除映射会立即撤销所有权并恢复既有 Raw Input/键盘门控路径。
    pub fn set_capture_targets(&self, enabled: bool, mapped_mask: u64) {
        let mut notes = CaptureNotes::default();
        let _connection = lock(&self.current);
        let usages: BTreeSet<u16> = if enabled {
            ENHANCED_CAPTURE_BUTTON_USAGES
                .iter()
                .filter_map(|(button, usage)| {
                    (((mapped_mask >> button.ordinal()) & 1) == 1).then_some(*usage)
                })
                .collect()
        } else {
            BTreeSet::new()
        };
        let generation = {
            let mut targets = lock(&self.targets);
            if targets.usages == usages {
                return;
            }
            targets.generation = targets.generation.wrapping_add(1).max(1);
            targets.usages = usages.clone();
            targets.generation
        };

        // 先撤销旧所有权，再让 Helper 切换目标；期间自定义映射暂停，
        // 不恢复无设备归属的全局键盘吞键。
        revoke_capture(&self.shared, &self.targets, &self.sender, &mut notes.0);
        notes.0.push(format!(
            "enhanced_capture event=targets_changed generation={generation} enabled={enabled} usages={}",
            format_usage_payload(&usages)
        ));
    }
}

impl Drop for Rc003Bridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let mut notes = CaptureNotes::default();
        let mut connection = lock(&self.current);
        if let Some(conn) = connection.take() {
            conn.stream.disconnect();
        }
        revoke_capture(&self.shared, &self.targets, &self.sender, &mut notes.0);
        drop(connection);
        if let Some(worker) = lock(&self.worker).take() {
            let _ = worker.join();
        }
        // 描述文件是"桥接现在可用"的唯一凭据：必须随桥接一起消失，
        // 否则助手会一直对着一个死端口重连。
        if let Some(path) = self.file.as_ref() {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// 描述文件正文。**纯函数**：字段增删要在这里一眼看清，测试直接对字符串断言。
///
/// `log` 字段（2026-10-01 新增）把主程序**真实**的诊断日志路径交给助手，让两侧
/// 日志落进同一个文件——报障后一次拉取即可覆盖「主程序 + 提权助手」两段链路。
/// 助手读不到该字段时按约定回退到 `<描述文件目录>\Logs\sayall-diagnostic.log`。
fn descriptor_body(
    port: u16,
    pipe_name: Option<&str>,
    token: &str,
    log_path: Option<&Path>,
) -> String {
    let pipe_line = pipe_name
        .map(|name| format!("pipe={name}\n"))
        .unwrap_or_default();
    let log_line = log_path
        .map(|path| format!("log={}\n", path.display()))
        .unwrap_or_default();
    format!(
        "version={BRIDGE_PROTOCOL_VERSION}\nport={port}\n{pipe_line}token={token}\npid={}\n{log_line}",
        std::process::id()
    )
}

/// 原子写出描述文件（先写临时文件再改名，避免助手读到半截内容）。
fn write_bridge_file(
    dir: &Path,
    port: u16,
    pipe_name: Option<&str>,
    token: &str,
    log_path: Option<&Path>,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let final_path = dir.join(BRIDGE_FILE_NAME);
    let temp_path = dir.join(format!("{BRIDGE_FILE_NAME}.tmp"));
    let body = descriptor_body(port, pipe_name, token, log_path);
    {
        let mut file = std::fs::File::create(&temp_path).map_err(|e| e.to_string())?;
        file.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&temp_path, &final_path).map_err(|e| e.to_string())?;
    Ok(final_path)
}

/// 鉴权被拒的诊断行。**纯函数**，便于对字段逐一断言。
///
/// 为什么必须有：被拒是"连接通了、但身份/版本不对"的**独立事实**，与"助手根本
/// 没连上来"是两条完全不同的故障路径。2026-10-01 用户现场只有"桥停在 listening、
/// 没有任何 `helper_authenticated`"，而 DENY 分支当时不留任何日志——于是主程序
/// 日志无法区分「助手没启动」与「助手来了被拒（新旧版本混装 / 陈旧 token）」。
/// 该行只记原因、助手进程号与累计次数：不含 token 值、不含任何路径。
fn deny_note(reason: &str, helper_pid: u32, denied_total: u64) -> String {
    format!(
        "rc003_bridge event=helper_denied reason={reason} helper_pid={helper_pid} \
         denied_total={denied_total} retryable=true"
    )
}

/// 监听主循环：非阻塞 accept + 短睡，保证停止标志能被及时观察到。
///
/// **每个连接交给独立线程**，监听线程立刻回到 accept。这不是为了并发（预期只有一个
/// 助手），而是"接管"能真正生效的前提：早先的实现把 `handle_connection` 直接串在
/// accept 循环里，于是旧连接没结束之前**新连接根本不会被 accept**——"后来的助手
/// 顶掉旧连接"这段代码永远不会执行。自测里 beta 收不到 `OK` 就是这么暴露的。
fn accept_loop(
    listener: TcpListener,
    stop: Arc<AtomicBool>,
    shared: Arc<Mutex<BridgeShared>>,
    current: Arc<Mutex<Option<CurrentConn>>>,
    targets: Arc<Mutex<CaptureTargets>>,
    next_id: Arc<AtomicU64>,
    pipe_name: Option<String>,
    token: &str,
    sender: Sender<EngineMessage>,
) {
    listener.set_nonblocking(true).ok();
    #[cfg(windows)]
    let mut pending_pipe =
        pipe_name
            .as_deref()
            .and_then(|name| match create_named_pipe_server(name) {
                Ok(pipe) => Some(pipe),
                Err(error) => {
                    note(format!(
                    "rc003_bridge transport=pipe phase=failed reason=create_error detail={error}"
                ));
                    None
                }
            });
    #[cfg(not(windows))]
    let _ = pipe_name;

    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, addr)) => {
                stream.set_nonblocking(false).ok();
                stream.set_nodelay(true).ok();
                stream.set_read_timeout(Some(READ_POLL)).ok();
                if stream.set_write_timeout(Some(READ_POLL)).is_err() {
                    note(
                        "rc003_bridge event=accept_rejected reason=write_timeout_config_failed"
                            .to_owned(),
                    );
                    continue;
                }
                adopt_connection(
                    BridgeIo::Tcp(stream),
                    format!("tcp:{addr}"),
                    &stop,
                    &shared,
                    &current,
                    &targets,
                    &next_id,
                    token,
                    &sender,
                );
            }
            Err(ref error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(error) => {
                note(format!("rc003_bridge event=accept_error detail={error}"));
            }
        }

        #[cfg(windows)]
        if let Some(pipe) = pending_pipe.as_ref() {
            match named_pipe_connected(pipe) {
                Ok(true) => {
                    let connected = pending_pipe.take().expect("checked Some");
                    adopt_connection(
                        BridgeIo::Pipe(connected),
                        "named_pipe".to_owned(),
                        &stop,
                        &shared,
                        &current,
                        &targets,
                        &next_id,
                        token,
                        &sender,
                    );
                    pending_pipe = pipe_name
                        .as_deref()
                        .and_then(|name| create_named_pipe_server(name).ok());
                }
                Ok(false) => {}
                Err(error) => {
                    note(format!(
                        "rc003_bridge transport=pipe event=accept_error detail={error}"
                    ));
                    pending_pipe = pipe_name
                        .as_deref()
                        .and_then(|name| create_named_pipe_server(name).ok());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[allow(clippy::too_many_arguments)]
fn adopt_connection(
    stream: BridgeIo,
    source: String,
    stop: &Arc<AtomicBool>,
    shared: &Arc<Mutex<BridgeShared>>,
    current: &Arc<Mutex<Option<CurrentConn>>>,
    targets: &Arc<Mutex<CaptureTargets>>,
    next_id: &Arc<AtomicU64>,
    token: &str,
    sender: &Sender<EngineMessage>,
) {
    let my_id = next_id.fetch_add(1, Ordering::Relaxed);
    let clone = match stream.try_clone() {
        Ok(clone) => clone,
        Err(error) => {
            note(format!("rc003_bridge event=clone_failed detail={error}"));
            return;
        }
    };
    let mut notes = CaptureNotes::default();
    let mut connection = lock(current);
    let previous = connection.replace(CurrentConn {
        id: my_id,
        stream: clone,
    });
    let replaced = previous.is_some();
    if let Some(previous) = previous {
        previous.stream.disconnect();
    }
    {
        let mut state = lock(shared);
        state.accepted_total += 1;
        if replaced {
            state.replaced_total += 1;
            state.phase = BridgePhase::Listening;
            state.helper_pid = 0;
        }
    }
    if replaced {
        revoke_capture(shared, targets, sender, &mut notes.0);
        notes.0.push(format!(
            "rc003_bridge event=replaced_by_new_connection from={source}"
        ));
    }
    drop(connection);
    drop(notes);
    let thread_stop = Arc::clone(stop);
    let thread_shared = Arc::clone(shared);
    let thread_current = Arc::clone(current);
    let thread_targets = Arc::clone(targets);
    let thread_sender = sender.clone();
    let thread_token = token.to_string();
    let spawned = std::thread::Builder::new()
        .name("sayall-rc003-bridge-conn".to_owned())
        .spawn(move || {
            handle_connection(
                stream,
                my_id,
                &thread_stop,
                &thread_shared,
                &thread_current,
                &thread_targets,
                &thread_token,
                &thread_sender,
            )
        });
    if spawned.is_err() {
        note("rc003_bridge event=conn_thread_spawn_failed".to_string());
        let mut guard = lock(current);
        if guard.as_ref().map(|conn| conn.id == my_id).unwrap_or(false) {
            *guard = None;
        }
    }
}

#[cfg(windows)]
fn create_named_pipe_server(name: &str) -> std::io::Result<File> {
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let handle = unsafe {
        CreateNamedPipeW(
            PCWSTR(wide.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            PIPE_BUFFER_BYTES,
            PIPE_BUFFER_BYTES,
            0,
            None,
        )
    };
    if handle.is_invalid() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

#[cfg(windows)]
fn named_pipe_connected(pipe: &File) -> std::io::Result<bool> {
    match unsafe { ConnectNamedPipe(HANDLE(pipe.as_raw_handle()), None) } {
        Ok(()) => Ok(true),
        Err(error) if error.code() == HRESULT::from_win32(ERROR_PIPE_CONNECTED.0) => Ok(true),
        Err(error) if error.code() == HRESULT::from_win32(ERROR_PIPE_LISTENING.0) => Ok(false),
        Err(error) => Err(std::io::Error::from_raw_os_error(error.code().0 & 0xFFFF)),
    }
}

/// 这个 id 是否仍是当前连接（连接被顶替后旧线程必须尽快让位）。
fn is_current(current: &Mutex<Option<CurrentConn>>, id: u64) -> bool {
    lock(current)
        .as_ref()
        .map(|conn| conn.id == id)
        .unwrap_or(false)
}

/// 单个助手连接的生命周期：HELLO → 收边沿 → （静默/断开/BYE）释放全部。
fn handle_connection(
    stream: BridgeIo,
    my_id: u64,
    stop: &Arc<AtomicBool>,
    shared: &Arc<Mutex<BridgeShared>>,
    current: &Arc<Mutex<Option<CurrentConn>>>,
    targets: &Arc<Mutex<CaptureTargets>>,
    token: &str,
    sender: &Sender<EngineMessage>,
) {
    let read_stream = match stream.try_clone() {
        Ok(clone) => clone,
        Err(error) => {
            note(format!("rc003_bridge event=clone_failed detail={error}"));
            return;
        }
    };
    let mut reader = BufReader::new(read_stream);
    let mut writer = stream;
    let mut pending: Vec<u8> = Vec::new();
    let mut authenticated = false;
    let mut last_sent_generation = 0u64;
    // 普通按键增强始终关闭可选报告语音合成；本地 ATVV + SendInput 保留
    // 输入设备读回门禁。等待 A - 确认，未确认时幂等重发 S -。
    let mut synth_confirmed = false;
    let mut synth_sent_at: Option<Instant> = None;
    let mut synth_resend_attempts: u64 = 0;
    let mut deny_count = 0u32;
    let started = Instant::now();
    let mut last_rx = Instant::now();
    // 本轮连接内实际投递给映射引擎的边沿数（用于"边沿到底有没有到"这个问题）。
    let mut session_edges = 0u64;
    // Local receive order for E/O diagnostics; never changes protocol or ownership.
    let mut capture_rx_seq = 0u64;
    let mut observation: Option<CaptureObservation> = None;
    let mut capture_rejected_total = 0u64;
    let mut last_rejection = None;
    let mut last_received_usages: Option<BTreeSet<u16>> = None;
    let mut ownership_refresh_attempted = false;
    // 不用初值：所有出口都在 break 前赋值，给它一个"默认值"只会掩盖漏赋值的分支。
    let drop_reason: &str;

    'outer: loop {
        if stop.load(Ordering::Relaxed) {
            drop_reason = "bridge_stopping";
            break;
        }
        // 被更新连接接管：主动让位，避免两个连接同时往引擎投边沿。
        if !is_current(current, my_id) {
            drop_reason = "replaced";
            break;
        }
        // 切出所有完整行（可能一轮读回多行）。
        loop {
            let Some(position) = pending.iter().position(|byte| *byte == b'\n') else {
                break;
            };
            let line_bytes: Vec<u8> = pending.drain(..=position).collect();
            let raw = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]).into_owned();
            let parsed = match parse_bridge_line(&raw) {
                Ok(parsed) => parsed,
                Err(_) => {
                    lock(shared).malformed_total += 1;
                    continue;
                }
            };
            let Some(message) = parsed else {
                continue;
            };
            let mut notes = CaptureNotes::default();
            let connection = lock(current);
            if connection.as_ref().is_none_or(|conn| conn.id != my_id) {
                drop_reason = "replaced";
                break 'outer;
            }
            last_rx = Instant::now();
            if !authenticated {
                match message {
                    BridgeLine::Hello {
                        version,
                        token: got,
                        helper_pid,
                    } => {
                        let version_ok = version == BRIDGE_PROTOCOL_VERSION;
                        // TCP 令牌用于防误连；命名管道由 Windows 本机命名对象 ACL
                        // 约束到可访问该用户对象的进程，且拒绝远程客户端。同用户进程
                        // 本来就能注入主程序，令牌对该路径不增加安全边界。现场还证实
                        // WFP/TUN 环境会让提权任务读到旧描述 token，因此管道路径必须
                        // 以 OS 对象身份为准，否则会在传输已通时被陈旧 token 拒绝。
                        let token_ok = writer.uses_os_identity() || token_matches(token, &got);
                        if !version_ok || !token_ok {
                            deny_count += 1;
                            let reason = if version_ok {
                                "token_mismatch"
                            } else {
                                "version_mismatch"
                            };
                            let denied_total = {
                                let mut state = lock(shared);
                                state.denied_total += 1;
                                state.denied_total
                            };
                            notes.0.push(deny_note(reason, helper_pid, denied_total));
                            let _ = write_line(&mut writer, &format!("DENY {reason}"));
                            if deny_count >= MAX_DENY {
                                drop_reason = "deny_limit";
                                break 'outer;
                            }
                            continue;
                        }
                        authenticated = true;
                        {
                            let mut state = lock(shared);
                            state.phase = BridgePhase::Connected;
                            state.helper_pid = helper_pid;
                            // 新助手会话不继承上一实例的恢复窗口。
                        }
                        clear_ownership(shared, &mut notes.0);
                        notes.0.push(format!(
                            "rc003_bridge event=helper_authenticated helper_pid={helper_pid} version={version} transport={} auth={}",
                            writer.transport(),
                            if writer.uses_os_identity() { "os_pipe_acl" } else { "token" }
                        ));
                        let target = lock(targets).clone();
                        last_sent_generation = target.generation;
                        let _ = write_line(
                            &mut writer,
                            &format!(
                                "OK {BRIDGE_PROTOCOL_VERSION} {} {}",
                                target.generation,
                                format_usage_payload(&target.usages)
                            ),
                        );
                        // 显式关闭报告语音合成，防止继承助手残留配置。
                        synth_confirmed = false;
                        synth_resend_attempts = 0;
                        // None 也必须显式发送：helper/agent 可以跨应用或 helper 重启
                        // 常驻，省略 S - 会让上一轮 RightAlt 合成继续生效。
                        let line = voice_synth_off_line();
                        synth_sent_at =
                            write_line(&mut writer, &line).ok().map(|()| Instant::now());
                        // 门内延迟能力声明（2026-10-03）：与 S 行同批、独立一行；
                        // 当前固定 Helper 按 T/S/W 同一协议解析。
                        let gate_ok = write_line(&mut writer, &voice_gate_line()).is_ok();
                        notes.0.push(format!(
                            "rc003_bridge event=voice_gate_sent on=true scope=hello write_ok={gate_ok}"
                        ));
                        notes.0.push(format!(
                            "rc003_bridge event=voice_synth_sent to=off scope=hello await=ack local_voice=send_input write_ok={}",
                            synth_sent_at.is_some()
                        ));
                        note(
                            "rc003_bridge event=voice_synth_gate active=false reason=hello"
                                .to_owned(),
                        );
                    }
                    _ => {
                        // 未鉴权前只接受 HELLO。这不是防攻击（同用户进程挡不住），
                        // 而是防止"半个协议"被当成有效会话。
                        drop_reason = "hello_required";
                        break 'outer;
                    }
                }
                continue;
            }
            match message {
                BridgeLine::Edges {
                    conn_gen,
                    epoch,
                    observation_seq,
                    generation,
                    usages,
                } => {
                    capture_rx_seq = capture_rx_seq.wrapping_add(1);
                    let target = lock(targets).clone();
                    let fresh = lock(shared)
                        .ownership_sampled_tick_ms
                        .is_some_and(capture_sample_fresh);
                    let rejection = match observation.as_ref() {
                        None => Some("ownership_unobserved"),
                        Some(current)
                            if current.conn_gen != conn_gen
                                || current.epoch != epoch
                                || current.generation != generation
                                || generation != target.generation =>
                        {
                            Some("epoch_or_target")
                        }
                        Some(current) if observation_seq < current.min_edge_seq => {
                            Some("pregrant_observation")
                        }
                        Some(current) if observation_seq > current.observation_seq => {
                            Some("unobserved_sequence")
                        }
                        Some(current) if current.state != CaptureState::Active => {
                            Some("capture_inactive")
                        }
                        _ if !fresh => Some("ownership_expired"),
                        _ => None,
                    };
                    if let Some(reason) = rejection {
                        if !fresh {
                            revoke_capture(shared, targets, sender, &mut notes.0);
                        }
                        capture_rejected_total += 1;
                        if last_rejection != Some(reason) {
                            notes.0.push(format!("enhanced_capture event=capture_rejected kind=edge reason={reason} conn_gen={conn_gen} epoch={epoch} observation_seq={observation_seq} target_generation={generation}"));
                            last_rejection = Some(reason);
                        }
                        continue;
                    }
                    let wanted: BTreeSet<u16> = usages.into_iter().collect();
                    let diagnostic_snapshot =
                        edge_diagnostic_needed(last_received_usages.as_ref(), &wanted).then(|| {
                            let target = lock(targets).clone();
                            let owned = lock(shared).owned.clone();
                            (target, owned)
                        });
                    let edges = apply_usages(shared, targets, &wanted);
                    // Track the raw report so even a filtered Back DOWN has a logged UP.
                    if let Some((target, owned)) = diagnostic_snapshot {
                        notes.0.push(edge_diagnostic_note(
                            capture_rx_seq,
                            &target,
                            &wanted,
                            &owned,
                            edges.len(),
                        ));
                    }
                    last_received_usages = Some(wanted);
                    for edge in edges {
                        if sender.send(EngineMessage::DriverEdge(edge)).is_err() {
                            drop_reason = "engine_gone";
                            break 'outer;
                        }
                        lock(shared).edges_applied += 1;
                        if session_edges == 0 {
                            // 只记**本轮第一次投递**。这是"边沿真的到了映射引擎"的
                            // 第一手证据——在此之前，"助手已连接"只证明传输段通了，
                            // 证明不了边沿有没有被投出去。逐条记会淹没日志（按住连发时
                            // 每秒可能十几条），而"有没有到"只需要回答一次，
                            // 数量看收尾那行的 `edges=`。
                            let pressed = lock(shared)
                                .pressed
                                .iter()
                                .map(|usage| format!("0x{usage:04X}"))
                                .collect::<Vec<_>>()
                                .join(",");
                            notes.0.push(format!(
                                "rc003_bridge event=first_edge pressed={pressed} \
                                 note=边沿已投递给映射引擎，此后按键动作由映射配置决定"
                            ));
                        }
                        session_edges += 1;
                    }
                }
                BridgeLine::Observed {
                    conn_gen,
                    epoch,
                    usages,
                } => {
                    if !observation.as_ref().is_some_and(|current| {
                        current.conn_gen == conn_gen
                            && current.epoch == epoch
                            && current.state != CaptureState::Closed
                    }) {
                        capture_rejected_total += 1;
                        if last_rejection != Some("observation_epoch") {
                            notes.0.push(format!("enhanced_capture event=capture_rejected kind=observation reason=epoch conn_gen={conn_gen} epoch={epoch}"));
                            last_rejection = Some("observation_epoch");
                        }
                        continue;
                    }
                    let observed: BTreeSet<_> = usages
                        .into_iter()
                        .filter(|u| BRIDGE_ALLOWED_USAGES.contains(u))
                        .collect();
                    let _ = sender.send(EngineMessage::HidObservation(usage_mask(&observed)));
                }
                BridgeLine::Ping => {}
                BridgeLine::Ownership {
                    conn_gen,
                    epoch,
                    observation_seq,
                    sampled_tick_ms,
                    generation,
                    state: reported_state,
                    usages,
                } => {
                    capture_rx_seq = capture_rx_seq.wrapping_add(1);
                    let app_consumed_tick_ms = capture_tick_ms();
                    let old = observation.as_ref();
                    let rejection = old.and_then(|current| {
                        if conn_gen < current.conn_gen {
                            Some("old_connection")
                        } else if conn_gen == current.conn_gen
                            && current.state == CaptureState::Closed
                        {
                            Some("connection_closed")
                        } else if conn_gen == current.conn_gen
                            && (epoch < current.epoch || observation_seq <= current.observation_seq)
                        {
                            Some("old_observation")
                        } else {
                            None
                        }
                    });
                    if let Some(reason) = rejection {
                        capture_rejected_total += 1;
                        if last_rejection != Some(reason) {
                            notes.0.push(format!("enhanced_capture event=ownership_rejected reason={reason} conn_gen={conn_gen} epoch={epoch} observation_seq={observation_seq}"));
                            last_rejection = Some(reason);
                        }
                        continue;
                    }
                    let reported: BTreeSet<u16> = usages.into_iter().collect();
                    let target = lock(targets).clone();
                    let reason = if sampled_tick_ms > capture_tick_ms() {
                        Some("future_sample")
                    } else if !capture_sample_fresh(sampled_tick_ms) {
                        Some("stale_sample")
                    } else if reported_state != CaptureState::Active {
                        Some(match reported_state {
                            CaptureState::Closed => "closed",
                            _ => "inactive",
                        })
                    } else if generation != target.generation || reported != target.usages {
                        Some("target_mismatch")
                    } else {
                        None
                    };
                    let changed = old.is_none_or(|current| {
                        current.conn_gen != conn_gen
                            || current.epoch != epoch
                            || current.generation != generation
                            || current.state != reported_state
                    });
                    let local_sample = lock(shared).ownership_sampled_tick_ms;
                    let locally_active = local_sample.is_some_and(capture_sample_fresh);
                    let min_edge_seq = if changed || !locally_active {
                        observation_seq
                    } else {
                        old.expect("unchanged observation exists").min_edge_seq
                    };
                    if changed || (local_sample.is_some() && reason.is_some()) {
                        revoke_capture(shared, targets, sender, &mut notes.0);
                    } else if local_sample.is_some() && !locally_active {
                        let old_owned_mask = usage_mask(&lock(shared).owned);
                        if sender
                            .send(EngineMessage::CaptureContinuityReset { old_owned_mask })
                            .is_err()
                        {
                            notes.0.push("enhanced_capture event=ownership_handoff_failed reason=engine_closed".to_owned());
                            drop_reason = "engine_closed";
                            break 'outer;
                        }
                        let released = release_capture_usages(shared, targets, sender);
                        notes.0.push(format!(
                            "enhanced_capture event=ownership_handoff conn_gen={conn_gen} epoch={epoch} observation_seq={observation_seq} target_generation={generation} old_sampled_tick_ms={} sampled_tick_ms={sampled_tick_ms} released={released} reset_queued=true availability=preserved",
                            local_sample.expect("handoff requires an accepted sample"),
                        ));
                    }
                    observation = Some(CaptureObservation {
                        conn_gen,
                        epoch,
                        observation_seq,
                        min_edge_seq,
                        generation,
                        state: reported_state,
                    });
                    if let Some(reason) = reason {
                        capture_rejected_total += 1;
                        if changed || last_rejection != Some(reason) {
                            notes.0.push(format!("enhanced_capture event=ownership_rejected reason={reason} scope=local mapping=revoked conn_gen={conn_gen} epoch={epoch} observation_seq={observation_seq} target_generation={generation} sampled_tick_ms={sampled_tick_ms}"));
                            last_rejection = Some(reason);
                        }
                    } else {
                        let new_mask = usage_mask(&reported);
                        let resumed = crate::key_gate::enhanced_owned_mask() == 0 && new_mask != 0;
                        {
                            let mut state = lock(shared);
                            state.owned = reported;
                            state.ownership_sampled_tick_ms = Some(sampled_tick_ms);
                        }
                        crate::key_gate::set_enhanced_owned_mask(new_mask);
                        let mapping = lock(shared).mapping.upgrade();
                        if let Some(mapping) = mapping {
                            if let Some(mask) = mapping.set_capture_owned(new_mask) {
                                notes.0.push(format!("enhanced_capture event=engine_ownership mask={mask} available={}", mask != 0));
                            }
                        }
                        if changed || resumed {
                            let sample_age = app_consumed_tick_ms
                                .checked_sub(sampled_tick_ms)
                                .map_or_else(|| "none".to_owned(), |age| age.to_string());
                            notes.0.push(format!(
                                "enhanced_capture event=ownership_resumed rx_seq={capture_rx_seq} conn_gen={conn_gen} epoch={epoch} observation_seq={observation_seq} min_edge_seq={min_edge_seq} target_generation={generation} owned_mask={new_mask} sampled_tick_ms={sampled_tick_ms} app_consumed_tick_ms={app_consumed_tick_ms} sample_age_ms={sample_age} clock_origin=helper_receive_tick",
                            ));
                        }
                        last_rejection = None;
                    }
                }
                BridgeLine::SynthAck { to } => {
                    if to.is_none() {
                        synth_confirmed = true;
                        notes.0.push("rc003_bridge event=voice_synth_ack to=off confirm=agent local_voice=send_input".to_owned());
                    } else {
                        synth_confirmed = false;
                        synth_sent_at = None;
                        notes.0.push(format!(
                            "rc003_bridge event=voice_synth_ack to={} stale=true expected=off",
                            to.map(|u| format!("0x{u:04X}"))
                                .unwrap_or_else(|| "off".to_owned())
                        ));
                    }
                }
                BridgeLine::Bye { reason } => {
                    notes
                        .0
                        .push(format!("rc003_bridge event=helper_bye reason={reason}"));
                    drop_reason = "helper_bye";
                    break 'outer;
                }
                // 鉴权后重复 HELLO：当作协议噪音忽略，不改状态。
                BridgeLine::Hello { .. } => {}
            }
        }
        if pending.len() > MAX_LINE_BYTES {
            drop_reason = "line_too_long";
            break;
        }
        if !authenticated && started.elapsed() > HELLO_TIMEOUT {
            drop_reason = "hello_timeout";
            break;
        }
        if authenticated && last_rx.elapsed() > SILENCE_TIMEOUT {
            drop_reason = "silence_timeout";
            break;
        }
        if authenticated {
            let mut notes = CaptureNotes::default();
            let connection = lock(current);
            if connection.as_ref().is_none_or(|conn| conn.id != my_id) {
                drop_reason = "replaced";
                break;
            }
            let target = lock(targets).clone();
            if target.generation != last_sent_generation {
                if write_line(
                    &mut writer,
                    &format!(
                        "T {} {}",
                        target.generation,
                        format_usage_payload(&target.usages)
                    ),
                )
                .is_err()
                {
                    drop_reason = "target_write_error";
                    break;
                }
                last_sent_generation = target.generation;
            }
            {
                if !synth_confirmed
                    && synth_resend_due(synth_sent_at.map(|at| at.elapsed()), synth_resend_attempts)
                {
                    let line = voice_synth_off_line();
                    if write_line(&mut writer, &line).is_err() {
                        drop_reason = "voice_synth_write_error";
                        break;
                    }
                    synth_resend_attempts += 1;
                    synth_sent_at = Some(Instant::now());
                    // 前 3 次逐条、之后每 60 次一条。
                    if synth_resend_attempts <= 3 || synth_resend_attempts % 60 == 0 {
                        notes.0.push(format!(
                            "rc003_bridge event=voice_synth_resend to=off attempt={}",
                            synth_resend_attempts
                        ));
                    }
                }
            }
            let ownership_check = lock(shared)
                .ownership_sampled_tick_ms
                .map(|sampled_tick_ms| {
                    let decision_tick_ms = capture_tick_ms();
                    (
                        sampled_tick_ms,
                        decision_tick_ms,
                        decision_tick_ms.checked_sub(sampled_tick_ms),
                    )
                });
            if let Some((sampled_tick_ms, decision_tick_ms, age_ms)) =
                ownership_check.filter(|(_, _, age)| !capture_sample_age_fresh(*age))
            {
                if !ownership_refresh_attempted {
                    ownership_refresh_attempted = true;
                    match read_available_capture_input(&mut reader, &mut pending) {
                        Ok(bytes) => {
                            let complete = pending.contains(&b'\n');
                            notes.0.push(format!(
                                "enhanced_capture event=ownership_refresh_probe bytes={bytes} complete_line={complete} local_connection_id={my_id}"
                            ));
                            if complete {
                                continue 'outer;
                            }
                            if pending.len() == MAX_LINE_BYTES {
                                drop_reason = "line_too_long";
                                break;
                            }
                        }
                        Err(_) => {
                            drop_reason = "read_error";
                            break;
                        }
                    }
                }
                let current_observation = observation
                    .as_ref()
                    .expect("accepted ownership has a connection observation");
                let sample_age = age_ms.map_or_else(|| "none".to_owned(), |age| age.to_string());
                let diagnostic = format!(
                    "enhanced_capture event=ownership_timeout timeout_ms={} scope=local mapping=revoked clock_origin=helper_receive_tick last_accepted_sampled_tick_ms={sampled_tick_ms} decision_tick_ms={decision_tick_ms} sample_age_ms={sample_age} current_rx_seq={capture_rx_seq} local_connection_id={my_id} current_conn_gen={} current_epoch={} current_observation_seq={} current_target_generation={} buffered_bytes={} pending_bytes={}",
                    OWNERSHIP_TIMEOUT.as_millis(),
                    current_observation.conn_gen,
                    current_observation.epoch,
                    current_observation.observation_seq,
                    current_observation.generation,
                    reader.buffer().len(),
                    pending.len(),
                );
                revoke_capture(shared, targets, sender, &mut notes.0);
                notes.0.push(diagnostic);
            }
        }
        ownership_refresh_attempted = false;
        match reader.read_until(b'\n', &mut pending) {
            Ok(0) => {
                drop_reason = "peer_closed";
                break;
            }
            Ok(_) => {}
            // 空闲必须退避：`PIPE_NOWAIT` 无数据时立即返回，直接 continue
            // 会把本线程变成 100% 占核的忙等（见 `PIPE_IDLE_BACKOFF`）。
            Err(ref error) if retryable_bridge_read(error) => bridge_idle_backoff(),
            Err(ref error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => {
                drop_reason = "read_error";
                break;
            }
        }
    }

    // 收尾：任何退出路径都必须释放全部按下状态（fail-open 的最后一环）。
    //
    // **例外：`replaced`**。被新连接接管的旧连接手里那份按下状态已经归接手方所有，
    // 此时释放会误伤接手方刚建立的按下状态（表现为"刚按下就被松开"）。
    // 接手方收到的是**绝对状态**集合，自己会重建正确状态；即便它什么都不发，
    // 它自己的静默看门狗也会兜底。同理，共享状态也不该由旧连接改写。
    let mut notes = CaptureNotes::default();
    let mut connection = lock(current);
    let still_mine = connection.as_ref().is_some_and(|conn| conn.id == my_id);
    let mut released_count = 0u64;
    if still_mine {
        // **先落状态、再投边沿**：边沿是"释放已经发生"的通知，一旦投出，任何观察者
        // （测试、诊断读取、引擎侧回查）都可能立刻读快照——若此时计数与按下集合还没落，
        // 就会读到"已释放但未计数、pressed 仍非空"的半更新状态。2026-10-03 CI 双核实测：
        // 顺序反过来时 `silence_watchdog_releases_pressed_buttons` 以「边沿已到、计数为 0」
        // 失败（本地快机 15/15 全过、CI 慢机可复现），根因是这条竞争，不是等待余量不足。
        {
            let mut state = lock(shared);
            if drop_reason != "helper_bye" {
                state.watchdog_release_total += 1;
            }
            state.last_rx = None;
            if state.phase == BridgePhase::Connected {
                state.phase = BridgePhase::Listening;
            }
            state.helper_pid = 0;
        }
        released_count = revoke_capture(shared, targets, sender, &mut notes.0) as u64;
    }
    writer.disconnect();
    if still_mine {
        *connection = None;
    }
    drop(connection);
    drop(notes);
    note(format!(
        "rc003_bridge event=closed reason={drop_reason} edges={session_edges} \
         released={released_count} capture_rejected_total={capture_rejected_total} dropped={}",
        lock(shared).usages_dropped
    ));
}

/// 本地语音路径固定 SendInput，报告层只发送关闭合成命令。
/// 助手侧解析见 `hardware/RC003/helper/src/main.rs` 的 `parse_bridge_synth_line`。
fn voice_synth_off_line() -> String {
    "S -".to_owned()
}

/// 门内延迟能力声明（`W 1`）。助手侧解析见 `parse_bridge_gate_line`。
///
/// 保留固定 Helper 的协议能力声明。报告语音合成关闭时不参与语音时序。
fn voice_gate_line() -> String {
    "W 1".to_owned()
}

/// 未收到 agent 回执时的 S 行重发节奏：0.5s / 1.5s / 3s，之后每 5s 一次。
///
/// 与 targets 的 ack 重发同哲学：快节奏覆盖偶发丢行/助手晚读，慢节奏兜底；
/// S 行是绝对状态、agent 侧幂等，重复发送无害。`elapsed = None`（上次写失败）
/// 表示应立即重试。仍未确认期间门禁保持 false —— 失败可见（走注入兜底），
/// 而不是"以为合成生效"地静默吞掉（2026-10-03 加固）。
fn synth_resend_due(elapsed: Option<Duration>, attempts: u64) -> bool {
    let Some(elapsed) = elapsed else {
        return true;
    };
    let wait = match attempts {
        0 => Duration::from_millis(500),
        1 => Duration::from_millis(1_500),
        2 => Duration::from_millis(3_000),
        _ => Duration::from_millis(5_000),
    };
    elapsed >= wait
}

fn retryable_bridge_read(error: &std::io::Error) -> bool {
    error.kind() == ErrorKind::WouldBlock
        || error.kind() == ErrorKind::TimedOut
        || error.raw_os_error() == Some(232) // ERROR_NO_DATA（PIPE_NOWAIT）
}

/// 空闲读退避（`PIPE_IDLE_BACKOFF` 的唯一落点，便于用计数器做回归判据）。
fn bridge_idle_backoff() {
    #[cfg(test)]
    IDLE_BACKOFFS.fetch_add(1, Ordering::Relaxed);
    std::thread::sleep(PIPE_IDLE_BACKOFF);
}

/// 测试用：空闲退避次数。判据见 `idle_named_pipe_connection_is_throttled`。
#[cfg(test)]
static IDLE_BACKOFFS: AtomicU64 = AtomicU64::new(0);

fn write_line(stream: &mut impl Write, line: &str) -> std::io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

/// 应用一份绝对状态集合：过滤白名单 → 差分 → 逐按钮边沿。**调用方负责投递**。
///
/// 差分是必要的：助手侧发的是"当前按下的集合"，而引擎要的是按键边沿。
fn apply_usages(
    shared: &Arc<Mutex<BridgeShared>>,
    targets: &Arc<Mutex<CaptureTargets>>,
    wanted: &BTreeSet<u16>,
) -> Vec<ButtonEdge> {
    let mut state = lock(shared);
    let allowed = lock(targets).usages.clone();
    let before = state.pressed.clone();
    let mut accepted = BTreeSet::new();
    for usage in wanted {
        if BRIDGE_ALLOWED_USAGES.contains(usage) && allowed.contains(usage) {
            accepted.insert(*usage);
        } else {
            state.usages_dropped += 1;
        }
    }
    state.pressed = accepted;
    let mut edges = Vec::new();
    for usage in before.difference(&state.pressed) {
        if let Some(button) = button_for_usage(*usage) {
            edges.push(ButtonEdge {
                button,
                is_pressed: false,
            });
        }
    }
    for usage in state.pressed.difference(&before) {
        if let Some(button) = button_for_usage(*usage) {
            edges.push(ButtonEdge {
                button,
                is_pressed: true,
            });
        }
    }
    edges
}

fn edge_diagnostic_note(
    sequence: u64,
    target: &CaptureTargets,
    wanted: &BTreeSet<u16>,
    owned: &BTreeSet<u16>,
    decoded_edges: usize,
) -> String {
    let whitelisted: BTreeSet<_> = wanted
        .iter()
        .copied()
        .filter(|usage| BRIDGE_ALLOWED_USAGES.contains(usage))
        .collect();
    let accepted: BTreeSet<_> = whitelisted.intersection(&target.usages).copied().collect();
    let reason = if wanted.is_empty() {
        "released_all"
    } else if whitelisted.len() != wanted.len() {
        "outside_whitelist"
    } else if accepted.len() != wanted.len() {
        "not_targeted"
    } else if !accepted.is_subset(owned) {
        "ownership_missing"
    } else {
        "ownership_confirmed"
    };
    // Target/ownership are the snapshot at E reception; decoded_edges describes
    // apply_usages, not successful delivery or an action in a third-party app.
    format!(
        "rc003_bridge event=edge_received rx_seq={sequence} target_generation={} phase=received state_snapshot=true usages={} unknown_usages={} target_mask={} owned_mask={} accepted_mask={} dropped_usages={} decoded_edges={decoded_edges} reason={reason}",
        target.generation,
        format_usage_payload(&whitelisted),
        wanted.len() - whitelisted.len(),
        usage_mask(&target.usages),
        usage_mask(owned),
        usage_mask(&accepted),
        wanted.len() - accepted.len(),
    )
}

fn edge_diagnostic_needed(previous: Option<&BTreeSet<u16>>, wanted: &BTreeSet<u16>) -> bool {
    previous.is_none_or(|previous| {
        previous != wanted && (previous.contains(&0x00F1) || wanted.contains(&0x00F1))
    })
}

/// 从描述文件里读出 `key=value`。助手侧有等价实现（零依赖手写解析）。
///
/// 放在这里是为了**同一份解析规则只写一次**：助手与主程序对描述文件的
/// 理解必须逐字一致，否则会出现"主程序写了、助手读不出"的静默失配。
pub fn parse_descriptor(text: &str) -> Option<(u16, String, u32)> {
    let mut port = None;
    let mut token = None;
    let mut version = 0u32;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "version" => version = value.trim().parse().unwrap_or(0),
            "port" => port = value.trim().parse().ok(),
            "token" => token = Some(value.trim().to_string()),
            _ => {}
        }
    }
    if version != BRIDGE_PROTOCOL_VERSION {
        return None;
    }
    let port = port?;
    let token = token?;
    if token.is_empty() {
        return None;
    }
    Some((port, token, version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw_input::RemoteButton;
    use std::sync::mpsc::channel;

    struct BlockedDiagnostic {
        needle: &'static str,
        entered: Sender<String>,
        release: std::sync::mpsc::Receiver<()>,
    }

    static BLOCKED_DIAGNOSTIC: Mutex<Option<BlockedDiagnostic>> = Mutex::new(None);

    struct TestDiagnosticSink {
        records: Sender<String>,
        failures: usize,
    }

    static TEST_DIAGNOSTIC_SINK: Mutex<Option<TestDiagnosticSink>> = Mutex::new(None);

    pub(super) fn diagnostic_fixture_enabled() -> bool {
        lock(&BLOCKED_DIAGNOSTIC).is_some() || lock(&TEST_DIAGNOSTIC_SINK).is_some()
    }

    pub(super) fn write_test_diagnostic(record: &str) -> Option<std::io::Result<bool>> {
        let mut sink = lock(&TEST_DIAGNOSTIC_SINK);
        let sink = sink.as_mut()?;
        if sink.failures != 0 {
            sink.failures -= 1;
            return Some(Err(std::io::Error::new(
                ErrorKind::Other,
                "fixture write failed",
            )));
        }
        Some(
            sink.records
                .send(record.to_owned())
                .map(|_| true)
                .map_err(|error| std::io::Error::new(ErrorKind::BrokenPipe, error.to_string())),
        )
    }

    pub(super) fn block_diagnostic_if_selected(message: &str) {
        let blocked = {
            let mut selected = lock(&BLOCKED_DIAGNOSTIC);
            if selected
                .as_ref()
                .is_some_and(|selected| message.contains(selected.needle))
            {
                selected.take()
            } else {
                None
            }
        };
        if let Some(blocked) = blocked {
            let _ = blocked.entered.send(message.to_owned());
            let _ = blocked.release.recv();
        }
    }

    fn targets(usages: &[u16]) -> Arc<Mutex<CaptureTargets>> {
        Arc::new(Mutex::new(CaptureTargets {
            generation: 1,
            usages: usages.iter().copied().collect(),
        }))
    }

    #[cfg(windows)]
    fn diagnostic_pipe_peer() -> (
        Arc<Rc003Bridge>,
        File,
        std::sync::mpsc::Receiver<EngineMessage>,
    ) {
        let (sender, receiver) = channel();
        let (bridge, stream) = diagnostic_pipe_peer_with_sender(sender);
        while receiver.try_recv().is_ok() {}
        (bridge, stream, receiver)
    }

    #[cfg(windows)]
    fn diagnostic_pipe_peer_with_sender(sender: Sender<EngineMessage>) -> (Arc<Rc003Bridge>, File) {
        let dir =
            std::env::temp_dir().join(format!("sayall-bridge-diagnostic-{}", generate_token()));
        let bridge = Rc003Bridge::start_in(dir.clone(), sender);
        bridge.set_capture_targets(true, mask(&[RemoteButton::Back]));
        let descriptor = std::fs::read_to_string(dir.join(BRIDGE_FILE_NAME)).unwrap();
        let pipe = descriptor
            .lines()
            .find_map(|line| line.strip_prefix("pipe="))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(pipe)
            {
                Ok(stream) => break stream,
                Err(error) if Instant::now() < deadline => {
                    let _ = error;
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("test pipe unavailable: {error}"),
            }
        };
        writeln!(stream, "HELLO {BRIDGE_PROTOCOL_VERSION} local-pipe 9100").unwrap();
        let mut ack = Vec::new();
        while !String::from_utf8_lossy(&ack).contains("W 1\n") {
            read_pipe_chunk(&mut stream, &mut ack);
        }
        let generation = bridge.snapshot().target_generation;
        writeln!(
            stream,
            "A -\nO 4 2 90 {} {generation} active f1",
            sampled_tick()
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while bridge.snapshot().owned_usages.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(bridge.snapshot().owned_usages, vec![0xf1]);
        (bridge, stream)
    }

    fn diagnostic_fixture_barrier() {
        let (entered, observed) = channel();
        let (release, released) = channel();
        release.send(()).unwrap();
        *lock(&BLOCKED_DIAGNOSTIC) = Some(BlockedDiagnostic {
            needle: "capture_test event=diagnostic_barrier",
            entered,
            release: released,
        });
        note("capture_test event=diagnostic_barrier".to_owned());
        observed.recv_timeout(Duration::from_secs(2)).unwrap();
    }

    fn wait_driver_release(receiver: &std::sync::mpsc::Receiver<EngineMessage>) -> bool {
        let deadline = Instant::now() + OWNERSHIP_TIMEOUT + READ_POLL * 8;
        while Instant::now() < deadline {
            match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(EngineMessage::DriverEdge(ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: false,
                })) => return true,
                Ok(_) => {}
                Err(_) => return false,
            }
        }
        false
    }

    #[test]
    fn diagnostic_queue_disconnect_and_full_have_independent_health() {
        let health = BridgeDiagnosticHealth::default();
        let (sender, receiver) = mpsc::sync_channel(1);
        enqueue_diagnostic(&sender, "first".into(), &health);
        enqueue_diagnostic(&sender, "full".into(), &health);
        assert_eq!(health.snapshot().queue_full, 1);
        assert_eq!(receiver.recv().unwrap(), "first");
        drop(receiver);
        enqueue_diagnostic(&sender, "closed".into(), &health);
        assert_eq!(health.snapshot().queue_closed, 1);
        assert_eq!(health.snapshot().write_failed, 0);
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_blocked_worker_is_bounded_and_reused_after_bridge_restart() {
        let _gate = crate::key_gate::lock_gate_tests();
        let (bridge, stream, _) = diagnostic_pipe_peer();
        diagnostic_fixture_barrier();
        let (records, observed_records) = channel();
        *lock(&TEST_DIAGNOSTIC_SINK) = Some(TestDiagnosticSink {
            records,
            failures: 0,
        });
        let (entered, blocked) = channel();
        let (release, released) = channel();
        *lock(&BLOCKED_DIAGNOSTIC) = Some(BlockedDiagnostic {
            needle: "capture_test event=blocked_writer",
            entered,
            release: released,
        });
        note("capture_test event=blocked_writer".into());
        blocked.recv_timeout(Duration::from_secs(2)).unwrap();
        let starts = DIAGNOSTIC_WORKER_STARTS.load(Ordering::Relaxed);
        let before = bridge.snapshot().diagnostics;
        let delayed_record = diagnostic_record("capture_test event=prepared_before_release");
        enqueue_diagnostic(
            DIAGNOSTIC_WORKER.get().unwrap().as_ref().unwrap(),
            delayed_record.clone(),
            &DIAGNOSTIC_HEALTH,
        );
        for sequence in 1..DIAGNOSTIC_QUEUE_CAPACITY + 8 {
            note(format!(
                "capture_test event=queue_pressure sequence={sequence}"
            ));
        }
        assert_eq!(
            bridge.snapshot().diagnostics.queue_full - before.queue_full,
            8
        );
        drop(stream);
        drop(bridge);
        let (replacement, replacement_stream, _) = diagnostic_pipe_peer();
        let snapshot = replacement.snapshot();
        let same_worker = DIAGNOSTIC_WORKER_STARTS.load(Ordering::Relaxed) == starts;
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut recovered = false;
        let mut retained_event_record = false;
        while Instant::now() < deadline && !(recovered && retained_event_record) {
            if let Ok(record) =
                observed_records.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                recovered |= record.contains("event=diagnostic_recovered");
                retained_event_record |= record == delayed_record;
            }
        }
        diagnostic_fixture_barrier();
        *lock(&TEST_DIAGNOSTIC_SINK) = None;
        assert!(same_worker);
        assert_eq!(starts, 1);
        assert_eq!(snapshot.owned_usages, vec![0xf1]);
        assert!(recovered);
        assert!(retained_event_record);
        drop(replacement_stream);
    }

    #[test]
    fn diagnostic_write_failure_is_visible_and_recovery_is_aggregated() {
        let _gate = crate::key_gate::lock_gate_tests();
        diagnostic_fixture_barrier();
        let before = DIAGNOSTIC_HEALTH.snapshot();
        let (records, observed) = channel();
        *lock(&TEST_DIAGNOSTIC_SINK) = Some(TestDiagnosticSink {
            records,
            failures: 1,
        });
        note("capture_test event=write_failure".into());
        note("capture_test event=write_recovered".into());
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut summary = None;
        while Instant::now() < deadline && summary.is_none() {
            if let Ok(record) =
                observed.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                if record.contains("event=diagnostic_recovered") {
                    summary = Some(record);
                }
            }
        }
        diagnostic_fixture_barrier();
        *lock(&TEST_DIAGNOSTIC_SINK) = None;
        assert_eq!(
            DIAGNOSTIC_HEALTH.snapshot().write_failed,
            before.write_failed + 1
        );
        assert!(summary
            .unwrap()
            .contains(&format!("write_failed={}", before.write_failed + 1)));
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_sink_blocked_pipe_reader_still_processes_up_and_inactive() {
        let _gate = crate::key_gate::lock_gate_tests();
        let (bridge, mut stream, receiver) = diagnostic_pipe_peer();
        diagnostic_fixture_barrier();
        let (entered, blocked) = channel();
        let (release, released) = channel();
        *lock(&BLOCKED_DIAGNOSTIC) = Some(BlockedDiagnostic {
            needle: "event=edge_received",
            entered,
            release: released,
        });
        let before = bridge.snapshot();
        let generation = before.target_generation;
        writeln!(stream, "E 4 2 90 {generation} f1").unwrap();
        blocked.recv_timeout(Duration::from_secs(2)).unwrap();
        writeln!(
            stream,
            "E 4 2 90 {generation} -\nO 4 3 91 {} {generation} inactive -\nR 4 3 f1",
            sampled_tick()
        )
        .unwrap();
        let up_while_blocked = wait_driver_release(&receiver);
        let deadline = Instant::now() + READ_POLL * 8;
        let mut inactive_epoch_observed = false;
        while Instant::now() < deadline {
            match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(EngineMessage::HidObservation(observed))
                    if observed == mask(&[RemoteButton::Back]) =>
                {
                    inactive_epoch_observed = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        let snapshot = bridge.snapshot();
        release.send(()).unwrap();
        assert!(up_while_blocked, "UP waited for the diagnostic sink latch");
        assert_eq!(snapshot.edges_applied, before.edges_applied + 2);
        assert_eq!(
            snapshot.watchdog_release_total,
            before.watchdog_release_total
        );
        assert!(
            inactive_epoch_observed,
            "inactive epoch was not processed while the sink latch was held"
        );
        assert!(
            snapshot.owned_usages.is_empty(),
            "inactive waited for the diagnostic sink latch"
        );
        assert!(snapshot.pressed_usages.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_sink_blocked_pipe_reader_still_expires_ownership() {
        let _gate = crate::key_gate::lock_gate_tests();
        let (bridge, mut stream, receiver) = diagnostic_pipe_peer();
        diagnostic_fixture_barrier();
        let (entered, blocked) = channel();
        let (release, released) = channel();
        *lock(&BLOCKED_DIAGNOSTIC) = Some(BlockedDiagnostic {
            needle: "event=edge_received",
            entered,
            release: released,
        });
        let generation = bridge.snapshot().target_generation;
        writeln!(stream, "E 4 2 90 {generation} f1").unwrap();
        blocked.recv_timeout(Duration::from_secs(2)).unwrap();
        let released_while_blocked = wait_driver_release(&receiver);
        let snapshot = bridge.snapshot();
        release.send(()).unwrap();
        assert!(
            released_while_blocked,
            "lease watchdog waited for the diagnostic sink latch"
        );
        assert!(snapshot.owned_usages.is_empty());
        assert!(snapshot.pressed_usages.is_empty());
    }

    fn mask(buttons: &[RemoteButton]) -> u64 {
        buttons
            .iter()
            .fold(0, |mask, button| mask | (1u64 << button.ordinal()))
    }

    struct CapturePeer {
        bridge: Arc<Rc003Bridge>,
        stream: TcpStream,
        receiver: std::sync::mpsc::Receiver<EngineMessage>,
        generation: u64,
    }

    impl CapturePeer {
        fn start() -> Self {
            let (sender, receiver) = channel();
            Self::start_with_sender(sender, receiver)
        }

        fn start_with_sender(
            sender: Sender<EngineMessage>,
            receiver: std::sync::mpsc::Receiver<EngineMessage>,
        ) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "sayall-capture-state-{}-{}",
                std::process::id(),
                generate_token()
            ));
            let bridge = Rc003Bridge::start_in(dir.clone(), sender);
            bridge.set_capture_targets(true, mask(&[RemoteButton::Back]));
            let text = std::fs::read_to_string(dir.join(BRIDGE_FILE_NAME)).unwrap();
            let (port, token, _) = parse_descriptor(&text).unwrap();
            let stream = Self::connect(port, &token);
            let generation = bridge.snapshot().target_generation;
            while receiver.try_recv().is_ok() {}
            Self {
                bridge,
                stream,
                receiver,
                generation,
            }
        }

        fn connect(port: u16, token: &str) -> TcpStream {
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            writeln!(stream, "HELLO {BRIDGE_PROTOCOL_VERSION} {token} 9003").unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert!(line.starts_with("OK "), "{line}");
            stream
        }

        fn send(&mut self, lines: &str) -> Vec<EngineMessage> {
            let before = self.bridge.snapshot().malformed_total;
            writeln!(self.stream, "{lines}\nCAPTURE_TEST_BARRIER").unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.bridge.snapshot().malformed_total == before && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
            assert!(
                self.bridge.snapshot().malformed_total > before,
                "IPC barrier not received"
            );
            self.receiver.try_iter().collect()
        }

        fn ownership(
            &mut self,
            conn: u64,
            epoch: u64,
            seq: u64,
            sample: u64,
            state: &str,
        ) -> Vec<EngineMessage> {
            self.send(&format!(
                "O {conn} {epoch} {seq} {sample} {} {state} f1",
                self.generation
            ))
        }
    }

    fn sampled_tick() -> u64 {
        #[cfg(windows)]
        {
            unsafe { windows::Win32::System::SystemInformation::GetTickCount64() }
        }
        #[cfg(not(windows))]
        {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }
    }

    fn driver_edges(messages: &[EngineMessage]) -> Vec<ButtonEdge> {
        messages
            .iter()
            .filter_map(|message| match message {
                EngineMessage::DriverEdge(edge) => Some(*edge),
                _ => None,
            })
            .collect()
    }

    #[derive(Default)]
    struct CaptureInjector(AtomicU64);

    impl crate::button_mapping::MappingInjector for CaptureInjector {
        fn tap(&self, _chord: &crate::send_input::KeyChord) -> Result<(), String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn scroll(
            &self,
            _direction: crate::send_input::ScrollDirection,
            _steps: u16,
        ) -> Result<(), String> {
            unreachable!()
        }
        fn mouse_click(&self, _kind: crate::send_input::MouseClickKind) -> Result<(), String> {
            unreachable!()
        }
        fn mouse_move(
            &self,
            _direction: crate::send_input::MoveDirection,
            _distance: u16,
        ) -> Result<(), String> {
            unreachable!()
        }
        fn launch_app(&self, _target: &str) -> Result<(), String> {
            unreachable!()
        }
        fn focus_frontmost(&self) -> Result<(), String> {
            unreachable!()
        }
        fn launch_app_and_focus(
            &self,
            _target: &str,
            _profile: Option<&crate::focus::AppFocusProfile>,
            _cancellation: Option<&AtomicBool>,
        ) -> Result<(), String> {
            unreachable!()
        }
        fn learn_focus_target(&self) -> Result<crate::focus::RecordedFocusTarget, String> {
            unreachable!()
        }
    }

    #[test]
    fn capture_ipc_epoch_reset_releases_engine_merger_and_accepts_next_press() {
        let _gate = crate::key_gate::lock_gate_tests();
        let injector = Arc::new(CaptureInjector::default());
        let snapshot = Arc::new(Mutex::new(crate::raw_input::RawInputSnapshot::default()));
        let runtime = Arc::new(crate::button_mapping::ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(crate::UsageCounters::default()),
            snapshot.clone(),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        ));
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        let mut mappings = crate::send_input::ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Back,
            crate::send_input::ButtonActions {
                single: crate::send_input::ButtonAction::Shortcut {
                    chord: crate::send_input::KeyChord {
                        keys: vec![crate::send_input::KeyCode::Backspace],
                    },
                },
                hold_repeat: Some(crate::send_input::HoldRepeat::Off),
                ..Default::default()
            },
        );
        runtime.set_mappings(mappings);
        let (_unused_sender, receiver) = channel();
        let mut peer = CapturePeer::start_with_sender(runtime.sender(), receiver);
        peer.bridge.attach_mapping_runtime(&runtime);
        peer.ownership(4, 2, 90, sampled_tick(), "active");
        peer.send(&format!("E 4 2 90 {} f1", peer.generation));
        assert!(runtime.wait_for_idle(Duration::from_secs(2)));
        assert_eq!(runtime.snapshot().fired_gestures, 1);
        assert_eq!(lock(&snapshot).active_buttons, vec![RemoteButton::Back]);
        peer.ownership(4, 4, 92, sampled_tick(), "active");
        assert!(runtime.wait_for_idle(Duration::from_secs(2)));
        assert!(lock(&snapshot).active_buttons.is_empty());
        peer.send(&format!("E 4 2 90 {} -", peer.generation));
        assert!(runtime.wait_for_idle(Duration::from_secs(2)));
        assert_eq!(runtime.snapshot().fired_gestures, 1);
        peer.send(&format!(
            "E 4 4 92 {} f1\nE 4 4 92 {} -",
            peer.generation, peer.generation
        ));
        assert!(runtime.wait_for_idle(Duration::from_secs(2)));
        assert_eq!(
            runtime.snapshot().fired_gestures,
            2,
            "new press remained blocked by old merger hold"
        );
        assert_eq!(injector.0.load(Ordering::SeqCst), 0);
        assert!(lock(&snapshot).active_buttons.is_empty());
    }

    #[test]
    fn capture_ipc_rejects_stale_future_and_reordered_observations() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        peer.ownership(4, 2, 90, sampled_tick(), "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
        peer.ownership(4, 2, 91, sampled_tick() - 2_050, "active");
        assert!(
            peer.bridge.snapshot().owned_usages.is_empty(),
            "queued O renewed ownership"
        );
        peer.ownership(4, 2, 92, sampled_tick(), "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
        peer.ownership(4, 2, 93, sampled_tick() + 10_000, "active");
        assert!(
            peer.bridge.snapshot().owned_usages.is_empty(),
            "future O renewed ownership"
        );
        peer.ownership(4, 3, 94, sampled_tick(), "inactive");
        peer.ownership(4, 2, 93, sampled_tick(), "active");
        assert!(
            peer.bridge.snapshot().owned_usages.is_empty(),
            "old seq revived ownership"
        );
        peer.ownership(4, 4, 95, sampled_tick(), "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
    }

    #[test]
    fn capture_ipc_false_and_coalesced_epoch_clear_held_before_resume() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        peer.ownership(4, 2, 90, sampled_tick(), "active");
        assert_eq!(
            driver_edges(&peer.send(&format!("E 4 2 90 {} f1", peer.generation))).len(),
            1
        );
        let revoked = peer.ownership(4, 3, 91, sampled_tick(), "inactive");
        assert_eq!(
            driver_edges(&revoked),
            vec![ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: false
            }]
        );
        assert!(peer.bridge.snapshot().pressed_usages.is_empty());
        assert!(peer.bridge.snapshot().owned_usages.is_empty());
        peer.ownership(4, 4, 92, sampled_tick(), "active");
        peer.send(&format!("E 4 4 92 {} f1", peer.generation));
        let resumed = peer.ownership(4, 6, 94, sampled_tick(), "active");
        assert_eq!(
            driver_edges(&resumed),
            vec![ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: false
            }]
        );
        let old = peer.send(&format!(
            "E 4 4 92 {} -\nE 3 6 94 {} f1\nE 4 6 94 {} f1\nE 4 6 94 {} -",
            peer.generation, peer.generation, peer.generation, peer.generation
        ));
        assert_eq!(
            driver_edges(&old),
            vec![
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: true
                },
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: false
                }
            ]
        );
    }

    #[test]
    fn capture_ipc_closed_cannot_revive_but_new_helper_resets_namespace() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        peer.ownership(4, 2, 90, sampled_tick(), "active");
        peer.ownership(4, 3, 91, sampled_tick(), "closed");
        peer.ownership(4, 9, 900, sampled_tick(), "active");
        assert!(peer.bridge.snapshot().owned_usages.is_empty());
        peer.ownership(5, 1, 1, sampled_tick(), "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
        peer.ownership(4, 20, 901, sampled_tick(), "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
        let text = std::fs::read_to_string(peer.bridge.file.as_ref().unwrap()).unwrap();
        let (port, token, _) = parse_descriptor(&text).unwrap();
        peer.stream = CapturePeer::connect(port, &token);
        peer.ownership(1, 1, 1, sampled_tick(), "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
    }

    #[test]
    fn capture_ipc_same_epoch_renewal_preserves_fast_press_release_fifo() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        peer.ownership(4, 2, 90, sampled_tick(), "active");
        let sample = sampled_tick();
        let messages = peer.send(&format!(
            "E 4 2 90 {} f1\nO 4 2 91 {sample} {} active f1\nE 4 2 90 {} -",
            peer.generation, peer.generation, peer.generation
        ));
        assert_eq!(
            driver_edges(&messages),
            vec![
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: true
                },
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: false
                }
            ]
        );
    }

    #[test]
    fn capture_ipc_local_expiry_fences_old_edges_when_same_epoch_resumes() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        let started = Instant::now();
        peer.ownership(4, 2, 90, sampled_tick() - 1_000, "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
        let deadline = started + Duration::from_millis(1_200);
        while !peer.bridge.snapshot().owned_usages.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(
            peer.bridge.snapshot().owned_usages.is_empty(),
            "receipt reset the sample budget"
        );
        peer.ownership(4, 2, 95, sampled_tick(), "active");
        assert_eq!(peer.bridge.snapshot().owned_usages, vec![0xf1]);
        let future = peer.send(&format!("E 4 2 96 {} f1", peer.generation));
        assert!(
            driver_edges(&future).is_empty(),
            "unobserved ownership sequence authorized E"
        );
        let old = peer.send(&format!(
            "E 4 2 90 {} f1\nE 4 2 90 {} -",
            peer.generation, peer.generation
        ));
        assert!(
            driver_edges(&old).is_empty(),
            "pre-expiry FIFO edges replayed after new grant"
        );
        let fresh = peer.send(&format!(
            "E 4 2 95 {} f1\nO 4 2 96 {} {} active f1\nE 4 2 95 {} -",
            peer.generation,
            sampled_tick(),
            peer.generation,
            peer.generation
        ));
        assert_eq!(
            driver_edges(&fresh),
            vec![
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: true
                },
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: false
                }
            ]
        );
    }

    #[test]
    fn capture_ipc_blocked_diagnostic_allows_target_change_and_stop() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        diagnostic_fixture_barrier();
        let (entered, blocked) = channel();
        let (release, released) = channel();
        *lock(&BLOCKED_DIAGNOSTIC) = Some(BlockedDiagnostic {
            needle: "event=ownership_resumed",
            entered,
            release: released,
        });
        writeln!(
            peer.stream,
            "O 4 2 90 {} {} active f1",
            sampled_tick(),
            peer.generation
        )
        .unwrap();
        blocked.recv_timeout(Duration::from_secs(2)).unwrap();
        let bridge = peer.bridge.clone();
        let (changed, change_done) = channel();
        let changer = std::thread::spawn(move || {
            bridge.set_capture_targets(true, mask(&[RemoteButton::Home]));
            changed.send(()).unwrap();
        });
        let progressed = change_done.recv_timeout(Duration::from_millis(750)).is_ok();
        let snapshot = peer.bridge.snapshot();
        let (stopped, stop_done) = channel();
        let stopper = std::thread::spawn(move || {
            drop(peer);
            let _ = stopped.send(());
        });
        let stop_progressed = stop_done.recv_timeout(Duration::from_millis(750)).is_ok();
        release.send(()).unwrap();
        changer.join().unwrap();
        stopper.join().unwrap();
        assert!(
            progressed,
            "blocked diagnostic retained the connection lock"
        );
        assert!(
            stop_progressed,
            "stop waited for another thread's diagnostic sink"
        );
        assert!(snapshot.owned_usages.is_empty());
        assert_eq!(snapshot.target_usages, vec![0x4a]);
    }

    #[test]
    fn capture_ipc_timeout_records_decision_sample_and_resumed_consumption() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        diagnostic_fixture_barrier();
        let (records, observed) = channel();
        *lock(&TEST_DIAGNOSTIC_SINK) = Some(TestDiagnosticSink {
            records,
            failures: 0,
        });
        let sample = sampled_tick();
        peer.ownership(4, 2, 90, sample, "active");
        let deadline = Instant::now() + OWNERSHIP_TIMEOUT + READ_POLL * 8;
        while !peer.bridge.snapshot().owned_usages.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        let expired = peer.bridge.snapshot().owned_usages.is_empty();
        let resumed_sample = sampled_tick();
        peer.ownership(4, 2, 95, resumed_sample, "active");
        diagnostic_fixture_barrier();
        *lock(&TEST_DIAGNOSTIC_SINK) = None;
        let captured: Vec<_> = observed.try_iter().collect();
        assert!(expired, "ownership did not expire");
        let timeout = captured
            .iter()
            .find(|record| record.contains("event=ownership_timeout"))
            .expect("timeout diagnostic missing");
        let field = |record: &str, name: &str| -> u64 {
            record
                .split_whitespace()
                .find_map(|part| part.strip_prefix(&format!("{name}=")))
                .unwrap_or_else(|| panic!("missing {name} in diagnostic"))
                .parse()
                .unwrap()
        };
        assert_eq!(field(timeout, "last_accepted_sampled_tick_ms"), sample);
        let decision = field(timeout, "decision_tick_ms");
        assert_eq!(field(timeout, "sample_age_ms"), decision - sample);
        assert!(decision - sample >= OWNERSHIP_TIMEOUT.as_millis() as u64);
        assert_eq!(field(timeout, "current_rx_seq"), 1);
        assert!(field(timeout, "local_connection_id") > 0);
        assert_eq!(field(timeout, "current_conn_gen"), 4);
        assert_eq!(field(timeout, "current_epoch"), 2);
        assert_eq!(field(timeout, "current_observation_seq"), 90);
        assert_eq!(field(timeout, "current_target_generation"), peer.generation);
        assert_eq!(field(timeout, "pending_bytes"), 0);
        let _buffered = field(timeout, "buffered_bytes");
        let resumed = captured
            .iter()
            .find(|record| {
                record.contains("event=ownership_resumed") && record.contains("observation_seq=95")
            })
            .expect("resumed diagnostic missing");
        let consumed = field(resumed, "app_consumed_tick_ms");
        assert_eq!(field(resumed, "sample_age_ms"), consumed - resumed_sample);
        assert_eq!(field(resumed, "sampled_tick_ms"), resumed_sample);
        assert!(consumed >= decision);
        let descriptor = std::fs::read_to_string(peer.bridge.file.as_ref().unwrap()).unwrap();
        let (_, token, _) = parse_descriptor(&descriptor).unwrap();
        assert!(!timeout.contains(&token));
        assert!(!resumed.contains(&token));
    }

    #[test]
    fn capture_sample_age_boundary_remains_strict() {
        assert!(capture_sample_age_fresh(Some(0)));
        assert!(capture_sample_age_fresh(Some(1_499)));
        assert!(!capture_sample_age_fresh(Some(1_500)));
        assert!(!capture_sample_age_fresh(Some(1_501)));
        assert!(!capture_sample_age_fresh(None));
    }

    #[cfg(windows)]
    #[test]
    fn capture_pipe_readable_fresh_observation_does_not_revoke_before_consumption() {
        let _gate = crate::key_gate::lock_gate_tests();
        let _gate_handle = crate::key_gate::KeyGate::start();
        struct ResetSink;
        impl Drop for ResetSink {
            fn drop(&mut self) {
                *lock(&TEST_DIAGNOSTIC_SINK) = None;
            }
        }
        let _sink_reset = ResetSink;
        let injector = Arc::new(CaptureInjector::default());
        let runtime = Arc::new(crate::button_mapping::ButtonMappingRuntime::new(
            injector.clone(),
            Arc::new(crate::UsageCounters::default()),
            Arc::new(Mutex::new(crate::raw_input::RawInputSnapshot::default())),
            Arc::new(crate::macro_runtime::MacroRuntime::new(Arc::new(
                crate::macro_runtime::UnsupportedStepperSource,
            ))),
        ));
        runtime.set_input_context(crate::RemoteModel::Rc003, true);
        let mut mappings = crate::send_input::ButtonMappings::default();
        mappings.actions.insert(
            RemoteButton::Back,
            crate::send_input::ButtonActions {
                single: crate::send_input::ButtonAction::Shortcut {
                    chord: crate::send_input::KeyChord {
                        keys: vec![crate::send_input::KeyCode::Backspace],
                    },
                },
                double: crate::send_input::ButtonAction::Shortcut {
                    chord: crate::send_input::KeyChord {
                        keys: vec![crate::send_input::KeyCode::Enter],
                    },
                },
                hold_repeat: Some(crate::send_input::HoldRepeat::Off),
                ..Default::default()
            },
        );
        runtime.set_mappings(mappings);
        let (bridge, mut stream) = diagnostic_pipe_peer_with_sender(runtime.sender());
        bridge.attach_mapping_runtime(&runtime);
        diagnostic_fixture_barrier();
        let (records, observed) = channel();
        *lock(&TEST_DIAGNOSTIC_SINK) = Some(TestDiagnosticSink {
            records,
            failures: 0,
        });
        let generation = bridge.snapshot().target_generation;
        let old_sample = sampled_tick() - 1_300;
        writeln!(stream, "O 4 2 91 {old_sample} {generation} active f1").unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let record = observed
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap();
            if record.contains("event=engine_ownership") && record.contains("available=true") {
                break;
            }
        }
        assert_eq!(
            lock(&bridge.shared).ownership_sampled_tick_ms,
            Some(old_sample)
        );
        writeln!(stream, "E 4 2 91 {generation} f1").unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while bridge.snapshot().pressed_usages.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(bridge.snapshot().pressed_usages, vec![0xf1]);
        assert!(runtime.wait_for_idle(Duration::from_secs(1)));
        assert_eq!(runtime.snapshot().fired_gestures, 0);
        let evidence = (|| -> Result<_, &'static str> {
            let connection = bridge.current.lock().map_err(|_| "current_poisoned")?;
            let server = connection
                .as_ref()
                .ok_or("missing_connection")?
                .stream
                .try_clone()
                .map_err(|_| "clone_failed")?;
            let held_targets = bridge.targets.lock().map_err(|_| "targets_poisoned")?;
            drop(connection);
            let deadline = Instant::now() + Duration::from_secs(1);
            loop {
                match bridge.current.try_lock() {
                    Err(std::sync::TryLockError::WouldBlock) => break,
                    Err(std::sync::TryLockError::Poisoned(_)) => return Err("current_poisoned"),
                    Ok(guard) => drop(guard),
                }
                if Instant::now() >= deadline {
                    return Err("reader_not_held");
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            while capture_tick_ms() - old_sample < OWNERSHIP_TIMEOUT.as_millis() as u64 {
                if Instant::now() >= deadline {
                    return Err("old_sample_not_expired");
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            let fresh_sample = sampled_tick();
            let fresh_line = format!("O 4 2 95 {fresh_sample} {generation} active f1\n");
            stream
                .write_all(fresh_line.as_bytes())
                .map_err(|_| "write_failed")?;
            let BridgeIo::Pipe(server) = server else {
                return Err("not_named_pipe");
            };
            let mut readable = vec![0u8; fresh_line.len() + 1];
            let mut copied = 0;
            let mut available = 0;
            unsafe {
                windows::Win32::System::Pipes::PeekNamedPipe(
                    windows::Win32::Foundation::HANDLE(server.as_raw_handle()),
                    Some(readable.as_mut_ptr().cast()),
                    readable.len() as u32,
                    Some(&mut copied),
                    Some(&mut available),
                    None,
                )
            }
            .map_err(|_| "peek_failed")?;
            if available as usize != fresh_line.len()
                || &readable[..copied as usize] != fresh_line.as_bytes()
            {
                return Err("complete_fresh_line_not_readable");
            }
            let release_tick = capture_tick_ms();
            if release_tick - old_sample < OWNERSHIP_TIMEOUT.as_millis() as u64
                || !capture_sample_age_fresh(release_tick.checked_sub(fresh_sample))
            {
                return Err("sample_age_precondition_failed");
            }
            drop(held_targets);
            Ok((fresh_sample, release_tick, copied))
        })();
        let (fresh_sample, release_tick, readable_bytes) = evidence.unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while lock(&bridge.shared).ownership_sampled_tick_ms != Some(fresh_sample)
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        let accepted_sample = lock(&bridge.shared).ownership_sampled_tick_ms;
        let owned = bridge.snapshot().owned_usages;
        assert!(runtime.wait_for_idle(Duration::from_secs(1)));
        let cancelled_fires = runtime.snapshot().fired_gestures;
        let cancelled_pressed = bridge.snapshot().pressed_usages;
        diagnostic_fixture_barrier();
        let captured: Vec<_> = observed.try_iter().collect();
        writeln!(stream, "E 4 2 91 {generation} f1").unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        let old_edge_rejected = loop {
            match observed.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(record) if record.contains("reason=pregrant_observation") => break true,
                Ok(_) => {}
                Err(_) => break false,
            }
        };
        writeln!(stream, "E 4 2 95 {generation} f1\nE 4 2 95 {generation} -").unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while runtime.snapshot().fired_gestures == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        let new_fires = runtime.snapshot().fired_gestures;
        assert!(runtime.wait_for_idle(Duration::from_secs(1)));
        drop(stream);
        drop(bridge);
        diagnostic_fixture_barrier();
        *lock(&TEST_DIAGNOSTIC_SINK) = None;
        assert_eq!(accepted_sample, Some(fresh_sample));
        assert_eq!(owned, vec![0xf1]);
        assert_eq!(cancelled_fires, 0, "old held gesture fired during handoff");
        assert!(cancelled_pressed.is_empty());
        assert!(old_edge_rejected);
        assert_eq!(
            new_fires, 1,
            "new edge did not clear the cancellation fence"
        );
        assert_eq!(injector.0.load(Ordering::SeqCst), 1);
        assert!(
            !captured
                .iter()
                .any(|record| record.contains("event=ownership_timeout")),
            "already readable fresh O caused timeout: old_sample={old_sample} fresh_sample={fresh_sample} readable_bytes={readable_bytes} release_tick={release_tick}"
        );
        assert!(!captured.iter().any(|record| {
            record.contains("event=engine_ownership") && record.contains("available=false")
        }));
    }

    #[cfg(windows)]
    #[test]
    fn capture_pipe_queued_invalid_input_does_not_extend_expired_ownership() {
        let _gate = crate::key_gate::lock_gate_tests();
        struct ResetSink;
        impl Drop for ResetSink {
            fn drop(&mut self) {
                *lock(&TEST_DIAGNOSTIC_SINK) = None;
            }
        }
        let _sink_reset = ResetSink;
        for case in [
            "empty",
            "partial",
            "stale",
            "future",
            "epoch",
            "target",
            "sequence",
            "malformed",
            "flood",
            "no_newline_flood",
            "closed_receiver",
        ] {
            let (bridge, mut stream, receiver) = diagnostic_pipe_peer();
            let mut receiver = Some(receiver);
            let generation = bridge.snapshot().target_generation;
            let old_sample = sampled_tick() - 1_300;
            writeln!(stream, "O 4 2 91 {old_sample} {generation} active f1").unwrap();
            let deadline = Instant::now() + Duration::from_secs(1);
            while lock(&bridge.shared).ownership_sampled_tick_ms != Some(old_sample)
                && Instant::now() < deadline
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(
                lock(&bridge.shared).ownership_sampled_tick_ms,
                Some(old_sample)
            );
            let closed_evidence = if matches!(case, "no_newline_flood" | "closed_receiver") {
                diagnostic_fixture_barrier();
                let (records, observed) = channel();
                *lock(&TEST_DIAGNOSTIC_SINK) = Some(TestDiagnosticSink {
                    records,
                    failures: 0,
                });
                Some(observed)
            } else {
                None
            };
            let connection = bridge.current.lock().unwrap();
            let held_targets = bridge.targets.lock().unwrap();
            drop(connection);
            let deadline = Instant::now() + Duration::from_secs(1);
            let reader_held = loop {
                match bridge.current.try_lock() {
                    Err(std::sync::TryLockError::WouldBlock) => break true,
                    Err(std::sync::TryLockError::Poisoned(_)) => break false,
                    Ok(guard) => drop(guard),
                }
                if Instant::now() >= deadline {
                    break false;
                }
                std::thread::sleep(Duration::from_millis(1));
            };
            while capture_tick_ms() - old_sample < 1_500 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            let fresh = sampled_tick();
            let payload = match case {
                "empty" => String::new(),
                "partial" => format!("O 4 2 95 {fresh} {generation} active f1"),
                "stale" => format!("O 4 2 95 {old_sample} {generation} active f1\n"),
                "future" => format!("O 4 2 95 {} {generation} active f1\n", fresh + 10_000),
                "epoch" => format!("O 4 1 95 {fresh} {generation} active f1\n"),
                "target" => format!("O 4 2 95 {fresh} {} active f1\n", generation + 1),
                "sequence" => format!("O 4 2 90 {fresh} {generation} active f1\n"),
                "malformed" => "invalid ownership\n".to_owned(),
                "flood" => "invalid\n".repeat(MAX_LINE_BYTES / 8),
                "no_newline_flood" => "x".repeat(MAX_LINE_BYTES),
                "closed_receiver" => {
                    drop(receiver.take());
                    format!("O 4 2 95 {fresh} {generation} active f1\n")
                }
                _ => unreachable!(),
            };
            let written = stream.write_all(payload.as_bytes());
            let release_tick = capture_tick_ms();
            drop(held_targets);
            assert!(reader_held);
            written.unwrap();
            assert!(release_tick - old_sample >= 1_500);
            let deadline = Instant::now() + Duration::from_secs(1);
            while !bridge.snapshot().owned_usages.is_empty() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(1));
            }
            let snapshot = bridge.snapshot();
            drop(stream);
            drop(bridge);
            if let Some(closed_evidence) = closed_evidence {
                let reason = if case == "no_newline_flood" {
                    "line_too_long"
                } else {
                    "engine_closed"
                };
                let deadline = Instant::now() + Duration::from_secs(1);
                let closed = loop {
                    match closed_evidence
                        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                    {
                        Ok(record) if record.contains(&format!("event=closed reason={reason}")) => {
                            break true
                        }
                        Ok(_) => {}
                        Err(_) => break false,
                    }
                };
                *lock(&TEST_DIAGNOSTIC_SINK) = None;
                assert_ne!(
                    snapshot.phase,
                    BridgePhase::Connected,
                    "{case} did not close before test cleanup"
                );
                assert!(closed, "{case} missing its own close reason");
            }
            assert!(
                snapshot.owned_usages.is_empty(),
                "{case} extended ownership"
            );
            assert!(snapshot.pressed_usages.is_empty());
            if let Some(receiver) = receiver {
                assert!(receiver
                    .try_iter()
                    .any(|event| matches!(event, EngineMessage::HidObservation(0))));
            } else {
                assert_ne!(snapshot.phase, BridgePhase::Connected);
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn capture_available_read_has_one_fixed_byte_budget_and_preserves_partial_line() {
        let _gate = crate::key_gate::lock_gate_tests();
        let (bridge, mut stream, _) = diagnostic_pipe_peer();
        let connection = bridge.current.lock().unwrap();
        let server = connection.as_ref().unwrap().stream.try_clone().unwrap();
        let held_targets = bridge.targets.lock().unwrap();
        drop(connection);
        let deadline = Instant::now() + Duration::from_secs(1);
        let reader_held = loop {
            match bridge.current.try_lock() {
                Err(std::sync::TryLockError::WouldBlock) => break true,
                Err(std::sync::TryLockError::Poisoned(_)) => break false,
                Ok(guard) => drop(guard),
            }
            if Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        let mut reader = BufReader::new(server);
        let mut pending = vec![b'p'; MAX_LINE_BYTES - 16];
        let written = stream.write_all(&vec![b'x'; MAX_LINE_BYTES]);
        let read = read_available_capture_input(&mut reader, &mut pending);
        let second_read = read_available_capture_input(&mut reader, &mut pending);
        drop(held_targets);
        drop(stream);
        drop(bridge);
        assert!(reader_held);
        written.unwrap();
        assert_eq!(read.unwrap(), 16);
        assert_eq!(second_read.unwrap(), 0);
        assert_eq!(pending.len(), MAX_LINE_BYTES);
        assert!(pending[..MAX_LINE_BYTES - 16]
            .iter()
            .all(|byte| *byte == b'p'));
        assert!(pending[MAX_LINE_BYTES - 16..]
            .iter()
            .all(|byte| *byte == b'x'));
        assert!(!pending.contains(&b'\n'));
    }

    #[test]
    fn capture_ipc_blocked_tcp_reader_times_out_releases_and_allows_replacement() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawSocket;
            #[link(name = "Ws2_32")]
            extern "system" {
                fn setsockopt(
                    socket: usize,
                    level: i32,
                    option: i32,
                    value: *const u8,
                    length: i32,
                ) -> i32;
            }
            let receive_bytes = 1_024i32;
            assert_eq!(
                unsafe {
                    setsockopt(
                        peer.stream.as_raw_socket() as usize,
                        0xffff,
                        0x1002,
                        (&receive_bytes as *const i32).cast(),
                        std::mem::size_of::<i32>() as i32,
                    )
                },
                0
            );
        }
        peer.ownership(4, 2, 90, sampled_tick(), "active");
        peer.send(&format!("E 4 2 90 {} f1", peer.generation));
        diagnostic_fixture_barrier();
        let (entered, closed) = channel();
        let (release, released) = channel();
        release.send(()).unwrap();
        *lock(&BLOCKED_DIAGNOSTIC) = Some(BlockedDiagnostic {
            needle: "reason=voice_synth_write_error",
            entered,
            release: released,
        });
        let mut writer = lock(&peer.bridge.current)
            .as_ref()
            .unwrap()
            .stream
            .try_clone()
            .unwrap();
        let payload = vec![b'x'; 16 * 1024 * 1024];
        let started = Instant::now();
        let error = loop {
            match writer.write_all(&payload) {
                Ok(()) => assert!(
                    started.elapsed() < Duration::from_secs(1),
                    "TCP receive buffer never filled"
                ),
                Err(error) => break error,
            }
        };
        assert!(
            matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
            "{error}"
        );
        let closed_note = closed.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(closed_note.contains("reason=voice_synth_write_error"));
        assert!(peer.bridge.snapshot().owned_usages.is_empty());
        assert!(peer.bridge.snapshot().pressed_usages.is_empty());
        assert_eq!(
            driver_edges(&peer.receiver.try_iter().collect::<Vec<_>>()),
            vec![ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: false
            }]
        );
        let text = std::fs::read_to_string(peer.bridge.file.as_ref().unwrap()).unwrap();
        let (port, token, _) = parse_descriptor(&text).unwrap();
        peer.stream = CapturePeer::connect(port, &token);
        peer.ownership(1, 1, 1, sampled_tick(), "active");
        let fresh = peer.send(&format!(
            "E 1 1 1 {} f1\nE 1 1 1 {} -",
            peer.generation, peer.generation
        ));
        assert_eq!(driver_edges(&fresh).len(), 2);
    }

    #[test]
    fn capture_ipc_inactive_observation_ignores_old_epoch_and_connection() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        peer.ownership(4, 3, 93, sampled_tick(), "inactive");
        let messages = peer.send("R 3 3 28\nR 4 2 28\nR 4 3 f1");
        let observed: Vec<_> = messages
            .iter()
            .filter_map(|message| match message {
                EngineMessage::HidObservation(mask) => Some(*mask),
                _ => None,
            })
            .collect();
        assert_eq!(observed, vec![mask(&[RemoteButton::Back])]);
        assert!(driver_edges(&messages).is_empty());
    }

    #[test]
    fn capture_ipc_new_target_rejects_queued_old_generation_edges() {
        let _gate = crate::key_gate::lock_gate_tests();
        let mut peer = CapturePeer::start();
        peer.ownership(4, 2, 90, sampled_tick(), "active");
        let old_generation = peer.generation;
        peer.bridge
            .set_capture_targets(true, mask(&[RemoteButton::Back, RemoteButton::Home]));
        peer.generation = peer.bridge.snapshot().target_generation;
        peer.send(&format!(
            "O 4 3 91 {} {} active f1,4a",
            sampled_tick(),
            peer.generation
        ));
        let rejected = peer.send(&format!(
            "E 4 2 90 {old_generation} f1\nE 4 3 91 {old_generation} f1\nE 3 3 91 {} f1",
            peer.generation
        ));
        assert!(driver_edges(&rejected).is_empty());
        assert!(peer.bridge.snapshot().pressed_usages.is_empty());
        let messages = peer.send(&format!(
            "E 4 3 91 {} f1\nE 4 3 91 {} -",
            peer.generation, peer.generation
        ));
        assert_eq!(
            driver_edges(&messages),
            vec![
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: true
                },
                ButtonEdge {
                    button: RemoteButton::Back,
                    is_pressed: false
                }
            ]
        );
    }

    #[test]
    fn capture_protocol_v3_requires_one_version_and_rejects_legacy_data() {
        assert_eq!(BRIDGE_PROTOCOL_VERSION, 3);
        for line in ["O 1 f1", "E 1 f1", "R 1 f1"] {
            assert!(
                parse_bridge_line(line).is_err(),
                "accepted legacy frame: {line}"
            );
        }
    }

    #[test]
    fn capture_protocol_v3_decodes_epoch_data_without_reinterpreting_usages() {
        for line in ["O 4 2 90 1000 8 active f1", "E 4 2 90 8 f1", "R 4 2 f1"] {
            let parsed = parse_bridge_line(line).unwrap().unwrap();
            let usages = match parsed {
                BridgeLine::Ownership { usages, .. }
                | BridgeLine::Edges { usages, .. }
                | BridgeLine::Observed { usages, .. } => usages,
                other => panic!("unexpected frame: {other:?}"),
            };
            assert_eq!(usages, vec![0xf1], "misdecoded capture frame: {line}");
        }
    }

    #[test]
    fn back_edge_diagnostic_distinguishes_target_filtering_from_pending_ownership() {
        let target = CaptureTargets {
            generation: 7,
            usages: [0x00f1].into_iter().collect(),
        };
        let wanted = [0x00f1].into_iter().collect();
        let unowned = edge_diagnostic_note(4, &target, &wanted, &BTreeSet::new(), 1);
        assert!(unowned.contains("event=edge_received rx_seq=4 target_generation=7"));
        assert!(unowned.contains("usages=f1"));
        assert!(unowned.contains("reason=ownership_missing"));
        assert!(unowned.contains("decoded_edges=1"));
        assert!(unowned.contains("dropped_usages=0"));

        let owned = edge_diagnostic_note(6, &target, &wanted, &wanted, 1);
        assert!(owned.contains("reason=ownership_confirmed"));
        assert_ne!(owned, unowned);

        let disabled = CaptureTargets {
            generation: 8,
            usages: BTreeSet::new(),
        };
        let dropped = edge_diagnostic_note(8, &disabled, &wanted, &wanted, 0);
        assert!(dropped.contains("reason=not_targeted"));
        assert!(dropped.contains("dropped_usages=1"));
        assert!(dropped.contains("decoded_edges=0"));
    }

    #[test]
    fn edge_diagnostic_reports_release_and_whitelist_filter_without_inventing_delivery() {
        let target = CaptureTargets {
            generation: 9,
            usages: [0x00f1].into_iter().collect(),
        };
        let rejected = edge_diagnostic_note(
            1,
            &target,
            &[0xffff].into_iter().collect(),
            &target.usages,
            0,
        );
        assert!(rejected.contains("reason=outside_whitelist"));
        assert!(rejected.contains("dropped_usages=1"));
        assert!(!rejected.contains("ffff"));
        let released = edge_diagnostic_note(2, &target, &BTreeSet::new(), &BTreeSet::new(), 1);
        assert!(released.contains("reason=released_all"));
        assert!(released.contains("phase=received"));
        assert!(!released.contains("enqueued"));
        assert!(!released.contains("passed"));
    }

    #[test]
    fn edge_diagnostic_logs_back_release_and_filters_repeated_snapshots() {
        let back = [0x00f1].into_iter().collect();
        let all_up = BTreeSet::new();
        let other = [0x0028].into_iter().collect();
        assert!(edge_diagnostic_needed(None, &all_up));
        assert!(edge_diagnostic_needed(Some(&all_up), &back));
        assert!(!edge_diagnostic_needed(Some(&back), &back));
        assert!(edge_diagnostic_needed(Some(&back), &all_up));
        assert!(!edge_diagnostic_needed(Some(&all_up), &all_up));
        assert!(!edge_diagnostic_needed(Some(&all_up), &other));
    }

    #[test]
    fn voice_synth_line_only_disables_report_synthesis() {
        assert_eq!(voice_synth_off_line(), "S -");
    }

    #[test]
    fn observation_protocol_is_independent_of_capture_targets_and_excludes_voice() {
        assert_eq!(
            parse_bridge_line("R 4 1 f1,28,65,80"),
            Ok(Some(BridgeLine::Observed {
                conn_gen: 4,
                epoch: 1,
                usages: vec![0xf1, 0x28, 0x65, 0x80]
            }))
        );
        assert_eq!(
            parse_bridge_line("R 4 2 -"),
            Ok(Some(BridgeLine::Observed {
                conn_gen: 4,
                epoch: 2,
                usages: vec![]
            }))
        );
        assert!(parse_bridge_line("R invalid f1").is_err());
        let physical: BTreeSet<_> = ENHANCED_CAPTURE_BUTTON_USAGES
            .iter()
            .map(|(_, usage)| *usage)
            .chain([VOICE_KEY_HID_USAGE])
            .filter(|u| BRIDGE_ALLOWED_USAGES.contains(u))
            .collect();
        assert_eq!(usage_mask(&physical), (1 << 13) - 1);
        assert!(!physical.contains(&VOICE_KEY_HID_USAGE));
    }

    #[test]
    fn voice_gate_line_declares_capability() {
        // helper 侧 parse_bridge_gate_line 的对侧编码，逐字符对齐。
        assert_eq!(voice_gate_line(), "W 1");
    }

    #[test]
    fn parses_synth_ack_lines() {
        // 助手转发 agent 回执的 `A` 行：绝对状态，`-` = 关闭。
        assert_eq!(
            parse_bridge_line("A 00E6").unwrap(),
            Some(BridgeLine::SynthAck { to: Some(0x00E6) })
        );
        assert_eq!(
            parse_bridge_line("A e6").unwrap(),
            Some(BridgeLine::SynthAck { to: Some(0x00E6) })
        );
        assert_eq!(
            parse_bridge_line("A -").unwrap(),
            Some(BridgeLine::SynthAck { to: None })
        );
        assert!(parse_bridge_line("A zz").is_err());
    }

    #[test]
    fn synth_resend_schedule_is_fast_then_steady() {
        // 上次写失败（None）→ 立即重试。
        assert!(synth_resend_due(None, 0));
        // 快节奏：0.5s / 1.5s / 3s。
        assert!(!synth_resend_due(Some(Duration::from_millis(499)), 0));
        assert!(synth_resend_due(Some(Duration::from_millis(500)), 0));
        assert!(!synth_resend_due(Some(Duration::from_millis(1_499)), 1));
        assert!(synth_resend_due(Some(Duration::from_millis(1_500)), 1));
        assert!(!synth_resend_due(Some(Duration::from_millis(2_999)), 2));
        assert!(synth_resend_due(Some(Duration::from_millis(3_000)), 2));
        // 慢节奏兜底：之后每 5s。
        assert!(!synth_resend_due(Some(Duration::from_millis(4_999)), 9));
        assert!(synth_resend_due(Some(Duration::from_millis(5_000)), 9));
    }

    #[test]
    fn parses_hello_and_rejects_malformed() {
        assert_eq!(
            parse_bridge_line("HELLO 1 abc123 4242").unwrap(),
            Some(BridgeLine::Hello {
                version: 1,
                token: "abc123".to_string(),
                helper_pid: 4242
            })
        );
        assert!(parse_bridge_line("HELLO 1 abc123").is_err());
        assert!(parse_bridge_line("HELLO x abc123 1").is_err());
        assert!(parse_bridge_line("HELLO 1  4242").is_err());
        assert!(parse_bridge_line("WHAT 1 2 3").is_err());
    }

    #[test]
    fn parses_edges_in_hex_and_empty_means_release() {
        assert_eq!(
            parse_bridge_line("E 4 2 90 8 f1,80").unwrap(),
            Some(BridgeLine::Edges {
                conn_gen: 4,
                epoch: 2,
                observation_seq: 90,
                generation: 8,
                usages: vec![0x00F1, 0x0080]
            })
        );
        // 带 0x 前缀与大小写混用都要接受：日志与脚本里两种写法都会出现。
        assert_eq!(
            parse_bridge_line("E 4 2 90 8 0x00F1,0X81").unwrap(),
            Some(BridgeLine::Edges {
                conn_gen: 4,
                epoch: 2,
                observation_seq: 90,
                generation: 8,
                usages: vec![0x00F1, 0x0081]
            })
        );
        assert_eq!(
            parse_bridge_line("E 4 2 90 8 -").unwrap(),
            Some(BridgeLine::Edges {
                conn_gen: 4,
                epoch: 2,
                observation_seq: 90,
                generation: 8,
                usages: Vec::new()
            })
        );
        assert!(parse_bridge_line("E 4 2 90 8").is_err());
        assert!(parse_bridge_line("E 1 zz").is_err());
        assert!(parse_bridge_line("E notanumber f1").is_err());
        // 空行与 CRLF 要被容忍（助手侧写的是 LF，但人工用脚本联调时会有 CRLF）。
        assert_eq!(parse_bridge_line("\r\n").unwrap(), None);
        assert_eq!(parse_bridge_line("P 1\r").unwrap(), Some(BridgeLine::Ping));
    }

    #[test]
    fn parses_dynamic_capture_ownership() {
        assert_eq!(
            parse_bridge_line("O 4 2 90 1000 7 active 28,4a,f1").unwrap(),
            Some(BridgeLine::Ownership {
                conn_gen: 4,
                epoch: 2,
                observation_seq: 90,
                sampled_tick_ms: 1000,
                state: CaptureState::Active,
                generation: 7,
                usages: vec![0x0028, 0x004A, 0x00F1],
            })
        );
        assert_eq!(
            parse_bridge_line("O 4 3 91 1001 8 inactive -").unwrap(),
            Some(BridgeLine::Ownership {
                conn_gen: 4,
                epoch: 3,
                observation_seq: 91,
                sampled_tick_ms: 1001,
                state: CaptureState::Inactive,
                generation: 8,
                usages: Vec::new(),
            })
        );
        assert!(parse_bridge_line("O nope f1").is_err());
    }

    #[test]
    fn bridge_whitelist_matches_the_semantic_button_table() {
        let bridge: BTreeSet<_> = BRIDGE_ALLOWED_USAGES.into_iter().collect();
        let semantic: BTreeSet<_> = ENHANCED_CAPTURE_BUTTON_USAGES
            .iter()
            .map(|(_, usage)| *usage)
            .collect();
        assert_eq!(bridge, semantic);
        assert!(!bridge.contains(&0x003E), "voice must stay on ATVV");
    }

    #[test]
    fn token_comparison_is_exact() {
        assert!(token_matches("abcd", "abcd"));
        assert!(!token_matches("abcd", "abce"));
        assert!(!token_matches("abcd", "abc"));
        assert!(!token_matches("", ""));
        assert!(!token_matches("abcd", ""));
    }

    #[test]
    fn descriptor_round_trip() {
        let text = "version=3\nport=53124\ntoken=deadbeefcafe\npid=999\n";
        assert_eq!(
            parse_descriptor(text),
            Some((53124, "deadbeefcafe".to_string(), 3))
        );
        // 版本不符必须整份拒绝：宁可不可用，也不要跑一个半懂的协议。
        assert_eq!(parse_descriptor("version=1\nport=1\ntoken=x\n"), None);
        assert_eq!(parse_descriptor("port=1\ntoken=x\n"), None);
        assert_eq!(parse_descriptor("version=3\nport=1\n"), None);
        assert_eq!(parse_descriptor("version=3\nport=1\ntoken=\n"), None);
    }

    #[test]
    fn descriptor_body_publishes_optional_pipe_and_log_fields() {
        let body = descriptor_body(
            53124,
            Some(r"\\.\pipe\SayAllPersonal.Rc003Bridge"),
            "tok",
            None,
        );
        assert!(body.starts_with("version=3\nport=53124\n"), "{body:?}");
        assert!(
            body.contains("pipe=\\\\.\\pipe\\SayAllPersonal.Rc003Bridge\n"),
            "{body:?}"
        );
        assert!(body.contains("token=tok\n"), "{body:?}");
        assert!(body.contains("pid="), "{body:?}");
        assert!(
            !body.contains("log="),
            "日志未初始化时不得凭空写出路径：{body:?}"
        );
        // 日志已初始化时必须发布 `log=`：助手据此把日志与主程序写进同一文件。
        let body = descriptor_body(1, None, "t", Some(Path::new(r"C:\x\sayall-diagnostic.log")));
        assert!(
            body.contains("log=C:\\x\\sayall-diagnostic.log\n"),
            "{body:?}"
        );
        // 新字段对解析方透明：主程序自己的解析器（与助手逐字对齐）照常工作。
        assert_eq!(parse_descriptor(&body), Some((1, "t".to_string(), 3)));
    }

    #[test]
    fn deny_note_distinguishes_rejection_from_silence() {
        // 被拒必须留下一条**独立**记录：否则"助手没来"与"助手来了被拒"在日志里
        // 是同一形状（2026-10-01 现场正是如此）。
        let line = deny_note("version_mismatch", 4242, 3);
        assert!(line.contains("event=helper_denied"), "{line}");
        assert!(line.contains("reason=version_mismatch"), "{line}");
        assert!(line.contains("helper_pid=4242"), "{line}");
        assert!(line.contains("denied_total=3"), "{line}");
        // 隐私边界：不落 token 值、不落任何路径。
        assert!(!line.contains("token="), "{line}");
        assert!(!line.contains(":\\"), "{line}");
    }

    #[cfg(windows)]
    #[test]
    fn named_pipe_bypasses_loopback_filters_and_delivers_edges() {
        // 桥的生命周期会改写全局门控：按 key_gate 的测试约定串行。
        let _gate = crate::key_gate::lock_gate_tests();
        let dir = std::env::temp_dir().join(format!(
            "sayall-bridge-pipe-descriptor-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (sender, receiver) = channel();
        let bridge = Rc003Bridge::start_in(dir.clone(), sender);
        bridge.set_capture_targets(true, mask(&[RemoteButton::Back]));
        let text = std::fs::read_to_string(dir.join(BRIDGE_FILE_NAME)).expect("描述文件");
        let pipe = text
            .lines()
            .find_map(|line| line.strip_prefix("pipe="))
            .filter(|line| line.starts_with(r"\\.\pipe\SayAllPersonal.Rc003Bridge"))
            .unwrap_or_else(|| {
                panic!(
                    "计划任务 Helper 的 TCP loopback 会被本机 WFP/TUN 重定向，描述文件必须发布命名管道：{text:?}"
                )
            });
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(pipe)
            {
                Ok(stream) => break stream,
                Err(error) if Instant::now() < deadline => {
                    let _ = error;
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("命名管道连接失败: {error}"),
            }
        };
        stream
            .write_all(
                format!("HELLO {BRIDGE_PROTOCOL_VERSION} stale-token-from-previous-app 9001\n")
                    .as_bytes(),
            )
            .unwrap();
        stream.flush().unwrap();

        let mut ack = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while ack.iter().filter(|byte| **byte == b'\n').count() < 2 && Instant::now() < deadline {
            let mut chunk = [0u8; 256];
            match stream.read(&mut chunk) {
                Ok(0) => panic!("命名管道在应答前关闭"),
                Ok(count) => ack.extend_from_slice(&chunk[..count]),
                Err(error) if retryable_bridge_read(&error) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("读取命名管道应答失败: {error}"),
            }
        }
        assert!(
            String::from_utf8_lossy(&ack).starts_with("OK "),
            "命名管道必须以本机 ACL 身份完成 HELLO/OK，不能被陈旧描述 token 卡死；实际收到 {ack:?}"
        );
        assert!(
            String::from_utf8_lossy(&ack).lines().any(|line| line == "S -"),
            "握手必须重放关闭态，避免 helper 重启后 resident agent 保留上一轮 RightAlt；实际收到 {ack:?}"
        );

        let generation = bridge.snapshot().target_generation;
        stream
            .write_all(
                format!(
                    "O 1 1 1 {} {generation} active f1\nE 1 1 1 {generation} f1\n",
                    sampled_tick()
                )
                .as_bytes(),
            )
            .unwrap();
        stream.flush().unwrap();
        let edge = loop {
            let message = receiver
                .recv_timeout(Duration::from_secs(2))
                .expect("命名管道边沿必须送入映射引擎");
            if matches!(message, EngineMessage::DriverEdge(_)) {
                break message;
            }
        };
        assert!(matches!(
            edge,
            EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Back,
                is_pressed: true
            })
        ));
        drop(bridge);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 测试用：读一段管道数据（可重试错误退避后重试，其余错误直接失败）。
    fn read_pipe_chunk(stream: &mut std::fs::File, out: &mut Vec<u8>) {
        let mut chunk = [0u8; 256];
        match stream.read(&mut chunk) {
            Ok(0) => {}
            Ok(count) => out.extend_from_slice(&chunk[..count]),
            Err(error) if retryable_bridge_read(&error) => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("读取命名管道失败: {error}"),
        }
    }

    /// 普通按键增强不启用报告语音合成；错误启用回执必须重新发送关闭命令。
    #[cfg(windows)]
    #[test]
    fn voice_synth_stays_off_and_rejects_stale_enable_ack() {
        // 启停真实门控（桥的连接/收尾会改写它）：按 key_gate 的测试约定串行。
        let _gate = crate::key_gate::lock_gate_tests();
        let dir = std::env::temp_dir().join(format!(
            "sayall-bridge-synth-ack-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (sender, _receiver) = channel();
        let bridge = Rc003Bridge::start_in(dir.clone(), sender);
        // 所有权只在目标集非空时成立：先给一个真实目标（三键恒在，见模块文档）。
        bridge.set_capture_targets(true, mask(&[RemoteButton::Back]));

        let text = std::fs::read_to_string(dir.join(BRIDGE_FILE_NAME)).expect("描述文件");
        let pipe = text
            .lines()
            .find_map(|line| line.strip_prefix("pipe="))
            .filter(|line| line.starts_with(r"\\.\pipe\SayAllPersonal.Rc003Bridge"))
            .expect("描述文件必须发布命名管道");
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(pipe)
            {
                Ok(stream) => break stream,
                Err(error) if Instant::now() < deadline => {
                    let _ = error;
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("命名管道连接失败: {error}"),
            }
        };
        stream
            .write_all(format!("HELLO {BRIDGE_PROTOCOL_VERSION} stale-token 9001\n").as_bytes())
            .unwrap();
        stream.flush().unwrap();

        // 握手重放当前合成目标（`S 00E6` = 右 Alt）。
        let mut received = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !String::from_utf8_lossy(&received).contains("S -") && Instant::now() < deadline {
            read_pipe_chunk(&mut stream, &mut received);
        }
        assert!(
            String::from_utf8_lossy(&received).contains("S -"),
            "握手必须关闭报告合成；实际收到 {received:?}"
        );
        // 未确认 → 按节奏重发（首次 500ms；这里给 1.5s 窗口）。
        let before = received.len();
        let deadline = Instant::now() + Duration::from_millis(1_500);
        while Instant::now() < deadline {
            read_pipe_chunk(&mut stream, &mut received);
            if String::from_utf8_lossy(&received[before..]).contains("S -") {
                break;
            }
        }
        assert!(
            String::from_utf8_lossy(&received[before..]).contains("S -"),
            "未收到回执时必须重发 S 行（幂等自愈）；实际收到 {received:?}"
        );

        stream.write_all(b"A -\n").unwrap();
        stream.flush().unwrap();
        std::thread::sleep(Duration::from_millis(120));
        let before = received.len();
        stream.write_all(b"A 00E6\n").unwrap();
        stream.flush().unwrap();
        let deadline = Instant::now() + Duration::from_millis(400);
        while !String::from_utf8_lossy(&received[before..]).contains("S -")
            && Instant::now() < deadline
        {
            read_pipe_chunk(&mut stream, &mut received);
        }
        assert!(String::from_utf8_lossy(&received[before..]).contains("S -"));
        assert!(!String::from_utf8_lossy(&received).contains("S 00E6"));

        drop(bridge);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 空闲命名管道连接不得忙等（上游 2026-10-02 实验：Helper 一连上，
    /// `sayall-rc003-bridge-conn` 单线程吃满一个核，应用 UI 被饿死、
    /// 点不动也关不掉）。判据 = 空闲窗口内退避次数必须被限速：去掉
    /// `bridge_idle_backoff` 里的 sleep（阳性对照）时同一窗口会到数十万次；
    /// 闸值 5000 留了两个数量级余量（并行测试各自 ≤200 次/400ms）。
    #[cfg(windows)]
    #[test]
    fn idle_named_pipe_connection_is_throttled() {
        // 桥的生命周期会改写全局门控：按 key_gate 的测试约定串行。
        let _gate = crate::key_gate::lock_gate_tests();
        let dir = std::env::temp_dir().join(format!(
            "sayall-bridge-idle-backoff-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (sender, _receiver) = channel();
        let bridge = Rc003Bridge::start_in(dir.clone(), sender);
        let text = std::fs::read_to_string(dir.join(BRIDGE_FILE_NAME)).expect("描述文件");
        let pipe = text
            .lines()
            .find_map(|line| line.strip_prefix("pipe="))
            .expect("描述文件必须发布命名管道")
            .to_owned();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&pipe)
            {
                Ok(stream) => break stream,
                Err(error) if Instant::now() < deadline => {
                    let _ = error;
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("命名管道连接失败: {error}"),
            }
        };
        stream
            .write_all(
                format!("HELLO {BRIDGE_PROTOCOL_VERSION} stale-token-from-previous-app 9002\n")
                    .as_bytes(),
            )
            .unwrap();
        stream.flush().unwrap();
        let mut ack = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !ack.contains(&b'\n') && Instant::now() < deadline {
            let mut chunk = [0u8; 256];
            match stream.read(&mut chunk) {
                Ok(0) => panic!("命名管道在应答前关闭"),
                Ok(count) => ack.extend_from_slice(&chunk[..count]),
                Err(error) if retryable_bridge_read(&error) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("读取命名管道应答失败: {error}"),
            }
        }
        assert!(
            String::from_utf8_lossy(&ack).starts_with("OK "),
            "握手失败：{ack:?}"
        );

        // 握手完成后连接进入空闲（对端不再发数据）——这正是先前忙等的窗口。
        let before = IDLE_BACKOFFS.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(400));
        let backoffs = IDLE_BACKOFFS.load(Ordering::Relaxed) - before;
        assert!(
            backoffs < 5_000,
            "空闲命名管道在 400ms 内退避 {backoffs} 次：退避缺失（忙等）或间隔被改小"
        );
        drop(bridge);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn whitelist_blocks_non_target_usages() {
        let shared = Arc::new(Mutex::new(BridgeShared::default()));
        let targets = targets(&[0x00F1]);
        // 主页属于增强白名单，但不在本代动态目标中；未知 usage 也必须丢弃。
        let wanted: BTreeSet<u16> = [0x00F1, 0x004A, 0x1234].into_iter().collect();
        let edges = apply_usages(&shared, &targets, &wanted);
        assert_eq!(
            edges,
            vec![ButtonEdge {
                button: crate::raw_input::RemoteButton::Back,
                is_pressed: true
            }],
            "只有 0x00F1 该通过；0x4A 与 0x1234 必须被丢弃"
        );
        let state = lock(&shared);
        assert_eq!(state.usages_dropped, 2);
        assert_eq!(state.pressed, [0x00F1].into_iter().collect());
    }

    #[test]
    fn diff_emits_press_then_release() {
        let shared = Arc::new(Mutex::new(BridgeShared::default()));
        let targets = targets(&[0x0080]);
        let press = apply_usages(&shared, &targets, &[0x0080].into_iter().collect());
        assert_eq!(
            press,
            vec![ButtonEdge {
                button: crate::raw_input::RemoteButton::VolumeUp,
                is_pressed: true
            }]
        );
        // 同一集合重复上报不得产生重复边沿（助手侧"变化才发"，但去重不能只靠对端）。
        assert!(apply_usages(&shared, &targets, &[0x0080].into_iter().collect()).is_empty());
        let release = apply_usages(&shared, &targets, &BTreeSet::new());
        assert_eq!(
            release,
            vec![ButtonEdge {
                button: crate::raw_input::RemoteButton::VolumeUp,
                is_pressed: false
            }]
        );
        assert!(lock(&shared).pressed.is_empty());
    }

    #[test]
    fn dynamic_targets_accept_other_buttons_only_while_selected() {
        let shared = Arc::new(Mutex::new(BridgeShared::default()));
        let targets = targets(&[0x004A]);
        assert_eq!(
            apply_usages(&shared, &targets, &[0x004A].into_iter().collect()),
            vec![ButtonEdge {
                button: RemoteButton::Home,
                is_pressed: true,
            }]
        );

        lock(&targets).usages.clear();
        assert_eq!(
            apply_usages(&shared, &targets, &BTreeSet::new()),
            vec![ButtonEdge {
                button: RemoteButton::Home,
                is_pressed: false,
            }]
        );
        assert!(apply_usages(&shared, &targets, &[0x004A].into_iter().collect()).is_empty());
        assert_eq!(lock(&shared).usages_dropped, 1);
    }

    /// 端到端（离线、免提权、免设备）：真 TcpStream 走完整协议。
    #[test]
    fn end_to_end_loopback_delivers_edges() {
        // 桥的生命周期会改写全局门控：按 key_gate 的测试约定串行。
        let _gate = crate::key_gate::lock_gate_tests();
        let dir = std::env::temp_dir().join(format!("sayall-bridge-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (sender, receiver) = channel();
        let bridge = Rc003Bridge::start_in(dir.clone(), sender);
        bridge.set_capture_targets(true, mask(&[RemoteButton::Back, RemoteButton::VolumeUp]));

        let descriptor_path = dir.join(BRIDGE_FILE_NAME);
        let text = std::fs::read_to_string(&descriptor_path).expect("描述文件必须已写出");
        let (port, token, _version) = parse_descriptor(&text).expect("描述文件必须可解析");

        let mut stream = TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
            .expect("必须能连上桥接端口");
        stream
            .set_read_timeout(Some(Duration::from_millis(2_000)))
            .ok();
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));

        let mut greeting = String::new();
        let _ = reader.read_line(&mut greeting);
        assert!(greeting.is_empty(), "鉴权前桥接不得主动发任何东西");

        // 错令牌必须被拒，且不得进入已连接状态。
        stream.write_all(b"HELLO 1 wrongtoken 777\n").unwrap();
        stream.flush().unwrap();
        let mut deny = String::new();
        reader.read_line(&mut deny).expect("必须收到 DENY");
        assert!(deny.starts_with("DENY "), "实际收到 {deny:?}");

        // 正确令牌。
        let hello = format!("HELLO {BRIDGE_PROTOCOL_VERSION} {token} 777\n");
        stream.write_all(hello.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut ok = String::new();
        reader.read_line(&mut ok).expect("必须收到 OK");
        assert!(ok.starts_with("OK "), "实际收到 {ok:?}");
        let ok_parts: Vec<_> = ok.trim().split(' ').collect();
        let generation = ok_parts[2].parse::<u64>().expect("OK 携带目标代次");
        assert_eq!(ok_parts[3], "80,f1", "OK 携带当前动态目标");
        let mut synth = String::new();
        reader
            .read_line(&mut synth)
            .expect("HELLO 必须重放合成状态");
        assert_eq!(synth, "S -\n", "初始关闭态也必须是绝对状态");
        // 门内延迟能力声明（2026-10-03）：与 S 行同批、紧随其后（编码见 voice_gate_line）。
        let mut gate = String::new();
        reader
            .read_line(&mut gate)
            .expect("HELLO 必须声明门内延迟能力");
        assert_eq!(gate, "W 1\n", "能力声明必须与 helper 的 W 行解析逐字符对齐");
        // 按 2026-10-03 加固协议回执关闭态：未确认期间主程序会按节奏重发 S 行，
        // 确认后停止——后续断言才不会被重发行干扰（回执同时也是门禁闭环的输入）。
        stream.write_all(b"A -\n").unwrap();
        stream.flush().unwrap();
        stream
            .write_all(format!("O 1 1 1 {} {generation} active f1,80\n", sampled_tick()).as_bytes())
            .unwrap();
        stream.flush().unwrap();
        for _ in 0..20 {
            if bridge.snapshot().owned_usages == vec![0x0080, 0x00F1] {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert_eq!(
            bridge.snapshot().owned_usages,
            vec![0x0080, 0x00F1],
            "只有 agent ACK 后才建立增强所有权"
        );

        // 两个当前目标键按下。
        stream
            .write_all(format!("E 1 1 1 {generation} f1,80\n").as_bytes())
            .unwrap();
        stream.flush().unwrap();
        let mut first = Vec::new();
        let mut second = Vec::new();
        for _ in 0..40 {
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(EngineMessage::DriverEdge(edge)) => {
                    if edge.is_pressed {
                        if edge.button == crate::raw_input::RemoteButton::Back {
                            first.push(edge);
                        } else {
                            second.push(edge);
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
            if !first.is_empty() && !second.is_empty() {
                break;
            }
        }
        assert_eq!(first.len(), 1, "返回键应有且只有一次按下边沿");
        assert_eq!(second.len(), 1, "音量+应有且只有一次按下边沿");

        // 全部释放。
        stream
            .write_all(format!("E 1 1 1 {generation} -\n").as_bytes())
            .unwrap();
        stream.flush().unwrap();
        let mut released = 0;
        for _ in 0..40 {
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(EngineMessage::DriverEdge(edge)) if !edge.is_pressed => released += 1,
                Ok(_) => {}
                Err(_) => break,
            }
            if released == 2 {
                break;
            }
        }
        assert_eq!(released, 2, "两个键都必须收到释放边沿");

        // 助手若仍连着但不再续报目标所有权，主程序必须在 helper 的 2 秒租约前
        // 先撤销报告层所有权并暂停自定义映射。P 只维持桥接连接，
        // 不应续期所有权。
        for _ in 0..7 {
            stream.write_all(b"P\n").unwrap();
            stream.flush().unwrap();
            std::thread::sleep(Duration::from_millis(250));
        }
        for _ in 0..20 {
            if bridge.snapshot().owned_usages.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(
            bridge.snapshot().owned_usages.is_empty(),
            "所有权超时后必须撤销映射能力，物理键盘继续透传"
        );

        // 热更新为主页键：主程序先撤销旧所有权，再下发新一代目标；主页边沿
        // 此后应走同一条 GateEdge 通道，而不是全局物理键盘钩子。
        bridge.set_capture_targets(true, mask(&[RemoteButton::Home]));
        // 所有权租约到期时桥接会先写一行 `S -` 之类的通知；热更新行 `T ...` 可能排在
        // 它后面。读到 `T ` 为止，不要把通知误判成热更新失败（2026-10-25 全量测试偶发）。
        let mut target_line = String::new();
        for _ in 0..8 {
            target_line.clear();
            reader
                .read_line(&mut target_line)
                .expect("必须收到 T 热更新");
            if target_line.starts_with("T ") {
                break;
            }
        }
        assert!(target_line.starts_with("T "), "实际收到 {target_line:?}");
        assert!(target_line.trim().ends_with("4a"));
        let generation = bridge.snapshot().target_generation;
        stream
            .write_all(
                format!(
                    "O 1 2 2 {} {generation} active 4a\nE 1 2 2 {generation} 4a\n",
                    sampled_tick()
                )
                .as_bytes(),
            )
            .unwrap();
        stream.flush().unwrap();
        let home = loop {
            let message = receiver
                .recv_timeout(Duration::from_secs(2))
                .expect("主页边沿应被动态桥接");
            if matches!(message, EngineMessage::DriverEdge(_)) {
                break message;
            }
        };
        assert!(matches!(
            home,
            EngineMessage::DriverEdge(ButtonEdge {
                button: RemoteButton::Home,
                is_pressed: true
            })
        ));

        bridge.set_capture_targets(false, mask(&[RemoteButton::Home]));
        assert!(bridge.snapshot().owned_usages.is_empty());
        assert!(
            bridge.snapshot().target_usages.is_empty(),
            "关闭全按键支持必须清空增强目标并恢复原生输入"
        );

        drop(bridge);
        assert!(
            !descriptor_path.exists(),
            "桥接停止后描述文件必须被清理，否则助手会一直对着死端口重连"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 新连接接管旧连接后，**接手方必须继续正常工作**。
    ///
    /// 这条用例钉住一个"写出来又被自测抓到"的缺陷：收尾时无条件清掉当前连接，
    /// 会把刚接手的新连接一起清掉，于是新连接下一轮认为自己被替换、主动让位——
    /// 两个连接互相让位，桥接整体哑掉。现场表现是助手重启后按键没有响应，
    /// 而且日志上看不出任何错误（两边都只是安静退出）。
    ///
    /// 同时覆盖第二半：被接管的旧连接**不得**释放状态，否则会误伤接手方
    /// 刚建立的按下状态（表现为"刚按下就被松开"）。
    #[test]
    fn replacement_takes_over_and_survivor_keeps_working() {
        // 桥的生命周期会改写全局门控：按 key_gate 的测试约定串行。
        let _gate = crate::key_gate::lock_gate_tests();
        let dir =
            std::env::temp_dir().join(format!("sayall-bridge-replace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (sender, receiver) = channel();
        let bridge = Rc003Bridge::start_in(dir.clone(), sender);
        bridge.set_capture_targets(true, mask(&[RemoteButton::Back, RemoteButton::VolumeUp]));

        let text = std::fs::read_to_string(dir.join(BRIDGE_FILE_NAME)).expect("描述文件");
        let (port, token, _version) = parse_descriptor(&text).expect("可解析");

        let connect = |helper_pid: u32| {
            let mut stream =
                TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).expect("连接");
            stream
                .set_read_timeout(Some(Duration::from_millis(2_000)))
                .ok();
            let hello = format!("HELLO {BRIDGE_PROTOCOL_VERSION} {token} {helper_pid}\n");
            stream.write_all(hello.as_bytes()).unwrap();
            stream.flush().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut ok = String::new();
            reader.read_line(&mut ok).unwrap();
            assert!(ok.starts_with("OK "), "实际收到 {ok:?}");
            stream
        };

        let mut alpha = connect(1001);
        let generation = bridge.snapshot().target_generation;
        alpha
            .write_all(
                format!(
                    "O 1 1 1 {} {generation} active f1,80\nE 1 1 1 {generation} f1\n",
                    sampled_tick()
                )
                .as_bytes(),
            )
            .unwrap();
        alpha.flush().unwrap();
        let mut saw_back = false;
        for _ in 0..40 {
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(EngineMessage::DriverEdge(edge))
                    if edge.is_pressed && edge.button == crate::raw_input::RemoteButton::Back =>
                {
                    saw_back = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(saw_back, "alpha 必须先被认作当前连接并能投递边沿");

        // beta 接管。等 alpha 走完让位收尾：它不该释放任何状态。
        let mut beta = connect(1002);
        std::thread::sleep(Duration::from_millis(600));
        while receiver.try_recv().is_ok() {}
        assert_eq!(bridge.snapshot().replaced_total, 1, "必须记录一次接管");

        beta.write_all(
            format!(
                "O 1 1 1 {} {generation} active f1,80\nE 1 1 1 {generation} 80\n",
                sampled_tick()
            )
            .as_bytes(),
        )
        .unwrap();
        beta.flush().unwrap();
        let mut saw_volume = false;
        for _ in 0..40 {
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(EngineMessage::DriverEdge(edge))
                    if edge.is_pressed
                        && edge.button == crate::raw_input::RemoteButton::VolumeUp =>
                {
                    saw_volume = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(saw_volume, "接手方 beta 必须还能正常投递边沿");

        drop(bridge);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 静默看门狗：助手被强杀（不回 BYE、不发释放）时，主程序必须自行释放全部按键。
    ///
    /// 没有这一环，引擎会永远以为按键还按着，进而触发长按/连发语义——
    /// 这是 fail-open 合同里最容易漏掉、也最难在现场察觉的一条。
    #[test]
    fn silence_watchdog_releases_pressed_buttons() {
        // 桥的生命周期会改写全局门控：按 key_gate 的测试约定串行。
        let _gate = crate::key_gate::lock_gate_tests();
        let dir =
            std::env::temp_dir().join(format!("sayall-bridge-watchdog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (sender, receiver) = channel();
        let bridge = Rc003Bridge::start_in(dir.clone(), sender);
        bridge.set_capture_targets(true, mask(&[RemoteButton::Back]));

        let text = std::fs::read_to_string(dir.join(BRIDGE_FILE_NAME)).expect("描述文件");
        let (port, token, _version) = parse_descriptor(&text).expect("可解析");

        let mut stream =
            TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).expect("连接");
        stream
            .set_read_timeout(Some(Duration::from_millis(2_000)))
            .ok();
        let hello = format!("HELLO {BRIDGE_PROTOCOL_VERSION} {token} 2001\n");
        stream.write_all(hello.as_bytes()).unwrap();
        stream.flush().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut ok = String::new();
        reader.read_line(&mut ok).unwrap();
        assert!(ok.starts_with("OK "), "实际收到 {ok:?}");

        let generation = bridge.snapshot().target_generation;
        stream
            .write_all(
                format!(
                    "O 1 1 1 {} {generation} active f1\nE 1 1 1 {generation} f1\n",
                    sampled_tick()
                )
                .as_bytes(),
            )
            .unwrap();
        stream.flush().unwrap();
        let mut pressed = false;
        for _ in 0..40 {
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(EngineMessage::DriverEdge(edge)) if edge.is_pressed => {
                    pressed = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(pressed, "先要有一次按下边沿");

        // 模拟"助手被强杀"：不写 BYE、不发释放。
        //
        // 必须 `shutdown` 而不是只 `drop(stream)`：测试里 reader 持有同一 socket 的
        // 另一个句柄，只丢一个引用连接并不会断开，服务器读不到 EOF，
        // "被强杀"这个场景就没被真正构造出来（第一版测试正是栽在这里）。
        stream.shutdown(std::net::Shutdown::Both).ok();
        drop(stream);
        drop(reader);
        let mut released = false;
        for _ in 0..60 {
            match receiver.recv_timeout(Duration::from_millis(500)) {
                Ok(EngineMessage::DriverEdge(edge)) if !edge.is_pressed => {
                    released = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(
            released,
            "连接消失后必须自动释放按下状态，否则映射会卡在长按/连发语义"
        );
        // Edge delivery precedes the worker's final snapshot update. Observe
        // that completion separately instead of racing the channel receiver.
        let deadline = Instant::now() + Duration::from_secs(1);
        while bridge.snapshot().watchdog_release_total == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            bridge.snapshot().watchdog_release_total >= 1,
            "强制释放必须被计数，否则现场无法区分'助手正常收尾'与'被强杀'"
        );

        drop(bridge);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 轮询等待（桥接线程是异步的，断言不能靠固定 sleep 压时序）。
    fn wait_until(limit: Duration, mut probe: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if probe() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}
