//! Event names emitted to the renderer. Match `src/shared/ipc.ts:rendererEventChannels` so
//! the tauri-bridge can use the same constants on both sides.

use tauri::{AppHandle, Emitter, Runtime};

use crate::contracts::{AppSnapshot, EffectiveLanguage};

pub const EVENT_SNAPSHOT_CHANGED: &str = "easy-virtual-display:snapshot-changed";
pub const EVENT_LANGUAGE_CHANGED: &str = "easy-virtual-display:language-changed";

pub fn emit_snapshot<R: Runtime>(app: &AppHandle<R>, snapshot: &AppSnapshot) {
    let _ = app.emit(EVENT_SNAPSHOT_CHANGED, snapshot);
}

pub fn emit_language_changed<R: Runtime>(app: &AppHandle<R>, language: EffectiveLanguage) {
    let _ = app.emit(EVENT_LANGUAGE_CHANGED, language);
}
