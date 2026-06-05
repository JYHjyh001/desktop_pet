use std::process::Command;

use serde::Serialize;

use crate::app_data::WechatClawbotSettings;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WechatClawbotSendResult {
    pub ok: bool,
    pub message: String,
}

pub fn send_message(
    settings: &WechatClawbotSettings,
    message: &str,
) -> Result<WechatClawbotSendResult, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("微信消息不能为空".to_string());
    }

    if !settings.enabled {
        return Err("微信 ClawBot 未启用".to_string());
    }

    let command = settings.openclaw_command.trim();
    if command.is_empty() {
        return Err("请填写 OpenClaw 命令，例如 openclaw".to_string());
    }

    let channel = settings.channel.trim();
    if channel.is_empty() {
        return Err("请填写 ClawBot 通道名，例如 openclaw-weixin".to_string());
    }

    let target = settings.target.trim();
    if target.is_empty() {
        return Err("请填写微信目标会话 ID 或联系人标识".to_string());
    }

    let mut process = Command::new(command);
    process.args(["message", "send", "--channel", channel, "--target", target]);

    let account = settings.account.trim();
    if !account.is_empty() {
        process.args(["--account", account]);
    }

    process.args(["--message", message]);

    let output = process
        .output()
        .map_err(|err| format!("无法运行 OpenClaw 命令：{err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            format!("OpenClaw 发送失败，退出码 {:?}", output.status.code())
        } else {
            format!("OpenClaw 发送失败：{detail}")
        });
    }

    Ok(WechatClawbotSendResult {
        ok: true,
        message: "已通过 ClawBot 发送到微信".to_string(),
    })
}
