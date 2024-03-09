use crate::pal::PalStore;
use crate::{job::JobResult, pal::run_retest};
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use job::Job;
use pal::{run_pal, CompileConfig, PalType};
use session::Session;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::ErrorKind,
    path::Path,
};

pub mod job;
pub mod pal;
pub mod parser;
pub mod session;
pub mod threadpool;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path of compiler to compile source file, default: gcc
    #[arg(short, long)]
    compiler: Option<String>,
    /// Arguments passed to compiler, default: -Wall -Wextra -lm
    #[arg(long)]
    compiler_args: Option<String>,
    /// Time limits for tested program to run(in seconds), default: 10
    #[arg(short, long)]
    timeout: Option<u64>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compare program's output with expected output defined in test config file
    Check {
        /// Path of the source of the program to test
        source: String,
        /// Path of the test config file, default: foo.test for source foo.c
        test_config: Option<String>,
    },
    /// Compare program's output with that from "standard program", with every possible input defined in test config file
    Pal {
        /// Path of the source of the program to test
        source: String,
        /// Path of the source of the "standard program", default: foo_std.c for foo.c
        std_source: Option<String>,
        /// Path of the test config file, default: foo.test for source foo.c
        test_config: Option<String>,
    },
    /// Compare program's output with that from "standard program", with random generated input defined in test config file
    RandomPal {
        /// Path of the source of the program to test
        source: String,
        /// Path of the source of the "standard program", default: foo_std.c for foo.c
        std_source: Option<String>,
        /// Path of the test config file, default: foo.test for source foo.c
        test_config: Option<String>,
    },
    /// Access test results of previous test, or recheck after fixing bugs
    Session {
        #[command(subcommand)]
        subcommand: Option<SessionCommands>,
    },
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Load information(input, actual_output, expected_output) from failed tests
    Load {
        /// Number of tests information to load, default: 1
        #[arg(short, long)]
        num: Option<usize>,
        /// Specify type of failing reason to load(WA, TLE, REG, OE)
        #[arg(short = 't', long)]
        job_type: Option<String>,
    },
    /// Retest failed tests after fixing bugs
    Continue,
    /// Retest accepted and failed tests after fixing bugs
    Retest,
}
pub enum CLIError {
    InvalidArgument(String),
    IOError(String),
    PalError(String),
    OtherError(String),
    EnvironmentError(String),
    ParseError(String),
}

impl std::fmt::Debug for CLIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::InvalidArgument(s) => write!(f, "Argument invalid: {}", s),
            Self::IOError(s) => write!(f, "I/O error occured: {}", s),
            Self::PalError(s) => write!(f, "{}", s),
            Self::OtherError(s) => write!(f, "Other error: {}", s),
            Self::EnvironmentError(s) => write!(f, "Failed getting environment information: {}", s),
            Self::ParseError(s) => write!(f, "Failed parsing file: {}", s),
        }
    }
}

pub fn run_from_session(session: Session) -> Result<(), CLIError> {
    let current_working_directory = Path::new(&session.current_working_directory).to_path_buf();

    println!(
        "Current working directory: {}",
        current_working_directory.to_str().unwrap()
    );

    let compiler = session.compiler;
    let compiler_arguments = session.compiler_arguments;

    let timeout = session.timeout;

    let test_config = session.test_config_filename;

    let test_config_path = current_working_directory.join(&test_config);

    let test_config_str = fs::read_to_string(test_config_path)
        .map_err(|e| CLIError::IOError(format!("Cannot read test config: {:?}", e)))?;

    match session.pal_type {
        PalType::Check => {
            let compiler_config = CompileConfig {
                compiler,
                args: compiler_arguments,
                source: session.source,
                std_source: None,
                work_directory: current_working_directory.to_str().unwrap().to_string(),
            };
            run_pal(
                pal::PalType::Check,
                compiler_config,
                &test_config_str,
                session.job_store_filepath,
                timeout,
            )
            .map_err(|e| CLIError::PalError(format!("Error while running tests: {:?}", e)))?;
        }
        PalType::Pal => {
            let compiler_config = CompileConfig {
                compiler,
                args: compiler_arguments,
                source: session.source,
                std_source: Some(session.std_source),
                work_directory: current_working_directory.to_str().unwrap().to_string(),
            };
            run_pal(
                pal::PalType::Pal,
                compiler_config,
                &test_config_str,
                session.job_store_filepath,
                timeout,
            )
            .map_err(|e| CLIError::PalError(format!("Error while running tests: {:?}", e)))?;
        }
        PalType::RandomPal => {
            let compiler_config = CompileConfig {
                compiler,
                args: compiler_arguments,
                source: session.source,
                std_source: Some(session.std_source),
                work_directory: current_working_directory.to_str().unwrap().to_string(),
            };
            run_pal(
                pal::PalType::RandomPal,
                compiler_config,
                &test_config_str,
                session.job_store_filepath,
                timeout,
            )
            .map_err(|e| CLIError::PalError(format!("Error while running tests: {:?}", e)))?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

pub fn run(cli: Cli) -> Result<(), CLIError> {
    println!(
        "Running on: {}, CPU cores: {}",
        env::consts::OS,
        std::thread::available_parallelism().unwrap()
    );
    let projects_dirs = ProjectDirs::from("", "ReedThree", "reed_pal").ok_or_else(|| {
        CLIError::EnvironmentError(String::from("Cannot get config dir from system."))
    })?;
    let data_dir = projects_dirs.data_dir();
    println!("Data directory: {}", data_dir.to_str().unwrap());
    match cli.command {
        Commands::Session { subcommand } => {
            let session_path = data_dir.to_path_buf().join("session.json");
            if !session_path.exists() {
                return Err(CLIError::InvalidArgument(format!(
                    "Session file: {} does not exists. Run a test first.",
                    session_path.to_str().unwrap()
                )));
            }
            let session: Session =
                serde_json::from_reader(File::open(&session_path).map_err(|e| {
                    CLIError::IOError(format!(
                        "Cannot open session file: {} for {:?}",
                        session_path.to_str().unwrap(),
                        e
                    ))
                })?)
                .map_err(|e| CLIError::ParseError(format!("{:?}", e)))?;
            println!("Session id: {}", session.uuid);
            let job_store_filepath = &session.job_store_filepath;
            println!("Reading results from: {}...", job_store_filepath);
            let mut pal_store: PalStore = serde_json::from_reader(
                File::open(job_store_filepath)
                    .map_err(|e| CLIError::IOError(format!("Cannot reading results: {:?}", e)))?,
            )
            .map_err(|e| CLIError::ParseError(format!("Cannot parsing results file: {:?}", e)))?;

            match subcommand {
                Some(subcommand) => match subcommand {
                    SessionCommands::Load { num, job_type } => {
                        if pal_store.job_failed.len() == 0 {
                            println!("No failed test to load.");
                        } else {
                            let num = num.unwrap_or(1);
                            let default_job_type = pal_store.job_failed[0].1.to_string();
                            let job_type = job_type.unwrap_or(default_job_type);
                            println!("job_type: {}", job_type);
                            let mut shown_count = 0;
                            for (job, job_result, shown) in &mut pal_store.job_failed {
                                if job_result.to_string() == job_type && !*shown {
                                    *shown = true;
                                    shown_count += 1;
                                    show_job((&job, &job_result), &session.test_info_directory)?;
                                }
                                if shown_count >= num {
                                    break;
                                }
                            }

                            if shown_count == 0 {
                                println!("No such job type: {}", job_type);
                            }
                        }
                        serde_json::to_writer(
                            File::create(job_store_filepath).map_err(|e| {
                                CLIError::IOError(format!("Cannot writing results: {:?}", e))
                            })?,
                            &pal_store,
                        )
                        .map_err(|e| {
                            CLIError::IOError(format!("Cannot writing results: {:?}", e))
                        })?;
                    }
                    SessionCommands::Continue => {
                        let compile_config = CompileConfig {
                            compiler: session.compiler.clone(),
                            args: session.compiler_arguments.clone(),
                            source: session.source.clone(),
                            std_source: None,
                            work_directory: session.current_working_directory.clone(),
                        };
                        let mut job_list = Vec::new();
                        pal_store
                            .job_failed
                            .iter()
                            .for_each(|(job, _, _)| job_list.push(job.clone()));
                        if job_list.len() == 0 {
                            println!("No failed test to run.")
                        } else {
                            run_retest(
                                compile_config,
                                job_list,
                                &session.job_store_filepath,
                                session.timeout,
                            )
                            .map_err(|e| {
                                CLIError::PalError(format!("Error while running tests: {:?}", e))
                            })?;
                        }
                    }
                    SessionCommands::Retest => {
                        let compile_config = CompileConfig {
                            compiler: session.compiler.clone(),
                            args: session.compiler_arguments.clone(),
                            source: session.source.clone(),
                            std_source: None,
                            work_directory: session.current_working_directory.clone(),
                        };
                        let mut job_list = Vec::new();
                        pal_store
                            .job_passed
                            .iter()
                            .for_each(|(job, _, _)| job_list.push(job.clone()));
                        pal_store
                            .job_failed
                            .iter()
                            .for_each(|(job, _, _)| job_list.push(job.clone()));
                        run_retest(
                            compile_config,
                            job_list,
                            &session.job_store_filepath,
                            session.timeout,
                        )
                        .map_err(|e| {
                            CLIError::PalError(format!("Error while running tests: {:?}", e))
                        })?;
                    }
                },
                None => {
                    println!(
                        "PASSED: {}, FAILED: {}",
                        pal_store.job_passed.len(),
                        pal_store.job_failed.len()
                    );

                    let mut failed_by_type = HashMap::new();

                    if pal_store.job_failed.len() > 0 {
                        println!("Of failed tests: ");
                        pal_store
                            .job_failed
                            .iter()
                            .for_each(|(job, job_result, _)| {
                                failed_by_type
                                    .entry(job_result.to_string())
                                    .or_insert(Vec::new())
                                    .push(job);
                            })
                    }

                    failed_by_type.iter().for_each(|(job_result, job_lst)| {
                        println!("{}: {}", job_result, job_lst.len());
                    })
                }
            }
        }
        _ => {
            let create_data_directory = fs::create_dir_all(data_dir);

            if create_data_directory
                .as_ref()
                .is_err_and(|e| e.kind() != ErrorKind::AlreadyExists)
            {
                return Err(CLIError::IOError(format!(
                    "Cannot create data directory: {:?}",
                    create_data_directory.err().unwrap()
                )));
            }

            let mut session = Session::build(cli, data_dir.to_path_buf())?;
            println!("Session id: {}", session.uuid);

            run_from_session(session.clone())?;
            session.run = true;
            fs::write(
                data_dir.to_path_buf().join("session.json"),
                serde_json::to_vec(&session).unwrap(),
            )
            .map_err(|e| CLIError::IOError(format!("Cannot save session data: {:?}", e)))?;
        }
    }

    Ok(())
}

pub fn show_job(
    job_info: (&Job, &JobResult),
    test_info_directory: &String,
) -> Result<(), CLIError> {
    let (job, job_result) = job_info;
    println!("{}(Job id = {})", job_result, job.id);
    let test_info_directory = Path::new(test_info_directory).to_path_buf();

    let create_result = fs::create_dir(test_info_directory.join(format!("{}", job.id)));
    if create_result
        .as_ref()
        .is_err_and(|e| e.kind() != ErrorKind::AlreadyExists)
    {
        return Err(CLIError::IOError(format!(
            "Cannot create output directory to save job logs: {} for {:?}",
            test_info_directory
                .join(format!("{}", job.id))
                .to_str()
                .unwrap(),
            create_result.err().unwrap()
        )));
    }

    let in_path = test_info_directory
        .join(format!("{}", job.id))
        .join("in.txt");
    let actual_out_path = test_info_directory
        .join(format!("{}", job.id))
        .join("actual_out.txt");
    let expected_out_path = test_info_directory
        .join(format!("{}", job.id))
        .join("expected_out.txt");

    println!("Input file: {}", in_path.to_str().unwrap());
    fs::write(&in_path, &job.input).map_err(|e| {
        CLIError::IOError(format!(
            "Cannot write to {} for {:?}",
            in_path.to_str().unwrap(),
            e
        ))
    })?;

    println!("Actual output file: {}", actual_out_path.to_str().unwrap());
    fs::write(&actual_out_path, &job.actual_output).map_err(|e| {
        CLIError::IOError(format!(
            "Cannot write to {} for {:?}",
            in_path.to_str().unwrap(),
            e
        ))
    })?;

    println!(
        "Expected output file: {}",
        expected_out_path.to_str().unwrap()
    );
    fs::write(&expected_out_path, &job.expected_output).map_err(|e| {
        CLIError::IOError(format!(
            "Cannot write to {} for {:?}",
            in_path.to_str().unwrap(),
            e
        ))
    })?;

    Ok(())
}
