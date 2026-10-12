#![forbid(unsafe_code)]

fn main() -> bevy::app::AppExit {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "--flythrough")
    {
        return match origins::flythrough::app(arguments.into_iter().skip(1)) {
            Ok(mut app) => app.run(),
            Err(error) => {
                eprintln!("{error}");
                bevy::app::AppExit::error()
            }
        };
    }
    origins::app(arguments.into_iter()).run()
}
