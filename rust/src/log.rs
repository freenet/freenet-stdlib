#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(not(feature = "contract"))]
        tracing::info!($($arg)*);
        #[cfg(feature = "contract")]
        info(&format!($($arg)*));
    };
}

pub fn info(msg: &str) {
    let ptr = msg.as_ptr() as _;
    unsafe {
        __frnt__logger__info(crate::global::INSTANCE_ID, ptr, msg.len() as _);
    }
}


#[link(wasm_import_module = "freenet_log")]
extern "C" {
    #[doc(hidden)]
    fn __frnt__logger__info(id: i64, ptr: i64, len: i32);
}

#[test]
fn log_non_contract() {
    use tracing::level_filters::LevelFilter;

    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(LevelFilter::INFO)
        .init();
    info!("n={}, y={:?}", 1, 2);
    info!("zk");
}
