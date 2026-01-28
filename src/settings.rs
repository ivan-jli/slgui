use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use slint::Color;
use std::{
    fs::File,
    net::{SocketAddr, SocketAddrV4},
    path::PathBuf,
};

use crate::InterfaceDefinitionSlint;

pub struct Settings {
    settings: JsonSettings,
}

impl Settings {
    pub fn new(file_path: PathBuf) -> Result<Self> {
        return if let Ok(f) = File::open(&file_path) {
            match serde_json::from_reader(f) {
                Ok(v) => Ok(Self { settings: v }),
                Err(e) => Err(anyhow!("failed to deserialize settings file: {}", e)),
            }
        } else {
            if let Ok(f) = File::create(&file_path) {
                let empty_settings = JsonSettings::default();
                serde_json::to_writer_pretty(f, &empty_settings)?;
                bail!("new settings file successfully created {}. Fill them in and restart the application", &file_path.to_string_lossy());
            } else {
                bail!("failed to create empty settings");
            }
        };
    }
    pub fn get_slint_interface_definition(&self) -> Result<InterfaceDefinitionSlint> {
        let result = self.settings.interface_definition.clone().try_into();
        if result.is_err() {
            println!("failed to get slint interface definition");
        }
        result
    }

    pub fn get_connection_settings(&self) -> SocketAddr {
        self.settings.connection_settings
    }
}

#[derive(Deserialize, Serialize)]
struct JsonSettings {
    version: String,
    connection_settings: SocketAddr,
    interface_definition: InterfaceDefinition,
}

impl Default for JsonSettings {
    fn default() -> Self {
        Self {
            version: "0.1.0".into(),
            connection_settings: SocketAddr::V4(SocketAddrV4::new(
                "0.0.0.0".parse().unwrap(),
                8268,
            )),
            interface_definition: InterfaceDefinition {
                bg_color: "#333333".into(),
                text_color_zone_1: "#BBBBBB".into(),
                text_color_zone_2: "#AAFFAA".into(),
            },
        }
    }
}

// #[derive(Deserialize, Serialize)]
// struct ConnectionSettings {

//     ip: IpAddr,
//     port: u16,
// }

#[derive(Deserialize, Serialize, Clone)]
struct InterfaceDefinition {
    bg_color: String,
    text_color_zone_1: String,
    text_color_zone_2: String,
}

impl TryFrom<InterfaceDefinition> for InterfaceDefinitionSlint {
    type Error = anyhow::Error;
    fn try_from(value: InterfaceDefinition) -> Result<Self> {
        Ok(Self {
            text_color_zone_1: convert_string_rgb_color(&value.text_color_zone_1)?,
            bg_color: convert_string_rgb_color(&value.bg_color)?,
            text_color_zone_2: convert_string_rgb_color(&value.text_color_zone_2)?,
        })
    }
}

fn convert_string_rgb_color(input: &str) -> Result<Color> {
    if input.len() == 7 && input.starts_with('#') {
        let r = u8::from_str_radix(&input[1..3], 16);
        let g = u8::from_str_radix(&input[3..5], 16);
        let b = u8::from_str_radix(&input[5..7], 16);
        if r.is_err() || g.is_err() || b.is_err() {
            bail!("color conversion faield")
        } else {
            return Ok(Color::from_rgb_u8(r.unwrap(), g.unwrap(), b.unwrap()));
        }
    }
    bail!("color conversion failed, expected format: #FFFFFF")
}
