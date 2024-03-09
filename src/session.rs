use crate::pal::PalType;
use crate::{CLIError, Cli, Commands};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use uuid::Uuid;
#[derive(Clone, Deserialize, Serialize)]
pub struct Session {
    pub uuid: Uuid,
    pub pal_type: PalType,
    pub current_working_directory: String,
    pub compiler: String,
    pub compiler_arguments: String,
    pub timeout: u64,
    pub source: String,
    pub std_source: String,
    pub test_config_filename: String,
    pub test_info_directory: String,
    pub job_store_filepath: String,
    pub run: bool,
}

impl Session {
    pub fn build(cli: Cli, data_directory: PathBuf) -> Result<Session, CLIError> {
        let current_working_directory = env::current_dir().map_err(|e| {
            CLIError::EnvironmentError(format!("Cannot get current working directory: {:?}", e))
        })?;

        let compiler = cli.compiler.unwrap_or(String::from("gcc"));
        let compiler_arguments = cli
            .compiler_args
            .unwrap_or(String::from("-Wall -Wextra -lm"));

        match &cli.command {
            Commands::Check {
                source,
                test_config,
            } => {
                let source_path = current_working_directory.join(source);
                let test_config_path =
                    build_test_config_path(&current_working_directory, &source_path, test_config)?;
                let job_store_filepath = build_job_store_filename(&source_path, data_directory)?
                    .to_str()
                    .unwrap()
                    .to_string();
                let source_prefix = source_path
                    .file_stem()
                    .ok_or_else(|| {
                        CLIError::InvalidArgument(format!(
                            "Invalid source filename: {}",
                            source_path.to_str().unwrap()
                        ))
                    })?
                    .to_str()
                    .unwrap();
                let test_info_directory = current_working_directory
                    .join(source)
                    .parent()
                    .unwrap_or(Path::new("/"))
                    .join("tests_info")
                    .join(source_prefix);
                Ok(Session {
                    uuid: Uuid::new_v4(),
                    pal_type: PalType::Check,
                    current_working_directory: current_working_directory
                        .to_str()
                        .unwrap()
                        .to_string(),
                    compiler,
                    compiler_arguments,
                    timeout: cli.timeout.unwrap_or(10),
                    source: source_path.to_str().unwrap().to_string(),
                    std_source: String::new(),
                    test_config_filename: test_config_path.to_str().unwrap().to_string(),
                    test_info_directory: test_info_directory.to_str().unwrap().to_string(),
                    job_store_filepath,
                    run: false,
                })
            }
            Commands::Pal {
                source,
                std_source,
                test_config,
            } => {
                let source_path = current_working_directory.join(source);
                let test_config_path =
                    build_test_config_path(&current_working_directory, &source_path, test_config)?;
                let job_store_filepath = build_job_store_filename(&source_path, data_directory)?
                    .to_str()
                    .unwrap()
                    .to_string();
                let source_prefix = source_path
                    .file_stem()
                    .ok_or_else(|| {
                        CLIError::InvalidArgument(format!(
                            "Invalid source filename: {}",
                            source_path.to_str().unwrap()
                        ))
                    })?
                    .to_str()
                    .unwrap();
                let test_info_directory = current_working_directory
                    .join(source)
                    .parent()
                    .unwrap_or(Path::new("/"))
                    .join("tests_info")
                    .join(source_prefix);
                Ok(Session {
                    uuid: Uuid::new_v4(),
                    pal_type: PalType::Pal,
                    current_working_directory: current_working_directory
                        .to_str()
                        .unwrap()
                        .to_string(),
                    compiler,
                    compiler_arguments,
                    timeout: cli.timeout.unwrap_or(10),
                    source: source_path.to_str().unwrap().to_string(),
                    std_source: build_std_source_path(
                        &current_working_directory,
                        &source_path,
                        std_source,
                    )?
                    .to_str()
                    .unwrap()
                    .to_string(),
                    test_config_filename: test_config_path.to_str().unwrap().to_string(),
                    test_info_directory: test_info_directory.to_str().unwrap().to_string(),
                    job_store_filepath,
                    run: false,
                })
            }
            Commands::RandomPal {
                source,
                std_source,
                test_config,
            } => {
                let source_path = current_working_directory.join(source);
                let test_config_path =
                    build_test_config_path(&current_working_directory, &source_path, test_config)?;
                let job_store_filepath = build_job_store_filename(&source_path, data_directory)?
                    .to_str()
                    .unwrap()
                    .to_string();
                let source_prefix = source_path
                    .file_stem()
                    .ok_or_else(|| {
                        CLIError::InvalidArgument(format!(
                            "Invalid source filename: {}",
                            source_path.to_str().unwrap()
                        ))
                    })?
                    .to_str()
                    .unwrap();
                let test_info_directory = current_working_directory
                    .join(source)
                    .parent()
                    .unwrap_or(Path::new("/"))
                    .join("tests_info")
                    .join(source_prefix);
                Ok(Session {
                    uuid: Uuid::new_v4(),
                    pal_type: PalType::RandomPal,
                    current_working_directory: current_working_directory
                        .to_str()
                        .unwrap()
                        .to_string(),
                    compiler,
                    compiler_arguments,
                    timeout: cli.timeout.unwrap_or(10),
                    source: source_path.to_str().unwrap().to_string(),
                    std_source: build_std_source_path(
                        &current_working_directory,
                        &source_path,
                        std_source,
                    )?
                    .to_str()
                    .unwrap()
                    .to_string(),
                    test_config_filename: test_config_path.to_str().unwrap().to_string(),
                    test_info_directory: test_info_directory.to_str().unwrap().to_string(),
                    job_store_filepath,
                    run: false,
                })
            }
            _ => Err(CLIError::OtherError(format!(
                "Calling Session::build() with invalid cli subcommand"
            ))),
        }
    }
}

fn build_test_config_path(
    cwd: &PathBuf,
    source_path: &PathBuf,
    test_config_optional: &Option<String>,
) -> Result<PathBuf, CLIError> {
    let source_prefix = source_path
        .file_stem()
        .ok_or_else(|| {
            CLIError::InvalidArgument(format!(
                "Invalid source filename: {}",
                source_path.to_str().unwrap()
            ))
        })?
        .to_str()
        .unwrap();

    if test_config_optional.is_none() {
        let root = Path::new("/");
        Ok(source_path
            .parent()
            .unwrap_or_else(|| root)
            .join(format!("{}.test", source_prefix)))
    } else {
        Ok(cwd.join(test_config_optional.as_ref().unwrap()))
    }
}

fn build_job_store_filename(
    source_path: &PathBuf,
    data_directory: PathBuf,
) -> Result<PathBuf, CLIError> {
    let source_prefix = source_path
        .file_stem()
        .ok_or_else(|| {
            CLIError::InvalidArgument(format!(
                "Invalid source filename: {}",
                source_path.to_str().unwrap()
            ))
        })?
        .to_str()
        .unwrap();

    Ok(data_directory.join(format!("{}_store.json", source_prefix)))
}

fn build_std_source_path(
    cwd: &PathBuf,
    source_path: &PathBuf,
    std_source_optional: &Option<String>,
) -> Result<PathBuf, CLIError> {
    let source_prefix = source_path
        .file_stem()
        .ok_or_else(|| {
            CLIError::InvalidArgument(format!(
                "Invalid source filename: {}",
                source_path.to_str().unwrap()
            ))
        })?
        .to_str()
        .unwrap();

    let source_suffix = source_path.extension();

    match source_suffix {
        Some(suffix_os_str) => {
            let std_source = std_source_optional.clone().unwrap_or(format!(
                "{}_std.{}",
                source_prefix,
                suffix_os_str.to_str().unwrap()
            ));
            Ok(cwd.join(std_source))
        }
        None => {
            let std_source = std_source_optional
                .clone()
                .unwrap_or(format!("{}_std", source_prefix));
            Ok(cwd.join(std_source))
        }
    }
}
