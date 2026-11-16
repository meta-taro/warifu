; Windows のインストーラー（NSIS）に足す手順。
;
; 席（Claude Code など）が画面の中の cli\warifu.exe を口に使っていると、
; 更新のときにその exe だけ書き換えられず、画面は新しくコマンドは古いまま残る。
; 使っている exe は消せないが、名前は変えられる。入れる前に名前を変えて退け、
; 新しい cli\warifu.exe を置けるようにする。動いている席は退けたほうを使い続け、
; 立ち上げ直せば新しいほうを使う（画面の［コマンドも上げる］と同じ考え）。

!macro NSIS_HOOK_PREINSTALL
  ; 前に退けたものは消す。まだ使われていて消せなければ、別の名へ退ける
  Delete "$INSTDIR\cli\warifu.exe.old"
  ClearErrors
  Rename "$INSTDIR\cli\warifu.exe" "$INSTDIR\cli\warifu.exe.old"
  IfErrors 0 +3
    Delete "$INSTDIR\cli\warifu.exe.old2"
    Rename "$INSTDIR\cli\warifu.exe" "$INSTDIR\cli\warifu.exe.old2"
  ClearErrors
!macroend

; `warifu setup` が席の口として写した、束の外のコマンド（$INSTDIR\warifu.exe）も上げる。
; 前は画面の［コマンドも上げる］を人が押すまで古いまま残った。
; 写しが無い（setup を打っていない）なら何もしない。新しく作らない。
; 画面の本体が同じ名前なら触らない（本体を消さないための守り）。
!macro NSIS_HOOK_POSTINSTALL
  StrCmp "${MAINBINARYNAME}" "warifu" warifu_outside_done
  IfFileExists "$INSTDIR\warifu.exe" 0 warifu_outside_done
  IfFileExists "$INSTDIR\cli\warifu.exe" 0 warifu_outside_done
  Delete "$INSTDIR\warifu.exe.old"
  ClearErrors
  Rename "$INSTDIR\warifu.exe" "$INSTDIR\warifu.exe.old"
  IfErrors 0 warifu_outside_copy
    Delete "$INSTDIR\warifu.exe.old2"
    ClearErrors
    Rename "$INSTDIR\warifu.exe" "$INSTDIR\warifu.exe.old2"
    IfErrors warifu_outside_done
  warifu_outside_copy:
  CopyFiles /SILENT "$INSTDIR\cli\warifu.exe" "$INSTDIR\warifu.exe"
  warifu_outside_done:
  ClearErrors
!macroend
