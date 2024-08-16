use std::io::Write;
use std::{env, process};

use log::LevelFilter;

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
        sox::init::run_file(args.get(1).unwrap().to_string());
    } else {
        sox::init::run_prompt();
    }
}
