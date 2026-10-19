//! Disk-only transformation. Runtime config/DTO serialization remains unchanged.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
const PREFIX: &str = "dpapi:v1:";
const ERROR: &str = "本机密钥保护失败，原配置未修改；请使用原 Windows 用户和机器恢复设置。";

fn paths(value: &Value) -> Vec<String> {
    let mut paths = vec![
        "/ai/apiKey".into(),
        "/translation/apiKey".into(),
        "/wechatClawbot/bridgeToken".into(),
    ];
    if let Some(profiles) = value.pointer("/ai/profiles").and_then(Value::as_array) {
        paths.extend((0..profiles.len()).map(|index| format!("/ai/profiles/{index}/apiKey")));
    }
    paths
}

pub fn decrypt(value: &mut Value) -> Result<(), String> {
    for path in paths(value) {
        if let Some(field) = value.pointer_mut(&path) {
            if let Some(encoded) = field.as_str().and_then(|text| text.strip_prefix(PREFIX)) {
                let bytes = STANDARD.decode(encoded).map_err(|_| ERROR)?;
                *field =
                    Value::String(String::from_utf8(dpapi(&bytes, false)?).map_err(|_| ERROR)?);
            }
        }
    }
    Ok(())
}

pub fn protect(
    value: &mut Value,
    previous: Option<&Value>,
    explicit_save: bool,
) -> Result<(), String> {
    for path in paths(value) {
        let Some(field) = value.pointer_mut(&path) else {
            continue;
        };
        let Some(secret) = field.as_str() else {
            continue;
        };
        if secret.is_empty() {
            continue;
        }
        let old = previous
            .and_then(|old| old.pointer(&path))
            .and_then(Value::as_str);
        // Automatic position/preference saves keep legacy plaintext as-is.
        // Previously protected fields never fall back to plaintext.
        if !explicit_save && old == Some(secret) && !secret.starts_with(PREFIX) {
            continue;
        }
        *field = Value::String(format!(
            "{PREFIX}{}",
            STANDARD.encode(dpapi(secret.as_bytes(), true)?)
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn dpapi(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(bytes.len()).map_err(|_| ERROR)?,
        pbData: bytes.as_ptr() as *mut u8,
    };
    struct OutputBlob(CRYPT_INTEGER_BLOB);
    impl Drop for OutputBlob {
        fn drop(&mut self) {
            if !self.0.pbData.is_null() {
                unsafe {
                    // Volatile writes keep clearing observable to the compiler.
                    for index in 0..self.0.cbData as usize {
                        std::ptr::write_volatile(self.0.pbData.add(index), 0);
                    }
                    LocalFree(self.0.pbData as _);
                }
            }
        }
    }
    let mut output = OutputBlob(CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    });
    // DPAPI owns output; copy, zero and release it on both success and failure.
    unsafe {
        let ok = if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.0,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output.0,
            )
        };
        if ok == 0 || output.0.pbData.is_null() {
            return Err(ERROR.into());
        }
        let result = std::slice::from_raw_parts(output.0.pbData, output.0.cbData as usize).to_vec();
        Ok(result)
    }
}

#[cfg(not(windows))]
fn dpapi(_bytes: &[u8], _encrypt: bool) -> Result<Vec<u8>, String> {
    Err("当前平台不支持 Windows DPAPI，未写入密钥。".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn explicit_save_encrypts_all_fields_and_round_trips() {
        let original = json!({"ai":{"apiKey":"global-placeholder","profiles":[{"apiKey":"profile-placeholder"}]},"translation":{"apiKey":"translation-placeholder"},"wechatClawbot":{"bridgeToken":"bridge-placeholder"}});
        let mut value = original.clone();
        protect(&mut value, Some(&original), true).unwrap();
        assert!(!value.to_string().contains("placeholder"));
        decrypt(&mut value).unwrap();
        assert_eq!(value, original);
    }

    #[test]
    fn automatic_save_does_not_migrate_legacy_but_keeps_encrypted_fields_protected() {
        let old = json!({"ai":{"apiKey":"synthetic-key"}});
        let mut value = old.clone();
        protect(&mut value, Some(&old), false).unwrap();
        assert_eq!(value, old);
        protect(&mut value, Some(&old), true).unwrap();
        let protected = value.clone();
        decrypt(&mut value).unwrap();
        protect(&mut value, Some(&protected), false).unwrap();
        assert!(value["ai"]["apiKey"].as_str().unwrap().starts_with(PREFIX));
        decrypt(&mut value).unwrap();
        assert_eq!(value, old);
    }

    #[test]
    fn corrupt_protection_fails_without_clear_or_plaintext_fallback() {
        let mut value = json!({"translation":{"apiKey":"dpapi:v1:AAAA"}});
        let original = value.clone();
        assert!(decrypt(&mut value).is_err());
        assert_eq!(value, original);
    }

    #[test]
    fn protected_fields_can_be_replaced_or_cleared_explicitly() {
        let mut previous = json!({"translation":{"apiKey":"old-placeholder"}});
        protect(&mut previous, None, true).unwrap();
        let mut replaced = json!({"translation":{"apiKey":"new-placeholder"}});
        protect(&mut replaced, Some(&previous), true).unwrap();
        decrypt(&mut replaced).unwrap();
        assert_eq!(replaced["translation"]["apiKey"], "new-placeholder");
        let mut cleared = json!({"translation":{"apiKey":""}});
        protect(&mut cleared, Some(&previous), true).unwrap();
        decrypt(&mut cleared).unwrap();
        assert_eq!(cleared["translation"]["apiKey"], "");
    }
}
