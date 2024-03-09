use crate::job::JobResult;
use crate::threadpool::ThreadPool;
use crate::{
    job::{run_job, Job},
    parser::parse,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;
#[derive(Serialize, Deserialize)]
pub struct PalStore {
    pub job_passed: Vec<(Job, JobResult, bool)>,
    pub job_failed: Vec<(Job, JobResult, bool)>,
    pub pal_info: PalInfo,
}

#[derive(PartialEq, Eq)]
pub enum PalError {
    ParseError(String),
    CompileError(String),
    RunTestError(String),
    IOError(String),
    LoadStoreError(String),
}
#[derive(Clone, Deserialize, Serialize)]
pub enum PalType {
    Check,
    Pal,
    RandomPal,
    Retest,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct PalInfo {
    pub prog: String,
    pub work_directory: String,
    pub out_directory: String,
    pub test_info_directory: String,
    pub job_store_filepath: String,
    pub std: Option<String>,
    pub timeout_sec: u64,
}

pub struct CompileConfig {
    pub compiler: String,
    pub args: String,
    pub source: String,
    pub std_source: Option<String>,
    pub work_directory: String,
}

impl std::fmt::Display for PalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Check => write!(f, "Check"),
            Self::Pal => write!(f, "Pal"),
            Self::RandomPal => write!(f, "RandomPal"),
            Self::Retest => write!(f, "Retest"),
        }
    }
}

impl std::fmt::Debug for PalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::ParseError(e) => write!(f, "Cannot parse test config: {}", e),
            Self::CompileError(e) => write!(f, "CE: {}", e),
            Self::IOError(e) => write!(f, "I/O Error while running pal: {}", e),
            Self::RunTestError(e) => write!(f, "Error while preparing for test: {}", e),
            Self::LoadStoreError(e) => write!(f, "Error while loading store file: {}", e),
        }
    }
}

impl CompileConfig {
    pub fn command(&self) -> String {
        format!("{} {}", self.compiler, self.args)
    }
}

pub fn compile(
    compile_config: CompileConfig,
    timeout_sec: u64,
    job_store_path: &str,
) -> Result<PalInfo, String> {
    let work_directory_path = PathBuf::from_str(&compile_config.work_directory)
        .map_err(|e| format!("Cannot parser work directory: {:?}", e))?;
    let source_prefix = Path::new(&compile_config.source)
        .file_stem()
        .ok_or_else(|| format!("Invalid source filename: {}", compile_config.source))?
        .to_str()
        .unwrap();

    let source = work_directory_path
        .join(&compile_config.source)
        .to_str()
        .unwrap()
        .to_string();

    let mut output_dir = Path::new(&source).to_path_buf();
    output_dir.pop();

    let mut test_info_dir = output_dir.clone();

    output_dir.push("out");

    test_info_dir.push("tests_info");
    test_info_dir.push(source_prefix);

    if fs::create_dir(output_dir.clone()).is_err_and(|x| x.kind() != ErrorKind::AlreadyExists) {
        return Err(format!(
            "Failed to create output dir: {}",
            output_dir.to_str().unwrap()
        ));
    }

    if fs::create_dir_all(&test_info_dir).is_err_and(|e| e.kind() != ErrorKind::AlreadyExists) {
        return Err(format!(
            "Cannot create test info directory: {}",
            test_info_dir.to_str().unwrap()
        ));
    }

    let output = output_dir.join(source_prefix).to_str().unwrap().to_string();

    // Compile user program
    let mut args: Vec<&str> = compile_config.args.split(" ").collect();
    args.push(source.as_str());
    args.push("-o");
    args.push(output.as_str());

    let p = Command::new(compile_config.compiler.clone())
        .args(args)
        .current_dir(work_directory_path.to_str().unwrap())
        .output()
        .map_err(|e| format!("Failed to launch compiler: {}", e))?;

    if !p.status.success() {
        return Err(format!(
            "user program compile failed: \n{}{}",
            String::from_utf8(p.stdout).unwrap(),
            String::from_utf8(p.stderr).unwrap(),
        ));
    }

    match &compile_config.std_source {
        Some(std_source) => {
            let std_source_prefix = Path::new(&std_source)
                .file_stem()
                .ok_or_else(|| format!("Invalid source filename: {}", std_source))?
                .to_str()
                .unwrap();
            let std_source = work_directory_path
                .join(&std_source)
                .to_str()
                .unwrap()
                .to_string();
            let std_output = output_dir
                .join(std_source_prefix)
                .to_str()
                .unwrap()
                .to_string();
            // Compile std program
            let mut args: Vec<&str> = compile_config.args.split(" ").collect();
            args.push(std_source.as_str());
            args.push("-o");
            args.push(std_output.as_str());

            let p = Command::new(compile_config.compiler)
                .args(args)
                .current_dir(work_directory_path.to_str().unwrap())
                .output()
                .map_err(|e| format!("Failed to launch compiler: {}", e))?;

            if p.status.success() {
                Ok(PalInfo {
                    prog: String::from(output),
                    work_directory: String::from(work_directory_path.to_str().unwrap()),
                    out_directory: String::from(output_dir.to_str().unwrap()),
                    test_info_directory: String::from(test_info_dir.to_str().unwrap()),
                    job_store_filepath: job_store_path.to_string(),
                    std: Some(String::from(std_output)),
                    timeout_sec,
                })
            } else {
                Err(format!(
                    "std program compile failed: \n{}{}",
                    String::from_utf8(p.stdout).unwrap(),
                    String::from_utf8(p.stderr).unwrap(),
                ))
            }
        }

        None => Ok(PalInfo {
            prog: String::from(output),
            work_directory: String::from(work_directory_path.to_str().unwrap()),
            out_directory: String::from(output_dir.to_str().unwrap()),
            test_info_directory: String::from(test_info_dir.to_str().unwrap()),
            job_store_filepath: job_store_path.to_string(),
            std: None,
            timeout_sec,
        }),
    }
}

pub fn run_pal(
    pal_type: PalType,
    compile_config: CompileConfig,
    test_config: &str,
    job_store_path: String,
    timeout_sec: u64,
) -> Result<(), PalError> {
    let now = Instant::now();
    println!("Running for type: {}", pal_type);
    println!("Parsing config...");
    let job_list =
        parse(&pal_type, test_config).map_err(|e| PalError::ParseError(format!("{:?}", e)))?;

    let parse_time = now.elapsed().as_millis();

    let now = Instant::now();

    println!("Job count: {}", job_list.len());
    let thread_count = job_list
        .len()
        .min(std::thread::available_parallelism().unwrap().into());

    let mut passed = 0;
    let mut failed = 0;

    println!("Compiling using: {}", compile_config.command());
    let pal_info = compile(compile_config, timeout_sec, &job_store_path)
        .map_err(|e| PalError::CompileError(e))?;

    let compile_time = now.elapsed().as_millis();
    let now = Instant::now();

    println!("Running jobs using {} threads...", thread_count);
    println!("Test info directory: {}", &pal_info.test_info_directory);
    println!("A \".\" indicates a passed test. A \"X\" indicates a failed test: ");

    let mut pool = ThreadPool::new(thread_count);

    let (tx, rx) = mpsc::channel();

    let pal_type_arc = Arc::new(pal_type);
    let pal_info_arc = Arc::new(pal_info.clone());

    for job in job_list {
        let this_tx = tx.clone();
        let this_pal_type = Arc::clone(&pal_type_arc);
        let this_pal_info = Arc::clone(&pal_info_arc);
        pool.execute(move || {
            let result = run_job(this_pal_type, this_pal_info, job);

            this_tx.send(result).unwrap();
        });
    }

    drop(tx);

    let mut job_passed = Vec::new();
    let mut job_failed = Vec::new();

    for (job, job_result) in rx {
        if !job_result.is_passed() {
            failed += 1;
            print!("X");
            job_failed.push((job, job_result, false));

            pool.shutdown();
        } else {
            passed += 1;
            print!(".");
            job_passed.push((job, job_result, false));
        }
    }

    drop(pool);

    println!();

    let store = PalStore {
        job_passed,
        job_failed,
        pal_info,
    };

    let run_time = now.elapsed().as_millis();

    save_pal(&job_store_path, store)?;

    summarize(([passed, failed], [parse_time, compile_time, run_time]));

    Ok(())
}

pub fn run_retest(
    compile_config: CompileConfig,
    job_list: Vec<Job>,
    job_store_path: &String,
    timeout_sec: u64,
) -> Result<(), PalError> {
    let now = Instant::now();
    println!("Retesting...");

    let parse_time = now.elapsed().as_millis();

    let now = Instant::now();

    println!("Job count: {}", job_list.len());
    let thread_count = job_list
        .len()
        .min(std::thread::available_parallelism().unwrap().into());

    let mut passed = 0;
    let mut failed = 0;

    println!("Compiling using: {}", compile_config.command());
    let pal_info = compile(compile_config, timeout_sec, &job_store_path)
        .map_err(|e| PalError::CompileError(e))?;

    let compile_time = now.elapsed().as_millis();
    let now = Instant::now();

    println!("Running jobs using {} threads...", thread_count);
    println!("Test info directory: {}", &pal_info.test_info_directory);
    println!("A \".\" indicates a passed test. A \"X\" indicates a failed test: ");
    let mut pool = ThreadPool::new(thread_count);

    let (tx, rx) = mpsc::channel();

    let pal_type_arc = Arc::new(PalType::Retest);
    let pal_info_arc = Arc::new(pal_info.clone());

    for job in job_list {
        let this_tx = tx.clone();
        let this_pal_type = Arc::clone(&pal_type_arc);
        let this_pal_info = Arc::clone(&pal_info_arc);
        pool.execute(move || {
            let result = run_job(this_pal_type, this_pal_info, job);

            this_tx.send(result).unwrap();
        });
    }

    drop(tx);

    let mut job_passed = Vec::new();
    let mut job_failed = Vec::new();

    for (job, job_result) in rx {
        if !job_result.is_passed() {
            failed += 1;
            print!("X");
            job_failed.push((job, job_result, false));

            pool.shutdown();
        } else {
            passed += 1;
            print!(".");
            job_passed.push((job, job_result, false));
        }
    }

    drop(pool);

    println!();

    let store = PalStore {
        job_passed,
        job_failed,
        pal_info,
    };

    let run_time = now.elapsed().as_millis();

    save_pal(&job_store_path, store)?;

    summarize(([passed, failed], [parse_time, compile_time, run_time]));

    Ok(())
}

fn save_pal(job_store_path: &str, store: PalStore) -> Result<(), PalError> {
    let store_path = Path::new(job_store_path);
    println!("Saving test result to {}...", store_path.to_str().unwrap());
    let store_bytes = serde_json::to_vec(&store).expect("Pal serialize should succeed");
    fs::write(store_path, &store_bytes)
        .map_err(|e| PalError::IOError(format!("Failed save test info for: {:?}", e)))?;
    Ok(())
}

fn summarize(info: ([usize; 2], [u128; 3])) {
    let ([passed, failed], [parse_time, compile_time, run_time]) = info;
    if failed == 0 {
        println!("PASSED: pass = {}, fail = {}", passed, failed);
    } else {
        println!("FAILED: pass = {}, fail = {}", passed, failed);
    }

    println!(
        "time: {}ms(total) = {}ms(parse) + {}ms(compile) + {}ms(run)",
        parse_time + compile_time + run_time,
        parse_time,
        compile_time,
        run_time
    );
}

pub fn parse_store(json_content: &str) -> Result<PalStore, PalError> {
    let store = serde_json::from_str(json_content)
        .map_err(|e| PalError::LoadStoreError(format!("Failed parse store file: {:?}", e)))?;

    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::Path;
    #[test]
    fn test_compile_check_success() {
        let work_dir = std::env::current_dir().unwrap();
        let source = work_dir
            .join("tests")
            .join("compile")
            .join("success.c")
            .to_str()
            .unwrap()
            .to_string();
        let args = String::from("-Wall -lm -Wextra");
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args,
            source,
            std_source: None,
            work_directory: work_dir.to_str().unwrap().to_string(),
        };
        let job_store_path = work_dir.join("tests/compile/tests_info/success.json");
        let pal_info = compile(compile_config, 10, job_store_path.to_str().unwrap()).unwrap();

        assert!(Path::new(&pal_info.prog).exists());

        fs::remove_file(&pal_info.prog).unwrap();
        let mut output_dir = Path::new(&pal_info.prog).to_path_buf();
        output_dir.pop();
        fs::remove_dir(output_dir).unwrap();
    }

    #[test]
    fn test_compile_check_failed() {
        let work_dir = std::env::current_dir().unwrap();
        let source = work_dir
            .join("tests")
            .join("compile")
            .join("failed.c")
            .to_str()
            .unwrap()
            .to_string();
        let args = String::from("-Wall -lm -Wextra");
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args,
            source,
            std_source: None,
            work_directory: work_dir.to_str().unwrap().to_string(),
        };
        let job_store_path = work_dir.join("tests/compile/tests_info/failed.json");
        let pal_info = compile(compile_config, 10, job_store_path.to_str().unwrap());

        assert!(pal_info.is_err());
    }

    #[test]
    fn test_compile_pal_success() {
        let work_dir = std::env::current_dir().unwrap();
        let source = work_dir
            .join("tests")
            .join("compile")
            .join("pal")
            .join("success.c")
            .to_str()
            .unwrap()
            .to_string();
        let std_source = work_dir
            .join("tests")
            .join("compile")
            .join("pal")
            .join("success_std.c")
            .to_str()
            .unwrap()
            .to_string();
        let args = String::from("-Wall -lm -Wextra");
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args,
            source,
            std_source: Some(std_source),
            work_directory: work_dir.to_str().unwrap().to_string(),
        };
        let job_store_path = work_dir.join("tests/compile/pal/tests_info/success.json");
        let pal_info = compile(compile_config, 10, job_store_path.to_str().unwrap()).unwrap();
        let std_prog = pal_info.std.unwrap();
        assert!(Path::new(&pal_info.prog).exists());
        assert!(Path::new(&std_prog).exists());
        fs::remove_file(&pal_info.prog).unwrap();
        fs::remove_file(&std_prog).unwrap();
        let mut output_dir = Path::new(&pal_info.prog).to_path_buf();
        output_dir.pop();
        fs::remove_dir(output_dir).unwrap();
    }

    #[test]
    fn test_compile_pal_user_ce() {
        let work_dir = std::env::current_dir().unwrap();
        let source = work_dir
            .join("tests")
            .join("compile")
            .join("pal")
            .join("user_ce.c")
            .to_str()
            .unwrap()
            .to_string();
        let std_source = work_dir
            .join("tests")
            .join("compile")
            .join("pal")
            .join("user_ce_std.c")
            .to_str()
            .unwrap()
            .to_string();
        let args = String::from("-Wall -lm -Wextra");
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args,
            source,
            std_source: Some(std_source),
            work_directory: work_dir.to_str().unwrap().to_string(),
        };
        let job_store_path = work_dir.join("tests/compile/pal/tests_info/user_ce.json");
        let result = compile(compile_config, 10, job_store_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_run_pal_check_success() {
        let cwd = env::current_dir().unwrap();
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args: String::from("-Wall -Wextra -lm"),
            source: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("check")
                    .join("success.c")
                    .to_str()
                    .unwrap(),
            ),
            std_source: None,
            work_directory: String::from(cwd.join("tests").join("pal").to_str().unwrap()),
        };
        let test_config = fs::read_to_string("tests/pal/check/success.test").unwrap();
        let job_store_path = cwd.join("tests/pal/check/tests_info/success.json");
        let pal_result = run_pal(
            PalType::Check,
            compile_config,
            &test_config,
            job_store_path.to_str().unwrap().to_string(),
            10,
        );

        assert!(pal_result.is_ok());
    }
    #[test]
    fn test_run_pal_check_ce() {
        let cwd = env::current_dir().unwrap();
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args: String::from("-Wall -Wextra -lm"),
            source: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("check")
                    .join("ce.c")
                    .to_str()
                    .unwrap(),
            ),
            std_source: None,
            work_directory: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("check")
                    .to_str()
                    .unwrap(),
            ),
        };
        let test_config = fs::read_to_string("tests/pal/check/ce.test").unwrap();
        let job_store_path = cwd.join("tests/pal/check/tests_info/ce.json");
        let pal_result = run_pal(
            PalType::Check,
            compile_config,
            &test_config,
            job_store_path.to_str().unwrap().to_string(),
            10,
        );

        match pal_result {
            Err(PalError::CompileError(_)) => {}
            _ => panic!(),
        }
    }
    #[test]
    fn test_run_pal_check_wa() {
        let cwd = env::current_dir().unwrap();
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args: String::from("-Wall -Wextra -lm"),
            source: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("check")
                    .join("wa.c")
                    .to_str()
                    .unwrap(),
            ),
            std_source: None,
            work_directory: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("check")
                    .to_str()
                    .unwrap(),
            ),
        };
        let test_config = fs::read_to_string("tests/pal/check/wa.test").unwrap();
        let job_store_path = cwd.join("tests/pal/check/tests_info/ce.json");
        let pal_result: Result<(), PalError> = run_pal(
            PalType::Check,
            compile_config,
            &test_config,
            job_store_path.to_str().unwrap().to_string(),
            10,
        );
        assert!(pal_result.is_ok());
    }

    #[test]
    #[ignore]
    fn test_run_pal_pal_success() {
        let cwd = env::current_dir().unwrap();
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args: String::from("-Wall -Wextra -lm"),
            source: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("pal")
                    .join("success.c")
                    .to_str()
                    .unwrap(),
            ),
            std_source: Some(String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("pal")
                    .join("success_std.c")
                    .to_str()
                    .unwrap(),
            )),
            work_directory: String::from(
                cwd.join("tests").join("pal").join("pal").to_str().unwrap(),
            ),
        };
        let test_config = fs::read_to_string("tests/pal/pal/success.test").unwrap();
        let job_store_path = cwd.join("tests/pal/pal/tests_info/success.json");
        let pal_result = run_pal(
            PalType::Pal,
            compile_config,
            &test_config,
            job_store_path.to_str().unwrap().to_string(),
            10,
        );

        assert!(pal_result.is_ok());
    }

    #[test]
    #[ignore]
    fn test_run_pal_random_success() {
        let cwd = env::current_dir().unwrap();
        let compile_config = CompileConfig {
            compiler: String::from("gcc"),
            args: String::from("-Wall -Wextra -lm"),
            source: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("random_pal")
                    .join("success.c")
                    .to_str()
                    .unwrap(),
            ),
            std_source: Some(String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("random_pal")
                    .join("success_std.c")
                    .to_str()
                    .unwrap(),
            )),
            work_directory: String::from(
                cwd.join("tests")
                    .join("pal")
                    .join("random_pal")
                    .to_str()
                    .unwrap(),
            ),
        };
        let test_config = fs::read_to_string("tests/pal/random_pal/success.test").unwrap();
        let job_store_path = cwd.join("tests/pal/random_pal/tests_info/success.json");
        let pal_result = run_pal(
            PalType::RandomPal,
            compile_config,
            &test_config,
            job_store_path.to_str().unwrap().to_string(),
            10,
        );

        assert!(pal_result.is_ok());
    }
}
