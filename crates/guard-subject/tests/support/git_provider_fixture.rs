use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

// 此 fixture 必须先退出以释放 Windows 映像锁，不能等待替换者；替换者最长5秒，测试父进程通过继承管道 EOF 等待其结束。
#[allow(clippy::zombie_processes)]
fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.first().is_some_and(|s| s == "--mutate") {
        let path = &arguments[1];
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if fs::write(path, b"replaced pinned executable after exit").is_ok() {
                return;
            }
            if Instant::now() >= deadline {
                std::process::exit(91);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    if std::env::var_os("SUBJECT_PROVIDER_POISON").is_some() {
        eprintln!("unapproved environment reached Git provider");
        std::process::exit(92);
    }
    let executable = std::env::var_os("SUBJECT_PROVIDER_REAL_GIT").expect("explicit real Git");
    let result = Command::new(executable).args(&arguments).output().unwrap();
    use std::io::Write;
    std::io::stdout().write_all(&result.stdout).unwrap();
    std::io::stderr().write_all(&result.stderr).unwrap();
    if let Some(mutator) = std::env::var_os("SUBJECT_PROVIDER_MUTATOR") {
        // 继承管道使主进程读到 EOF 前必已完成替换；原 fixture 执行子进程避免 Windows 映像锁。
        Command::new(mutator)
            .arg("--mutate")
            .arg(std::env::current_exe().unwrap())
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
    }
    std::process::exit(result.status.code().unwrap_or(93));
}
