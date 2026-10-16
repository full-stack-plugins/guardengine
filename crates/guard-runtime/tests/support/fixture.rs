use std::{
    fs,
    io::{self, Write},
    thread,
    time::Duration,
};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = io::stdout().lock();
    out.write_all(b"partial violation\n").unwrap();
    out.flush().unwrap();
    eprintln!("private stderr");
    // 握手必须发生在原始前缀确实写出之后，测试不依赖启动延迟推测输出。
    if let Some(path) = args.get(1) {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        file.write_all(b"x").unwrap();
    }
    match args.first().map(String::as_str) {
        Some(mode @ ("read" | "read-hold" | "tamper" | "git-probe")) => {
            println!("SOURCE:{}", fs::read_to_string("src/Example.java").unwrap());
            println!("ROOT:{}", std::env::current_dir().unwrap().display());
            println!("ARG:{}", args.get(2).map(String::as_str).unwrap_or(""));
            println!(
                "CEILING:{}",
                std::env::var("GIT_CEILING_DIRECTORIES").unwrap_or_default()
            );
            out.flush().unwrap();
            if mode == "read-hold" {
                fs::write(format!("{}.ready", args[1]), b"source emitted").unwrap();
                thread::sleep(Duration::from_secs(60));
            } else if mode == "tamper" {
                fs::write("src/Example.java", b"MATERIALIZED TAMPER").unwrap();
            } else if mode == "git-probe" {
                let discovery = std::process::Command::new(&args[3])
                    .args(["rev-parse", "--show-toplevel"])
                    .output()
                    .unwrap();
                println!("DISCOVERY:{}", discovery.status.success());
                let unrestricted = std::process::Command::new(&args[3])
                    .env_remove("GIT_CEILING_DIRECTORIES")
                    .args(["rev-parse", "--show-toplevel"])
                    .output()
                    .unwrap();
                println!("UNRESTRICTED_DISCOVERY:{}", unrestricted.status.success());
            }
        }
        Some("large") => {
            out.write_all(&vec![b'x'; 2300000]).unwrap();
        }
        Some("truncate") => {
            out.write_all(&vec![b'x'; 5000000]).unwrap();
        }
        Some("fail") => std::process::exit(7),
        Some("wait") => thread::sleep(Duration::from_secs(60)),
        Some("drift") => {
            fs::write("src/Example.java", b"drift").unwrap();
        }
        _ => {}
    }
}
