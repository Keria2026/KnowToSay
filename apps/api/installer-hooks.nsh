; Refresh existing shortcuts on upgrade using a new icon resource path, so
; Explorer does not reuse the cached icon embedded in the previous executable.
!macro RefreshBrandShortcut SHORTCUT
  !insertmacro IsShortcutTarget "${SHORTCUT}" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $0
  ${If} $0 = 1
    CreateShortcut "${SHORTCUT}" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\brand-v2.ico" 0
    !insertmacro SetLnkAppUserModelId "${SHORTCUT}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro RefreshBrandShortcut "$DESKTOP\${PRODUCTNAME}.lnk"
  !insertmacro RefreshBrandShortcut "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !if "${STARTMENUFOLDER}" != ""
    !insertmacro RefreshBrandShortcut "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
  !endif
  ; SHCNE_ASSOCCHANGED / SHCNF_IDLIST: tell Explorer to refresh its icon cache.
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
