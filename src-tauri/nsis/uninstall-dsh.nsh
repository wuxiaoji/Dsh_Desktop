; NSIS hooks for the DeepSeek Harness Desktop installer/uninstaller.
;
; Design:
; 1) PREINSTALL:  write an upgrade marker (.dsh-upgrade-flag).  When the new
;    installer upgrades an existing installation it silently runs the OLD
;    uninstaller first; that old uninstaller sees the marker and skips the
;    global dsh cleanup, so updating the app never removes dsh.
; 2) PREUNINSTALL: a real uninstall removes the global dsh package
;    (@deepseek-ai/dsh) ONLY if this app itself installed it (the
;    .dsh-installed-by-app marker written by the launcher after a successful
;    auto-install).  A dsh the user installed independently is left alone.
; 3) POSTINSTALL:  remove the upgrade marker.

!macro NSIS_HOOK_PREINSTALL
  CreateDirectory "$INSTDIR"
  FileOpen $0 "$INSTDIR\.dsh-upgrade-flag" w
  FileClose $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; 升级流程：新安装器先写入 .dsh-upgrade-flag，再静默调用旧卸载器。
  ; 此时仅删除标记，不清除全局 dsh。
  IfFileExists "$INSTDIR\.dsh-upgrade-flag" 0 dsh_real_uninstall
    Delete "$INSTDIR\.dsh-upgrade-flag"
    Goto dsh_uninstall_done
  dsh_real_uninstall:
  ; 真正卸载：仅当本应用自动安装过 dsh（存在标记）时清理全局包，尽力而为
  IfFileExists "$INSTDIR\.dsh-installed-by-app" 0 dsh_uninstall_done
    DetailPrint "正在卸载全局 DeepSeek Harness (@deepseek-ai/dsh) ..."
    nsExec::ExecToLog '"$SYSDIR\cmd.exe" /C npm uninstall --global @deepseek-ai/dsh'
    Pop $0
    DetailPrint "npm uninstall 退出码: $0"
  dsh_uninstall_done:
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Delete "$INSTDIR\.dsh-upgrade-flag"
!macroend
