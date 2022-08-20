#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use tauri::{ CustomMenuItem, SystemTray, SystemTrayMenu, SystemTrayMenuItem, SystemTrayEvent };
use tauri::api::notification::Notification;
use tauri::Manager;

// use std::collections::HashMap;

use std::thread;

use std::sync::{ Arc, Mutex };

use once_cell::sync::Lazy;

static MACROS: Lazy<Mutex<Macros>> = Lazy::new(|| Mutex::new(Macros::new()));

fn get_macros() -> Macros {
    MACROS.lock().unwrap().clone()
}

fn set_macros(macros: Macros) {
    *MACROS.lock().unwrap() = macros;
}

fn main() {
    thread::spawn(move || {
        listen_macro_actions();
    });

    tauri::Builder
        ::default()
        .plugin(
            tauri_plugin_single_instance::init(|app, argv, cwd| {
                println!("{}, {argv:?}, {cwd}", app.package_info().name);
                let window = app.get_window("main").unwrap();
                window.show().unwrap();
            })
        )
        .invoke_handler(tauri::generate_handler![update_macros])
        .system_tray(
            SystemTray::new().with_menu(
                SystemTrayMenu::new()
                    .add_item(CustomMenuItem::new("open".to_string(), "Open"))
                    .add_native_item(SystemTrayMenuItem::Separator)
                    .add_item(CustomMenuItem::new("quit".to_string(), "Quit"))
            )
        )
        .on_system_tray_event(|app, event| {
            match event {
                SystemTrayEvent::MenuItemClick { id, .. } => {
                    // let item_handle = app.tray_handle().get_item(&id);
                    match id.as_str() {
                        "quit" => {
                            let window = app.get_window("main").unwrap();
                            window.close().unwrap();
                        }
                        "open" => {
                            let window = app.get_window("main").unwrap();
                            window.show().unwrap();
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use inputbot::{ KeybdKey };

use std::collections::HashMap;

fn listen_macro_actions() {
    let keys_pressed: Arc<Mutex<HashMap<KeybdKey, bool>>> = Arc::new(Mutex::new(HashMap::new()));
    KeybdKey::bind_all(move |event| {
        let mut keys_pressed = keys_pressed.lock().unwrap();

        keys_pressed.insert(event, true);

        // Loop through all keys and check if they are pressed. If not, remove them from the map.
        let mut remove = Vec::new();
        for key in keys_pressed.keys() {
            if !KeybdKey::is_pressed(*key) {
                remove.push(*key);
            }
        }
        for key in remove {
            keys_pressed.remove(&key);
        }

        let mut keys_pressed_js: Vec<String> = vec![];
        for (key, value) in keys_pressed.iter() {
            if *value {
                keys_pressed_js.push(js_key(*key));
            }
        }

        'macros: for macro_ in get_macros() {
            // Check if macro_.macro_.initiators is Some
            if macro_.macro_.initiators.is_some() {
                let initiators = macro_.macro_.initiators.as_ref().unwrap();
                // Check if the initiators are pressed
                for initiator in initiators {
                    if initiator.type_ == "keypress" {
                        let keys = initiator.data.keys.as_ref().unwrap();
                        for key in keys {
                            if !keys_pressed_js.contains(key) {
                                continue 'macros;
                            }
                        }
                        run_macro_initiator(&initiator, &macro_);
                    }
                }
            }
        }
    });

    // Call this to start listening for bound inputs.
    inputbot::handle_input_events();
}

fn run_macro_initiator(initiator: &Initiator, macro_: &Macro) {
    println!("Running macro initiator from macro \"{}\"", macro_.name);
    execute_macro_code(&initiator.executes);
}

fn execute_macro_code(code: &Vec<Execution>) {
    for execution in code {
        match execution.type_.as_str() {
            "wait" => {
                let time = execution.data.time.as_ref().unwrap();
                thread::sleep(std::time::Duration::from_millis((time * 1000.0) as u64));
            }
            "notification" => {
                let title = execution.data.title.as_ref().unwrap();
                let message = execution.data.message.as_ref().unwrap();
                let _ = Notification::new("code-macros")
                    .title(title)
                    .body(message)
                    .show();
            }
            _ => todo!()
        }
    }
}

use serde::{ Deserialize, Serialize };
use serde_json::value::Value;

type Macros = Vec<Macro>;

#[derive(Serialize, Deserialize, Clone)]
struct Macro {
    name: String,
    description: String,
    macro_: MacroMacro,
}

impl std::fmt::Debug for Macro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\nName: {:?}\nDescription: {:?}\nMacro:{:?}\n",
            self.name,
            self.description,
            self.macro_
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct MacroMacro {
    initiators: Option<Vec<Initiator>>,
    functions: Option<Vec<Function>>,
}

impl std::fmt::Debug for MacroMacro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n Initiators: {:?} \n Functions: {:?}\n", self.initiators, self.functions)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Initiator {
    type_: String,
    data: InitiatorData,
    executes: Vec<Execution>,
}

impl std::fmt::Debug for Initiator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n Type: {:?} \n Data: {:?} \n Executes: {:?}",
            self.type_,
            self.data,
            self.executes
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct InitiatorData {
    keys: Option<Vec<String>>,
    activate_time: Option<String>,
    time: Option<InitiatorKeypressTime>,
    cron: Option<String>,
    app_path: Option<String>,
}

impl std::fmt::Debug for InitiatorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n Keys: {:?} \n Activate Time: {:?} \n Time: {:?} \n Cron: {:?} \n App Path: {:?}",
            self.keys,
            self.activate_time,
            self.time,
            self.cron,
            self.app_path
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct InitiatorKeypressTime {
    min: f64,
    max: f64,
}

impl std::fmt::Debug for InitiatorKeypressTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n Min: {:?} \n Max: {:?}", self.min, self.max)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Execution {
    type_: String,
    data: ExecutionData,
    code_inside: Value,
}

impl std::fmt::Debug for Execution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n Type: {:?} \n Data: {:?} \n Code inside: {:?}",
            self.type_,
            self.data,
            self.code_inside
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ExecutionData {
    time: Option<f64>,
    title: Option<String>,
    message: Option<String>
}

impl std::fmt::Debug for ExecutionData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n Time: {:?} \n Title: {:?} \n Message: {:?}",
            self.time,
            self.title,
            self.message
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Function {
    name: String,
    parameters: Vec<Parameter>,
    executes: Vec<Execution>,
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n Name: {:?} \n Parameters: {:?} \n Executes: {:?}",
            self.name,
            self.parameters,
            self.executes
        )
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Parameter {
    name: String,
    type_: String,
    default_value: String,
}

impl std::fmt::Debug for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n Name: {:?} \n Type: {:?} \n Default value: {:?}",
            self.name,
            self.type_,
            self.default_value
        )
    }
}

#[tauri::command]
fn update_macros(macros: Macros) {
    set_macros(macros);
}

fn print_macros(macros: Macros) {
    for macro_ in macros {
        println!("{}", macro_.name);
    }
}

// Turn the key enum into the same format as comes from the macro config
fn js_key(key: KeybdKey) -> String {
    let mut enum_to_key:HashMap<String, String> = HashMap::new();
    enum_to_key.insert("AKey".to_string(),         "a".to_string()        );
    enum_to_key.insert("BKey".to_string(),         "b".to_string()        );
    enum_to_key.insert("CKey".to_string(),         "c".to_string()        );
    enum_to_key.insert("DKey".to_string(),         "d".to_string()        );
    enum_to_key.insert("EKey".to_string(),         "e".to_string()        );
    enum_to_key.insert("FKey".to_string(),         "f".to_string()        );
    enum_to_key.insert("GKey".to_string(),         "g".to_string()        );
    enum_to_key.insert("HKey".to_string(),         "h".to_string()        );
    enum_to_key.insert("IKey".to_string(),         "i".to_string()        );
    enum_to_key.insert("JKey".to_string(),         "j".to_string()        );
    enum_to_key.insert("KKey".to_string(),         "k".to_string()        );
    enum_to_key.insert("LKey".to_string(),         "l".to_string()        );
    enum_to_key.insert("MKey".to_string(),         "m".to_string()        );
    enum_to_key.insert("NKey".to_string(),         "n".to_string()        );
    enum_to_key.insert("OKey".to_string(),         "o".to_string()        );
    enum_to_key.insert("PKey".to_string(),         "p".to_string()        );
    enum_to_key.insert("QKey".to_string(),         "q".to_string()        );
    enum_to_key.insert("RKey".to_string(),         "r".to_string()        );
    enum_to_key.insert("SKey".to_string(),         "s".to_string()        );
    enum_to_key.insert("TKey".to_string(),         "t".to_string()        );
    enum_to_key.insert("UKey".to_string(),         "u".to_string()        );
    enum_to_key.insert("VKey".to_string(),         "v".to_string()        );
    enum_to_key.insert("WKey".to_string(),         "w".to_string()        );
    enum_to_key.insert("XKey".to_string(),         "x".to_string()        );
    enum_to_key.insert("YKey".to_string(),         "y".to_string()        );
    enum_to_key.insert("ZKey".to_string(),         "z".to_string()        );
    enum_to_key.insert("Numrow0Key".to_string(),   "0".to_string()        );
    enum_to_key.insert("Numrow1Key".to_string(),   "1".to_string()        );
    enum_to_key.insert("Numrow2Key".to_string(),   "2".to_string()        );
    enum_to_key.insert("Numrow3Key".to_string(),   "3".to_string()        );
    enum_to_key.insert("Numrow4Key".to_string(),   "4".to_string()        );
    enum_to_key.insert("Numrow5Key".to_string(),   "5".to_string()        );
    enum_to_key.insert("Numrow6Key".to_string(),   "6".to_string()        );
    enum_to_key.insert("Numrow7Key".to_string(),   "7".to_string()        );
    enum_to_key.insert("Numrow8Key".to_string(),   "8".to_string()        );
    enum_to_key.insert("Numrow9Key".to_string(),   "9".to_string()        );
    enum_to_key.insert("Numpad0Key".to_string(),   "0".to_string()        );
    enum_to_key.insert("Numpad1Key".to_string(),   "1".to_string()        );
    enum_to_key.insert("Numpad2Key".to_string(),   "2".to_string()        );
    enum_to_key.insert("Numpad3Key".to_string(),   "3".to_string()        );
    enum_to_key.insert("Numpad4Key".to_string(),   "4".to_string()        );
    enum_to_key.insert("Numpad5Key".to_string(),   "5".to_string()        );
    enum_to_key.insert("Numpad6Key".to_string(),   "6".to_string()        );
    enum_to_key.insert("Numpad7Key".to_string(),   "7".to_string()        );
    enum_to_key.insert("Numpad8Key".to_string(),   "8".to_string()        );
    enum_to_key.insert("Numpad9Key".to_string(),   "9".to_string()        );
    enum_to_key.insert("BackspaceKey".to_string(), "backspace".to_string());
    enum_to_key.insert("TabKey".to_string(),       "tab".to_string()      );
    enum_to_key.insert("EnterKey".to_string(),     "enter".to_string()    );
    enum_to_key.insert("EscapeKey".to_string(),    "escape".to_string()   );
    enum_to_key.insert("SpaceKey".to_string(),     "space".to_string()    );
    enum_to_key.insert("F1Key".to_string(),        "f1".to_string()       );
    enum_to_key.insert("F2Key".to_string(),        "f2".to_string()       );
    enum_to_key.insert("F3Key".to_string(),        "f3".to_string()       );
    enum_to_key.insert("F4Key".to_string(),        "f4".to_string()       );
    enum_to_key.insert("F5Key".to_string(),        "f5".to_string()       );
    enum_to_key.insert("F6Key".to_string(),        "f6".to_string()       );
    enum_to_key.insert("F7Key".to_string(),        "f7".to_string()       );
    enum_to_key.insert("F8Key".to_string(),        "f8".to_string()       );
    enum_to_key.insert("F9Key".to_string(),        "f9".to_string()       );
    enum_to_key.insert("F10Key".to_string(),       "f10".to_string()      );
    enum_to_key.insert("CapsLockKey".to_string(),  "capslock".to_string() );
    enum_to_key.insert("QuoteKey".to_string(),     "'".to_string()        );
    enum_to_key.insert("SemicolonKey".to_string(), ";".to_string()        );
    enum_to_key.insert("CommaKey".to_string(),     ",".to_string()        );
    enum_to_key.insert("PeriodKey".to_string(),    ".".to_string()        );
    enum_to_key.insert("SlashKey".to_string(),     "/".to_string()        );
    enum_to_key.insert("BackslashKey".to_string(), "\\".to_string()       );
    enum_to_key.insert("MinusKey".to_string(),     "-".to_string()        );
    enum_to_key.insert("EqualKey".to_string(),     "=".to_string()        );
    enum_to_key.insert("LBracketKey".to_string(),  "[".to_string()        );
    enum_to_key.insert("RBracketKey".to_string(),  "]".to_string()        );
    enum_to_key.insert("BackquoteKey".to_string(), "`".to_string()        );
    enum_to_key.insert("LShiftKey".to_string(),    "shift".to_string()    );
    enum_to_key.insert("RShiftKey".to_string(),    "shift".to_string()    );
    enum_to_key.insert("LControlKey".to_string(),  "control".to_string()  );
    enum_to_key.insert("RControlKey".to_string(),  "control".to_string()  );

    let key_string = format!("{:?}", key);

    return enum_to_key.get(&key_string).unwrap().to_string();
}