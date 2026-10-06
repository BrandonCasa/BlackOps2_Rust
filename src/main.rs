#![forbid(unsafe_code)]

fn main() -> bevy::app::AppExit {
    origins::app(std::env::args_os().skip(1)).run()
}
