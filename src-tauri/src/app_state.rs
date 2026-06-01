//! Process-wide runtime state managed by Tauri. Holds the JSON-RPC backend handle and the
//! mutable bits the 11 Seam-A commands need to read & mutate (settings, effective language,
//! install-prompt latch).
//!
//! Locks are `std::sync::Mutex` (not `tokio::sync::Mutex`) because every critical section
//! is short and does no `.await` — keep them out of the async machinery.

use std::sync::{Arc, Mutex};

use crate::boundaries::SystemBoundaries;
use crate::contracts::{
    resolve_effective_language, AppSettings, AppSnapshot, EffectiveLanguage, HostSnapshot,
};
use crate::elevator::Elevator;
use crate::host::StdioJsonRpcBackend;

pub struct AppRuntime {
    pub backend: Arc<StdioJsonRpcBackend>,
    pub boundaries: SystemBoundaries,
    pub elevator: Arc<dyn Elevator>,
    pub system_locale: String,
    settings: Mutex<AppSettings>,
    effective_language: Mutex<EffectiveLanguage>,
    install_prompt_shown: Mutex<bool>,
}

impl AppRuntime {
    pub fn new(
        backend: Arc<StdioJsonRpcBackend>,
        boundaries: SystemBoundaries,
        elevator: Arc<dyn Elevator>,
        settings: AppSettings,
        system_locale: String,
    ) -> Self {
        let effective_language = resolve_effective_language(settings.language, &system_locale);
        Self {
            backend,
            boundaries,
            elevator,
            system_locale,
            settings: Mutex::new(settings),
            effective_language: Mutex::new(effective_language),
            install_prompt_shown: Mutex::new(false),
        }
    }

    pub fn settings_snapshot(&self) -> AppSettings {
        self.settings.lock().expect("settings poisoned").clone()
    }

    pub fn effective_language(&self) -> EffectiveLanguage {
        *self.effective_language.lock().expect("language poisoned")
    }

    pub fn compose_app_snapshot(&self, host: HostSnapshot) -> AppSnapshot {
        AppSnapshot {
            host,
            settings: self.settings_snapshot(),
            effective_language: self.effective_language(),
        }
    }

    /// Replace the settings wholesale (caller has already merged the patch). Returns the
    /// new effective language *only when it changed*, so callers know whether to emit
    /// the `language-changed` event.
    pub fn replace_settings(&self, next: AppSettings) -> Option<EffectiveLanguage> {
        let new_eff = resolve_effective_language(next.language, &self.system_locale);

        {
            let mut cur = self.settings.lock().expect("settings poisoned");
            *cur = next;
        }

        let mut cur_eff = self.effective_language.lock().expect("language poisoned");
        if *cur_eff != new_eff {
            *cur_eff = new_eff;
            Some(new_eff)
        } else {
            None
        }
    }

    pub fn install_prompt_shown(&self) -> bool {
        *self
            .install_prompt_shown
            .lock()
            .expect("install prompt poisoned")
    }

    pub fn set_install_prompt_shown(&self, shown: bool) {
        *self
            .install_prompt_shown
            .lock()
            .expect("install prompt poisoned") = shown;
    }

    /// Atomically claim the install-prompt latch under a single lock. Returns `true` iff
    /// the caller is the one that flipped it from false→true (and so should show the
    /// dialog). Concurrent snapshot broadcasts would otherwise race on a read-then-write
    /// pair and stack two dialogs.
    pub fn try_claim_install_prompt(&self) -> bool {
        let mut shown = self
            .install_prompt_shown
            .lock()
            .expect("install prompt poisoned");
        if *shown {
            false
        } else {
            *shown = true;
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boundaries::SystemBoundaries;
    use crate::contracts::{default_app_settings, empty_host_snapshot, AppLanguage};
    use crate::host::resolve_stdio_command;

    fn build_runtime() -> AppRuntime {
        // No real AppHandle here, so we can't call resolve_stdio_command. Instead build a
        // backend with a no-op factory — the tests below don't start it.
        let factory = std::sync::Arc::new(|| {
            // Anything; never invoked.
            let _ = resolve_stdio_command;
            crate::host::HostCommand {
                program: "noop".into(),
                args: vec![],
                cwd: std::env::temp_dir(),
                env: std::collections::HashMap::new(),
            }
        });
        let backend = StdioJsonRpcBackend::new(factory, empty_host_snapshot());
        AppRuntime::new(
            backend,
            SystemBoundaries::noop_for_tests(),
            crate::elevator::create(),
            default_app_settings(),
            "en-US".into(),
        )
    }

    #[test]
    fn effective_language_changes_when_user_picks_zh() {
        let rt = build_runtime();
        assert_eq!(rt.effective_language(), EffectiveLanguage::En);

        let mut next = rt.settings_snapshot();
        next.language = AppLanguage::ZhCn;
        let changed = rt.replace_settings(next);
        assert_eq!(changed, Some(EffectiveLanguage::ZhCn));
        assert_eq!(rt.effective_language(), EffectiveLanguage::ZhCn);
    }

    #[test]
    fn effective_language_stays_when_only_other_keys_change() {
        let rt = build_runtime();
        let mut next = rt.settings_snapshot();
        next.keep_screen_on = !next.keep_screen_on;
        let changed = rt.replace_settings(next);
        assert!(changed.is_none());
    }

    #[test]
    fn try_claim_install_prompt_is_one_shot_until_reset() {
        let rt = build_runtime();
        assert!(rt.try_claim_install_prompt());
        assert!(!rt.try_claim_install_prompt());
        assert!(!rt.try_claim_install_prompt());
        rt.set_install_prompt_shown(false);
        assert!(rt.try_claim_install_prompt());
    }

    #[test]
    fn compose_app_snapshot_includes_current_runtime_fields() {
        let rt = build_runtime();
        let mut host = empty_host_snapshot();
        host.revision = 42;
        let snap = rt.compose_app_snapshot(host);
        assert_eq!(snap.host.revision, 42);
        assert_eq!(snap.effective_language, EffectiveLanguage::En);
        assert_eq!(snap.settings.language, AppLanguage::System);
    }
}
