#![forbid(unsafe_code)]

fn main() -> bevy::app::AppExit {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.iter().any(|a| a == "--prepare-textures") {
        #[cfg(feature = "prepare-textures")]
        {
            if arguments.len() != 4
                || arguments[0] != "--prepare-textures"
                || arguments[2] != "--report"
            {
                eprintln!(
                    "Usage: origins.exe --prepare-textures <batch.json> --report <results.json>"
                );
                return bevy::app::AppExit::error();
            }
            return match origins::texture_preparation::run(
                arguments[1].as_ref(),
                arguments[3].as_ref(),
            ) {
                Ok(true) => bevy::app::AppExit::Success,
                Ok(false) => bevy::app::AppExit::error(),
                Err(error) => {
                    eprintln!("Texture preparation: {error}");
                    bevy::app::AppExit::error()
                }
            };
        }
        #[cfg(not(feature = "prepare-textures"))]
        {
            eprintln!("This executable excludes texture preparation; build with prepare-textures.");
            return bevy::app::AppExit::error();
        }
    }
    origins::app(arguments.into_iter()).run()
}
