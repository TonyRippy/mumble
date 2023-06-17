use npm_rs::*;
use std::convert::TryFrom;
use std::io::Error;
use std::process::{ExitCode, ExitStatus};

fn to_exit_code(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(rc32) => match u8::try_from(rc32) {
            Ok(rc8) => ExitCode::from(rc8),
            _ => ExitCode::FAILURE,
        },
        _ => ExitCode::FAILURE,
    }
}

fn main() -> Result<ExitCode, Error> {
    // Build the client UX assets in the ui/ directory.
    println!("cargo:rerun-if-changed=ui/src");
    Ok(to_exit_code(
        NpmEnv::default()
            .set_path("ui")
            .init_env()
            .install(None)
            .run("build")
            .exec()?,
    ))
}
