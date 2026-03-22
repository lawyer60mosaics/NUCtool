use std::{
    fs,
    path::PathBuf
};

#[cfg(unix)]
use std::{io::Read, fs::File};

use crate::plug::struct_set::FanData;
pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap()
        .join("com.nuc.x15.fan.cyear.app")
}
pub fn get_config_file_path() -> Result<PathBuf, String> {
    // 获取应用的配置目录
    let config_dir = get_config_dir();
    // 配置文件名
    let config_file = config_dir.join("fan_config.json");
    println!("{:?}", &config_file);
    Ok(config_file)
}

#[cfg(unix)]
pub fn find_hwmon_with_name() -> PathBuf {
    let hwmon_dir = "/sys/class/hwmon";

    match fs::read_dir(hwmon_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name_path = path.join("name");
                    if name_path.exists() {
                        if let Ok(mut name_file) = File::open(&name_path) {
                            let mut content = String::new();
                            if name_file.read_to_string(&mut content).is_ok()
                                && content.trim() == "uniwill"
                            {
                                println!("找到匹配的hwmon设备: {:?}", path);
                                return path;
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("无法读取hwmon目录 {}: {}", hwmon_dir, e);
        }
    }

    // 未找到时返回一个占位路径并打印明确错误，而非直接 panic
    // 实际访问时会因路径不存在而失败，但程序可以正常启动并提示用户
    eprintln!("❌ 未找到 uniwill hwmon 设备，请确保 uniwill-laptop 驱动已正确加载！");
    eprintln!("   尝试运行: sudo modprobe uniwill-laptop");
    PathBuf::from("/sys/class/hwmon/hwmon_not_found")
}

#[tauri::command]
pub async fn save_fan_config(fan_data: FanData) -> Result<(), String> {
    let config_path = get_config_file_path()?;

    // 确保配置目录存在
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {}", e))?;
    }

    let json_data = serde_json::to_string_pretty(&fan_data)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(&config_path, json_data)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;

    println!("风扇配置已保存至: {:?}", config_path);
    Ok(())
}

#[tauri::command]
pub async fn load_fan_config() -> Result<FanData, String> {
    let config_path = get_config_file_path()?;
    if !config_path.exists() {
        return Err(format!("配置文件不存在: {:?}", config_path));
    }
    let json_data = fs::read_to_string(&config_path)
        .map_err(|e| format!("读取配置文件失败: {}", e))?;
    let fan_data: FanData = serde_json::from_str(&json_data)
        .map_err(|e| format!("解析配置文件失败: {}", e))?;
    println!("风扇配置已加载: {:?}", config_path);
    Ok(fan_data)
}
