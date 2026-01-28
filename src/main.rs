// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
use settings::*;

use slint::{Model, VecModel};
use std::net::SocketAddr;
use std::rc::Rc;
use std::thread;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app_settings = Settings::new("settings.json".into()).expect("the settings shall load ok");
    let ui = AppWindow::new()?;

    // Weak handle so background thread doesn't keep UI alive forever
    let ui_weak = ui.as_weak();
    ui.set_weight(0);
    ui.set_interface_definition(
        app_settings
            .get_slint_interface_definition()
            .unwrap_or_default(),
    );
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_weight(ui.get_weight() + 100);
        }
    });
    ui.on_request_set_value({
        let ui_handle = ui.as_weak();
        move |value| {
            let ui = ui_handle.unwrap();
            ui.set_weight(value);
        }
    });
    // Start Tokio on a background thread
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            serve(app_settings.get_connection_settings(), ui_weak).await;
        });
    });

    ui.run()
}

async fn serve(socket: SocketAddr, ui_weak: slint::Weak<AppWindow>) {
    println!("binding tcp listener to {}", socket);
    let listener = TcpListener::bind(socket).await.expect("bind failed");

    loop {
        println!("waiting for socket connection");
        let (mut socket, _) = listener.accept().await.unwrap();
        loop {
            println!("peer connection accepted");
            let mut buf = [0u8; 1024];
            let n = socket.read(&mut buf).await.unwrap();
            if n == 0 {
                println!("read 0 bytes. Peer disconnected");
                break;
            }
            let text = String::from_utf8_lossy(&buf[..n]).to_string();

            let ui_weak = ui_weak.clone();
            // Send update to Slint UI thread
            let _ = slint::invoke_from_event_loop(move || {
                let ui = ui_weak.unwrap();
                let the_model_rc = ui.get_list_of_main_program_messages();
                let the_model = the_model_rc
                    .as_any()
                    .downcast_ref::<VecModel<MessageRow>>()
                    .expect("we set a VecModel earlier");
                the_model.push(MessageRow {
                    message: text.into(),
                    priority: "high".into(),
                    ts: "000000".into(),
                });
                println!("the_model row count: {}", the_model.row_count());
            });
        } //loop
    }
} //fn
