use crate::pal::{PalInfo, PalType};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub enum ChildError {
    TimeOut(u64),
    SpawnError(String),
    InputOutputError(String),
    InvalidExitCode(Option<i32>),
}

impl std::fmt::Debug for ChildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::TimeOut(timeout) => {
                write!(f, "Child process haven't exited for {} secs", timeout)
            }
            Self::SpawnError(e) => {
                write!(f, "Failed to spawn child process: {}", e)
            }
            Self::InputOutputError(e) => {
                write!(f, "Failed to talk to child process: {}", e)
            }
            Self::InvalidExitCode(code) => {
                if code.is_none() {
                    write!(f, "Child terminated by signal.")
                } else {
                    write!(f, "Child returned: {}", code.unwrap())
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Job {
    pub id: usize,
    pub input: Vec<u8>,
    pub expected_output: Vec<u8>,
    pub actual_output: Vec<u8>,
}
#[derive(Deserialize, Serialize)]
pub enum JobResult {
    Success,
    Accepted,
    WrongAnswer,
    TimeLimitExceed,
    RuntimeError,
    OtherError(String),
    StdProgramError(ChildError),
}

impl std::fmt::Display for JobResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Accepted => write!(f, "."),
            Self::Success => write!(f, ""),
            Self::WrongAnswer => write!(f, "WA"),
            Self::TimeLimitExceed => write!(f, "TLE"),
            Self::RuntimeError => write!(f, "REG"),
            Self::OtherError(s) => write!(f, "OE({})", s),
            Self::StdProgramError(e) => write!(f, "STDERR({:?})", e),
        }
    }
}

impl JobResult {
    pub fn is_passed(&self) -> bool {
        match &self {
            Self::Accepted => true,
            Self::Success => true,
            _ => false,
        }
    }
}

pub fn run_prog(
    prog: &str,
    work_directory: &str,
    timeout_sec: u64,
    input: &[u8],
) -> Result<Vec<u8>, ChildError> {
    let mut out_buffer = Vec::new();

    let mut p = Command::new(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(work_directory)
        .spawn()
        .map_err(|e| ChildError::SpawnError(format!("{:?}", e)))?;

    let mut child_stdin = p
        .stdin
        .take()
        .ok_or_else(|| ChildError::InputOutputError(String::from("Child stdin is None")))?;

    let mut child_stdout = p
        .stdout
        .take()
        .ok_or_else(|| ChildError::InputOutputError(String::from("Child stdout is None")))?;

    child_stdin
        .write(input)
        .map_err(|e| ChildError::InputOutputError(format!("Cannot write to child stdin: {}", e)))?;

    drop(child_stdin);

    let wait_result = p.wait_timeout(Duration::from_secs(timeout_sec)).unwrap();

    if wait_result.is_none() {
        return Err(ChildError::TimeOut(timeout_sec));
    }

    let child_status_code = wait_result.unwrap().code();

    if child_status_code.is_none() {
        return Err(ChildError::InvalidExitCode(None));
    }

    let child_status_code = child_status_code.unwrap();

    if child_status_code != 0 {
        return Err(ChildError::InvalidExitCode(Some(child_status_code)));
    }

    child_stdout.read_to_end(&mut out_buffer).map_err(|e| {
        ChildError::InputOutputError(format!("Cannot read from child stdout: {}", e))
    })?;

    Ok(out_buffer)
}

pub fn run_job(pal_type: Arc<PalType>, pal_info: Arc<PalInfo>, job: Job) -> (Job, JobResult) {
    match *pal_type {
        PalType::Check => run_job_check(pal_info, job),
        PalType::Pal => run_job_pal(pal_info, job),
        PalType::RandomPal => run_job_pal(pal_info, job),
        PalType::Retest => run_job_check(pal_info, job),
    }
}

fn run_job_check(pal_info: Arc<PalInfo>, mut job: Job) -> (Job, JobResult) {
    let run_result = run_prog(
        &pal_info.prog,
        &pal_info.work_directory,
        pal_info.timeout_sec,
        &job.input,
    );

    if run_result.is_err() {
        let run_error = run_result.err().unwrap();
        match run_error {
            ChildError::TimeOut(_) => (job, JobResult::TimeLimitExceed),
            ChildError::InputOutputError(e) => (job, JobResult::OtherError(e)),
            ChildError::InvalidExitCode(None) => (job, JobResult::RuntimeError),
            ChildError::InvalidExitCode(Some(_)) => (job, JobResult::RuntimeError),
            ChildError::SpawnError(e) => (job, JobResult::OtherError(e)),
        }
    } else {
        let output = run_result.unwrap();
        // for text output, trim before compare
        // for binary output, just compare
        job.actual_output = output.clone();
        match String::from_utf8(job.expected_output.clone()) {
            Ok(s) => {
                let expected_output_trim = s.trim_end().as_bytes();
                job.expected_output = expected_output_trim.to_vec();
                match String::from_utf8(output) {
                    Ok(s) => {
                        let actual_outpupt_trim = s.trim_end().as_bytes();
                        job.actual_output = actual_outpupt_trim.to_vec();
                        if expected_output_trim == actual_outpupt_trim {
                            (job, JobResult::Accepted)
                        } else {
                            (job, JobResult::WrongAnswer)
                        }
                    }
                    Err(_) => (job, JobResult::WrongAnswer),
                }
            }
            Err(_) => {
                if output == job.expected_output {
                    (job, JobResult::Accepted)
                } else {
                    (job, JobResult::WrongAnswer)
                }
            }
        }
    }
}

fn run_job_pal(pal_info: Arc<PalInfo>, mut job: Job) -> (Job, JobResult) {
    let user_run_result = run_prog(
        &pal_info.prog,
        &pal_info.work_directory,
        pal_info.timeout_sec,
        &job.input,
    );

    let std_program = pal_info.std.as_ref().unwrap();

    if user_run_result.is_err() {
        let run_error = user_run_result.err().unwrap();
        match run_error {
            ChildError::TimeOut(_) => (job, JobResult::TimeLimitExceed),
            ChildError::InputOutputError(e) => (job, JobResult::OtherError(e)),
            ChildError::InvalidExitCode(None) => (job, JobResult::RuntimeError),
            ChildError::InvalidExitCode(Some(_)) => (job, JobResult::RuntimeError),
            ChildError::SpawnError(e) => (job, JobResult::OtherError(e)),
        }
    } else {
        let user_output = user_run_result.unwrap();
        let std_run_result = run_prog(
            &std_program,
            &pal_info.work_directory,
            pal_info.timeout_sec,
            &job.input,
        );
        if std_run_result.is_err() {
            return (
                job,
                JobResult::StdProgramError(std_run_result.err().unwrap()),
            );
        }
        let std_output = std_run_result.unwrap();
        // for text output, trim before compare
        // for binary output, just compare
        job.actual_output = user_output.clone();
        job.expected_output = std_output;
        match String::from_utf8(job.expected_output.clone()) {
            Ok(s) => {
                let expected_output_trim = s.trim_end().as_bytes();
                job.expected_output = expected_output_trim.to_vec();
                match String::from_utf8(user_output) {
                    Ok(s) => {
                        let actual_outpupt_trim = s.trim_end().as_bytes();
                        job.actual_output = actual_outpupt_trim.to_vec();
                        if expected_output_trim == actual_outpupt_trim {
                            (job, JobResult::Accepted)
                        } else {
                            (job, JobResult::WrongAnswer)
                        }
                    }
                    Err(_) => (job, JobResult::WrongAnswer),
                }
            }
            Err(_) => {
                if job.actual_output == job.expected_output {
                    (job, JobResult::Accepted)
                } else {
                    (job, JobResult::WrongAnswer)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_run_prog_io() {
        let work_dir = std::env::current_dir().unwrap();
        let test_prog = work_dir.join("tests").join("run_prog").join("io");
        let child_work_dir = work_dir.join("tests").join("run_prog").join("work_dir");

        let input = "Hello world!".as_bytes();

        let output = run_prog(
            test_prog.to_str().unwrap(),
            child_work_dir.to_str().unwrap(),
            10,
            input,
        )
        .unwrap();

        assert_eq!(input, output.as_slice());
    }

    #[test]
    fn test_run_prog_timeout() {
        let work_dir = std::env::current_dir().unwrap();
        let test_prog = work_dir.join("tests").join("run_prog").join("timeout");
        let child_work_dir = work_dir.join("tests").join("run_prog").join("work_dir");
        let run_result = run_prog(
            test_prog.to_str().unwrap(),
            child_work_dir.to_str().unwrap(),
            1,
            &[],
        );

        assert_eq!(run_result.err().unwrap(), ChildError::TimeOut(1));
    }

    #[test]
    fn test_run_prog_invalid_return() {
        let work_dir = std::env::current_dir().unwrap();
        let test_prog = work_dir
            .join("tests")
            .join("run_prog")
            .join("invalid_return");
        let child_work_dir = work_dir.join("tests").join("run_prog").join("work_dir");
        let run_result = run_prog(
            test_prog.to_str().unwrap(),
            child_work_dir.to_str().unwrap(),
            3,
            &[],
        );

        assert_eq!(
            run_result.err().unwrap(),
            ChildError::InvalidExitCode(Some(1))
        );
    }

    #[test]
    fn test_run_prog_sigsegv() {
        let work_dir = std::env::current_dir().unwrap();
        let test_prog = work_dir.join("tests").join("run_prog").join("sigsegv");
        let child_work_dir = work_dir.join("tests").join("run_prog").join("work_dir");
        let run_result = run_prog(
            test_prog.to_str().unwrap(),
            child_work_dir.to_str().unwrap(),
            3,
            &[],
        );

        assert_eq!(run_result.err().unwrap(), ChildError::InvalidExitCode(None));
    }
}
