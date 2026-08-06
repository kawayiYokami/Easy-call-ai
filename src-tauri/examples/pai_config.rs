use easy_call_ai::pai_config_tool;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match pai_config_tool::run_cli(&args) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
