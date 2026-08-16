!macro NSIS_HOOK_POSTINSTALL
  ; The versioned icon path changes the Shell cache key when the artwork changes.
  ${If} ${FileExists} "$DESKTOP\Vidmetry.lnk"
    Delete "$DESKTOP\Vidmetry.lnk"
    CreateShortCut "$DESKTOP\Vidmetry.lnk" "$INSTDIR\vidmetry.exe" "" "$INSTDIR\shortcut-icon-achromatic-v2.ico" 0 SW_SHOWNORMAL "" "Vidmetry"
  ${EndIf}

  ${If} ${FileExists} "$SMPROGRAMS\Vidmetry.lnk"
    Delete "$SMPROGRAMS\Vidmetry.lnk"
    CreateShortCut "$SMPROGRAMS\Vidmetry.lnk" "$INSTDIR\vidmetry.exe" "" "$INSTDIR\shortcut-icon-achromatic-v2.ico" 0 SW_SHOWNORMAL "" "Vidmetry"
  ${EndIf}

  System::Call 'shell32.dll::SHChangeNotify(i, i, i, i) v (0x08000000, 0, 0, 0)'
!macroend
