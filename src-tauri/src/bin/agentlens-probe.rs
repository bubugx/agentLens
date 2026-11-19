fn main() {
    let path = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .expect("Pass an isolated data directory for this read-only probe");
    match agentlens_lib::probe(&path) {
        Ok(data) => println!("{}", serde_json::to_string_pretty(&data).unwrap()),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1)
        }
    }
}
