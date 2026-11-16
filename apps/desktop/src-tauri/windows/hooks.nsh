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
