#![forbid(unsafe_code)]

fn main() -> bevy::app::AppExit {
    match origins::flythrough::app(std::env::args_os().skip(1)) {
        Ok(mut app) => app.run(),
        Err(error) => {
            eprintln!("{error}");
            bevy::app::AppExit::error()
        }
    }
}
