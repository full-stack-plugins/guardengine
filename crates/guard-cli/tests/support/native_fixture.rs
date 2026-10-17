use std::{fs, io::Write, path::Path};
fn main() {
    let executable = std::env::current_exe().unwrap();
    if Path::new(&executable).file_stem().unwrap() == "git" {
        if let Ok(marker) = std::env::var("EVIL_GIT_MARKER") {
            fs::write(marker, b"fake git executed").unwrap();
        }
        std::process::exit(42);
    }
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--version") {
        assert!(
            !Path::new("src/Example.java").exists(),
            "probe must never use candidate cwd"
        );
        assert!(
            std::env::var("GUARDCORE_GIT_PROVIDER_DIGEST").is_err(),
            "probe must not inherit actual run environment"
        );
        println!("guard-cli-fixture/1");
        return;
    }
    let content = fs::read_to_string("src/Example.java").unwrap();
    println!("OBSERVED={content}");
    std::io::stdout().flush().unwrap();
    if let Some(marker) = args.get(2) {
        use std::fs::OpenOptions;
        writeln!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(marker)
                .unwrap(),
            "executed"
        )
        .unwrap();
    }
    fs::write("generated-by-fixture.txt", b"private output").unwrap();
    if content.contains("TIMEOUT") {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
    if content.contains("FAIL") {
        std::process::exit(7);
    }
}
