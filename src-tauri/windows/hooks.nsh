; Tauri NSIS installer hooks for Easy Virtual Display. During an interactive
; uninstall, offer to also remove the Parsec Virtual Display Driver; a silent uninstall
; (/S, e.g. an in-place upgrade) keeps the driver via /SD IDNO.
;
; Wired in tauri.conf.json via bundle.windows.nsis.installerHooks. Tauri's NSIS template
; !include's this file near the TOP of the script (right after the StrFunc includes) —
; which is BEFORE `!insertmacro MUI_LANGUAGE ...` runs. That ordering is the catch: at
; include time the ${LANG_*} ids are not defined yet and the MUI language tables aren't
; loaded, so a top-level `LangString` here binds to an undefined language id and resolves
; to an EMPTY string at runtime (the uninstall prompt then shows Yes/No with no text).
; So we DON'T use LangString — we pick the prompt at runtime from $LANGUAGE (the selected
; LCID). English is the default; we override for the two Chinese LCIDs we ship.
;
; Resource layout (verified against a real Tauri 2 NSIS package, Phase 0.5 #3):
; bundle.resources maps `native/EasyVirtualDisplay.Host/bin/publish/` to `native`, and the
; Tauri NSIS template places each bundle target directly under $INSTDIR (no `resources\`
; prefix). So Host.exe lands at $INSTDIR\native\EasyVirtualDisplay.Host.exe.

Var EvdRemoveDriver
Var EvdDriverPrompt

; Runs in the uninstall section just before any files are deleted, so Host.exe is still
; present when we delegate the silent driver-removal call to it.
!macro NSIS_HOOK_PREUNINSTALL
  StrCpy $EvdRemoveDriver "0"

  ; Resolve the prompt by runtime language (see header note for why not LangString).
  ; LCIDs: 2052 = Simplified Chinese, 1028 = Traditional Chinese; anything else => English.
  StrCpy $EvdDriverPrompt "Also remove the Parsec Virtual Display Driver from this system?$\r$\n$\r$\nChoosing 'No' will leave the driver installed; you can remove it later from Windows 'Apps' settings."
  StrCmp $LANGUAGE 2052 evd_lang_zh_cn 0
  StrCmp $LANGUAGE 1028 evd_lang_zh_tw evd_lang_done
  evd_lang_zh_cn:
    StrCpy $EvdDriverPrompt "是否同时从本机移除 Parsec 虚拟显示驱动？$\r$\n$\r$\n选择「否」将保留驱动；你之后仍可在 Windows「应用」设置中卸载它。"
    Goto evd_lang_done
  evd_lang_zh_tw:
    StrCpy $EvdDriverPrompt "是否同時從本機移除 Parsec 虛擬顯示驅動？$\r$\n$\r$\n選擇「否」將保留驅動；你之後仍可在 Windows「應用」設定中卸載它。"
  evd_lang_done:

  MessageBox MB_YESNO|MB_ICONQUESTION "$EvdDriverPrompt" /SD IDNO IDYES evd_remove_driver IDNO evd_ask_done
  evd_remove_driver:
    StrCpy $EvdRemoveDriver "1"
  evd_ask_done:

  ${If} $EvdRemoveDriver == "1"
    ${If} ${FileExists} "$INSTDIR\native\EasyVirtualDisplay.Host.exe"
      ExecWait '"$INSTDIR\native\EasyVirtualDisplay.Host.exe" uninstall-driver --silent'
    ${EndIf}
  ${EndIf}
!macroend
