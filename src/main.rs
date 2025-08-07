mod registry_helpers;

use std::io::ErrorKind;
use clap::{Parser, Subcommand};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;
use crate::registry_helpers::{create_raw_value_from_json, parse_raw_value};

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "List available game IDs")]
    Games,
    #[command(about = "Run FPS unlocking for provided gameID")]
    Run {
        game_id: String,
        target_fps: u32,
        refresh_delay: u32,
    }
}

#[derive(Parser, Debug)]
#[command(name = "keqing_unlock")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    match args.command {
        Some(Commands::Games) => {
            println!(r#"Available game IDs:
 - GenshinImpact = hk4e_global
 - Honkai: StarRail = hkrpg_global
 - HonkaiImpact 3rd = bh3_global
 - WutheringWaves = wuwa_global
            "#);
        }
        Some(Commands::Run { game_id, target_fps, refresh_delay }) => {
            match game_id.as_str() {
                "hk4e_global" => {
                    println!("hk4e");
                }
                "hkrpg_global" => {
                    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
                    let hive = hkcu.open_subkey_with_flags("Software\\Cognosphere\\Star Rail", KEY_READ).map_err(|e| match e.kind() {
                        ErrorKind::NotFound => "Registry container not found!",
                        ErrorKind::PermissionDenied => "Permission denied!",
                        _ => {"Something catastrophic happened!"}
                    });
                    let hivew = hkcu.open_subkey_with_flags("Software\\Cognosphere\\Star Rail", KEY_SET_VALUE).map_err(|e| match e.kind() {
                        ErrorKind::NotFound => "Registry container not found!",
                        ErrorKind::PermissionDenied => "Permission denied!",
                        _ => {"Something catastrophic happened!"}
                    });

                    if let Ok(key) = hive {
                        let available: Vec<String> = key.enum_values().filter_map(|result| result.ok().map(|(name, _)| name)).collect();
                        let setting = "GraphicsSettings_Model";
                        let r = find_matching_value(&available, setting);
                        if r.is_some() {
                            let v = r.unwrap();
                            let graphics_settings = key.get_raw_value(v.clone())?;
                            let mut pretty_settings = parse_raw_value(&graphics_settings)?;
                            let fps = pretty_settings.get_mut("FPS");

                            if fps.is_none() {
                                println!("No FPS key found!");
                            } else {
                                if target_fps >= 120 { pretty_settings["FPS"] = serde_json::Value::Number(serde_json::Number::from(120)); } else { pretty_settings["FPS"] = serde_json::Value::Number(serde_json::Number::from(target_fps)); }
                                let updated = create_raw_value_from_json(&pretty_settings, &graphics_settings)?;
                                let r = hivew.unwrap().set_raw_value(v.clone(), &updated);
                                if r.is_ok() { println!("Honkai: StarRail FPS unlocked to {:?}", pretty_settings["FPS"].as_u64().unwrap()); } else { println!("Failed to unlock Honkai: StarRail FPS!")}
                            }
                        } else {
                            // TODO: Write the empty key following default values except modified fps
                            println!("No settings found!");
                        }
                    };
                }
                // nap_global does not need a case as "FPS: Unlimited" is a builtin setting
                "bh3_global" => {
                    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
                    let hive = hkcu.open_subkey_with_flags("Software\\miHoYo\\Honkai Impact 3rd", KEY_READ).map_err(|e| match e.kind() {
                        ErrorKind::NotFound => "Registry container not found!",
                        ErrorKind::PermissionDenied => "Permission denied!",
                        _ => {"Something catastrophic happened!"}
                    });
                    let hivew = hkcu.open_subkey_with_flags("Software\\miHoYo\\Honkai Impact 3rd", KEY_SET_VALUE).map_err(|e| match e.kind() {
                        ErrorKind::NotFound => "Registry container not found!",
                        ErrorKind::PermissionDenied => "Permission denied!",
                        _ => {"Something catastrophic happened!"}
                    });

                    if let Ok(key) = hive {
                        let available: Vec<String> = key.enum_values().filter_map(|result| result.ok().map(|(name, _)| name)).collect();
                        let setting = "PersonalGraphicsSettingV2";
                        let r = find_matching_value(&available, setting);
                        if r.is_some() {
                            let v = r.unwrap();
                            let graphics_settings = key.get_raw_value(v.clone())?;
                            let mut pretty_settings = parse_raw_value(&graphics_settings)?;
                            let mut pretty_settings1 = pretty_settings.clone();
                            let fps2 = pretty_settings.get_mut("TargetFrameRateForInLevel");
                            let fps1 = pretty_settings1.get_mut("TargetFrameRateForOthers");

                            if fps2.is_none() || fps1.is_none() {
                                println!("No TargetFrameRateForInLevel or TargetFrameRateForOthers key found!");
                            } else {
                                // Fallback to 120
                                if target_fps >= 300 { pretty_settings["TargetFrameRateForInLevel"] = serde_json::Value::Number(serde_json::Number::from(120)); } else { pretty_settings["TargetFrameRateForInLevel"] = serde_json::Value::Number(serde_json::Number::from(target_fps)); }
                                if target_fps >= 300 { pretty_settings["TargetFrameRateForOthers"] = serde_json::Value::Number(serde_json::Number::from(120)); } else { pretty_settings["TargetFrameRateForOthers"] = serde_json::Value::Number(serde_json::Number::from(target_fps)); }
                                let updated = create_raw_value_from_json(&pretty_settings, &graphics_settings)?;
                                let r = hivew.unwrap().set_raw_value(v.clone(), &updated);
                                if r.is_ok() { println!("HonkaiImpact 3rd FPS unlocked to {:?}", pretty_settings["TargetFrameRateForInLevel"].as_u64().unwrap()); } else { println!("Failed to unlock HonkaiImpact 3rd FPS!")}
                            }
                        } else {
                            // TODO: Write the empty key following default values except modified fps
                            println!("No settings found!");
                        }
                    };
                }
                "wuwa_global" => {
                    println!("wuwa");
                }
                // pgr_global does not need a case as game is too obscure to find anything about unlocking its FPS beyond 120
                &_ => { eprintln!("GameID not recognized! Use --help for help."); }
            }
        }
        _ => { eprintln!("No subcommand specified! Use --help for help."); }
    }
    Ok(())
}

fn find_matching_value(available_values: &[String], pattern: &str) -> Option<String> {
    for value in available_values {
        let value_lower = value.to_lowercase();
        let pattern_lower = pattern.to_lowercase();
        if matches_pattern(&value_lower, &pattern_lower) { return Some(value.clone()); }
    }
    None
}

fn matches_pattern(value: &str, pattern: &str) -> bool {
    if value == pattern { return true; }
    if value.starts_with(pattern) { return true; }
    if let Some(base_pattern) = pattern.strip_suffix("_h") {
        if let Some(pattern_index) = value.find(base_pattern) {
            let after_pattern = &value[pattern_index + base_pattern.len()..];
            if let Some(after_h) = after_pattern.strip_prefix("_h") {
                if after_h.chars().all(|c| c.is_ascii_digit()) { return true; }
            }
        }
    }
    if value.contains(pattern) && pattern.len() > 5 { return true; }
    false
}
