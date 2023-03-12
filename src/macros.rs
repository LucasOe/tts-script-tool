#[macro_export]
macro_rules! execute {
    ($api:ident, $($arg:tt)*) => {{
		let result = $api.execute(format!($($arg)*))?.return_value;
        serde_json::from_value(result).map_err(crate::error::Error::SerdeError)
    }}
}

#[macro_export]
macro_rules! print_info {
    ($label:expr) => {
        println!("{}", $label.green().bold());
    };
    ($label:expr, $($arg:tt)*) => {
        println!("{} {}", $label.yellow().bold(), format!($($arg)*));
    };
}
