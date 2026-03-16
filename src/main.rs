// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
use anyhow::{anyhow, bail, Context, Error, Result};
use network_comm::*;
use settings::*;
use slint::{Model, VecModel};
use std::net::SocketAddr;
use std::thread;
use tokio::net::TcpListener;
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app_settings = match load_settings() {
        Ok(v) => v,
        Err(e) => {
            println!("settings load failed: {e}");
            std::process::exit(1);
        },
    };
    // let app_settings = Settings::new("settings.json".into()).expect("valid settings file expected");
    let ui = AppWindow::new()?;

    // Weak handle so background thread doesn't keep UI alive forever
    let ui_weak = ui.as_weak();
    ui.set_weight(0);
    ui.set_interface_definition(
        app_settings
            .get_slint_interface_definition()
            .unwrap_or_default(),
    );
    // Start Tokio on a background thread
    let ui_weak_1 = ui_weak.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            serve(app_settings.get_connection_settings(), ui_weak_1).await;
        });
    });
    match init_gui(ui_weak) {
        Ok(_) => {
            println!("init_gui successful");
        },
        Err(e) => {
            eprintln!("init_gui failed: {e}")
        }
    }
    ui.run()
}

async fn serve(socket: SocketAddr, ui_weak: slint::Weak<AppWindow>) {
    println!("binding tcp listener to {}", socket);
    let listener = TcpListener::bind(socket).await.expect("bind failed");

    let mut packet_comm = PacketComm::new();

    loop {
        println!("waiting for socket connection");
        let (mut stream, _) = listener.accept().await.unwrap();
        println!("peer connection accepted");
        loop {
            let payload: Payload = match packet_comm.receive(&mut stream).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("error receiving packet: {}", e);
                    break;
                }
            };
            println!("Got command: {}, data: {}", payload.command, payload.data);
            // Send update to Slint UI thread
            if let Err(e) = process_gui(ui_weak.clone(), payload){
                eprintln!("process_gui failed: {}", e);
            };
        } //loop
    }
} //fn
  //

fn init_gui(ui_weak: slint::Weak<AppWindow>, ) -> Result<()> {
    let _ = slint::invoke_from_event_loop(move || {
        let ui = ui_weak.unwrap();
        let the_model_rc_messages = ui.get_list_of_main_program_messages();
            let the_model_messages = the_model_rc_messages
                .as_any()
                .downcast_ref::<VecModel<MessageRow>>()
                .expect("we set a VecModel earlier");
        the_model_messages.push(MessageRow { message: "".into(), priority: "".into(), ts: "".into() });
        the_model_messages.push(MessageRow { message: "".into(), priority: "".into(), ts: "".into() });
        the_model_messages.push(MessageRow { message: "".into(), priority: "".into(), ts: "".into() });
        the_model_messages.push(MessageRow { message: "".into(), priority: "".into(), ts: "".into() });
        
    });
    Ok(())
    
}

fn process_gui(ui_weak: slint::Weak<AppWindow>, payload: Payload) -> Result<()> {
    println!("process_gui: {:?}", payload.data);
    let closure = move || {
        let ui = ui_weak.unwrap();
        if payload.command.starts_with('A') {
            let row_number = match payload.command.chars().nth(1){
                Some(v) => {
                    let v = format!("{}", v);
                    usize::from_str_radix(&v, 10).context("payload parsing")? - 1 // starting to count from 0, whereas the row commands start at 1 (e.g. A1 for the first row)
                }
                None => {
                    bail!("invalid command");
                },
            };
            
            let the_model_rc_messages = ui.get_list_of_main_program_messages();
            let the_model_messages = the_model_rc_messages
                .as_any()
                .downcast_ref::<VecModel<MessageRow>>()
                .expect("we set a VecModel earlier");
            let mr = MessageRow { message: payload.data.into(), priority: "".into(), ts: "".into() };
            the_model_messages.set_row_data(row_number, mr);
            
            println!("the_model row count: {}", the_model_messages.row_count());
            Ok(())
        } else if payload.command.starts_with('P') {
            if let Ok(weight) = payload.data.parse::<i32>() {
                ui.set_weight(weight);
            } else {
                bail!("failed to parse weight");
            }
            Ok(())
        } else {
            bail!("unsupported gui command")
        }
    };
    let closure = move || {
        let _ = closure();
    };

    let _ = slint::invoke_from_event_loop(closure);
    Ok(())
}


fn load_settings() -> Result<Settings> {
    let settings_paths = vec![
        "settings.json", 
    ];

    for path in settings_paths.clone() {
        match Settings::new(path.into()) {
            Ok(v) => {
                println!("app_settings loaded");
                return Ok(v);
            },
            Err(e) => {
                println!("couldn't load {path}: {e}");
            }
        }
    }
    bail!(
        "failed to load settings. The following path were tried: {:?}",
        settings_paths
    );
}
