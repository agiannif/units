use colored::Colorize;

const LOG_LEVEL_WIDTH: usize = 7;

pub fn info(msg: &str) {
    let level = format!("{:<width$}", "info", width = LOG_LEVEL_WIDTH)
        .blue()
        .bold();
    println!("{level} {msg}");
}

pub fn success(msg: &str) {
    let level = format!("{:<width$}", "success", width = LOG_LEVEL_WIDTH)
        .green()
        .bold();
    println!("{level} {msg}");
}

pub fn warn(msg: &str) {
    let level = format!("{:<width$}", "warning", width = LOG_LEVEL_WIDTH)
        .yellow()
        .bold();
    println!("{level} {msg}");
}

pub fn error(msg: &str) {
    let level = format!("{:<width$}", "error", width = LOG_LEVEL_WIDTH)
        .red()
        .bold();
    println!("{level} {msg}");
}
