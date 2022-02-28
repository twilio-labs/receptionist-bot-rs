/// https://github.com/matklad/cargo-xtask/
///
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
};

type DynError = Box<dyn std::error::Error>;

// commands
const CMD_BUILD_SERVER_IMAGE: &str = "build-server-image";
const CMD_PUSH_IMAGE_AWS: &str = "push-image-aws";
const CMD_BUILD_LAMBDA_EVENTS: &str = "build-lambda-events";
const CMD_BUILD_LAMBDA_INTERACTIONS: &str = "build-lambda-interactions";
const CMD_BUILD_LAMBDA_COMMANDS: &str = "build-lambda-commands";
const CMD_BUILD_LAMBDA_ALL: &str = "build-lambda-all";

// lambda package names
const LAMBDA_EVENTS: &str = "rec_lambda_events";
const LAMBDA_INTERACTIONS: &str = "rec_lambda_interactions";
const LAMBDA_COMMANDS: &str = "rec_lambda_commands";

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e}");
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_deref() {
        Some(c) if c == CMD_BUILD_LAMBDA_EVENTS => prep_lambda_for_terraform(LAMBDA_EVENTS)?,
        Some(c) if c == CMD_BUILD_LAMBDA_COMMANDS => prep_lambda_for_terraform(LAMBDA_COMMANDS)?,
        Some(c) if c == CMD_BUILD_LAMBDA_INTERACTIONS => {
            prep_lambda_for_terraform(LAMBDA_INTERACTIONS)?
        }
        Some(c) if c == CMD_BUILD_LAMBDA_ALL => {
            let thread_handles =
                [LAMBDA_EVENTS, LAMBDA_COMMANDS, LAMBDA_INTERACTIONS].map(|pkg_name| {
                    thread::spawn(|| {
                        prep_lambda_for_terraform(pkg_name)
                            .expect("failed to prep lambda: {pkg_name}")
                    })
                });

            let mut errors = Vec::new();
            for t in thread_handles {
                if let Err(msg) = t.join() {
                    errors.push(msg);
                }
            }

            if !errors.is_empty() {
                return Err(format!("{:?}", errors).into());
            }
        }
        _ => print_help(),
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "\nTasks:
{CMD_BUILD_SERVER_IMAGE}         `docker build` the Receptionist webserver
{CMD_PUSH_IMAGE_AWS}             `docker push` the built image to AWS ECR
--
{CMD_BUILD_LAMBDA_EVENTS}         cross-compile [events] lambda binary
{CMD_BUILD_LAMBDA_COMMANDS}       cross-compile [commands] lambda binary
{CMD_BUILD_LAMBDA_INTERACTIONS}   cross-compile [interactions] lambda binary
{CMD_BUILD_LAMBDA_ALL}            cross-compile all lambdas
"
    )
}

#[allow(dead_code)]
fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        // .nth(1)  would be `./crates`, .nth(2) is the Root
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn build_lambda(package_name: &str) -> Result<(), DynError> {
    let status = Command::new("docker")
        .current_dir(project_root())
        .args(&[
            "run",
            "--rm",
            "-t",
            "-v",
            &format!("{}:/home/rust/src", project_root().display()),
            "messense/rust-musl-cross:aarch64-musl",
            "cargo",
            "build",
            &format!("--package={package_name}"),
            "--release",
        ])
        .status()?;

    if !status.success() {
        return Err("cargo build failed".into());
    }
    Ok(())
}

fn copy_lambda_binary_to_terraform_dir(package_name: &str) -> Result<(), DynError> {
    let binary_path = project_root().join(format!(
        "target/aarch64-unknown-linux-musl/release/{package_name}"
    ));

    let destination_dir =
        project_root().join(format!("terraform_aws/serverless/archives/{package_name}"));

    fs::create_dir_all(&destination_dir)?;
    fs::copy(&binary_path, destination_dir.join("bootstrap"))?;

    Ok(())
}

fn prep_lambda_for_terraform(package_name: &str) -> Result<(), DynError> {
    build_lambda(package_name)?;
    copy_lambda_binary_to_terraform_dir(package_name)?;
    Ok(())
}
