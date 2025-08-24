mod registry_helpers;
mod hk4e_helpers;

use std::collections::HashMap;
use std::io::{ErrorKind};
use std::path::Path;
use clap::{Parser, Subcommand};
use configparser::ini::Ini;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;
use crate::registry_helpers::{create_raw_value_from_json, parse_raw_value};

#[derive(Debug, Serialize, Deserialize)]
struct MenuDataDict {
    #[serde(rename = "___MetaType___")]
    meta_type: String,
    content: Vec<(i32, f64)>
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "List available game IDs")]
    Games,
    #[command(about = "Run FPS unlocking for provided gameID")]
    Run {
        game_id: String,
        target_fps: u32,
        refresh_delay: u64,
        game_path: String
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
        Some(Commands::Run { game_id, target_fps, refresh_delay, game_path }) => {
            match game_id.as_str() {
                "hk4e_global" => unsafe {
                    /*let target = "GenshinImpact.exe";
                    let r = wait_for_handle_by_name(target);
                    let pid = get_pid_from_handle(r);
                    let (base, _size) = get_module_base(pid, target).unwrap();*/
                    println!("Genshin support Soon");
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
                                eprintln!("No FPS key found!");
                            } else {
                                if target_fps >= 120 { pretty_settings["FPS"] = serde_json::Value::Number(serde_json::Number::from(120)); } else { pretty_settings["FPS"] = serde_json::Value::Number(serde_json::Number::from(target_fps)); }
                                let updated = create_raw_value_from_json(&pretty_settings, &graphics_settings)?;
                                let r = hivew.unwrap().set_raw_value(v.clone(), &updated);
                                if r.is_ok() { println!("Honkai: StarRail FPS unlocked to {:?}", pretty_settings["FPS"].as_u64().unwrap()); } else { eprintln!("Failed to unlock Honkai: StarRail FPS!"); }
                            }
                        } else {
                            // TODO: Write the empty key following default values except modified fps
                            eprintln!("No settings found!");
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
                                eprintln!("No TargetFrameRateForInLevel or TargetFrameRateForOthers key found!");
                            } else {
                                // Fallback to 60
                                if target_fps >= 300 { pretty_settings["TargetFrameRateForInLevel"] = serde_json::Value::Number(serde_json::Number::from(60)); } else { pretty_settings["TargetFrameRateForInLevel"] = serde_json::Value::Number(serde_json::Number::from(target_fps)); }
                                if target_fps >= 300 { pretty_settings["TargetFrameRateForOthers"] = serde_json::Value::Number(serde_json::Number::from(600)); } else { pretty_settings["TargetFrameRateForOthers"] = serde_json::Value::Number(serde_json::Number::from(target_fps)); }
                                let updated = create_raw_value_from_json(&pretty_settings, &graphics_settings)?;
                                let r = hivew.unwrap().set_raw_value(v.clone(), &updated);
                                if r.is_ok() { println!("HonkaiImpact 3rd FPS unlocked to {:?}", pretty_settings["TargetFrameRateForInLevel"].as_u64().unwrap()); } else { eprintln!("Failed to unlock HonkaiImpact 3rd FPS!"); }
                            }
                        } else {
                            // TODO: Write the empty key following default values except modified fps
                            eprintln!("No settings found!");
                        }
                    };
                }
                "wuwa_global" => {
                    let localstorage = Path::new(game_path.as_str()).join("Client/Saved/LocalStorage/LocalStorage.db");
                    let gameusersettings = Path::new(game_path.as_str()).join("Client/Saved/Config/WindowsNoEditor/GameUserSettings.ini");
                    if !localstorage.exists() { eprintln!("LocalStorage does not exist!"); }
                    if !gameusersettings.exists() { eprintln!("GameUserSettings does not exist!"); }

                   let fpsv = if target_fps >= 120 { 120 } else { target_fps };

                    let mut sqlc = Connection::open(localstorage.clone()).unwrap();
                    let menu_data_dict = MenuDataDict {
                        meta_type: "___Map___".to_string(),
                        content: vec![
                            (1, 100.0), (2, 100.0), (3, 100.0), (4, 100.0), (5, 0.0), (6, 0.0),
                            (7, -0.4658685302734375), (10, 3.0), (11, 3.0), (20, 0.0), (21, 0.0),
                            (22, 0.0), (23, 0.0), (24, 0.0), (25, 0.0), (26, 0.0), (27, 0.0),
                            (28, 0.0), (29, 0.0), (30, 0.0), (31, 0.0), (32, 0.0), (33, 0.0),
                            (34, 0.0), (35, 0.0), (36, 0.0), (37, 0.0), (38, 0.0), (39, 0.0),
                            (40, 0.0), (41, 0.0), (42, 0.0), (43, 0.0), (44, 0.0), (45, 0.0),
                            (46, 0.0), (47, 0.0), (48, 0.0), (49, 0.0), (50, 0.0), (51, 1.0),
                            (52, 1.0), (53, 0.0), (54, 3.0), (55, 1.0), (56, 2.0), (57, 1.0),
                            (58, 1.0), (59, 1.0), (61, 0.0), (62, 0.0), (63, 1.0), (64, 1.0),
                            (65, 0.0), (66, 0.0), (67, 3.0), (68, 2.0), (69, 100.0), (70, 100.0),
                            (79, 1.0), (81, 0.0), (82, 1.0), (83, 1.0), (84, 0.0), (85, 0.0),
                            (87, 0.0), (88, 0.0), (89, 50.0), (90, 50.0), (91, 50.0), (92, 50.0),
                            (93, 1.0), (99, 0.0), (100, 30.0), (101, 0.0), (102, 1.0),
                            (103, 0.0), (104, 50.0), (105, 0.0), (106, 0.3), (107, 0.0),
                            (112, 0.0), (113, 0.0), (114, 0.0), (115, 0.0), (116, 0.0),
                            (117, 0.0), (118, 0.0), (119, 0.0), (120, 0.0), (121, 1.0),
                            (122, 1.0), (123, 0.0), (130, 0.0), (131, 0.0), (132, 1.0),
                            (135, 1.0), (133, 0.0),
                        ],
                    };
                    let play_menu_info_dict: HashMap<&str, f64> = [
                        ("1", 100.0), ("2", 100.0), ("3", 100.0), ("4", 100.0), ("5", 0.0), ("6", 0.0),
                        ("7", -0.4658685302734375), ("10", 3.0), ("11", 3.0), ("20", 0.0), ("21", 0.0),
                        ("22", 0.0), ("23", 0.0), ("24", 0.0), ("25", 0.0), ("26", 0.0), ("27", 0.0),
                        ("28", 0.0), ("29", 0.0), ("30", 0.0), ("31", 0.0), ("32", 0.0), ("33", 0.0),
                        ("34", 0.0), ("35", 0.0), ("36", 0.0), ("37", 0.0), ("38", 0.0), ("39", 0.0),
                        ("40", 0.0), ("41", 0.0), ("42", 0.0), ("43", 0.0), ("44", 0.0), ("45", 0.0),
                        ("46", 0.0), ("47", 0.0), ("48", 0.0), ("49", 0.0), ("50", 0.0), ("51", 1.0),
                        ("52", 1.0), ("53", 0.0), ("54", 3.0), ("55", 1.0), ("56", 2.0), ("57", 1.0),
                        ("58", 1.0), ("59", 1.0), ("61", 0.0), ("62", 0.0), ("63", 1.0), ("64", 1.0),
                        ("65", 0.0), ("66", 0.0), ("67", 3.0), ("68", 2.0), ("69", 100.0), ("70", 100.0),
                        ("79", 1.0), ("81", 0.0), ("82", 1.0), ("83", 1.0), ("84", 0.0), ("85", 0.0),
                        ("87", 0.0), ("88", 0.0), ("89", 50.0), ("90", 50.0), ("91", 50.0), ("92", 50.0),
                        ("93", 1.0), ("99", 0.0), ("100", 30.0), ("101", 0.0), ("102", 1.0), ("103", 0.0),
                        ("104", 50.0), ("105", 0.0), ("106", 0.3), ("107", 0.0), ("112", 0.0), ("113", 0.0),
                        ("114", 0.0), ("115", 0.0), ("116", 0.0), ("117", 0.0), ("118", 0.0), ("119", 0.0),
                        ("120", 0.0), ("121", 1.0), ("122", 1.0), ("123", 0.0), ("130", 0.0), ("131", 0.0),
                        ("132", 1.0),
                    ].iter().cloned().collect();

                    sqlc.execute("DROP TRIGGER IF EXISTS prevent_custom_frame_rate_update", []).unwrap();

                    let trigger_sql = format!(r#"
        CREATE TRIGGER prevent_custom_frame_rate_update
        AFTER UPDATE OF value ON LocalStorage
        WHEN NEW.key = 'CustomFrameRate'
        BEGIN
            UPDATE LocalStorage
            SET value = {fps}
            WHERE key = 'CustomFrameRate';
        END;
        "#, fps = fpsv);

                    sqlc.execute(trigger_sql.as_str(), []).unwrap();
                    sqlc.execute("UPDATE LocalStorage SET value = ? WHERE key = 'CustomFrameRate'", [&fpsv.to_string()], ).unwrap();
                    sqlc.execute("DELETE FROM LocalStorage WHERE key IN ('MenuData', 'PlayMenuInfo')", [], ).unwrap();

                    let insert_records = vec![("MenuData", serde_json::to_string(&menu_data_dict)?), ("PlayMenuInfo", serde_json::to_string(&play_menu_info_dict)?)];
                    let tx = sqlc.transaction().unwrap();
                    {
                        let mut stmt = tx.prepare("INSERT INTO LocalStorage (key, value) VALUES (?, ?)").unwrap();
                        for (key, value) in &insert_records { stmt.execute((&key, &value)).unwrap(); }
                    }
                    tx.commit().unwrap();
                    sqlc.close().unwrap();

                    let mut ini = Ini::new();
                    ini.load(gameusersettings.as_path().to_str().unwrap()).unwrap();
                    ini.set("/Script/Engine.GameUserSettings", "FramePace", Some(fpsv.to_string())).unwrap();
                    let r = ini.write(gameusersettings.as_path().to_str().unwrap());
                    if r.is_ok() { println!("WutheringWaves FPS unlocked to {}", fpsv.to_string()); } else { eprintln!("Failed to unlock WutheringWaves FPS!"); }
                },
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
