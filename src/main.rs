
use std::{env, fs, process};
use std::io;
use std::io::Write;

use log::{info, LevelFilter};
use sox::core::SoxObj;

use sox::int::SoxInt;
use sox::payload;

fn main() {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Debug)
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        println!("Usage: sox [script]");
        // 64 is the exit code used when args passed to a script are incorrect
        process::exit(64);
    } else if args.len() == 2 {
        run_file(args.get(1).unwrap().to_string());
    } else {
        run_prompt();
    }
}

fn run_file(file_path: String) {
    let contents =
        fs::read_to_string(file_path).expect("Failed to read content of provided file path");
    run(contents)
}

fn run_prompt() {
    let stdin = io::stdin();
    println!("Welcome to sox");

    loop {
        print!("> ");
        let _ = io::stdout().flush();
        let mut buffer = String::new();
        stdin.read_line(&mut buffer).unwrap();
        if buffer.is_empty() {
            break;
        }
        run(buffer);
    }
}

fn run(source: String) {
    info!("Running interpreter...");
    let v = SoxInt::new(10).into_sox_obj();
    let val = payload!(v, SoxObj::Int).unwrap();
    info!("value is {:?}", val);
}
