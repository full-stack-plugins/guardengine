use std::{
    io::{self, Write},
    process::Command,
    thread,
    time::Duration,
};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args[0].as_str() {
        "output" => {
            println!("confirmed finding before interruption");
            eprintln!("private diagnostic");
        }
        "args" => println!("{:?}", &args[1..]),
        "environment" => {
            println!(
                "{}",
                std::env::var("GUARD_INHERITED_SECRET").unwrap_or_default()
            );
            println!("{}", std::env::var("PATH").unwrap_or_default());
            println!("{}", std::env::var("GUARD_EXPLICIT").unwrap_or_default());
        }
        "sleep" => {
            println!("confirmed finding before interruption");
            io::stdout().flush().unwrap();
            if let Some(marker) = args.get(1) {
                std::fs::write(marker, b"stdout flushed").unwrap();
            }
            thread::sleep(Duration::from_secs(60));
        }
        "fail" => {
            eprintln!("known failure");
            std::process::exit(7);
        }
        "touch" => {
            std::fs::write(&args[1], b"executed").unwrap();
        }
        "flood" => loop {
            io::stdout().write_all(&[b'x'; 4096]).unwrap();
            io::stderr().write_all(&[b'y'; 4096]).unwrap();
        },
        "tree" => {
            // 反例故意遗留子进程，由被测runner负责回收；不能在fixture中替实现收尾。
            #[allow(clippy::zombie_processes)]
            let child = Command::new(std::env::current_exe().unwrap())
                .args(["heartbeat", &args[1]])
                .spawn()
                .unwrap();
            println!("descendant={}", child.id());
            io::stdout().flush().unwrap();
            if args.get(2).is_some_and(|v| v == "exit") {
                return;
            }
            thread::sleep(Duration::from_secs(60));
        }
        "forest" => {
            let directory = std::path::Path::new(&args[1]);
            let mut pids = vec![std::process::id()];
            for name in ["leaf-one", "leaf-two"] {
                #[allow(clippy::zombie_processes)]
                let child = Command::new(std::env::current_exe().unwrap())
                    .args(["heartbeat", directory.join(name).to_str().unwrap()])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .unwrap();
                pids.push(child.id());
            }
            #[allow(clippy::zombie_processes)]
            let branch = Command::new(std::env::current_exe().unwrap())
                .args(["branch", &args[1]])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            pids.push(branch.id());
            let started = std::time::Instant::now();
            while !directory.join("grandchild.pid").exists() {
                assert!(started.elapsed() < Duration::from_secs(10));
                thread::sleep(Duration::from_millis(5));
            }
            let grandchild = std::fs::read_to_string(directory.join("grandchild.pid")).unwrap();
            pids.push(grandchild.parse().unwrap());
            println!("confirmed finding before forest stop");
            io::stdout().flush().unwrap();
            std::fs::write(
                directory.join("forest.pids.tmp"),
                pids.iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
            .unwrap();
            std::fs::rename(
                directory.join("forest.pids.tmp"),
                directory.join("forest.pids"),
            )
            .unwrap();
            while !directory.join("release").exists() {
                thread::sleep(Duration::from_millis(5));
            }
            if args[2] == "flood" {
                loop {
                    io::stdout().write_all(&[b'x'; 4096]).unwrap();
                }
            }
            if args[2] != "exit" {
                thread::sleep(Duration::from_secs(60));
            }
        }
        "branch" => {
            let directory = std::path::Path::new(&args[1]);
            #[allow(clippy::zombie_processes)]
            let grandchild = Command::new(std::env::current_exe().unwrap())
                .args(["heartbeat", directory.join("grandchild").to_str().unwrap()])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            std::fs::write(
                directory.join("grandchild.pid.tmp"),
                grandchild.id().to_string(),
            )
            .unwrap();
            std::fs::rename(
                directory.join("grandchild.pid.tmp"),
                directory.join("grandchild.pid"),
            )
            .unwrap();
            thread::sleep(Duration::from_secs(60));
        }
        "heartbeat" => loop {
            std::fs::write(&args[1], format!("{:?}", std::time::SystemTime::now())).unwrap();
            thread::sleep(Duration::from_millis(20));
        },
        _ => std::process::exit(2),
    }
}
