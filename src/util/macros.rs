

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { print!("debug: "); println!($($arg)*) }
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {  }
}

#[macro_export]
macro_rules! lock {
    ($mute:expr, $name:ident, $body:expr) => {{
        #[allow(unused_mut)]
        let mut $name = ($mute).lock().unwrap();
        $body
    }}
}

#[macro_export]
macro_rules! read_lock {
    ($mute:expr, $name:ident, $body:expr) => {{
        let $name = ($mute).read().unwrap();
        $body
    }}
}

#[macro_export]
macro_rules! write_lock {
    ($mute:expr, $name:ident, $body:expr) => {{
        let mut $name = ($mute).write().unwrap();
        $body
    }}
}