use std::env;
use std::io::Write;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{}:{} {} [{}] - {}",
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        sox::init::run_file(args.get(1).unwrap().to_string());
    } else {
        sox::init::run_prompt();
    }
}
