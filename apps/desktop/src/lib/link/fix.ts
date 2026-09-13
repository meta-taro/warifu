// **ふさがっているときの直し方を、その場で渡す。**
//
// `gh issue 9`（ASUS・2026-09-12）——
//
// > 画面用の規則を作るには**管理者権限が要る**ので、こちらでは足せません。人に頼みます。
//
// **割符は管理者権限を要求しない**（**D104**）。インストーラで規則を作れば済むが、
// **管理者を要求した瞬間「入れてみて」が止まる。**だから**入れ方は変えず、
// 直し方をその場で渡す。**
//
// **分からない環境には何も渡さない** —— 当てずっぽうのコマンドを人に打たせない。

/** Windows で、管理者の PowerShell に打つ 2 行。 */
const Windowsの直し方 = [
	'New-NetFirewallRule -DisplayName "warifu (in)" -Direction Inbound' +
		' -Program "$env:LOCALAPPDATA\\warifu\\warifu-desktop.exe" -Action Allow',
	'New-NetFirewallRule -DisplayName "warifu (out)" -Direction Outbound' +
		' -Program "$env:LOCALAPPDATA\\warifu\\warifu-desktop.exe" -Action Allow'
].join('\n');

/**
 * その環境での直し方。**無ければ `null`。**
 *
 * @param 名乗り `navigator.userAgent`
 */
export function ふさがりの直し方(名乗り: string): string | null {
	return 名乗り.includes('Windows') ? Windowsの直し方 : null;
}
