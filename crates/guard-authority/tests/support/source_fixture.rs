use std::io::{self, Write};
fn main() {
    println!("confirmed native diagnostic");
    io::stdout().flush().unwrap();
    eprintln!("private native stderr");
    std::process::exit(7);
}
