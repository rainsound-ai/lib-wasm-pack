use std::fmt::Display;
use std::io;
use std::io::Write;
use std::path::PathBuf;

const CRATE_VERSION: &str = include_cargo_toml::include_toml!("package"."version");

/// Run wasm-pack with the given arguments.
///
/// ```
/// let args = vec![
///     "build",
///     "--out-dir",
///     // If we just passed "target/built-test-crate",
///     // the output would be in "./test-crate/target/built-test-crate".
///     // So instead, we go up a directory.
///     "../target/built-test-crate",
///     // The input crate path is relative to the current directory.
///     "test-crate",
/// ];
///
/// lib_wasm_pack::run(args).expect("Running wasm-pack failed.");
/// ```
pub fn run<Args>(args: Args) -> Result<WasmPackOutput, WasmPackError>
where
    Args: IntoIterator,
    Args::Item: Into<std::ffi::OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    println!("Running wasm-pack with args: {:?}", args);

    let path_to_cli_executable = get_cli_executable_file()?;
    println!("Got CLI executable file: {:?}", path_to_cli_executable);
    println!("Executing CLI executable...");
    let output = duct::cmd(&path_to_cli_executable, args)
        .stderr_capture()
        .stdout_capture()
        .unchecked()
        .run()
        .map_err(WasmPackError::CouldntInvokeWasmPack)?;

    let (stdout, stderr) = get_stdout_and_stderr_from_process_output(&output);

    println!("CLI executable finished executing.");
    println!("CLI executable stdout: {}", &stdout);
    println!("CLI executable stderr: {}", &stderr);

    std::fs::remove_file(path_to_cli_executable)
        .map_err(WasmPackError::CouldntDeleteTemporaryFile)?;
    println!("Deleted temporary file.");

    if !output.status.success() {
        println!("CLI executable returned an error.");
        let error = WasmPackError::WasmPackReturnedAnError { stdout, stderr };
        return Err(error);
    }

    println!("CLI executable returned successfully.");
    let output = WasmPackOutput::new(stdout, stderr);
    Ok(output)
}

#[derive(Debug)]
pub struct WasmPackOutput {
    stdout: String,
    stderr: String,
}

impl WasmPackOutput {
    fn new(stdout: String, stderr: String) -> Self {
        Self { stdout, stderr }
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }
}

fn get_stdout_and_stderr_from_process_output(
    process_output: &std::process::Output,
) -> (String, String) {
    let stdout = String::from_utf8_lossy(&process_output.stdout)
        .trim()
        .to_string();

    let stderr = String::from_utf8_lossy(&process_output.stderr)
        .trim()
        .to_string();

    (stdout, stderr)
}

fn get_cli_executable_file() -> Result<PathBuf, WasmPackError> {
    let platform = guess_platform();
    println!("Guessed platform: {:?}", platform);
    let cli_executable_bytes = get_cli_executable_bytes(&platform);
    println!(
        "Got CLI executable bytes: {} bytes",
        cli_executable_bytes.len()
    );

    // We use a UUID in case multiple builds are running at the same time.
    let uuid = uuid::Uuid::new_v4().to_string();
    let temp_file_name = format!("wasm-pack-{}-v{}-{}", platform, CRATE_VERSION, uuid);
    let temp_file_path = std::env::current_dir()
        .map_err(WasmPackError::CouldntSaveCliExecutableToTemporaryFile)?
        .join("target")
        .join(temp_file_name);

    let mut temp_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&temp_file_path)
        .map_err(WasmPackError::CouldntSaveCliExecutableToTemporaryFile)?;
    println!("Created temporary file: {:?}", &temp_file_path);

    temp_file
        .write_all(cli_executable_bytes)
        .map_err(WasmPackError::CouldntSaveCliExecutableToTemporaryFile)?;
    println!("Wrote CLI executable bytes to temporary file.");

    // Make the file executable. This isn't supported on Windows, so we skip it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = temp_file
            .metadata()
            .map_err(WasmPackError::CouldntSaveCliExecutableToTemporaryFile)?
            .permissions();
        // 755 - owner can read/write/execute, group/others can read/execute.
        permissions.set_mode(0o755);
        temp_file
            .set_permissions(permissions)
            .map_err(WasmPackError::CouldntSaveCliExecutableToTemporaryFile)?;
        println!("Made temporary file executable.");
    }

    // Make sure the file is closed and written to disk.
    temp_file
        .sync_all()
        .map_err(WasmPackError::CouldntSaveCliExecutableToTemporaryFile)?;
    drop(temp_file);

    Ok(temp_file_path)
}

#[derive(Debug)]
enum Platform {
    MacOs,

    LinuxArm64,
    LinuxX64,

    Windows,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Platform::MacOs => "x86_64-apple-darwin",
            Platform::LinuxArm64 => "aarch64-unknown-linux-musl",
            Platform::LinuxX64 => "x86_64-unknown-linux-musl",
            Platform::Windows => "x86_64-pc-windows-msvc",
        };
        write!(f, "{}", name)
    }
}

fn guess_platform() -> Platform {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match os {
        "macos" => Platform::MacOs,
        "linux" => match arch {
            "x86_64" => Platform::LinuxX64,
            "aarch64" => Platform::LinuxArm64,
            _ => panic!("Unsupported architecture: {}", arch),
        },
        "windows" => Platform::Windows,
        _ => panic!("Unsupported OS: {}", os),
    }
}

fn get_cli_executable_bytes(platform: &Platform) -> &'static [u8] {
    match platform {
        Platform::MacOs => include_bytes!("./wasm-pack-v0.12.1-x86_64-apple-darwin/wasm-pack"),
        Platform::LinuxArm64 => {
            include_bytes!("./wasm-pack-v0.12.1-aarch64-unknown-linux-musl/wasm-pack")
        }
        Platform::LinuxX64 => {
            include_bytes!("./wasm-pack-v0.12.1-x86_64-unknown-linux-musl/wasm-pack")
        }
        Platform::Windows => {
            include_bytes!("./wasm-pack-v0.12.1-x86_64-pc-windows-msvc/wasm-pack.exe")
        }
    }
}

#[derive(Debug)]
pub enum WasmPackError {
    WasmPackReturnedAnError { stdout: String, stderr: String },
    CouldntInvokeWasmPack(io::Error),
    CouldntSaveCliExecutableToTemporaryFile(io::Error),
    CouldntDeleteTemporaryFile(io::Error),
}

impl Display for WasmPackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmPackError::WasmPackReturnedAnError { stdout, stderr } => {
                write!(f, "wasm-pack returned an error:\n\n")?;
                write!(f, "stdout:\n{}\n\n", stdout)?;
                write!(f, "stderr:\n{}\n\n", stderr)?;
                Ok(())
            }
            WasmPackError::CouldntInvokeWasmPack(error) => {
                write!(f, "Couldn't invoke wasm-pack: {}", error)
            }
            WasmPackError::CouldntSaveCliExecutableToTemporaryFile(error) => {
                write!(
                    f,
                    "Couldn't save wasm-pack executable to temporary file: {}",
                    error
                )
            }
            WasmPackError::CouldntDeleteTemporaryFile(error) => {
                write!(f, "Couldn't delete temporary file: {}", error)
            }
        }
    }
}

impl std::error::Error for WasmPackError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_correct() {
        let args = vec!["--version"];
        let output = run(&args).expect("Couldn't run `wasm-pack --version`.");
        let stdout = output.stdout();

        // Our crate versions are a string like "0.12.1-0.1.0". Everything before
        // the dash is the wasm-pack version. See the version policy in the README
        // for details.
        let expected_version = CRATE_VERSION.split('-').next().unwrap();
        let expected_version = format!("wasm-pack {}", expected_version);
        println!("Command stdout: {}", &stdout);
        println!("Expected version: {}", &expected_version);
        assert!(stdout.contains(&expected_version));
    }

    #[test]
    fn building_a_crate() {
        let input_crate_path = "test-crate";
        let built_crate_path = "target/built-test-crate";
        let built_crate_path_relative_to_input_crate = "../".to_string() + built_crate_path; // This is relative to the input crate path, so we have to go up a directory.

        let args = vec![
            "build",
            "--out-dir",
            &built_crate_path_relative_to_input_crate,
            input_crate_path,
        ];
        run(&args).expect("Couldn't run `wasm-pack`.");

        let built_js = std::fs::read_to_string(format!("{}/test_crate.js", built_crate_path))
            .expect("Couldn't read built JS file.");

        let expected_built_js = r#"import * as wasm from "./test_crate_bg.wasm";
import { __wbg_set_wasm } from "./test_crate_bg.js";
__wbg_set_wasm(wasm);
export * from "./test_crate_bg.js";
"#;

        assert_eq!(built_js, expected_built_js);

        let _ignore_errors = std::fs::remove_dir_all(built_crate_path);
    }

    #[test]
    fn input_file_not_found() {
        let input_crate_path = "fake-crate";
        let built_crate_path = "target/built-test-crate";
        let built_crate_path_relative_to_input_crate = "../".to_string() + built_crate_path; // This is relative to the input crate path, so we have to go up a directory.

        let args = vec![
            "build",
            "--out-dir",
            &built_crate_path_relative_to_input_crate,
            input_crate_path,
        ];

        let result = run(&args);

        if let Err(WasmPackError::WasmPackReturnedAnError { stdout, stderr }) = result {
            assert!(stdout.is_empty());
            assert!(stderr.contains("Error: crate directory is missing a `Cargo.toml` file; is `fake-crate` the wrong directory?"));
        } else {
            panic!("Expected WasmPackReturnedAnError error, got {:?}", result);
        }
    }
}
