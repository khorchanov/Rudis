macro_rules! parse_argument {
    ($value:expr, [$($enum:ident :: $variant:ident),+ $(,)?]) => {
        match $value {
            $(
                Literal::String(key) | Literal::BulkString(key)
                    if key == stringify!($variant) => Ok($enum::$variant),
            )+
            Literal::String(key) | Literal::BulkString(key) => {
                bail!("{key} is not a valid argument for SET command")
            }
            _ => bail!("Wrong ARGUMENT format For SET command"),
        }
    };
}