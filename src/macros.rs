#[macro_export]
macro_rules! execute {
    ($api:ident, $($arg:tt)*) => {{
		let result = $api.execute(format!($($arg)*))?.return_value;
        serde_json::from_value(result).map_err(Error::SerdeError)
    }}
}
