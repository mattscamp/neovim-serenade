use log::{debug, error, info, warn};
use neovim_lib::{Neovim, NeovimApi, CallError, Value};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::net::TcpStream;
use std::{thread, time::Duration};
use tungstenite::WebSocket;
use tungstenite::{connect, Message};
use tungstenite::stream::MaybeTlsStream;
use url::Url;

const CONNECTION: &'static str = "ws://localhost:17373";

#[derive(PartialEq)]
enum SerenadeMessages {
    GetEditorState,
    Diff,
    Undo,
    Redo,
    Save,
    Select,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    SwitchTab,
    Unknown(String),
}

impl From<String> for SerenadeMessages {
    fn from(cmd: String) -> Self {
        match &cmd[..] {
            "COMMAND_TYPE_GET_EDITOR_STATE" => SerenadeMessages::GetEditorState,
            "COMMAND_TYPE_DIFF" => SerenadeMessages::Diff,
            "COMMAND_TYPE_UNDO" => SerenadeMessages::Undo,
            "COMMAND_TYPE_REDO" => SerenadeMessages::Redo,
            "COMMAND_TYPE_SELECT" => SerenadeMessages::Select,
            "COMMAND_TYPE_SAVE" => SerenadeMessages::Save,
            "COMMAND_TYPE_CREATE_TAB" => SerenadeMessages::NewTab,
            "COMMAND_TYPE_CLOSE_TAB" => SerenadeMessages::CloseTab,
            "COMMAND_TYPE_NEXT_TAB" => SerenadeMessages::NextTab,
            "COMMAND_TYPE_PREVIOUS_TAB" => SerenadeMessages::PrevTab,
            "COMMAND_TYPE_SWITCH_TAB" => SerenadeMessages::SwitchTab,
            _ => SerenadeMessages::Unknown(cmd),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct HeartbeatData {
    id: String,
    app: Option<String>,
    #[serde(rename = "match")]
    match_term: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Heartbeat {
    message: String,
    data: HeartbeatData,
}

#[derive(Deserialize, Debug)]
struct SerenadeCommand {
    #[serde(rename = "type")]
    cmd_type: String,
    source: Option<String>,
    cursor: Option<u64>,
    cursorEnd: Option<u64>,
    limited: Option<bool>,
    index: Option<u64>,
    direction: Option<String>,
}

#[derive(Deserialize, Debug)]
struct SerenadeExecute {
    commandsList: Vec<SerenadeCommand>,
    commands: Vec<SerenadeCommand>,
}

#[derive(Deserialize, Debug)]
struct SerenadeResponse {
    execute: SerenadeExecute,
}

#[derive(Deserialize, Debug)]
struct SerenadeData {
    callback: String,
    response: SerenadeResponse,
}

#[derive(Deserialize, Debug)]
struct SerenadePayload {
    message: String,
    data: SerenadeData,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerenadeStateData {
    source: String,
    cursor: u64,
    selectionStart: u64,
    selectionEnd: u64,
    filename: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerenadeEditorState {
    message: String,
    data: SerenadeStateData,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerenadeCallbackMsg {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerenadeCallbackData {
    callback: String,
    data: SerenadeCallbackMsg,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerenadeStateCallbackData {
    callback: String,
    data: SerenadeEditorState,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerenadeStateCallback {
    message: String,
    data: SerenadeStateCallbackData,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerenadeCallback {
    message: String,
    data: SerenadeCallbackData,
}

pub struct SerenadeEventHandler {
    id: u8,
    is_paused: bool,
    client: WebSocket<MaybeTlsStream<TcpStream>>,
    rx: Receiver<String>,
    nvim: Arc<Mutex<Neovim>>,
    namespace: Option<i64>,
    selected_positions: Option<Vec<(u64, u64)>>,
}

impl SerenadeEventHandler {
    pub fn new(nvim: Arc<Mutex<Neovim>>, rx: Receiver<String>) -> SerenadeEventHandler {
        let client = SerenadeEventHandler::create_client(CONNECTION);

        info!("Successfully connected");

        let mut rng = rand::thread_rng();
        let random_id: u8 = rng.gen();
        let mut namespace = None;

        match nvim.lock() {
            Ok(mut nvim) => {
                namespace = Some(
                    unwrap!(nvim.create_namespace("Serenade"))
                );
            }
            _ => error!("Unable to lock nvim for \"redo\""),
        }

        return SerenadeEventHandler {
            id: random_id,
            is_paused: false,
            client,
            rx: rx,
            nvim: nvim,
            namespace,
            selected_positions: None,
        };
    }

    fn create_client(connection: &str) -> WebSocket<MaybeTlsStream<TcpStream>> {
        info!("Connecting to: {}", CONNECTION);

        if let Ok((socket, _)) = connect(Url::parse(connection).unwrap()) {
            return socket;
        }

        thread::sleep(Duration::from_millis(1000));

        return SerenadeEventHandler::create_client(connection);
    }

    pub fn heartbeat(&mut self, initial: bool) {
        let heartbeat_data = Heartbeat {
            message: "active".to_string(),
            data: HeartbeatData {
                id: self.id.to_string(),
                app: if initial {
                    Some(String::from("nvim"))
                } else {
                    None
                },
                match_term: if initial {
                    Some(String::from("tilix"))
                } else {
                    None
                },
            },
        };

        match serde_json::to_string(&heartbeat_data) {
            Ok(v) => {
                self.client.write_message(Message::text(&v)).unwrap();
                info!("Sent heartbeat {:?}", &v);
            }
            Err(e) => {
                warn!("Could not send heartbeat {:?}", e);
            }
        }
    }

    pub fn handle_events(&mut self) {
        let start = Instant::now();

        self.heartbeat(true);

        loop {
            // every minute
            if start.elapsed().as_secs() % 60 == 0 {
                self.heartbeat(false);
            }

            match self.rx.try_recv() {
                Ok(v) => match v.as_ref() {
                    "start" => self.is_paused = false,
                    "stop" => self.is_paused = true,
                    _ => error!("Not a recognized cmd: {}", v),
                },
                _ => {
                    debug!("No rx messages");
                }
            };

            let msg = match self.client.read_message() {
                Ok(m) => m,
                _ => continue,
            };

            debug!("again");
            let raw_msg = match msg {
                Message::Text(v) => v,
                _ => continue,
            };

            info!("received message from Serenade: {:?}", &raw_msg);
            
            let payload: SerenadePayload = match serde_json::from_str(&raw_msg) {
                Ok(v) => v,
                Err(e) => {
                    error!("Unable to parse Serenade Message: {:?}", &raw_msg);
                    continue;
                },
            };

            let mut cb1 = None;
            let mut cb2 = None;

            for command in &payload.data.response.execute.commandsList {
                let cmd = SerenadeMessages::from(command.cmd_type.to_string());
                if cmd == SerenadeMessages::GetEditorState {
                    cb1 = Some(SerenadeStateCallback {
                        message: String::from("callback"),
                        data: SerenadeStateCallbackData {
                            callback: String::from(&payload.data.callback),
                            data: self.get_editor_state(command.limited.unwrap_or_else(|| true)),
                        },
                    });
                } else if !self.is_paused {
                    let success = match cmd {
                        SerenadeMessages::Diff => {
                            self.diff(command.source.as_ref(), command.cursor.as_ref())
                        }
                        SerenadeMessages::Undo => self.undo(),
                        SerenadeMessages::Redo => self.redo(),
                        SerenadeMessages::Save => self.save(),
                        SerenadeMessages::Select => self.select(
                            command.cursor.unwrap_or_else(|| 0),
                            command.cursorEnd.unwrap_or_else(|| 0),
                        ),
                        SerenadeMessages::NewTab => self.create_buffer(),
                        SerenadeMessages::CloseTab => self.close_buffer(),
                        SerenadeMessages::NextTab => self.next_buffer(),
                        SerenadeMessages::PrevTab => self.prev_buffer(),
                        SerenadeMessages::SwitchTab => self.switch_buffer(command.index.unwrap_or_else(|| 0)),
                        _ => Ok(false)
                    };
                
                    if let Ok(passed) = success {
                        if passed == true {
                            cb2 = Some(SerenadeCallback {
                                message: String::from("callback"),
                                data: SerenadeCallbackData {
                                    callback: String::from(&payload.data.callback),
                                    data: SerenadeCallbackMsg {
                                        message: String::from("completed"),
                                    },
                                },
                            });
                        }
                    } 
                }

                info!("{}", command.cmd_type);
            }

            let mut cb_serialized = None;

            if cb1.is_some() {
                cb_serialized = Some(serde_json::to_string(&cb1.unwrap()).unwrap());
            } else if cb2.is_some() {
                cb_serialized = Some(serde_json::to_string(&cb2.unwrap()).unwrap());
            } else {
                continue;
            }

            debug!("writing to websocket: {:?}", cb_serialized);

            self.client
                .write_message(Message::text(cb_serialized.unwrap()))
                .unwrap();

            thread::sleep(Duration::from_millis(50));
        }
    }

    fn get_editor_state(&mut self, limited: bool) -> SerenadeEditorState {
        let mut result = SerenadeEditorState {
            message: String::from("editorState"),
            data: SerenadeStateData {
                source: String::from(""),
                cursor: 0,
                selectionStart: 0,
                selectionEnd: 0,
                filename: String::from(""),
            },
        };

        match self.nvim.lock() {
            Ok(mut nvim) => {
                let buffer = match nvim.get_current_buf() {
                    Ok(v) => v,
                    _ => {
                        error!("Could not get current buffer");
                        return result;
                    }
                };
                let full_file_name = buffer.get_name(&mut nvim).unwrap();
                let file_name_pieces: Vec<&str> = full_file_name.split('/').collect();
                let file_name = file_name_pieces[file_name_pieces.len() - 1];

                result.data.filename = String::from(file_name);
                if limited != true {
                    let window = nvim.get_current_win().unwrap();
                    let lines = buffer.get_lines(&mut nvim, 0, -1, false).unwrap();
                    let cursor = window.get_cursor(&mut nvim).unwrap();
                    let mark_start = buffer.get_mark(&mut nvim, "<").unwrap();
                    let mark_end = buffer.get_mark(&mut nvim, ">").unwrap();
                    result.data.source = lines.join("\n");
                    result.data.cursor =
                        SerenadeEventHandler::get_cursor_position(&result.data.source, cursor);
                    result.data.selectionStart =
                        SerenadeEventHandler::get_cursor_position(&result.data.source, mark_start);
                    result.data.selectionEnd =
                        SerenadeEventHandler::get_cursor_position(&result.data.source, mark_end);
                }
            }
            _ => error!("Unable to lock nvim for \"get editor state\""),
        }

        return result;
    }

    fn undo(&mut self) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(":undo")?;
                return Ok(true);
            },
            _ => {
                error!("Unable to lock nvim for \"undo\"");
                return Ok(false);
            },
        }
    }

    fn redo(&mut self) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(":redo")?;
                return Ok(true);
            },
            _ => {
                error!("Unable to lock nvim for \"redo\"");
                return Ok(false);
            },
        }
    }

    fn save(&mut self) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(":w")?;
                return Ok(true);
            },
            _ => {
                error!("Unable to lock nvim for \"save\"");
                return Ok(false);
            },
        }
    }

    fn select(&mut self, start: u64, end: u64) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => { 
                let buffer = nvim.get_current_buf()?;
                let lines = buffer.get_lines(&mut nvim, 0, -1, false)?;
                let source = lines.join("\n");
                let start_mark = SerenadeEventHandler::get_cursor_position_rev(&source, &start);
                let end_mark = SerenadeEventHandler::get_cursor_position_rev(&source, &end);
                
                nvim.command(&format!(":cal cursor({}, {})", start_mark.0, start_mark.1))?;
                
                if let Some(namespace) = self.namespace {
                    buffer.set_extmark(&mut nvim, namespace, (start_mark.0 -  1) as i64, (start_mark.1 - 1) as i64, vec![(Value::from("end_line"), Value::from(end_mark.0 - 1)), (Value::from("end_col"), Value::from(end_mark.1 - 1)), (Value::from("hl_group"), Value::from("Visual"))])?;
                }
                
                return Ok(true);
            },
            _ => {
                error!("Unable to lock nvim for \"select\"");
                return Ok(false)
            }
        }
    }

    fn switch_buffer(&mut self, index: u64) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(&format!(":b {}", index))?;
                return Ok(true);
            },
            _ => {
                error!("Unable to lock nvim for \"switch tab\"");
                return Ok(false);
            }
        }
    }

    fn close_buffer(&mut self) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(":bd")?;
                return Ok(true);
            }
            _ => {
                error!("Unable to lock nvim for \"close tab\"");
                return Ok(false);
            }
        }
    }

    fn create_buffer(&mut self) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(":enew")?;
                return Ok(true);
            }
            _ => {
                error!("Unable to lock nvim for \"create tab\"");
                return Ok(false);
            }
        }
    }

    fn next_buffer(&mut self) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(":bnext")?;
                return Ok(true);
            },
            _ => {
                error!("Unable to lock nvim for \"next tab\"");
                return Ok(false);
            }
        }
    }

    fn prev_buffer(&mut self) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                nvim.command(":bprevious")?;
                return Ok(true);
            }
            _ => {
                error!("Unable to lock nvim for \"previous tab\"");
                return Ok(false);
            }
        }
    }

    fn get_cursor_position(source: &str, cursor: (i64, i64)) -> u64 {
        let mut line_num = 1;
        let mut pos = 0;

        for chr in source.chars() {
            if chr == '\n' {
                line_num += 1;
            }
            pos += 1;
            if line_num == cursor.0 {
                break;
            }
        }

        return (pos + cursor.1) as u64;
    }

    fn get_cursor_position_rev(source: &str, cursor: &u64) -> (u64, u64) {
        let mut line_num: u64 = 1;
        let mut column: u64 = 0;
        let mut pos: u64 = 0;

        for chr in source.chars() {
            if *cursor > pos {
                if chr == '\n' {
                    line_num += 1;
                    column = 0;
                }
                pos += 1;
                column += 1;
            } else {
                break;
            }
        }

        return (line_num, column);
    }

    fn diff(&mut self, source: Option<&String>, cursor: Option<&u64>) -> Result<bool, CallError> {
        match self.nvim.lock() {
            Ok(mut nvim) => {
                let buffer = nvim.get_current_buf()?;
                let window = nvim.get_current_win()?;
                let cursor_pos = SerenadeEventHandler::get_cursor_position_rev(
                    &source.unwrap(),
                    cursor.unwrap(),
                );
                let lines: Vec<String> = source.unwrap().lines().map(|s| s.to_string()).collect();

                buffer.set_lines(&mut nvim, 0, -1, false, lines)?;
                window.set_cursor(&mut nvim, (cursor_pos.0 as i64, cursor_pos.1 as i64))?;
                    
                return Ok(true);
            }
            _ => {
                error!("Unable to lock nvim for \"diff\"");
                return Ok(false);
            }
        }
    }
}
