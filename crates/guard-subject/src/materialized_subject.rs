use crate::{CapturedSubject, materialization_io};
use std::path::{Path, PathBuf};
/// 持有私有真实候选输入目录及其冻结身份；不产生质量通过或执行许可。无Java对应类型。
pub struct MaterializedSubject {
    pub(crate) _scratch: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) captured: CapturedSubject,
}
impl MaterializedSubject {
    /// 返回只含候选文件的执行根；其生命周期由本对象所有权控制。
    pub fn path(&self) -> &Path {
        &self.root
    }
    /// 返回完全相同的冻结捕获身份，不包含随机临时目录作为候选身份。
    pub fn captured(&self) -> &CapturedSubject {
        &self.captured
    }
    /// 复验全部原始输入路径、类型、字节与适用mode；新增构建输出不计入原始输入。
    /// 返回全部输入仍匹配的结果，缺失、别名或内容变化返回错误。
    pub fn verify_inputs(&self) -> Result<(), String> {
        materialization_io::verify(&self.root, &self.captured.manifest)
    }
}
