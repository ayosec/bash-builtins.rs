//! Run scripts in tests/examples/*.sh, and compare their outputs with the
//! *.output files.

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json as json;

/// Variable to send the path of the test file.
const TEST_FILE_VAR: &str = "TEST_FILE_PATH";

macro_rules! check_output {
    ($path: expr, $output:expr) => {
        if !$output.status.success() {
            std::io::stdout().write_all(&$output.stdout).unwrap();
            std::io::stderr().write_all(&$output.stderr).unwrap();
            panic!("{:?} failed: {:?}", $path, $output.status);
        }
    };
}

/// Invoke `cargo-build` and read its output in JSON format.
///
/// Cargo should send the paths of every `.so` file in a
/// `"compiler-artifact"` message.
fn build_examples() -> Vec<(String, PathBuf)> {
    let build = Command::new(env::var("CARGO").unwrap())
        .arg("build")
        .arg("--quiet")
        .arg("--examples")
        .args(["--message-format", "json"])
        .output()
        .unwrap();

    check_output!("cargo build", build);

    let mut examples = Vec::new();

    for line in build.stdout.split(|c| *c == b'\n') {
        if let Ok(msg) = json::from_slice::<json::Value>(line) {
            if msg["reason"] == "compiler-artifact" {
                let target = &msg["target"];
                if target["kind"][0] == "example" {
                    let name = target["name"].as_str();
                    let file = msg["filenames"][0].as_str();

                    if let (Some(name), Some(file)) = (name, file) {
                        examples.push((name.to_string(), file.to_string().into()));
                    }
                }
            }
        }
    }

    examples.sort();
    examples
}

/// Create a shell script to launch the tests in a bash process.
///
/// Returns the path to the new file.
fn create_runner_file(target: &Path) -> PathBuf {
    let rc_path = target.join("init.sh");
    let mut output = BufWriter::new(File::create(&rc_path).unwrap());

    macro_rules! w {
        ($($t:tt)*) => {
            writeln!(&mut output, $($t)*).unwrap();
        }
    }

    w!("exec 2>&1");
    w!("load_example() {{");
    w!("\tcase \"$1\" in");

    for (name, path) in build_examples() {
        w!("\t\t{}) enable -f '{}' {} ;;", name, path.display(), name);
    }

    w!("\t\t*) echo \"missing $1 example\"; return 1 ;;");
    w!("\tesac");
    w!("}}");

    w!("source ${}", TEST_FILE_VAR);

    rc_path
}

#[test]
fn check_examples() {
    let target = {
        let mut target = match env::var_os("CARGO_TARGET_DIR") {
            Some(t) => PathBuf::from(t),

            None => {
                let mut target = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
                target.push("target");
                target
            }
        };

        target.push("examples");
        target.push(env::var_os("CARGO_PKG_NAME").unwrap());

        std::fs::create_dir_all(&target).unwrap();

        target
    };

    let test_runner = create_runner_file(&target);

    let mut failed = 0;
    for source in fs::read_dir("tests/examples").unwrap() {
        let path = source.unwrap().path();

        if path.extension() != Some(OsStr::new("sh")) {
            continue;
        }

        let expected_output = {
            let mut exp_path = path.clone();
            exp_path.set_extension("output");
            fs::read_to_string(exp_path).unwrap_or_default()
        };

        let bash = Command::new("bash")
            .env("LC_ALL", "C")
            .env("MALLOC_CHECK_", "2")
            .env(TEST_FILE_VAR, &path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg(&test_runner)
            .spawn()
            .unwrap();

        // Wait until the script is done.
        let output = bash.wait_with_output().unwrap();
        check_output!(path, output);

        // Capture output and compare with the expected one.
        //
        // The path of the runner script is replaced with "$RUNNER".
        let test_output = String::from_utf8(output.stdout)
            .unwrap()
            .replace(test_runner.to_str().unwrap_or_default(), "$RUNNER");

        if test_output != expected_output {
            let test_name = path.file_name().unwrap();
            let mut output_copy = target.join(test_name);
            output_copy.set_extension("current-output");

            failed += 1;
            eprintln!("### {}: failed", path.display());
            eprintln!("=== OUTPUT ({})\n{}\n", output_copy.display(), test_output);
            eprintln!("=== EXPECTED\n{}\n", expected_output);

            fs::write(output_copy, test_output).unwrap();
        }
    }

    assert_eq!(failed, 0);
}
