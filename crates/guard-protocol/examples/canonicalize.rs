use std::io::{self, Read, Write};
/// 规范字节验证辅助入口，只读 stdin、写 stdout；不执行检查或授予任何权限。
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    let canonical = guard_protocol::canonicalize(&bytes).map_err(io::Error::other)?;
    io::stdout().write_all(&canonical)?;
    Ok(())
}
