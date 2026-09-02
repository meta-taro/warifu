// Windows のリリースビルドで、裏にコンソール窓を出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    warifu_desktop_lib::run()
}
