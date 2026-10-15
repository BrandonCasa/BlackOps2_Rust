#![forbid(unsafe_code)]

fn main() -> bevy::app::AppExit {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    origins::app(arguments.into_iter()).run()
}
