use std::fmt;

use once_cell::sync::Lazy;
use regex::RegexSet;
use serde::{Deserialize, Serialize};

use crate::contracts::ErrorDetails;

/// Mirrors `src/shared/errors.ts:EASY_VIRTUAL_DISPLAY_ERROR_CODES`. Order does not matter; the
/// `snake_case` rename keeps the JSON identical to the TypeScript union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EasyVirtualDisplayErrorCode {
    DriverNotInstalled,
    DriverDisabled,
    DriverRestartRequired,
    DriverError,
    LimitExceeded,
    DisplayNotFound,
    UnsupportedMode,
    AdminCancelled,
    DriverInstallerMissing,
    DriverUninstallFailed,
    DriverUninstallNotInstalled,
    DotnetRuntimeMissing,
    NativeHostUnavailable,
    ConfigApplyTimeout,
}

/// The wire-level error shape the renderer sees when a Tauri command rejects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EasyVirtualDisplayError {
    pub code: EasyVirtualDisplayErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub details: Option<ErrorDetails>,
}

impl EasyVirtualDisplayError {
    pub fn new(code: EasyVirtualDisplayErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: ErrorDetails) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.details
            .get_or_insert_with(ErrorDetails::new)
            .insert(key.into(), value);
        self
    }
}

impl fmt::Display for EasyVirtualDisplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EasyVirtualDisplayError {}

impl From<EasyVirtualDisplayError> for serde_json::Value {
    fn from(value: EasyVirtualDisplayError) -> Self {
        serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
    }
}

/// Patterns mirrored verbatim from `src/main/errors.ts:DOTNET_RUNTIME_MISSING_PATTERNS`.
/// Compiled once via `RegexSet` for cheap repeat matching against accumulated stderr.
static DOTNET_RUNTIME_MISSING_PATTERNS: Lazy<RegexSet> = Lazy::new(|| {
    RegexSet::new([
        r"(?i)you must install or update \.net",
        r#"(?i)framework:\s*['"]microsoft\.netcore\.app['"]"#,
        r"(?i)aka\.ms/dotnet-core-applaunch",
        r"(?i)learn about runtime installation",
        r"(?i)\.net location:\s*not found",
    ])
    .expect("dotnet runtime missing patterns must compile")
});

pub const DOTNET_RUNTIME_MISSING_MESSAGE: &str =
    ".NET 8 Runtime (x64) is required to start the bundled native host.";

pub fn matches_dotnet_runtime_missing(text: &str) -> bool {
    DOTNET_RUNTIME_MISSING_PATTERNS.is_match(text)
}

/// Map a raw JSON-RPC error payload (the `error` object inside the response) into our
/// renderer-facing error. The .NET host's `HostErrorMapper.Normalize` already produces an
/// `{ code, message, details? }` shape inside `error.data`; we just lift that.
pub fn parse_rpc_error(error: &serde_json::Value) -> EasyVirtualDisplayError {
    if let Some(data) = error.get("data") {
        if let Ok(parsed) = serde_json::from_value::<EasyVirtualDisplayError>(data.clone()) {
            return parsed;
        }
    }

    let message = error
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Native host returned an error response.")
        .to_string();
    EasyVirtualDisplayError::new(EasyVirtualDisplayErrorCode::DriverError, message)
}

/// Parses a JSON `{ code, message, details? }` blob coming out of an elevated process's
/// stderr. If parsing fails or the code isn't one of ours, falls back to the supplied
/// `fallback_code` with `payload.trim()` as the message (or `fallback_message` when the
/// payload is empty). Mirrors `src/main/errors.ts:parseSerializedError`.
pub fn parse_serialized_error(
    payload: &str,
    fallback_code: EasyVirtualDisplayErrorCode,
    fallback_message: &str,
) -> EasyVirtualDisplayError {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return EasyVirtualDisplayError::new(fallback_code, fallback_message.to_string());
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Ok(parsed) = serde_json::from_value::<EasyVirtualDisplayError>(value.clone()) {
            return parsed;
        }
        if let Some(msg) = value.get("message").and_then(|m| m.as_str()) {
            if !msg.is_empty() {
                return EasyVirtualDisplayError::new(fallback_code, msg.to_string());
            }
        }
    }

    EasyVirtualDisplayError::new(fallback_code, trimmed.to_string())
}

/// Mirrors `src/main/errors.ts:isUserCancelledElevation`.
pub fn is_user_cancelled_elevation(payload: &str) -> bool {
    let p = payload.to_ascii_lowercase();
    p.contains("canceled by the user")
        || p.contains("cancelled by the user")
        || p.contains("operation was canceled")
        || p.contains("operation was cancelled")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serializes_as_camelcase_object() {
        let err = EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::DotnetRuntimeMissing,
            "missing",
        )
        .with_detail("stderr", serde_json::json!("oops"));

        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"code\":\"dotnet_runtime_missing\""));
        assert!(json.contains("\"message\":\"missing\""));
        assert!(json.contains("\"details\":{\"stderr\":\"oops\"}"));
    }

    #[test]
    fn error_without_details_omits_field() {
        let err = EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::NativeHostUnavailable,
            "down",
        );
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains("details"));
    }

    #[test]
    fn dotnet_missing_patterns_catch_known_messages() {
        assert!(matches_dotnet_runtime_missing(
            "You must install or update .NET to run this application."
        ));
        assert!(matches_dotnet_runtime_missing(
            "Framework: 'Microsoft.NETCore.App', version '8.0.0' (x64)"
        ));
        assert!(matches_dotnet_runtime_missing(
            "Learn about runtime installation:"
        ));
        assert!(matches_dotnet_runtime_missing(
            "https://aka.ms/dotnet-core-applaunch"
        ));
        assert!(matches_dotnet_runtime_missing(".NET location: Not found"));
    }

    #[test]
    fn dotnet_missing_patterns_reject_unrelated_text() {
        assert!(!matches_dotnet_runtime_missing("driver_not_installed"));
        assert!(!matches_dotnet_runtime_missing("unrelated stderr line"));
    }

    #[test]
    fn parse_rpc_error_lifts_data_field() {
        let payload = serde_json::json!({
            "code": -32000,
            "message": "outer",
            "data": {
                "code": "driver_not_installed",
                "message": "Driver is missing.",
            }
        });
        let parsed = parse_rpc_error(&payload);
        assert_eq!(parsed.code, EasyVirtualDisplayErrorCode::DriverNotInstalled);
        assert_eq!(parsed.message, "Driver is missing.");
    }

    #[test]
    fn parse_rpc_error_falls_back_to_driver_error() {
        let payload = serde_json::json!({
            "code": -32601,
            "message": "Method not found"
        });
        let parsed = parse_rpc_error(&payload);
        assert_eq!(parsed.code, EasyVirtualDisplayErrorCode::DriverError);
        assert_eq!(parsed.message, "Method not found");
    }

    #[test]
    fn parse_serialized_error_lifts_known_code_from_json() {
        let payload = r#"{"code":"driver_installer_missing","message":"Missing installer"}"#;
        let err = parse_serialized_error(
            payload,
            EasyVirtualDisplayErrorCode::DriverError,
            "fallback",
        );
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::DriverInstallerMissing);
        assert_eq!(err.message, "Missing installer");
    }

    #[test]
    fn parse_serialized_error_falls_back_when_json_lacks_known_code() {
        let payload = r#"{"foo":"bar","message":"raw text"}"#;
        let err = parse_serialized_error(
            payload,
            EasyVirtualDisplayErrorCode::DriverError,
            "fallback",
        );
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::DriverError);
        assert_eq!(err.message, "raw text");
    }

    #[test]
    fn parse_serialized_error_uses_raw_payload_when_not_json() {
        let payload = "Access is denied.";
        let err = parse_serialized_error(
            payload,
            EasyVirtualDisplayErrorCode::DriverUninstallFailed,
            "fallback",
        );
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::DriverUninstallFailed);
        assert_eq!(err.message, "Access is denied.");
    }

    #[test]
    fn parse_serialized_error_returns_fallback_for_empty_payload() {
        let err = parse_serialized_error(
            "   ",
            EasyVirtualDisplayErrorCode::DriverError,
            "default message",
        );
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::DriverError);
        assert_eq!(err.message, "default message");
    }

    #[test]
    fn user_cancelled_elevation_recognizes_variants() {
        assert!(is_user_cancelled_elevation(
            "The operation was canceled by the user."
        ));
        assert!(is_user_cancelled_elevation(
            "Operation was cancelled by the user"
        ));
        assert!(!is_user_cancelled_elevation("some other error"));
    }
}
