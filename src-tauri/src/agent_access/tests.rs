//! `agent_access` 的单元测试：状态落盘、四个客户端的写入/回滚、冲突与只读边界。

use super::*;
use sayall_core::agent_access::hash_agent_token;

/// 在公开CLI边界记录add-json输入, get返回对应的真实字段形状, 不接触本机客户端。
pub(super) fn claude_command_fixture(
    codes: Vec<i32>,
) -> impl Fn(&Path, &[String]) -> Result<(i32, String), String> + Send + Sync {
    let state = Mutex::new((std::collections::VecDeque::from(codes), None::<Value>));
    move |_, args| {
        let mut state = state.lock().unwrap();
        let status = state.0.pop_front().unwrap_or(0);
        if args.get(1).map(String::as_str) == Some("add-json") {
            state.1 = Some(serde_json::from_str(args.last().unwrap()).unwrap());
        }
        let output = if args.get(1).map(String::as_str) == Some("get") {
            let config = state.1.as_ref().unwrap();
            let environment = config["env"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(key, value)| format!("    {key}={}", value.as_str().unwrap()))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{AGENT_MCP_SERVER_NAME}:\n  Scope: User config (available in all your projects)\n  Status: ✓ Connected\n  Type: stdio\n  Command: {}\n  Args: {}\n  Environment:\n{environment}\n", config["command"].as_str().unwrap(), config["args"].as_array().unwrap().iter().map(|arg| arg.as_str().unwrap()).collect::<Vec<_>>().join(" "))
        } else {
            String::new()
        };
        Ok((status, output))
    }
}

struct Env {
    base: PathBuf,
    home: PathBuf,
    executable_directory: PathBuf,
    store: AgentAccessStore,
}

impl Env {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let base = std::env::temp_dir().join(format!(
            "sayall-agent-access-{}-{nanos}",
            std::process::id()
        ));
        let home = base.join("home");
        let executable_directory = base.join("bin");
        std::fs::create_dir_all(home.join(".codex")).expect("codex dir");
        std::fs::create_dir_all(home.join(".cursor")).expect("cursor dir");
        std::fs::create_dir_all(home.join(".config/opencode")).expect("opencode dir");
        std::fs::create_dir_all(&executable_directory).expect("bin dir");
        std::fs::write(executable_directory.join("sayall-mcp.exe"), b"MZ").expect("mcp binary");
        std::fs::write(executable_directory.join("claude.exe"), b"MZ").expect("claude stub");
        let store = AgentAccessStore::for_tests(
            base.join("mcp"),
            home.clone(),
            executable_directory.clone(),
        );
        store.set_enabled(true).expect("enable");
        Self {
            base,
            home,
            executable_directory,
            store,
        }
    }

    fn codex_config(&self) -> PathBuf {
        self.home.join(".codex/config.toml")
    }

    fn cursor_config(&self) -> PathBuf {
        self.home.join(".cursor/mcp.json")
    }

    fn open_code_config(&self) -> PathBuf {
        self.home.join(".config/opencode/opencode.json")
    }

    fn access_text(&self) -> String {
        std::fs::read_to_string(self.base.join("mcp/access.json")).expect("access file")
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.base).ok();
    }
}

#[test]
fn empty_snapshot_is_disabled_with_four_clients() {
    let env = Env::new();
    env.store.set_enabled(false).expect("disable");
    let snapshot = env.store.snapshot();
    assert!(!snapshot.enabled);
    assert!(snapshot.authorizations.is_empty());
    assert_eq!(
        snapshot
            .clients
            .iter()
            .map(|client| client.id.as_str())
            .collect::<Vec<_>>(),
        vec!["codex", "claude-code", "cursor", "opencode"]
    );
    assert!(snapshot.clients.iter().all(|client| client.available));
}

#[test]
fn opencode_uses_frontend_id_and_accepts_legacy_serde_id() {
    assert_eq!(
        serde_json::to_string(&AgentClientId::OpenCode).unwrap(),
        "\"opencode\""
    );
    assert_eq!(
        serde_json::from_str::<AgentClientId>("\"opencode\"").unwrap(),
        AgentClientId::OpenCode
    );
    assert_eq!(
        serde_json::from_str::<AgentClientId>("\"open-code\"").unwrap(),
        AgentClientId::OpenCode
    );
}

#[test]
fn enabling_persists_and_is_visible_to_a_fresh_store() {
    let env = Env::new();
    env.store.set_enabled(false).expect("disable");
    assert!(!env.store.snapshot().enabled);
    env.store.set_enabled(true).expect("enable");
    assert!(env.store.snapshot().enabled);
    assert!(env.access_text().contains("\"enabled\": true"));

    let reopened = AgentAccessStore::for_tests(
        env.base.join("mcp"),
        env.home.clone(),
        env.executable_directory.clone(),
    );
    assert!(reopened.snapshot().enabled);
}

#[test]
fn manual_authorization_returns_configs_and_stores_only_the_hash() {
    let env = Env::new();
    let created = env.store.create_manual("Codex 客户端").expect("created");
    assert_eq!(created.display_name, "Codex 客户端");
    assert_eq!(created.snapshot.authorizations.len(), 1);
    assert!(created.standard_json.contains("sayall-mcp.exe"));
    assert!(created.standard_json.contains("sayall_personal_history"));
    assert!(created
        .codex_toml
        .starts_with("[mcp_servers.sayall_personal_history]"));
    assert!(!created.codex_toml.contains("\\/"));

    // 从生成的配置里取回明文令牌，确认磁盘上只有哈希。
    let parsed: Value = serde_json::from_str(&created.standard_json).expect("json");
    let token = parsed["mcpServers"]["sayall_personal_history"]["env"]["SAYALL_MCP_ACCESS_TOKEN"]
        .as_str()
        .expect("token")
        .to_owned();
    let access = env.access_text();
    assert!(!access.contains(&token), "明文令牌不得落盘");
    assert!(access.contains(&hash_agent_token(&token)));
}

#[test]
fn revoke_hides_the_authorization_but_keeps_history() {
    let env = Env::new();
    let created = env.store.create_manual("Codex").expect("created");
    let client_id = created.snapshot.authorizations[0].client_id.clone();
    let snapshot = env.store.revoke(&client_id).expect("revoke");
    assert!(snapshot.authorizations.is_empty());
    let access: Value = serde_json::from_str(&env.access_text()).expect("json");
    let history = access["authorizations"].as_array().expect("array");
    assert_eq!(history.len(), 1);
    assert!(history[0]["revokedAtMs"].is_number());
}

#[test]
fn codex_connect_appends_managed_block_and_disconnect_removes_it() {
    let env = Env::new();
    let original = "[mcp_servers.other]\ncommand = \"other.exe\"\n";
    std::fs::write(env.codex_config(), original).expect("seed");

    let snapshot = env.store.connect(AgentClientId::Codex).expect("connect");
    assert_eq!(snapshot.authorizations.len(), 1);
    let text = std::fs::read_to_string(env.codex_config()).expect("read");
    assert!(text.contains("[mcp_servers.other]"), "不得改动别人的配置");
    assert!(text.contains(CODEX_BLOCK_BEGIN));
    assert!(text.contains("sayall-mcp.exe"));
    // Windows 路径的转义必须成对，否则 Codex 会拒绝整个文件。
    assert!(text.contains("\\\\sayall-mcp.exe") || text.contains("\\\\bin\\\\sayall-mcp.exe"));

    let snapshot = env
        .store
        .disconnect(AgentClientId::Codex)
        .expect("disconnect");
    assert!(snapshot.authorizations.is_empty());
    let text = std::fs::read_to_string(env.codex_config()).expect("read");
    assert!(!text.contains(CODEX_BLOCK_BEGIN));
    assert!(text.contains("[mcp_servers.other]"));
}

#[test]
fn revoke_removes_the_client_configuration_and_reconnect_succeeds() {
    let env = Env::new();
    let snapshot = env.store.connect(AgentClientId::Codex).expect("connect");
    let client_id = snapshot.authorizations[0].client_id.clone();
    assert!(
        std::fs::read_to_string(env.codex_config())
            .expect("read")
            .contains(CODEX_BLOCK_BEGIN),
        "连接后应有托管块"
    );

    // 用户点「撤销」：必须连配置一起清掉，否则重连必然撞冲突。
    let revoked = env.store.revoke(&client_id).expect("revoke");
    assert!(revoked.authorizations.is_empty());
    let text = std::fs::read_to_string(env.codex_config()).expect("read");
    assert!(!text.contains(CODEX_BLOCK_BEGIN), "撤销后托管块应被移除");

    let reconnected = env.store.connect(AgentClientId::Codex).expect("reconnect");
    assert_eq!(reconnected.authorizations.len(), 1);
    let text = std::fs::read_to_string(env.codex_config()).expect("read");
    assert_eq!(
        text.matches(CODEX_BLOCK_BEGIN).count(),
        1,
        "重连后恰好一个托管块：{text}"
    );
}

#[test]
fn connect_replaces_an_orphaned_managed_block_left_by_older_revoke() {
    let env = Env::new();
    // 旧版本撤销时没清理配置，留下带标记的孤儿块（用户实际撞到的现场）。
    let orphan = format!(
        "{begin}\n[mcp_servers.sayall_personal_history]\ncommand = \"old\\\\path.exe\"\nargs = [\"serve\"]\n{end}\n",
        begin = CODEX_BLOCK_BEGIN,
        end = CODEX_BLOCK_END
    );
    std::fs::write(env.codex_config(), &orphan).expect("seed");

    let snapshot = env.store.connect(AgentClientId::Codex).expect("connect");
    assert_eq!(snapshot.authorizations.len(), 1);
    let text = std::fs::read_to_string(env.codex_config()).expect("read");
    assert_eq!(
        text.matches(CODEX_BLOCK_BEGIN).count(),
        1,
        "孤儿块应被整体替换：{text}"
    );
    assert!(text.contains("sayall-mcp.exe"), "应写入当前可执行文件路径");
}

#[test]
fn codex_conflict_still_fires_for_a_foreign_entry_without_markers() {
    let env = Env::new();
    std::fs::write(
        env.codex_config(),
        "[mcp_servers.sayall_personal_history]\ncommand = \"someone-else.exe\"\n",
    )
    .expect("seed");
    let error = env.store.connect(AgentClientId::Codex).unwrap_err();
    assert_eq!(error.code, "configuration_conflict");
    assert!(env.store.snapshot().authorizations.is_empty());
}

#[test]
fn codex_conflict_leaves_no_authorization_behind() {
    let env = Env::new();
    std::fs::write(
        env.codex_config(),
        "[mcp_servers.sayall_personal_history]\ncommand = \"someone-else.exe\"\n",
    )
    .expect("seed");
    let error = env.store.connect(AgentClientId::Codex).unwrap_err();
    assert_eq!(error.code, "configuration_conflict");
    assert_eq!(error.client, Some(AgentClientId::Codex));
    assert!(
        env.store.snapshot().authorizations.is_empty(),
        "失败的连接不得留下授权"
    );
    let text = std::fs::read_to_string(env.codex_config()).expect("read");
    assert!(text.contains("someone-else.exe"), "冲突时不得覆盖");
}

#[test]
fn cursor_invalid_json_is_rejected_and_rolled_back() {
    let env = Env::new();
    std::fs::write(env.cursor_config(), "{ \"mcpServers\": { oops }").expect("seed");
    let error = env.store.connect(AgentClientId::Cursor).unwrap_err();
    assert_eq!(error.code, "invalid_client_configuration");
    assert!(env.store.snapshot().authorizations.is_empty());
    let text = std::fs::read_to_string(env.cursor_config()).expect("read");
    assert!(text.contains("oops"), "坏文件必须原样保留");
}

#[test]
fn cursor_merge_preserves_other_servers_and_removal_only_drops_ours() {
    let env = Env::new();
    std::fs::write(
        env.cursor_config(),
        serde_json::to_string_pretty(&serde_json::json!({
            "mcpServers": {"other": {"command": "other.exe"}},
            "unrelated": true
        }))
        .expect("json"),
    )
    .expect("seed");

    env.store.connect(AgentClientId::Cursor).expect("connect");
    let merged: Value =
        serde_json::from_str(&std::fs::read_to_string(env.cursor_config()).expect("read"))
            .expect("json");
    assert!(merged["mcpServers"]["other"].is_object());
    assert_eq!(
        merged["mcpServers"]["sayall_personal_history"]["type"],
        "stdio"
    );
    assert_eq!(merged["unrelated"], Value::Bool(true));

    env.store
        .disconnect(AgentClientId::Cursor)
        .expect("disconnect");
    let after: Value =
        serde_json::from_str(&std::fs::read_to_string(env.cursor_config()).expect("read"))
            .expect("json");
    assert!(after["mcpServers"]["other"].is_object());
    assert!(after["mcpServers"]["sayall_personal_history"].is_null());
    assert_eq!(after["unrelated"], Value::Bool(true));
}

#[test]
fn open_code_writes_a_local_entry() {
    let env = Env::new();
    env.store.connect(AgentClientId::OpenCode).expect("connect");
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(env.open_code_config()).expect("read"))
            .expect("json");
    let entry = &config["mcp"]["sayall_personal_history"];
    assert_eq!(entry["type"], "local");
    assert_eq!(entry["enabled"], Value::Bool(true));
    assert!(entry["command"][0]
        .as_str()
        .unwrap_or_default()
        .ends_with("sayall-mcp.exe"));
    assert_eq!(entry["command"][1], "serve");
}

#[test]
fn claude_code_rejection_rolls_the_authorization_back() {
    let mut env = Env::new();
    // 第一次是预检 `--version`（通过），第二次是 add-json（被客户端拒绝）。
    env.store.set_command_results(vec![0, 3]);
    let error = env.store.connect(AgentClientId::ClaudeCode).unwrap_err();
    assert_eq!(error.code, "client_installation_rejected");
    assert!(env.store.snapshot().authorizations.is_empty());
}

#[test]
fn claude_code_success_keeps_the_authorization() {
    let env = Env::new();
    env.store
        .connect(AgentClientId::ClaudeCode)
        .expect("connect");
    let snapshot = env.store.snapshot();
    assert_eq!(snapshot.authorizations.len(), 1);
    assert_eq!(
        snapshot.authorizations[0].integration,
        Some(AgentClientId::ClaudeCode)
    );
}

#[test]
fn unavailable_client_is_reported_without_writing_anything() {
    let mut env = Env::new();
    env.store.set_available(vec![AgentClientId::Codex]);
    let snapshot = env.store.snapshot();
    assert!(!snapshot.clients[2].available);
    let error = env.store.connect(AgentClientId::Cursor).unwrap_err();
    assert_eq!(error.code, "client_unavailable");
    assert!(env.store.snapshot().authorizations.is_empty());
}

#[test]
fn missing_mcp_binary_blocks_authorization_creation() {
    let env = Env::new();
    std::fs::remove_file(env.executable_directory.join("sayall-mcp.exe")).expect("remove");
    let error = env.store.create_manual("Codex").unwrap_err();
    assert_eq!(error.code, "local_component_missing");
    assert!(env.store.snapshot().authorizations.is_empty());
}

#[test]
fn stale_authorization_reports_needs_reconnect() {
    let env = Env::new();
    env.store.create_manual("Codex").expect("created");
    // 换一个可执行文件所在目录 = 安装位置变化。
    let moved = env.base.join("bin2");
    std::fs::create_dir_all(&moved).expect("bin2");
    std::fs::write(moved.join("sayall-mcp.exe"), b"MZ").expect("binary");
    let reopened = AgentAccessStore::for_tests(env.base.join("mcp"), env.home.clone(), moved);
    let snapshot = reopened.snapshot();
    assert!(snapshot.authorizations[0].needs_reconnect);
}

#[test]
fn managed_codex_range_matches_exactly_one_block() {
    let text = format!("a\n{CODEX_BLOCK_BEGIN}\nbody\n{CODEX_BLOCK_END}\nz\n");
    let (start, end) = managed_codex_range(&text).expect("range");
    assert_eq!(
        &text[start..end],
        format!("{CODEX_BLOCK_BEGIN}\nbody\n{CODEX_BLOCK_END}\n")
    );
    assert!(managed_codex_range("no block here").is_none());
}

#[test]
fn claude_code_success_without_matching_public_readback_rolls_authorization_back() {
    let mut env = Env::new();
    env.store.command_override = Some(std::sync::Arc::new(|_, _| Ok((0, String::new()))));
    let error = env.store.connect(AgentClientId::ClaudeCode).unwrap_err();
    assert_eq!(error.code, "client_configuration_write_failed");
    assert!(env.store.snapshot().authorizations.is_empty());
}

#[test]
fn personal_server_markers_do_not_match_upstream_configuration() {
    assert!(managed_codex_range("# BEGIN SAYALL MANAGED MCP sayall_history\nbody\n# END SAYALL MANAGED MCP sayall_history\n").is_none());
    assert_eq!(AGENT_MCP_SERVER_NAME, "sayall_personal_history");
}

#[test]
fn claude_install_failure_reports_unconfirmed_rollback_without_erasing_prior_authorization() {
    let mut env = Env::new();
    env.store.create_manual("Existing client").unwrap();
    let previous = env.store.read_state().unwrap().authorizations[0].clone();
    let temporary = env.base.join("mcp/access.json.tmp");
    env.store.command_override = Some(std::sync::Arc::new(move |_, args| {
        if args.get(1).map(String::as_str) == Some("add-json") {
            std::fs::create_dir(&temporary).unwrap();
            return Ok((3, String::new()));
        }
        Ok((0, String::new()))
    }));
    let error = env.store.connect(AgentClientId::ClaudeCode).unwrap_err();
    assert_eq!(error.code, "authorization_rollback_unconfirmed");
    let retained = env.store.read_state().unwrap();
    assert_eq!(retained.authorizations[0], previous);
    assert!(retained.enabled);
}

#[test]
fn failed_install_rolls_back_from_locked_state_even_if_the_file_was_corrupted() {
    let mut env = Env::new();
    env.store.create_manual("Existing client").unwrap();
    let previous = env.store.read_state().unwrap();
    let access = env.base.join("mcp/access.json");
    env.store.command_override = Some(std::sync::Arc::new(move |_, args| {
        if args.get(1).map(String::as_str) == Some("add-json") {
            std::fs::write(&access, b"{broken").unwrap();
            return Ok((3, String::new()));
        }
        Ok((0, String::new()))
    }));
    assert_eq!(
        env.store
            .connect(AgentClientId::ClaudeCode)
            .unwrap_err()
            .code,
        "client_installation_rejected"
    );
    assert_eq!(env.store.read_state().unwrap(), previous);
}

#[test]
fn claude_name_and_command_without_the_current_credentials_cannot_confirm_connection() {
    let mut env = Env::new();
    let executable = env.executable_directory.join("sayall-mcp.exe");
    env.store.command_override = Some(std::sync::Arc::new(move |_, _| {
        Ok((0, format!("{AGENT_MCP_SERVER_NAME}:\n  Type: stdio\n  Command: {}\n  Args: serve\n  Environment:\n    SAYALL_MCP_CLIENT_ID=old-client\n    SAYALL_MCP_ACCESS_TOKEN=old-token\n", executable.display())))
    }));
    assert_eq!(
        env.store
            .connect(AgentClientId::ClaudeCode)
            .unwrap_err()
            .code,
        "client_configuration_write_failed"
    );
    assert!(env.store.read_state().unwrap().authorizations.is_empty());
}

#[test]
fn failed_atomic_replace_preserves_the_existing_authorization_bytes() {
    let env = Env::new();
    let original = std::fs::read(env.store.access_file()).unwrap();
    let missing = env.base.join("missing-temporary.json");
    assert!(replace_file(&missing, &env.store.access_file()).is_err());
    assert_eq!(std::fs::read(env.store.access_file()).unwrap(), original);
}

#[test]
fn claude_readback_rejects_failed_status_changed_args_and_hidden_credentials() {
    for broken in ["status", "args", "client", "token", "hidden"] {
        let mut env = Env::new();
        let runner = claude_command_fixture(Vec::new());
        env.store.command_override = Some(std::sync::Arc::new(move |path, args| {
            let (status, output) = runner(path, args)?;
            let output = output
                .lines()
                .map(|line| {
                    if broken == "status" && line.contains("Status:") {
                        "  Status: ✘ Failed to connect".to_owned()
                    } else if broken == "args" && line.contains("Args:") {
                        "  Args: obsolete".to_owned()
                    } else if broken == "client" && line.contains("SAYALL_MCP_CLIENT_ID=") {
                        "    SAYALL_MCP_CLIENT_ID=previous-client".to_owned()
                    } else if matches!(broken, "token" | "hidden")
                        && line.contains("SAYALL_MCP_ACCESS_TOKEN=")
                    {
                        format!(
                            "    SAYALL_MCP_ACCESS_TOKEN={}",
                            if broken == "hidden" {
                                "[REDACTED]"
                            } else {
                                "previous-token"
                            }
                        )
                    } else {
                        line.to_owned()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            Ok((status, output))
        }));
        assert_eq!(
            env.store
                .connect(AgentClientId::ClaudeCode)
                .unwrap_err()
                .code,
            "client_configuration_write_failed",
            "{broken}"
        );
        assert!(
            env.store.read_state().unwrap().authorizations.is_empty(),
            "{broken}"
        );
    }
}
