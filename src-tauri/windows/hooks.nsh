!macro VIDMETRY_REGISTER_VIDEO_EXTENSION EXTENSION
  WriteRegStr HKCU "Software\Classes\.${EXTENSION}\OpenWithProgids" "Vidmetry.Video" ""
  WriteRegStr HKCU "Software\Classes\Applications\vidmetry.exe\SupportedTypes" ".${EXTENSION}" ""
  WriteRegStr HKCU "Software\Vidmetry\Capabilities\FileAssociations" ".${EXTENSION}" "Vidmetry.Video"
!macroend

!macro VIDMETRY_UNREGISTER_VIDEO_EXTENSION EXTENSION
  DeleteRegValue HKCU "Software\Classes\.${EXTENSION}\OpenWithProgids" "Vidmetry.Video"
  DeleteRegKey /ifempty HKCU "Software\Classes\.${EXTENSION}\OpenWithProgids"
!macroend

!macro VIDMETRY_REGISTER_VIDEO_INTEGRATION
  WriteRegStr HKCU "Software\Classes\Vidmetry.Video" "" "Vidmetry video"
  WriteRegStr HKCU "Software\Classes\Vidmetry.Video\DefaultIcon" "" "$INSTDIR\vidmetry.exe,0"
  WriteRegStr HKCU "Software\Classes\Vidmetry.Video\shell\open\command" "" '$\"$INSTDIR\vidmetry.exe$\" $\"%1$\"'

  WriteRegStr HKCU "Software\Classes\Applications\vidmetry.exe" "FriendlyAppName" "Vidmetry"
  WriteRegStr HKCU "Software\Classes\Applications\vidmetry.exe\DefaultIcon" "" "$INSTDIR\vidmetry.exe,0"
  WriteRegStr HKCU "Software\Classes\Applications\vidmetry.exe\shell\open\command" "" '$\"$INSTDIR\vidmetry.exe$\" $\"%1$\"'

  WriteRegStr HKCU "Software\Vidmetry\Capabilities" "ApplicationName" "Vidmetry"
  WriteRegStr HKCU "Software\Vidmetry\Capabilities" "ApplicationDescription" "Crop and trim videos with Vidmetry."
  WriteRegStr HKCU "Software\RegisteredApplications" "Vidmetry" "Software\Vidmetry\Capabilities"

  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "3gp"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "avi"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "flv"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "m2ts"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "m4v"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "mkv"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "mov"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "mp4"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "mpeg"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "mpg"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "mts"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "ogv"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "ts"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "vob"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "webm"
  !insertmacro VIDMETRY_REGISTER_VIDEO_EXTENSION "wmv"
!macroend

!macro VIDMETRY_UNREGISTER_VIDEO_INTEGRATION
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "3gp"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "avi"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "flv"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "m2ts"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "m4v"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "mkv"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "mov"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "mp4"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "mpeg"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "mpg"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "mts"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "ogv"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "ts"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "vob"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "webm"
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_EXTENSION "wmv"

  DeleteRegKey HKCU "Software\Classes\Applications\vidmetry.exe"
  DeleteRegKey HKCU "Software\Classes\Vidmetry.Video"
  DeleteRegKey HKCU "Software\Vidmetry\Capabilities"
  DeleteRegValue HKCU "Software\RegisteredApplications" "Vidmetry"
!macroend

!macro VIDMETRY_REGISTER_DIRECTORY_INTEGRATION
  WriteRegStr HKCU "Software\Classes\Directory\shell\Vidmetry" "" "Open with Vidmetry"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Vidmetry" "Icon" "$INSTDIR\vidmetry.exe,0"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Vidmetry" "MultiSelectModel" "Single"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Vidmetry\command" "" '$\"$INSTDIR\vidmetry.exe$\" $\"%1$\"'
  WriteRegDWORD HKCU "Software\Vidmetry" "NsisExplorerIntegrationEnabled" 1
!macroend

!macro VIDMETRY_UNREGISTER_DIRECTORY_INTEGRATION
  DeleteRegKey HKCU "Software\Classes\Directory\shell\Vidmetry"
  WriteRegDWORD HKCU "Software\Vidmetry" "NsisExplorerIntegrationEnabled" 0
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro VIDMETRY_REGISTER_VIDEO_INTEGRATION

  ClearErrors
  ReadRegDWORD $0 HKCU "Software\Vidmetry" "NsisExplorerIntegrationEnabled"
  ${If} ${Errors}
    ClearErrors
    ReadRegDWORD $0 HKCU "Software\Vidmetry" "ExplorerIntegrationEnabled"
    ${If} ${Errors}
      StrCpy $0 1
    ${EndIf}
  ${EndIf}
  ${If} $0 == 0
    !insertmacro VIDMETRY_UNREGISTER_DIRECTORY_INTEGRATION
  ${Else}
    !insertmacro VIDMETRY_REGISTER_DIRECTORY_INTEGRATION
  ${EndIf}

  System::Call 'shell32.dll::SHChangeNotify(i, i, i, i) v (0x08000000, 0, 0, 0)'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro VIDMETRY_UNREGISTER_VIDEO_INTEGRATION
  !insertmacro VIDMETRY_UNREGISTER_DIRECTORY_INTEGRATION
  DeleteRegValue HKCU "Software\Vidmetry" "NsisExplorerIntegrationEnabled"
  DeleteRegKey /ifempty HKCU "Software\Vidmetry"
  System::Call 'shell32.dll::SHChangeNotify(i, i, i, i) v (0x08000000, 0, 0, 0)'
!macroend
