fn main() -> std::process::ExitCode {
    guard_install::dispatch(
        &guard_install::Tool::new(
            "archguard",
            "0.1.0",
            "仅检查 Cargo 直接成员依赖。",
            &["check"],
        ),
        &std::env::args().skip(1).collect::<Vec<_>>(),
    )
    .unwrap_or(std::process::ExitCode::from(4))
}
