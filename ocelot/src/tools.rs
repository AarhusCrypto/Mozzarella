use git_version::git_version;
use serde::Serialize;
use std::fs::{read_to_string, File};
use std::io::{BufRead, BufReader};
use std::process;

#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkMetaData {
    pub hostname: String,
    pub username: String,
    pub timestamp: String,
    pub cmdline: Vec<String>,
    pub pid: u32,
    pub git_version: String,
}

impl BenchmarkMetaData {
    pub fn collect() -> Self {
        BenchmarkMetaData {
            hostname: get_hostname(),
            username: get_username(),
            timestamp: get_timestamp(),
            cmdline: get_cmdline(),
            pid: get_pid(),
            git_version: git_version!(args = ["--abbrev=40", "--always", "--dirty"]).to_string(),
        }
    }
}

pub fn run_command_with_args(cmd: &str, args: &[&str]) -> String {
    String::from_utf8(
        process::Command::new(cmd)
            .args(args)
            .output()
            .expect("process failed")
            .stdout,
    )
    .expect("utf-8 decoding failed")
    .trim()
    .to_string()
}

pub fn run_command(cmd: &str) -> String {
    String::from_utf8(
        process::Command::new(cmd)
            .output()
            .expect("process failed")
            .stdout,
    )
    .expect("utf-8 decoding failed")
    .trim()
    .to_string()
}

pub fn read_file(path: &str) -> String {
    read_to_string(path).expect("read_to_string failed")
}

pub fn get_username() -> String {
    run_command("whoami")
}

pub fn get_hostname() -> String {
    read_file("/proc/sys/kernel/hostname").trim().to_string()
}

pub fn get_timestamp() -> String {
    run_command_with_args("date", &["--iso-8601=s"])
}

pub fn get_cmdline() -> Vec<String> {
    let f = File::open("/proc/self/cmdline").expect("cannot open file");
    let mut reader = BufReader::new(f);
    let mut cmdline: Vec<String> = Vec::new();
    loop {
        let mut bytes = Vec::<u8>::new();
        let num_bytes = reader.read_until(0, &mut bytes).expect("read failed");
        if num_bytes == 0 {
            break;
        }
        bytes.pop();
        cmdline.push(String::from_utf8(bytes).expect("utf-8 decoding failed"))
    }
    cmdline
}

pub fn get_pid() -> u32 {
    process::id()
}
