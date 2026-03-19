mod app;
mod config;
mod game_detection;
mod optiscaler;
mod steam;
mod ui;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Custom log writer to filter out GTK layout warnings
unsafe extern "C" fn log_writer(
    level: glib::ffi::GLogLevelFlags,
    fields: *const glib::ffi::GLogField,
    n_fields: usize,
    _user_data: glib::ffi::gpointer,
) -> glib::ffi::GLogWriterOutput {
    // Filter out GTK warnings about widget min height
    if level == glib::ffi::G_LOG_LEVEL_WARNING {
        for i in 0..n_fields {
            let field = fields.add(i).read();
            if !field.key.is_null() {
                let key = std::ffi::CStr::from_ptr(field.key as *const i8);
                if key.to_bytes() == b"MESSAGE" && !field.value.is_null() {
                    let message = std::ffi::CStr::from_ptr(field.value as *const i8);
                    if let Ok(msg_str) = message.to_str() {
                        if msg_str.contains("Widget reports min height") {
                            return glib::ffi::G_LOG_WRITER_HANDLED;
                        }
                    }
                }
            }
        }
    }
    
    // Pass through to default handler for other messages
    glib::ffi::g_log_writer_default(level, fields, n_fields, _user_data)
}

fn main() -> Result<()> {
    // Suppress GTK warnings by setting log level to critical only
    // This filters out "Widget reports min height" layout warnings
    unsafe {
        glib::ffi::g_log_set_writer_func(
            Some(log_writer),
            std::ptr::null_mut(),
            None,
        );
    }
    
    // Initialize logging without timestamps
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "optiman=info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_target(false)
        )
        .init();

    // Run the GTK application
    let app = app::OptiManApp::new();
    let exit_code = app.run();
    
    std::process::exit(exit_code);
}
