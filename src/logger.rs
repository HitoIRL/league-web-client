use chrono::Local;
use fern::colors::{ColoredLevelConfig, Color};
use log::LevelFilter;

pub fn setup() -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .debug(Color::BrightBlue)
        .warn(Color::Yellow)
        .error(Color::Red);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            let date = Local::now();

            out.finish(format_args!(
                "{}[{} {} {}] {}\x1B[0m",
                format_args!(
                    "\x1B[{}m",
                    colors.get_color(&record.level()).to_fg_str()
                ),
                date.format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message,
            ))
        })
        .level(LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;

    Ok(())
}