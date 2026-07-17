// 防止 Windows release 版本弹出控制台窗口，勿删！
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    app_lib::run();
}
