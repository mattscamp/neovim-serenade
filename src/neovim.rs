use neovim_lib::{Neovim, NeovimApi};

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

enum NeoVimMessages {
    SerenadeStart,
    SerenadeStop,
    Unknown(String),
}

impl From<String> for NeoVimMessages {
    fn from(event: String) -> Self {
        match &event[..] {
            "serenade_start" => NeoVimMessages::SerenadeStart,
            "serenade_stop" => NeoVimMessages::SerenadeStop,
            _ => NeoVimMessages::Unknown(event),
        }
    }
}

/// EventHandler receives RPC requests, and maps them to right Serenade and Neovim commands.
pub struct NVimEventHandler {
    nvim: Arc<Mutex<Neovim>>,
    tx: Sender<String>,
}

impl NVimEventHandler {
    pub fn new(nvim: Arc<Mutex<Neovim>>, tx: Sender<String>) -> NVimEventHandler {
        NVimEventHandler { nvim, tx }
    }

    pub fn handle_events(&mut self) {
        let receiver = self.nvim.lock().unwrap().session.start_event_loop_channel();
        for (event, _values) in receiver {
            match NeoVimMessages::from(event) {
                NeoVimMessages::SerenadeStart => self.tx.send("start".to_string()).unwrap(),
                NeoVimMessages::SerenadeStop => self.tx.send("stop".to_string()).unwrap(),
                NeoVimMessages::Unknown(ev) => {
                    self.nvim
                        .lock()
                        .unwrap()
                        .command(&format!("echoerr \"{}\" Unknown command", ev))
                        .unwrap();
                }
            }
        }
    }
}
