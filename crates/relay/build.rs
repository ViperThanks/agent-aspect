//! Relay crate build script — 注入编译时间戳。

use std::env;

fn main() {
    // 注入编译时间，格式：MM-DD HH:MM
    // 优先使用 SOURCE_DATE_EPOCH（可重现构建），否则用当前时间
    let build_time = if let Ok(epoch) = env::var("SOURCE_DATE_EPOCH") {
        let secs: i64 = epoch.parse().unwrap_or(0);
        let naive = chrono::DateTime::from_timestamp(secs, 0)
            .unwrap_or_default()
            .naive_utc();
        naive.format("%m-%d %H:%M").to_string()
    } else {
        chrono::Local::now().format("%m-%d %H:%M").to_string()
    };
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
}
