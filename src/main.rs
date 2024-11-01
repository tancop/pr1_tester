use core::str;
use serde::Deserialize;
use std::{env, fs::File, io::BufReader, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    process::Command,
};

#[derive(Deserialize)]
struct SequenceEntry {
    #[serde(rename = "in")]
    input: Option<String>,
    #[serde(rename = "out")]
    output: Option<String>,
}

#[derive(Deserialize)]
struct Test {
    name: String,
    points: f32,
    files_available: Option<bool>,
    sequence: Vec<SequenceEntry>,
}

#[derive(Deserialize)]
struct InputFile {
    tests: Vec<Test>,
}

#[tokio::main]
async fn main() {
    _ = dotenvy::dotenv();

    let Ok(yaml_file) = File::open("input.yaml") else {
        println!("error: input.yaml not found");
        return;
    };

    let Ok(yaml) = serde_yml::from_reader::<BufReader<File>, InputFile>(BufReader::new(yaml_file))
    else {
        println!("error: invalid input.yaml");
        return;
    };

    let tests = yaml.tests;

    let Ok(compile_command) = env::var("TESTER_COMPILE_COMMAND") else {
        println!("error: compile command not set");
        return;
    };

    let compile_command = compile_command.split(" ").collect::<Vec<_>>();

    let Some((cmd, args)) = compile_command.split_first() else {
        println!("error: compile command is empty");
        return;
    };

    let mut cmd = Command::new(cmd);
    cmd.args(args);

    let mut child = match cmd.spawn() {
        Ok(val) => val,
        Err(err) => {
            println!("error: failed to spawn compiler: {:?}", err);
            return;
        }
    };

    match child.wait().await {
        Ok(code) => {
            if !code.success() {
                if let Some(code) = code.code() {
                    println!("error: compiler exited with code {}", code);
                } else {
                    println!("error: compiler exited with failure code");
                }
                return;
            }
        }
        Err(err) => {
            println!("error: compiler returned error {:?}", err);
            return;
        }
    }

    let Ok(run_command) = env::var("TESTER_RUN_COMMAND") else {
        println!("error: run command not set");
        return;
    };

    let Ok(input_files) = env::var("TESTER_INPUT_FILES") else {
        println!("error: input files not set");
        return;
    };

    let input_files = input_files.split(";").collect::<Vec<_>>();

    let mut success_count = 0;
    let mut points = 0.;
    let tests_count = tests.len();

    for test in tests {
        if test.files_available == Some(false) {
            // rename input files
            for path in &input_files {
                let Ok(_) = std::fs::rename(path, path.to_owned().to_owned() + ".disabled") else {
                    println!("error: couldn't rename input file {}", path);
                    return;
                };
            }
        }

        let mut cmd = Command::new(&run_command);
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());

        let Ok(mut child) = cmd.spawn() else {
            println!("test {}: failed to spawn", test.name);
            break;
        };

        let Some(stdout) = child.stdout.take() else {
            println!("test {}: failed to open stdout", test.name);
            break;
        };

        let Some(mut stdin) = child.stdin.take() else {
            println!("test {}: failed to open stdin", test.name);
            break;
        };

        let mut reader = tokio::io::BufReader::new(stdout);

        // handle non zero exit codes - segfault, stack overflow
        let test_name = test.name.clone();
        tokio::spawn(async move {
            if let Ok(status) = child.wait().await {
                if !status.success() {
                    if let Some(code) = status.code() {
                        println!("test {} failed with exit code {}", test_name, code);
                    }
                }
            }
        });

        let mut buf = Vec::new();

        const LINE_READ_TIMEOUT_MS: u64 = 10;

        let mut success = true;
        let mut output_line = 0;

        'seq: for i in 0..test.sequence.len() {
            let entry = &test.sequence[i];
            if let Some(input) = &entry.input {
                // write line + \n to process stdin
                let mut input_line = input.clone();
                input_line.push('\n');
                match stdin.write(input_line.as_bytes()).await {
                    Ok(_) => (),
                    Err(err) => {
                        println!("test \"{}\" failed to accept stdin: {}", test.name, err);
                        success = false;
                        break 'seq;
                    }
                };
            } else if let Some(output) = &entry.output {
                output_line += 1;
                let result = tokio::time::timeout(
                    Duration::from_millis(LINE_READ_TIMEOUT_MS),
                    reader.read_until(b'\n', &mut buf),
                )
                .await;

                if result.is_err() && buf.is_empty() {
                    // timed out, process is waiting for next input
                    continue;
                }

                let Ok(buf_string) = str::from_utf8(&buf) else {
                    println!("test \"{}\" failed: output is not valid UTF-8", test.name);
                    success = false;
                    break 'seq;
                };

                if buf_string.trim_end() != output.trim_end() {
                    println!(
                        "test \"{}\" failed on line {}: expected line \"{}\", found \"{}\"",
                        test.name,
                        output_line,
                        output,
                        buf_string.trim_end()
                    );
                    success = false;
                    break 'seq;
                }

                buf.clear();
            }
        }
        drop(stdin);

        if test.files_available == Some(false) {
            // rename input files back
            for path in &input_files {
                let Ok(_) = std::fs::rename(path.to_owned().to_owned() + ".disabled", path) else {
                    println!("error: couldn't restore input file {}", path);
                    return;
                };
            }
        }

        if success {
            success_count += 1;
            points += test.points;
            println!("test \"{}\" success", test.name);
        }
    }

    println!("----------------");
    println!(
        "total: {} failed, {} success",
        tests_count - success_count,
        success_count
    );
    println!("points: {:.2}", points);
}
