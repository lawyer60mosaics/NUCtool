use std::{
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    time::{Duration, SystemTime, UNIX_EPOCH},
    thread,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::Command,
};
use tauri::{Emitter, Manager, State, Window};
use colored::Colorize;

use crate::plug::{
    constants::{
        CONTROL_SLEEP_MS, FAN_RAMP_STEP, FAN_RAMP_STEP_DOWN,
        FAST_RAMP_UP, MAX_FAN_SPEED_PERCENT, MAX_SENSOR_RPM, MAX_TEMP_LIMIT,
        MAX_MONITOR_SAMPLE_INTERVAL_MS, MIN_MONITOR_SAMPLE_INTERVAL_MS, MIN_TEMP_LIMIT,
        SPEED_EMA_ALPHA, TEMP_EMA_ALPHA, TEMP_EMERGENCY, TEMP_JUMP_THRESHOLD,
    },
    fan_curve::find_speed_for_temp,
    ramp::ramp_speed_internal,
    config::get_monitor_log_file_path,
    mode_profiles::{
        default_cpu_fan_max_for_mode,
        default_gpu_fan_max_for_mode,
        default_hysteresis_for_mode,
        default_ramp_step_for_mode,
    },
    struct_set::{
        ApiFan, FanControlMode, FanControlState, FanControlPolicy, FanData,
    },
    models::ControlState,
};

// 推送线程全局唯一标志，防止重复调用累积线程
static SPEED_PUSH_RUNNING: AtomicBool = AtomicBool::new(false);
static FORCE_SHUTDOWN_QUEUED: AtomicBool = AtomicBool::new(false);
static SENSOR_PUSH_INTERVAL_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1000);

#[allow(dead_code)]
pub fn fan_init() {
    ApiFan::init().set_fan_control();
    println!("{}", "风扇初始化成功".green());
}

pub fn fan_reset() {
    ApiFan::init().set_fan_auto();
    println!("{}", "风扇状态重置".red());
}

/// 将百分比转换为硬件原始值并写入风扇
/// Windows: 0-100% → 0-200 (寄存器值)
/// Linux:   0-100% → 0-255 (PWM 值)
pub fn fan_set(left: i64, right: i64, driver: &ApiFan) {
    #[cfg(windows)]
    let (l, r) = ((left * 2).clamp(0, 200), (right * 2).clamp(0, 200));
    #[cfg(unix)]
    let (l, r): (i64, i64) = {
        let lv = ((2.55 * left as f64) as i64).clamp(0, 255);
        let rv = ((2.55 * right as f64) as i64).clamp(0, 255);
        // 接近满值时强制拉满，避免卡在 254 无法满转
        (if lv >= 254 { 255 } else { lv }, if rv >= 254 { 255 } else { rv })
    };
    println!("FAN_L: {}% / FAN_R: {}% RAW: {}/{} RESULT:{}", left, right, l, r, driver.set_fan(l, r));
}


/// 核心控制循环：读取温度，查找曲线，限速写入硬件
pub fn apply_fan_curve(
    fan_data: &FanData,
    driver: &ApiFan,
    state: &mut ControlState,
    window: Option<&Window>,
) {
    let cpu_out = driver.get_cpu_temp();
    let gpu_out = driver.get_gpu_temp();
    let control_sleep_ms = fan_data
        .monitor
        .sample_interval_ms
        .max(MIN_MONITOR_SAMPLE_INTERVAL_MS);

    if fan_data.monitor.log_enabled {
        append_monitor_log(fan_data, cpu_out, gpu_out, driver.get_fan_l(), driver.get_fan_r());
    }

    // 过滤异常温度读数（<= 0 或 > 110 视为硬件读数异常）
    if cpu_out < MIN_TEMP_LIMIT || cpu_out > MAX_TEMP_LIMIT || gpu_out < MIN_TEMP_LIMIT || gpu_out > MAX_TEMP_LIMIT {
        println!("温度读数异常 cpu={} gpu={}，跳过本次控制", cpu_out, gpu_out);
        thread::sleep(Duration::from_secs(2));
        return;
    }

    update_dynamic_mode(fan_data, state, cpu_out, gpu_out);
    update_tray_status(window, &state.active_mode, cpu_out, gpu_out);

    process_alerts(fan_data, state, cpu_out, gpu_out, window);

    // 初始化 EMA（首次调用时直接用当前读数）
    if state.ema_cpu == 0.0 {
        state.ema_cpu = cpu_out as f64;
    }
    if state.ema_gpu == 0.0 {
        state.ema_gpu = gpu_out as f64;
    }

    // 温度 EMA 平滑，减弱瞬时抖动
    let ema_cpu_new = TEMP_EMA_ALPHA * (cpu_out as f64) + (1.0 - TEMP_EMA_ALPHA) * state.ema_cpu;
    let ema_gpu_new = TEMP_EMA_ALPHA * (gpu_out as f64) + (1.0 - TEMP_EMA_ALPHA) * state.ema_gpu;
    let cpu_use = ema_cpu_new.round() as i64;
    let gpu_use = ema_gpu_new.round() as i64;

    println!("CPU:{}°C (EMA {:.2}) GPU:{}°C (EMA {:.2}) cacheL:{}% cacheR:{}%", cpu_out, ema_cpu_new, gpu_out, ema_gpu_new, state.fan_cache[0], state.fan_cache[1]);

    // 检测风扇被系统切回自动模式（如 OSD/电源计划切换），尝试重新接管但不要立即返回，避免留下硬件在自动高转速状态
    if driver.get_fan_mode() == 2 {
        println!("{}", "⚠️ 风扇被切回自动模式，正在恢复手动控制...".yellow());
        // 立即切回手动并重置缓存/EMA以便马上按曲线写入
        let set_ok = driver.set_fan_control();
        println!("恢复手动控制: {}", set_ok);
        state.fan_cache = [0, 0];
        state.ema_cpu = cpu_out as f64;
        state.ema_gpu = gpu_out as f64;
        // 不再 return；继续按曲线计算并写入，避免硬件长时间处于自动策略
    }

    // 过温保护：超过 TEMP_EMERGENCY 立即满转
    if cpu_out > TEMP_EMERGENCY || gpu_out > TEMP_EMERGENCY {
        if state.fan_cache[0] == 100 && state.fan_cache[1] == 100 {
            println!("{}", "⚠️ 过温，风扇已满转，跳过重复写入".red());
        } else {
            println!("{}", "🔥 过温保护触发，风扇全速运转！".red().bold());
            fan_set(100, 100, driver);
            (state.fan_cache[0], state.fan_cache[1]) = (100, 100);
        }
        thread::sleep(Duration::from_secs(4));
        // 更新 EMA 带入最新读数
        state.ema_cpu = ema_cpu_new;
        state.ema_gpu = ema_gpu_new;
        return;
    }

    // 过温解除后，若缓存仍为满速，重置为 0 使斜坡从目标速度直接跳入
    // 避免从 100% 以每步 10% 缓慢降速，带来长达数十秒的不必要满速
    if state.fan_cache[0] == 100 && state.fan_cache[1] == 100 {
        println!("{}", "✅ 过温已解除，重置缓存以快速恢复正常转速".green());
        state.fan_cache = [0, 0];
    }

    if fan_data.left_fan.is_empty() {
        println!("{}", "左风扇曲线数据无效（没有有效点）".red());
        return;
    }
    if fan_data.right_fan.is_empty() {
        println!("{}", "右风扇曲线数据无效（没有有效点）".red());
        return;
    }

    let constant_mode = is_constant_mode(&fan_data.control);
    if constant_mode {
        let target = constant_speed_target(&fan_data.control);
        if fan_data.control.control_mode.eq_ignore_ascii_case("constant") {
            println!("🔒 控制模式: 恒速模式，固定 {}%", target);
        } else {
            println!("🔒 控制模式: 恒速配置已启用，固定 {}%", target);
        }
        if state.fan_cache != [target, target] {
            fan_set(target, target, driver);
            state.fan_cache = [target, target];
        }
        state.ema_target_left = target as f64;
        state.ema_target_right = target as f64;
        state.ema_cpu = cpu_out as f64;
        state.ema_gpu = gpu_out as f64;
        thread::sleep(Duration::from_millis(control_sleep_ms));
        return;
    }

    // 从曲线查找目标转速（使用 EMA 后的温度）
    let mut target_left_raw = match find_speed_for_temp(&fan_data.left_fan, cpu_use, "左") {
        Some(s) => s.clamp(0, MAX_FAN_SPEED_PERCENT),
        None => return,
    };
    let mut target_right_raw = match find_speed_for_temp(&fan_data.right_fan, gpu_use, "右") {
        Some(s) => s.clamp(0, MAX_FAN_SPEED_PERCENT),
        None => return,
    };

    apply_strategy(
        &fan_data.control,
        cpu_use,
        gpu_use,
        &mut target_left_raw,
        &mut target_right_raw,
    );

    apply_gpu_linkage(
        &fan_data.control,
        gpu_use,
        &mut target_left_raw,
        &mut target_right_raw,
    );

    let cpu_cap = effective_cpu_max_percent(fan_data, state.active_mode.clone());
    let gpu_cap = effective_gpu_max_percent(fan_data, state.active_mode.clone());
    target_left_raw = target_left_raw.clamp(0, cpu_cap);
    target_right_raw = target_right_raw.clamp(0, gpu_cap);

    target_left_raw = apply_temperature_hysteresis(
        cpu_use,
        target_left_raw,
        state.fan_cache[0],
        &mut state.cpu_hysteresis_anchor_temp,
        effective_cpu_hysteresis(fan_data, state.active_mode.clone()),
    );
    target_right_raw = apply_temperature_hysteresis(
        gpu_use,
        target_right_raw,
        state.fan_cache[1],
        &mut state.gpu_hysteresis_anchor_temp,
        effective_gpu_hysteresis(fan_data, state.active_mode.clone()),
    );

    if state.force_max_fan {
        target_left_raw = MAX_FAN_SPEED_PERCENT;
        target_right_raw = MAX_FAN_SPEED_PERCENT;
    }

    // 如果我们刚刚从自动切回手动，马上写一次目标速度，确保硬件快速接管（避免短时间仍由系统自动策略控制而导致高转）
    if state.fan_cache == [0, 0] {
        // 此处为首次写入或刚重置缓存的情形，直接写入目标速度（clamp 已在上面做好）
        println!("⚡ 刚刚接管，立即写入目标速度 CPU:{}% GPU:{}%", target_left_raw, target_right_raw);
        fan_set(target_left_raw, target_right_raw, driver);
        (state.fan_cache[0], state.fan_cache[1]) = (target_left_raw, target_right_raw);
        // 更新 EMA 目标以反映写入
        state.ema_target_left = target_left_raw as f64;
        state.ema_target_right = target_right_raw as f64;
        // 更新温度 EMA 状态并等待下次循环
        state.ema_cpu = ema_cpu_new;
        state.ema_gpu = ema_gpu_new;
        thread::sleep(Duration::from_millis(control_sleep_ms));
        return;
    }


    // 对目标速度再做 EMA 平滑，减小目标抖动带来的写入频率
    if state.ema_target_left == 0.0 {
        state.ema_target_left = target_left_raw as f64;
    }
    if state.ema_target_right == 0.0 {
        state.ema_target_right = target_right_raw as f64;
    }
    let ema_target_left_new = SPEED_EMA_ALPHA * (target_left_raw as f64) + (1.0 - SPEED_EMA_ALPHA) * state.ema_target_left;
    let ema_target_right_new = SPEED_EMA_ALPHA * (target_right_raw as f64) + (1.0 - SPEED_EMA_ALPHA) * state.ema_target_right;
    let target_left = ema_target_left_new.round() as i64;
    let target_right = ema_target_right_new.round() as i64;

    let speed_hysteresis = effective_speed_hysteresis(fan_data, state.active_mode.clone());

    // 防抖：目标与当前差值均在滞后范围内时，跳过本次写入
    if (state.fan_cache[0] - target_left).abs()  <= speed_hysteresis
    && (state.fan_cache[1] - target_right).abs() <= speed_hysteresis {
        println!("{}", "Δ速度过小，保持当前状态（防抖）".green());
        // 更新 EMA 状态并休眠
        state.ema_cpu = ema_cpu_new;
        state.ema_gpu = ema_gpu_new;
        state.ema_target_left = ema_target_left_new;
        state.ema_target_right = ema_target_right_new;
        thread::sleep(Duration::from_millis(control_sleep_ms));
        return;
    }

    // 自适应斜坡：若温度短时间内快速上升，允许快速上升步进；下降时更保守
    let mode_step = default_ramp_step_for_mode(&state.active_mode);
    let mut max_up = fan_data.control.ramp_up_step.max(mode_step).max(1);
    let mut max_down = fan_data.control.ramp_down_step.max(mode_step).max(1);

    if max_up <= 0 {
        max_up = FAN_RAMP_STEP;
    }
    if max_down <= 0 {
        max_down = FAN_RAMP_STEP_DOWN;
    }

    if cpu_out - state.ema_cpu.round() as i64 >= TEMP_JUMP_THRESHOLD || gpu_out - state.ema_gpu.round() as i64 >= TEMP_JUMP_THRESHOLD {
        max_up = FAST_RAMP_UP;
    }

    if state.force_max_fan {
        max_up = FAST_RAMP_UP.max(max_up);
        max_down = 100;
    }

    // 斜坡限速（使用自适应步进）
    let ramp_left  = ramp_speed_internal(
        state.fan_cache[0],
        target_left,
        max_up,
        max_down,
        effective_min_speed(&fan_data.control, cpu_use),
    );
    let ramp_right = ramp_speed_internal(
        state.fan_cache[1],
        target_right,
        max_up,
        max_down,
        effective_min_speed(&fan_data.control, gpu_use),
    );

    if state.fan_cache[0] == ramp_left && state.fan_cache[1] == ramp_right {
        println!("{}", "✓ 斜坡后速度未变化，维持当前状态".green());
        // 更新 EMA 状态并休眠
        state.ema_cpu = ema_cpu_new;
        state.ema_gpu = ema_gpu_new;
        state.ema_target_left = ema_target_left_new;
        state.ema_target_right = ema_target_right_new;
        thread::sleep(Duration::from_millis(control_sleep_ms));
        return;
    }

    println!("🔄 CPU {}%→{}%  GPU {}%→{}%", state.fan_cache[0], ramp_left, state.fan_cache[1], ramp_right);
    fan_set(ramp_left, ramp_right, driver);
    (state.fan_cache[0], state.fan_cache[1]) = (ramp_left, ramp_right);

    // 更新 EMA 状态
    state.ema_cpu = ema_cpu_new;
    state.ema_gpu = ema_gpu_new;
    state.ema_target_left = ema_target_left_new;
    state.ema_target_right = ema_target_right_new;

    thread::sleep(Duration::from_millis(control_sleep_ms));
}

fn effective_min_speed(control: &FanControlPolicy, temp_now: i64) -> i64 {
    if control.zero_rpm_enabled && temp_now <= control.zero_rpm_threshold {
        0
    } else {
        control.min_speed.clamp(0, MAX_FAN_SPEED_PERCENT)
    }
}

fn is_constant_mode(control: &FanControlPolicy) -> bool {
    control.constant_speed_enabled || control.control_mode.eq_ignore_ascii_case("constant")
}

fn constant_speed_target(control: &FanControlPolicy) -> i64 {
    control.constant_speed.clamp(0, MAX_FAN_SPEED_PERCENT)
}

fn effective_cpu_max_percent(fan_data: &FanData, active_mode: FanControlMode) -> i64 {
    let mode_default = default_cpu_fan_max_for_mode(&active_mode);
    let configured = fan_data.control.cpu_fan_max_percent;
    if configured <= 0 {
        mode_default
    } else if active_mode == FanControlMode::Custom {
        configured.clamp(20, MAX_FAN_SPEED_PERCENT)
    } else {
        configured.clamp(20, mode_default)
    }
}

fn effective_gpu_max_percent(fan_data: &FanData, active_mode: FanControlMode) -> i64 {
    let mode_default = default_gpu_fan_max_for_mode(&active_mode);
    let configured = fan_data.control.gpu_fan_max_percent;
    if configured <= 0 {
        mode_default
    } else if active_mode == FanControlMode::Custom {
        configured.clamp(20, MAX_FAN_SPEED_PERCENT)
    } else {
        configured.clamp(20, mode_default)
    }
}

fn effective_cpu_hysteresis(fan_data: &FanData, active_mode: FanControlMode) -> i64 {
    let mode_default = default_hysteresis_for_mode(&active_mode);
    let configured = fan_data.control.cpu_hysteresis_bandwidth;
    if configured <= 0 {
        mode_default
    } else {
        configured.clamp(1, 12)
    }
}

fn effective_gpu_hysteresis(fan_data: &FanData, active_mode: FanControlMode) -> i64 {
    let mode_default = default_hysteresis_for_mode(&active_mode);
    let configured = fan_data.control.gpu_hysteresis_bandwidth;
    if configured <= 0 {
        mode_default
    } else {
        configured.clamp(1, 12)
    }
}

fn effective_speed_hysteresis(fan_data: &FanData, active_mode: FanControlMode) -> i64 {
    let configured = fan_data
        .control
        .cpu_hysteresis_bandwidth
        .max(fan_data.control.gpu_hysteresis_bandwidth)
        .clamp(1, 12);
    configured.max(default_hysteresis_for_mode(&active_mode).min(6))
}

fn apply_temperature_hysteresis(
    temp_now: i64,
    target: i64,
    current_speed: i64,
    anchor_temp: &mut i64,
    bandwidth: i64,
) -> i64 {
    if *anchor_temp == 0 {
        *anchor_temp = temp_now;
        return target;
    }

    if temp_now >= *anchor_temp {
        *anchor_temp = temp_now;
        return target;
    }

    if *anchor_temp - temp_now <= bandwidth.max(1) {
        return target.max(current_speed);
    }

    *anchor_temp = temp_now;
    target
}

fn apply_gpu_linkage(
    control: &FanControlPolicy,
    gpu_temp: i64,
    left_target: &mut i64,
    right_target: &mut i64,
) {
    if !control.gpu_linkage_enabled {
        return;
    }
    if gpu_temp < control.gpu_linkage_threshold {
        return;
    }

    let boosted_cpu = (*left_target + control.gpu_linkage_boost).clamp(0, MAX_FAN_SPEED_PERCENT);
    *left_target = (*left_target).max(boosted_cpu);
    *right_target = (*right_target).max(*left_target);
}

fn update_dynamic_mode(fan_data: &FanData, state: &mut ControlState, cpu_temp: i64, gpu_temp: i64) {
    if !matches!(fan_data.control.mode, FanControlMode::Office) {
        state.active_mode = fan_data.control.mode.clone();
        state.office_cooldown_ticks = 0;
        state.gaming_hot_ticks = 0;
        return;
    }

    let sample_secs = (fan_data.monitor.sample_interval_ms.max(500) as f64) / 1000.0;
    let gaming_required_ticks = (3.0 / sample_secs).ceil() as u32;
    let office_required_ticks = (180.0 / sample_secs).ceil() as u32;

    if gpu_temp >= fan_data.control.gpu_linkage_threshold {
        state.gaming_hot_ticks = state.gaming_hot_ticks.saturating_add(1);
        state.office_cooldown_ticks = 0;
    } else if cpu_temp < 40 {
        state.office_cooldown_ticks = state.office_cooldown_ticks.saturating_add(1);
        state.gaming_hot_ticks = 0;
    } else {
        state.gaming_hot_ticks = 0;
        state.office_cooldown_ticks = 0;
    }

    let next_mode = if state.gaming_hot_ticks >= gaming_required_ticks.max(1) {
        FanControlMode::Gaming
    } else if state.office_cooldown_ticks >= office_required_ticks.max(1) {
        FanControlMode::Office
    } else {
        state.active_mode.clone()
    };

    if next_mode != state.active_mode {
        println!(
            "🧭 模式自动切换: {} -> {}",
            state.active_mode.as_str(),
            next_mode.as_str()
        );
        state.active_mode = next_mode;
    }
}

fn update_tray_status(window: Option<&Window>, mode: &FanControlMode, cpu_temp: i64, gpu_temp: i64) {
    let Some(win) = window else { return };
    let Some(tray) = win.app_handle().tray_by_id("main_tray") else {
        return;
    };
    let tip = format!(
        "NUCtool | 模式:{} | CPU:{}°C GPU:{}°C",
        mode.as_str(),
        cpu_temp,
        gpu_temp
    );
    let _ = tray.set_tooltip(Some(tip));
}

fn apply_strategy(
    control: &FanControlPolicy,
    cpu_temp: i64,
    gpu_temp: i64,
    left_target: &mut i64,
    right_target: &mut i64,
) {
    match control.strategy.as_str() {
        "mix_max" => {
            let _ = cpu_temp.max(gpu_temp);
            let linked = (*left_target).max(*right_target);
            *left_target = linked;
            *right_target = linked;
        }
        "cpu_only" => {
            *right_target = *left_target;
        }
        "gpu_only" => {
            *left_target = *right_target;
        }
        _ => {}
    }
}

fn process_alerts(
    fan_data: &FanData,
    state: &mut ControlState,
    cpu_temp: i64,
    gpu_temp: i64,
    window: Option<&Window>,
) {
    let cpu_threshold = fan_data.alerts.cpu.threshold;
    let gpu_threshold = fan_data.alerts.gpu.threshold;

    let cpu_confirm = fan_data
        .alerts
        .cpu
        .actions
        .confirm_times
        .max(1);
    let gpu_confirm = fan_data
        .alerts
        .gpu
        .actions
        .confirm_times
        .max(1);
    let recover_delta = fan_data.alerts.recover_delta.max(1);

    if cpu_temp >= cpu_threshold {
        state.alert_cpu_high_count = state.alert_cpu_high_count.saturating_add(1);
        state.alert_cpu_recover_count = 0;
    } else if state.cpu_alert_active && cpu_temp <= cpu_threshold - recover_delta {
        state.alert_cpu_recover_count = state.alert_cpu_recover_count.saturating_add(1);
        state.alert_cpu_high_count = 0;
    } else {
        state.alert_cpu_high_count = 0;
        state.alert_cpu_recover_count = 0;
    }

    if gpu_temp >= gpu_threshold {
        state.alert_gpu_high_count = state.alert_gpu_high_count.saturating_add(1);
        state.alert_gpu_recover_count = 0;
    } else if state.gpu_alert_active && gpu_temp <= gpu_threshold - recover_delta {
        state.alert_gpu_recover_count = state.alert_gpu_recover_count.saturating_add(1);
        state.alert_gpu_high_count = 0;
    } else {
        state.alert_gpu_high_count = 0;
        state.alert_gpu_recover_count = 0;
    }

    if !state.cpu_alert_active && state.alert_cpu_high_count >= cpu_confirm {
        state.cpu_alert_active = true;
        apply_alert_actions("CPU", cpu_temp, &fan_data.alerts.cpu.actions, window);
    }

    if !state.gpu_alert_active && state.alert_gpu_high_count >= gpu_confirm {
        state.gpu_alert_active = true;
        apply_alert_actions("GPU", gpu_temp, &fan_data.alerts.gpu.actions, window);
    }

    if state.cpu_alert_active && state.alert_cpu_recover_count >= cpu_confirm {
        state.cpu_alert_active = false;
        println!("✅ CPU 温度告警已恢复");
    }

    if state.gpu_alert_active && state.alert_gpu_recover_count >= gpu_confirm {
        state.gpu_alert_active = false;
        println!("✅ GPU 温度告警已恢复");
    }

    state.force_max_fan = (state.cpu_alert_active && fan_data.alerts.cpu.actions.force_shutdown)
        || (state.gpu_alert_active && fan_data.alerts.gpu.actions.force_shutdown);
}

fn apply_alert_actions(
    sensor: &str,
    temp: i64,
    actions: &crate::plug::struct_set::AlertActions,
    window: Option<&Window>,
) {
    if actions.log {
        println!("[告警] {} 温度达到 {}°C", sensor, temp);
    }
    if actions.popup {
        println!("[弹窗] {} 温度过高：{}°C", sensor, temp);
    }
    if actions.sound {
        print!("\x07");
    }
    if actions.force_shutdown {
        println!("[保护动作] {} 触发强制保护，风扇将保持全速", sensor);
        schedule_force_shutdown(sensor, temp);
    }

    if let Some(win) = window {
        if actions.popup || actions.sound || actions.force_shutdown {
            let payload = serde_json::json!({
                "sensor": sensor,
                "temperature": temp,
                "popup": actions.popup,
                "sound": actions.sound,
                "force_shutdown": actions.force_shutdown,
            });
            if let Err(e) = win.emit("temp-alert", payload) {
                println!("发送温度告警事件失败: {:?}", e);
            }
        }
    }
}

fn schedule_force_shutdown(sensor: &str, temp: i64) {
    if FORCE_SHUTDOWN_QUEUED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let sensor = sensor.to_string();
    thread::spawn(move || {
        println!(
            "[强制关机预备] {} 持续高温 {}°C，5 秒后执行系统关机",
            sensor, temp
        );
        thread::sleep(Duration::from_secs(5));

        #[cfg(windows)]
        let result = Command::new("shutdown")
            .args(["/s", "/t", "5", "/f", "/c", "NUCtool thermal protection"]) 
            .status();
        #[cfg(unix)]
        let result = Command::new("shutdown")
            .args(["-h", "+1", "NUCtool thermal protection"]) 
            .status();

        match result {
            Ok(status) => {
                println!("已请求系统关机，状态: {:?}", status);
            }
            Err(e) => {
                println!("执行系统关机失败: {}", e);
                FORCE_SHUTDOWN_QUEUED.store(false, Ordering::SeqCst);
            }
        }
    });
}

fn monitor_log_path() -> Option<PathBuf> {
    get_monitor_log_file_path().ok()
}

fn append_monitor_log(
    fan_data: &FanData,
    cpu_temp: i64,
    gpu_temp: i64,
    cpu_fan: i64,
    gpu_fan: i64,
) {
    if !fan_data.monitor.log_enabled {
        return;
    }

    let safe_cpu_temp = cpu_temp.clamp(0, MAX_TEMP_LIMIT);
    let safe_gpu_temp = gpu_temp.clamp(0, MAX_TEMP_LIMIT);
    let safe_cpu_fan = cpu_fan.clamp(0, MAX_SENSOR_RPM);
    let safe_gpu_fan = gpu_fan.clamp(0, MAX_SENSOR_RPM);

    let Some(path) = monitor_log_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let exists = path.exists();
    let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            println!("写入监控日志失败: {}", e);
            return;
        }
    };

    if !exists {
        let _ = writeln!(
            file,
            "timestamp,cpu_temp,gpu_temp,cpu_fan_rpm,gpu_fan_rpm,strategy,preset,mode"
        );
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let _ = writeln!(
        file,
        "{},{},{},{},{},{},{},{}",
        timestamp,
        safe_cpu_temp,
        safe_gpu_temp,
        safe_cpu_fan,
        safe_gpu_fan,
        fan_data.control.strategy,
        fan_data.control.preset,
        fan_data.control.mode.as_str()
    );
}

/// 启动风扇数据推送线程（每 2.5 秒向前端 emit 一次）
/// 使用 AtomicBool 防止重复调用累积线程；emit 失败时安全退出而非 panic
#[tauri::command]
pub async fn get_fan_speeds(window: Window) {
    if SPEED_PUSH_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        println!("{}", "推送线程已在运行，忽略重复调用".yellow());
        return;
    }
    thread::spawn(move || {
        println!("{}", "✅ 风扇推送线程已启动".green());
        let driver = ApiFan::init();
        loop {
            let interval_ms = SENSOR_PUSH_INTERVAL_MS
                .load(Ordering::Relaxed)
                .clamp(MIN_MONITOR_SAMPLE_INTERVAL_MS, MAX_MONITOR_SAMPLE_INTERVAL_MS);
            thread::sleep(Duration::from_millis(interval_ms));
            if matches!(window.is_visible(), Ok(false)) {
                continue;
            }
            if let Err(e) = window.emit("get-fan-speeds", driver.get_fan_speeds()) {
                println!("推送失败（窗口可能已销毁）: {:?}，推送线程退出", e);
                break;
            }
        }
        SPEED_PUSH_RUNNING.store(false, Ordering::SeqCst);
        println!("{}", "📴 风扇推送线程已退出".yellow());
    });
}

#[tauri::command]
pub fn start_fan_control(window: Window, fan_data: serde_json::Value, state: State<FanControlState>) {
    let fan_data: FanData = match serde_json::from_value(fan_data) {
        Ok(v) => v,
        Err(e) => {
            println!("风扇配置解析失败: {}", e);
            return;
        }
    };

    SENSOR_PUSH_INTERVAL_MS.store(
        fan_data
            .monitor
            .sample_interval_ms
            .clamp(MIN_MONITOR_SAMPLE_INTERVAL_MS, MAX_MONITOR_SAMPLE_INTERVAL_MS.min(1000)),
        Ordering::Relaxed,
    );
    let is_running = Arc::clone(&state.is_running);
    let active_mode = Arc::clone(&state.active_mode);

    // If already running, request stop and spawn a delayed starter to wait for old thread to exit.
    let was_running = {
        let mut running = is_running.lock().unwrap();
        if *running {
            println!("{}", "⚠️ 已在运行，计划重启以应用新配置".yellow());
            *running = false;
            true
        } else {
            *running = true;
            false
        }
    };

    if was_running {
        // spawn a background thread that waits for the old control thread to exit, then starts new
        let is_running_clone = Arc::clone(&is_running);
        let active_mode_clone = Arc::clone(&active_mode);
        let fan_data_clone = fan_data.clone();
        let alert_window = window.clone();
        let wait_ms = fan_data
            .monitor
            .sample_interval_ms
            .max(CONTROL_SLEEP_MS)
            + 1500;
        thread::spawn(move || {
            // Wait the previous control loop max sleep time to ensure it exits (non-blocking for invoke)
            thread::sleep(Duration::from_millis(wait_ms));
            // Double-check state
            let mut running = is_running_clone.lock().unwrap();
            if *running {
                // If some other caller already started it, do nothing
                println!("{}", "❗ 无法重启风扇控制（状态仍为运行中）".red());
                return;
            }
            *running = true;
            drop(running);

            println!("{}", "✅ 启动风扇控制系统（延迟重启）".green().bold());
            let driver = ApiFan::init();
            driver.set_fan_control();
            let mut ctrl_state = ControlState::new(fan_data_clone.control.mode.clone());
            while *is_running_clone.lock().unwrap() {
                println!("---------------------------------------------------------------");
                apply_fan_curve(&fan_data_clone, &driver, &mut ctrl_state, Some(&alert_window));
                if let Ok(mut mode) = active_mode_clone.lock() {
                    *mode = ctrl_state.active_mode.as_str().to_string();
                }
            }
            println!("{}", "🛑 风扇控制线程已停止".red());
            println!("---------------------------------------------------------------");
        });
        return;
    }

    // start immediately (fast return from command)
    println!("{}", "✅ 启动风扇控制系统".green().bold());
    let is_running_clone = Arc::clone(&is_running);
    let active_mode_clone = Arc::clone(&active_mode);
    let fan_data_clone = fan_data.clone();
    let alert_window = window.clone();
    thread::spawn(move || {
        let driver = ApiFan::init();
        driver.set_fan_control();
        let mut ctrl_state = ControlState::new(fan_data_clone.control.mode.clone());

        while *is_running_clone.lock().unwrap() {
            println!("---------------------------------------------------------------");
            apply_fan_curve(&fan_data_clone, &driver, &mut ctrl_state, Some(&alert_window));
            if let Ok(mut mode) = active_mode_clone.lock() {
                *mode = ctrl_state.active_mode.as_str().to_string();
            }
        }
        println!("{}", "🛑 风扇控制线程已停止".red());
        println!("---------------------------------------------------------------");
    });
}

#[tauri::command]
pub fn get_current_fan_mode(state: State<FanControlState>) -> Result<String, String> {
    let mode = state
        .active_mode
        .lock()
        .map_err(|_| "读取模式状态失败".to_string())?;
    Ok(mode.clone())
}

#[tauri::command]
pub fn stop_fan_control(state: State<FanControlState>) {
    // 先释放锁，再 spawn，避免持锁期间 spawn
    {
        let mut is_running = state.is_running.lock().unwrap();
        *is_running = false;
    }
    if let Ok(mut mode) = state.active_mode.lock() {
        *mode = "office".to_string();
    }
    thread::spawn(move || {
        // 等待控制线程退出后再重置，避免竞争
        thread::sleep(Duration::from_millis(CONTROL_SLEEP_MS + 1500));
        fan_reset();
    });
    println!("{}", "✅ 风扇控制已停止".green());
}
