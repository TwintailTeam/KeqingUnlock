use serde_json::Value;
use winreg::RegValue;

pub fn parse_raw_value(raw_value: &RegValue) -> std::io::Result<Value> {
    match serde_json::from_slice(&raw_value.bytes) {
        Ok(value) => Ok(value),
        Err(_) => {
            let cleaned_value = clean_raw_value(raw_value);
            match serde_json::from_slice(&cleaned_value.bytes) {
                Ok(value) => Ok(value),
                Err(_) => {
                    let ultra_cleaned = ultra_clean_raw_value(raw_value);
                    match serde_json::from_slice(&ultra_cleaned.bytes) {
                        Ok(value) => Ok(value),
                        Err(_) => { Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to parse ultra-cleaned value!")) }
                    }
                }
            }
        }
    }
}

fn clean_raw_value(raw_value: &RegValue) -> RegValue {
    let cleaned_bytes: Vec<u8> = raw_value.bytes.iter().filter(|&&b| b != 0).copied().collect();
    RegValue { bytes: cleaned_bytes, vtype: raw_value.vtype.clone(), }
}

fn ultra_clean_raw_value(raw_value: &RegValue) -> RegValue {
    let cleaned_bytes: Vec<u8> = raw_value.bytes.iter().filter(|&&b| b >= 32 || b == 9 || b == 10 || b == 13).copied().collect();
    RegValue { bytes: cleaned_bytes, vtype: raw_value.vtype.clone(), }
}

pub fn create_raw_value_from_json(json_value: &Value, original_raw_value: &RegValue) -> std::io::Result<RegValue> {
    let json_bytes = serde_json::to_vec(json_value)?;
    Ok(RegValue { bytes: json_bytes, vtype: original_raw_value.vtype.clone() })
}
