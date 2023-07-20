use std::error::Error;
use env_logger::Builder;
use std::io::Write;

// Logger setup function
pub fn setup_logger() -> Result<(), Box<dyn Error + Send + Sync>> {
    Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, log::LevelFilter::Info)
        .init();
    Ok(())
}
