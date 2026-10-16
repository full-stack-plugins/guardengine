use crate::{
    Cancellation, CommandSpec, RunOutput, StopReason,
    process_tree::{ProcessTree, terminate_and_confirm},
};
use guard_protocol::{ExecutionBudget, ExecutionStatus};
use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::{Semaphore, mpsc},
    time::{Instant, timeout_at},
};

/// 同一批准预算下的有界进程执行器；不实现领域结论或授权。无 Java 对应类型。
#[derive(Clone)]
pub struct ProcessRunner {
    budget: ExecutionBudget,
    slots: Arc<Semaphore>,
}
impl ProcessRunner {
    /// 固定本执行器的时间、聚合输出和并发预算，返回不可变预算实例。
    pub fn new(budget: ExecutionBudget) -> Result<Self, String> {
        if budget.timeout_ms == 0
            || budget.timeout_ms > 86400000
            || budget.max_output_bytes == 0
            || budget.max_output_bytes > 8 * 1024 * 1024
            || budget.max_concurrency == 0
            || budget.max_concurrency > 64
        {
            return Err("invalid local execution budget".into());
        }
        Ok(Self {
            slots: Arc::new(Semaphore::new(budget.max_concurrency as usize)),
            budget,
        })
    }
    /// 执行显式宿主原生 argv，返回私有原始观察；不产生 PASS 或执行许可。
    /// 取消/超时/输出超限终止受管树；future丢弃也触发树清理。
    pub async fn run(&self, spec: CommandSpec, cancel: Cancellation) -> Result<RunOutput, String> {
        let deadline = Instant::now() + Duration::from_millis(self.budget.timeout_ms);
        self.run_with_deadline(spec, cancel, deadline).await
    }

    /// 使用调用方的绝对单调截止时间缩短运行；不能延长自身固定预算。
    /// 参数仍为原生命令、取消信号及std单调时刻；返回保留部分输出的原始执行观察。
    pub async fn run_until(
        &self,
        spec: CommandSpec,
        cancel: Cancellation,
        deadline: std::time::Instant,
    ) -> Result<RunOutput, String> {
        let deadline = Instant::from_std(deadline)
            .min(Instant::now() + Duration::from_millis(self.budget.timeout_ms));
        self.run_with_deadline(spec, cancel, deadline).await
    }

    async fn run_with_deadline(
        &self,
        spec: CommandSpec,
        cancel: Cancellation,
        deadline: Instant,
    ) -> Result<RunOutput, String> {
        if cancel.is_cancelled() {
            return Ok(RunOutput::stopped(StopReason::Cancelled));
        }
        if Instant::now() >= deadline {
            return Ok(RunOutput::stopped(StopReason::Timeout));
        }
        let permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Ok(RunOutput::stopped(StopReason::Cancelled)),
            permit = timeout_at(deadline, self.slots.clone().acquire_owned()) => match permit {
                Ok(Ok(permit)) => permit,
                _ => return Ok(RunOutput::stopped(StopReason::Timeout)),
            }
        };
        // 哈希文件操作移出异步工作线程；外部时间预算仍覆盖验证准备阶段。
        let checked = spec.clone();
        let permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Ok(RunOutput::stopped(StopReason::Cancelled)),
            result = timeout_at(deadline, tokio::task::spawn_blocking(move || (permit, checked.validate()))) => {
                match result {
                    Ok(result) => {
                        let (permit, validation) = result.map_err(|e| e.to_string())?;
                        validation?;
                        permit
                    },
                    Err(_) => return Ok(RunOutput::stopped(StopReason::Timeout)),
                }
            }
        };
        // 队列或阻塞核验完成后再次检查，截止时间已过时不得开始原生进程。
        if cancel.is_cancelled() {
            return Ok(RunOutput::stopped(StopReason::Cancelled));
        }
        if Instant::now() >= deadline {
            return Ok(RunOutput::stopped(StopReason::Timeout));
        }
        let mut command = Command::new(&spec.executable);
        command
            .args(&spec.args)
            .current_dir(&spec.directory)
            .env_clear()
            .envs(&spec.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut tree = ProcessTree::prepare(&mut command)?;
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let observed_start = crate::clock::now().ok();
        let started = tree.attach(&child, deadline.into_std())?;
        let mut output = RunOutput::stopped(StopReason::IoFailure);
        output.process_id = child.id();
        output.started_at = observed_start;
        let (sender, mut receiver) = mpsc::channel::<(bool, Result<Vec<u8>, String>)>(8);
        let out_task = tokio::spawn(read_pipe(
            child.stdout.take().ok_or("stdout missing")?,
            false,
            sender.clone(),
        ));
        let err_task = tokio::spawn(read_pipe(
            child.stderr.take().ok_or("stderr missing")?,
            true,
            sender.clone(),
        ));
        drop(sender);
        let reason = if !started {
            StopReason::Timeout
        } else {
            loop {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break StopReason::Cancelled,
                    _ = tokio::time::sleep_until(deadline) => break StopReason::Timeout,
                    event = receiver.recv(), if !receiver.is_closed() || !receiver.is_empty() => {
                        match event {
                            Some((stderr, Ok(bytes))) => if retain(&mut output, stderr, bytes, self.budget.max_output_bytes as usize) { break StopReason::OutputLimit; },
                            Some((_, Err(_))) => break StopReason::IoFailure,
                            None => {},
                        }
                    }
                    result = child.wait() => {
                        match result {
                            Ok(status) => { output.exit_code = status.code(); break StopReason::Exited; }
                            Err(_) => break StopReason::IoFailure,
                        }
                    }
                }
            }
        };
        output.reason = reason;
        output.status = match reason {
            StopReason::Exited if output.exit_code == Some(0) => ExecutionStatus::Completed,
            StopReason::Timeout => ExecutionStatus::TimedOut,
            StopReason::Cancelled => ExecutionStatus::Cancelled,
            _ => ExecutionStatus::Failed,
        };
        // 父进程正常退出也终止遗留子孙，防止持管道进程无限延迟返回。
        if terminate_and_confirm(&tree).await.is_err() {
            output.reason = StopReason::IoFailure;
            output.status = ExecutionStatus::Failed;
        }
        let _ = child.start_kill();
        if !matches!(
            timeout_at(Instant::now() + Duration::from_secs(2), child.wait()).await,
            Ok(Ok(_))
        ) {
            output.reason = StopReason::IoFailure;
            output.status = ExecutionStatus::Failed;
        }
        let drain = async {
            while let Some((stderr, chunk)) = receiver.recv().await {
                match chunk {
                    Ok(bytes) => {
                        retain(
                            &mut output,
                            stderr,
                            bytes,
                            self.budget.max_output_bytes as usize,
                        );
                    }
                    Err(_) => {
                        output.reason = StopReason::IoFailure;
                        output.status = ExecutionStatus::Failed;
                    }
                }
            }
        };
        if timeout_at(Instant::now() + Duration::from_secs(2), drain)
            .await
            .is_err()
        {
            output.output_truncated = true;
            output.reason = StopReason::IoFailure;
            output.status = ExecutionStatus::Failed;
        }
        out_task.abort();
        err_task.abort();
        if output.output_truncated && output.reason == StopReason::Exited {
            output.reason = StopReason::OutputLimit;
            output.status = ExecutionStatus::Failed;
        }
        output.finished_at = crate::clock::now().ok();
        if !matches!((&output.started_at, &output.finished_at), (Some(start), Some(end)) if !start.compare_instant(end).is_gt())
        {
            // 保留倒退的原始观察，不用当前时间补造或把结束时间钳成开始时间。
            output.reason = StopReason::ClockInvalid;
            output.status = ExecutionStatus::Failed;
        }
        let checked = spec;
        // 即使调用future在哈希中途取消，阻塞任务也继续持有并发令牌，不能无限堆积后台读取。
        let recheck = timeout_at(
            Instant::now() + Duration::from_secs(2),
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                checked.verify_executable()
            }),
        )
        .await;
        if !matches!(recheck, Ok(Ok(Ok(())))) {
            output.reason = StopReason::ExecutableChanged;
            output.status = ExecutionStatus::Failed;
        }
        Ok(output)
    }
}
async fn read_pipe(
    mut pipe: impl AsyncRead + Unpin,
    stderr: bool,
    sender: mpsc::Sender<(bool, Result<Vec<u8>, String>)>,
) {
    let mut buffer = [0; 4096];
    loop {
        let chunk = match pipe.read(&mut buffer).await {
            Ok(0) => return,
            Ok(len) => Ok(buffer[..len].to_vec()),
            Err(error) => Err(error.to_string()),
        };
        let failed = chunk.is_err();
        if sender.send((stderr, chunk)).await.is_err() || failed {
            return;
        }
    }
}
fn retain(output: &mut RunOutput, stderr: bool, mut bytes: Vec<u8>, max: usize) -> bool {
    let remaining = max - output.stdout.len() - output.stderr.len();
    if bytes.len() > remaining {
        bytes.truncate(remaining);
        output.output_truncated = true;
    }
    if stderr {
        output.stderr.extend(bytes);
    } else {
        output.stdout.extend(bytes);
    }
    output.output_truncated
}
