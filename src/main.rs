pub mod neovim;
mod serenade;

use log::{debug, LevelFilter};
use log4rs::{
    append::{
        console::{ConsoleAppender, Target},
        file::FileAppender,
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};
use std::env;
use std::sync::mpsc::channel;
use std::{thread, time};

use neovim_lib::{Neovim, NeovimApi, Session};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

fn init(tx: Sender<String>, rx: Receiver<String>) {
    let mut session = Session::new_parent().unwrap();
    session.set_infinity_timeout();

    let neovim = Neovim::new(session);

    let nvim_instance = Arc::new(Mutex::new(neovim));

    let mut nvim = neovim::NVimEventHandler::new(Arc::clone(&nvim_instance), tx);
    let mut serenade = serenade::SerenadeEventHandler::new(Arc::clone(&nvim_instance), rx);

    let serenade_thread = thread::spawn(move || serenade.handle_events());
    let nvim_thread = thread::spawn(move || nvim.handle_events());

    serenade_thread
        .join()
        .expect("The serenade thread has panicked");
    nvim_thread.join().expect("The neovim thread has panicked");
}

fn main() {
    let log_path = match env::var_os("NVIM_SERENADE_LOG_FILE") {
        Some(v) => format!("{:?}", v),
        None => "/tmp/neovim-serenade.log".to_string(),
    };

    // Build a stderr logger.
    let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

    // Logging to log file.
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(log_path)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(log::LevelFilter::Error)))
                .build("stderr", Box::new(stderr)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .appender("stderr")
                .build(LevelFilter::Warn),
        )
        .unwrap();

    let _handle = log4rs::init_config(config);

    let (tx, rx) = channel();

    init(tx, rx);
}
