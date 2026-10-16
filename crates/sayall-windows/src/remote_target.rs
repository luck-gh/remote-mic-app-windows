//! 当前 BLE 选择的进程内设备身份，仅用于 Raw Input 多设备消歧。
//! 保留个人 Helper 的独立精确来源契约，不写跨进程身份文件。

use std::sync::atomic::{AtomicU64, Ordering};

/// 0 = 未知（没有过滤条件，保持旧行为：唯一候选才绑定、多候选仍拒绝）。
static TARGET_ADDRESS: AtomicU64 = AtomicU64::new(0);

/// 发布/清除「正在用的遥控器」。`None` 或 0 = 未知。
pub fn set(address: Option<u64>) {
    let next = address.filter(|value| *value != 0).unwrap_or(0);
    let previous = TARGET_ADDRESS.swap(next, Ordering::Relaxed);
    if previous == next {
        return;
    }
    // 只记"有没有值"——地址本身是设备身份，不进日志（LOGGING.md 隐私红线）。
    note(format!(
        "remote_target action={} source=ble_session",
        if next == 0 { "cleared" } else { "published" }
    ));
}

/// 诊断日志（与其它平台模块同一个载体；非 Windows 上没有该载体，直接丢弃）。
fn note(message: String) {
    #[cfg(windows)]
    crate::ble::gatt_note(message);
    #[cfg(not(windows))]
    let _ = message;
}

/// 当前目标（`None` = 未知）。
pub fn get() -> Option<u64> {
    match TARGET_ADDRESS.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    }
}

/// 地址的 12 位十六进制小写形态——与 Raw Input 设备路径里的写法一致
/// （`…_dev_vid&012717_pid&32b8_rev&00a4_c05d39c2d9db#…`）。
pub fn address_token(address: u64) -> String {
    format!("{address:012x}")
}

/// 文本（设备路径 / 设备实例 id）是否属于这支遥控器。不区分大小写：
/// Raw Input 路径是小写、注册表实例 id 是大写。
pub fn matches_address(text: &str, address: u64) -> bool {
    text.to_ascii_lowercase().contains(&address_token(address))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 路径匹配：Raw Input 路径小写、注册表实例 id 大写，两种都要命中；别的地址不命中。
    #[test]
    fn address_matching_is_case_insensitive_and_exact() {
        let address = 0x00C0_5D39_C2D9_DB;
        assert_eq!(address_token(address), "c05d39c2d9db");
        let raw_input_path = r"\\?\hid#{00001812-0000-1000-8000-00805f9b34fb}_dev_vid&012717_pid&32b8_rev&00a4_c05d39c2d9db#a&29fe826e&1&0000#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}";
        assert!(matches_address(raw_input_path, address));
        let registry_instance = r"BTHLEDevice\{00001812-0000-1000-8000-00805F9B34FB}_DEV_VID&012717_PID&32B8_REV&00A4_C05D39C2D9DB\9&220DF34&1&0055";
        assert!(matches_address(registry_instance, address));
        // 另一支同型号遥控器：不命中（这正是 2026-10-13 现场的两支）。
        let other = r"BTHLEDevice\{00001812-0000-1000-8000-00805F9B34FB}_DEV_VID&012717_PID&32B8_REV&00A4_E85540B1B76B\9&2E2B90C9&1&0055";
        assert!(!matches_address(other, address));
    }
}
