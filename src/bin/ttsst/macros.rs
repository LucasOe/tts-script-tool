#[macro_export]
macro_rules! print_info {
    ($label:expr) => {{
        use colorize::AnsiColor;
        println!("{}", $label.green().bold());
    }};
    ($label:expr, $($arg:tt)*) => {{
        use colorize::AnsiColor;
        println!("{} {}", $label.yellow().bold(), format!($($arg)*));
    }};
}
