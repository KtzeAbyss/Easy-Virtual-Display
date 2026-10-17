//! Process-wide runtime state managed by Tauri. Holds the JSON-RPC backend handle and the
//! mutable bits the 11 Seam-A commands need to read & mutate (settings, effective language,
//! install-prompt latch).
//!
//! Locks are `std::sync::Mutex` (not `tokio::sync::Mutex`) because every critical section
//! is short and does no `.await` — keep them out of the async machinery.

use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Runtime};

use crate::boundaries::SystemBoundaries;
use crate::contracts::{
    empty_host_snapshot, resolve_effective_language, AppSettings, AppSnapshot, EffectiveLanguage,
    HostSnapshot,
};
use crate::elevator::Elevator;
use crate::errors::EasyVirtualDisplayErrorCode;
use crate::host::StdioJsonRpcBackend;
use crate::viewer::ViewerHost;

pub struct AppRuntime {
    pub backend: Arc<StdioJsonRpcBackend>,
    pub boundaries: SystemBoundaries,
    pub elevator: Arc<dyn Elevator>,
    pub viewers: Arc<dyn ViewerHost>,
    pub system_locale: String,
    settings: Mutex<AppSettings>,
    effective_language: Mutex<EffectiveLanguage>,
    install_prompt_shown: Mutex<bool>,
    fallback_error: Mutex<Option<EasyVirtualDisplayErrorCode>>,
    published_host: Mutex<PublishedHost>,
}

struct PublishedHost {
    session_id: u64,
    snapshot: HostSnapshot,
    /// Stamp of the last composed [`AppSnapshot`]. Command replies and events reach the
    /// renderer over separate channels, so it keeps whichever copy carries the higher stamp.
    seq: u64,
}

impl PublishedHost {
    fn accept(&mut self, session_id: u64, snapshot: HostSnapshot) -> bool {
        if session_id < self.session_id
            || (session_id == self.session_id && snapshot.revision <= self.snapshot.revision)
        {
            return false;
        }
        self.session_id = session_id;
        self.snapshot = snapshot;
        true
    }
}

impl AppRuntime {
    pub fn new(
        backend: Arc<StdioJsonRpcBackend>,
        boundaries: SystemBoundaries,
        elevator: Arc<dyn Elevator>,
        viewers: Arc<dyn ViewerHost>,
        settings: AppSettings,
        system_locale: String,
    ) -> Self {
        let effective_language = resolve_effective_language(settings.language, &system_locale);
        Self {
            backend,
            boundaries,
            elevator,
            viewers,
            system_locale,
            settings: Mutex::new(settings),
            effective_language: Mutex::new(effective_language),
            install_prompt_shown: Mutex::new(false),
            fallback_error: Mutex::new(None),
            published_host: Mutex::new(PublishedHost {
                session_id: 0,
                snapshot: empty_host_snapshot(),
                seq: 0,
            }),
        }
    }

    pub fn settings_snapshot(&self) -> AppSettings {
        self.settings.lock().expect("settings poisoned").clone()
    }

    pub fn effective_language(&self) -> EffectiveLanguage {
        *self.effective_language.lock().expect("language poisoned")
    }

    /// Latest host state as published to every consumer — the same state viewer reaping
    /// and the renderer work from, not the backend's possibly newer cache.
    pub fn published_host(&self) -> HostSnapshot {
        self.published_host
            .lock()
            .expect("published host poisoned")
            .snapshot
            .clone()
    }

    /// Compose the current app snapshot for a command reply, stamped in the same order
    /// as emitted snapshots.
    pub fn current_snapshot(&self) -> AppSnapshot {
        let mut published = self.published_host.lock().expect("published host poisoned");
        self.compose_snapshot(&mut published)
    }

    /// Publish a host-driven snapshot in sidecar-session order. Older callbacks are
    /// discarded before they can reap viewers or overwrite renderer state.
    pub fn publish_host_snapshot<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        session_id: u64,
        host: HostSnapshot,
    ) -> Option<AppSnapshot> {
        let mut published = self.published_host.lock().expect("published host poisoned");
        if !published.accept(session_id, host) {
            return None;
        }

        let alive: Vec<i32> = published
            .snapshot
            .displays
            .iter()
            .filter(|display| display.active)
            .map(|display| display.index)
            .collect();
        self.viewers.retain(&alive);

        let snapshot = self.compose_snapshot(&mut published);
        crate::events::emit_snapshot(app, &snapshot);
        Some(snapshot)
    }

    /// Re-emit the latest accepted host state with current settings and viewer indices.
    /// The same lock as host publication keeps full snapshots in emission order.
    pub fn republish_snapshot<R: Runtime>(&self, app: &AppHandle<R>) -> AppSnapshot {
        let mut published = self.published_host.lock().expect("published host poisoned");
        let snapshot = self.compose_snapshot(&mut published);
        crate::events::emit_snapshot(app, &snapshot);
        snapshot
    }

    /// Callers hold the `published_host` lock, so stamps follow composition order.
    fn compose_snapshot(&self, published: &mut PublishedHost) -> AppSnapshot {
        published.seq += 1;
        AppSnapshot {
            seq: published.seq,
            host: published.snapshot.clone(),
            settings: self.settings_snapshot(),
            effective_language: self.effective_language(),
            open_viewers: self.viewers.open_indices(),
            fallback_error: *self.fallback_error.lock().expect("fallback error poisoned"),
        }
    }

    /// Carried into every composed snapshot until the fallback boundary clears it.
    pub fn set_fallback_error(&self, error: Option<EasyVirtualDisplayErrorCode>) {
        *self.fallback_error.lock().expect("fallback error poisoned") = error;
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
            crate::viewer::create(std::sync::Arc::new(|| {})),
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
    fn current_snapshot_includes_published_host_and_runtime_fields() {
        let rt = build_runtime();
        let mut host = empty_host_snapshot();
        host.revision = 42;
        assert!(rt.published_host.lock().unwrap().accept(1, host));
        let snap = rt.current_snapshot();
        assert_eq!(snap.host.revision, 42);
        assert_eq!(snap.effective_language, EffectiveLanguage::En);
        assert_eq!(snap.settings.language, AppLanguage::System);
    }

    #[test]
    fn composed_snapshots_are_stamped_in_order() {
        let rt = build_runtime();
        let first = rt.current_snapshot();
        let second = rt.current_snapshot();
        assert!(second.seq > first.seq);
    }

    #[test]
    fn published_host_orders_revisions_within_each_session() {
        let mut published = PublishedHost {
            session_id: 1,
            snapshot: empty_host_snapshot(),
            seq: 0,
        };
        let mut revision_two = empty_host_snapshot();
        revision_two.revision = 2;
        let mut stale = empty_host_snapshot();
        stale.revision = 1;

        assert!(published.accept(1, revision_two));
        assert!(!published.accept(1, stale.clone()));
        assert_eq!(published.snapshot.revision, 2);
        assert!(published.accept(2, stale));
        assert_eq!(published.snapshot.revision, 1);
    }
}
