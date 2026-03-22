use std::{
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    time::Duration,
    thread
};
use tauri::{Emitter, State, Window};
use colored::Colorize;

use crate::plug::{
    struct_set::{
        FanControlState, ApiFan
    },
};

// 最低风扇占空比，防止风扇停转
const MIN_FAN_SPEED: i64 = 20;
// 单次调整的最大步进，避免骤升骤降（默认值）
const FAN_RAMP_STEP: i64 = 10;
// 下调步进（更平缓）
const FAN_RAMP_STEP_DOWN: i64 = 6;
// 目标速度与当前缓存差值小于该值则保持不动，减少抖动
const SPEED_HYSTERESIS: i64 = 4;
// 控制线程每次循环的休眠时间（毫秒）
// stop/start 等待时间必须 > 此值，确保旧线程能及时退出
const CONTROL_SLEEP_MS: u64 = 3000;
// 过温保护阈值，超过后立即满转（与 struct_set 中验证阈值 105°C 区分）
const TEMP_EMERGENCY: i64 = 95;
// 温度 EMA 平滑系数（越大越敏感，取值在 0..1）
const TEMP_EMA_ALPHA: f64 = 0.4;
// 速度 EMA 平滑系数
const SPEED_EMA_ALPHA: f64 = 0.6;
// 突变温度阈值（度）——若温度在一次读数中增长超过该值，允许更快升速
const TEMP_JUMP_THRESHOLD: i64 = 4;
// 在突变情况下的快速上升步进
const FAST_RAMP_UP: i64 = 20;

// 推送线程全局唯一标志，防止重复调用累积线程
static SPEED_PUSH_RUNNING: AtomicBool = AtomicBool::new(false);

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

/// 计算风扇百分比速度（线性插值），结果夹紧到 [0, 100]
/// - temp_old  : 上一个曲线点的温度
/// - speed_old : 上一个曲线点对应的风扇速度
/// - temp      : 当前区间上界温度（>= temp_now）
/// - speed     : 上界对应的风扇速度
/// - temp_now  : 当前实际温度
pub fn speed_handle(temp_old: i64, speed_old: i64, temp: i64, speed: i64, temp_now: i64) -> i64 {
    println!("interp temp_old:{} speed_old:{} temp:{} speed:{} now:{}",
             temp_old, speed_old, temp, speed, temp_now);

    let temp_diff = temp - temp_old;
    if temp_diff == 0 {
        return speed_old.clamp(0, 100);
    }
    let result = speed_old + ((speed - speed_old) * (temp_now - temp_old) / temp_diff);
    result.clamp(0, 100)
}

/// 从曲线 JSON 数组中，根据当前温度查找插值目标转速（百分比）
/// 提取为独立函数，消除左右风扇查找逻辑的重复代码
fn find_speed_for_temp(
    curve: &[serde_json::Value],
    temp_now: i64,
    side: &str,
) -> Option<i64> {
    let (mut temp_old, mut speed_old) = (0i64, 0i64);

    for (index, point) in curve.iter().enumerate() {
        let t = match point.get("temperature").and_then(|v| v.as_i64()) {
            Some(v) => v,
            None => {
                println!("{}风扇曲线点 temperature 字段缺失", side);
                return None;
            }
        };
        let s = match point.get("speed").and_then(|v| v.as_i64()) {
            Some(v) => v,
            None => {
                println!("{}风扇曲线点 speed 字段缺失", side);
                return None;
            }
        };

        if t >= temp_now {
            // 温度低于（或等于）第一个曲线点时直接返回该点速度
            let target = if index == 0 {
                s
            } else {
                speed_handle(temp_old, speed_old, t, s, temp_now)
            };
            return Some(target);
        }
        temp_old = t;
        speed_old = s;
    }
    // 温度超出曲线所有设定点，使用最后一个点的速度
    Some(speed_old)
}

/// 斜坡限速内部实现：限制单次调整幅度不超过指定步进，首次启动（cache=0）直接跳到目标
/// 修复：首次启动时也保证不低于 MIN_FAN_SPEED
fn ramp_speed_internal(cache: i64, target: i64, max_step_up: i64, max_step_down: i64) -> i64 {
    if cache == 0 {
        // 首次启动：直接跳到目标，但不低于最低转速
        target.max(MIN_FAN_SPEED)
    } else if target > cache {
        // 上升受上升步进限制
        let high = (cache + max_step_up).min(100);
        target.clamp(cache, high)
    } else if target < cache {
        // 下降受下降步进限制
        let low = (cache - max_step_down).max(MIN_FAN_SPEED);
        target.clamp(low, cache)
    } else {
        cache
    }
}

/// 向后兼容的 ramp_speed（测试/外部调用使用）——使用默认步进常量
pub fn ramp_speed(cache: i64, target: i64) -> i64 {
    ramp_speed_internal(cache, target, FAN_RAMP_STEP, FAN_RAMP_STEP)
}

/// 控制器运行时状态，用于保留 EMA 值与缓存
struct ControlState {
    fan_cache: [i64; 2],
    ema_cpu: f64,
    ema_gpu: f64,
    ema_target_left: f64,
    ema_target_right: f64,
}

impl ControlState {
    fn new() -> Self {
        ControlState {
            fan_cache: [0i64; 2],
            ema_cpu: 0.0,
            ema_gpu: 0.0,
            ema_target_left: 0.0,
            ema_target_right: 0.0,
        }
    }
}

/// 核心控制循环：读取温度，查找曲线，限速写入硬件
pub fn apply_fan_curve(
    left: &Option<&serde_json::Value>,
    right: &Option<&serde_json::Value>,
    driver: &ApiFan,
    state: &mut ControlState,
) {
    let cpu_out = driver.get_cpu_temp();
    let gpu_out = driver.get_gpu_temp();

    // 过滤异常温度读数（<= 0 或 > 110 视为硬件读数异常）
    if cpu_out <= 0 || cpu_out > 110 || gpu_out <= 0 || gpu_out > 110 {
        println!("温度读数异常 cpu={} gpu={}，跳过本次控制", cpu_out, gpu_out);
        thread::sleep(Duration::from_secs(2));
        return;
    }

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

    // 检测风扇被系统切回自动模式（如 OSD/电源计划切换），直接重新接管
    // 注意：放在过温保护之前，确保接管后下一轮能正确写入速度
    if driver.get_fan_mode() == 2 {
        println!("{}", "⚠️ 风扇被切回自动模式，正在恢复手动控制...".yellow());
        thread::sleep(Duration::from_secs(1));
        println!("恢复手动控制: {}", driver.set_fan_control());
        // 重置缓存，使接管后立即按曲线写入正确速度，避免因旧缓存导致斜坡延迟
        state.fan_cache = [0, 0];
        // 重置 EMA 以更快收敛到当前值
        state.ema_cpu = cpu_out as f64;
        state.ema_gpu = gpu_out as f64;
        thread::sleep(Duration::from_secs(1));
        return;
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

    // 安全获取曲线数组
    let left_arr = match left.and_then(|v| v.as_array()) {
        Some(a) => a,
        None => { println!("{}", "左风扇曲线数据无效".red()); return; }
    };
    let right_arr = match right.and_then(|v| v.as_array()) {
        Some(a) => a,
        None => { println!("{}", "右风扇曲线数据无效".red()); return; }
    };

    // 从曲线查找目标转速（使用 EMA 后的温度）
    let target_left_raw = match find_speed_for_temp(left_arr, cpu_use, "左") {
        Some(s) => s.clamp(MIN_FAN_SPEED, 100),
        None => return,
    };
    let target_right_raw = match find_speed_for_temp(right_arr, gpu_use, "右") {
        Some(s) => s.clamp(MIN_FAN_SPEED, 100),
        None => return,
    };

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

    // 防抖：目标与当前差值均在滞后范围内时，跳过本次写入
    if (state.fan_cache[0] - target_left).abs()  <= SPEED_HYSTERESIS
    && (state.fan_cache[1] - target_right).abs() <= SPEED_HYSTERESIS {
        println!("{}", "Δ速度过小，保持当前状态（防抖）".green());
        // 更新 EMA 状态并休眠
        state.ema_cpu = ema_cpu_new;
        state.ema_gpu = ema_gpu_new;
        state.ema_target_left = ema_target_left_new;
        state.ema_target_right = ema_target_right_new;
        thread::sleep(Duration::from_millis(CONTROL_SLEEP_MS));
        return;
    }

    // 自适应斜坡：若温度短时间内快速上升，允许快速上升步进；下降时更保守
    let mut max_up = FAN_RAMP_STEP;
    let mut max_down = FAN_RAMP_STEP_DOWN;
    if cpu_out - state.ema_cpu.round() as i64 >= TEMP_JUMP_THRESHOLD || gpu_out - state.ema_gpu.round() as i64 >= TEMP_JUMP_THRESHOLD {
        max_up = FAST_RAMP_UP;
    }

    // 斜坡限速（使用自适应步进）
    let ramp_left  = ramp_speed_internal(state.fan_cache[0], target_left, max_up, max_down);
    let ramp_right = ramp_speed_internal(state.fan_cache[1], target_right, max_up, max_down);

    if state.fan_cache[0] == ramp_left && state.fan_cache[1] == ramp_right {
        println!("{}", "✓ 斜坡后速度未变化，维持当前状态".green());
        // 更新 EMA 状态并休眠
        state.ema_cpu = ema_cpu_new;
        state.ema_gpu = ema_gpu_new;
        state.ema_target_left = ema_target_left_new;
        state.ema_target_right = ema_target_right_new;
        thread::sleep(Duration::from_millis(CONTROL_SLEEP_MS));
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

    thread::sleep(Duration::from_millis(CONTROL_SLEEP_MS));
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
            thread::sleep(Duration::from_secs_f64(2.5));
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
pub fn start_fan_control(fan_data: serde_json::Value, state: State<FanControlState>) {
    let is_running = Arc::clone(&state.is_running);

    {
        let mut running = is_running.lock().unwrap();
        if *running {
            println!("{}", "⚠️ 已在运行，重启以应用新配置".yellow());
            *running = false;
        }
    }
    // 等待旧线程退出：需覆盖最长单次休眠（过温路径 4s）
    thread::sleep(Duration::from_millis(CONTROL_SLEEP_MS + 1500));

    {
        let mut running = is_running.lock().unwrap();
        if *running {
            println!("{}", "❗ 无法重启风扇控制（状态仍为运行中）".red());
            return;
        }
        *running = true;
    }

    println!("{}", "✅ 启动风扇控制系统".green().bold());

    let is_running_clone = Arc::clone(&is_running);
    thread::spawn(move || {
        let driver = ApiFan::init();
        driver.set_fan_control();
        let mut ctrl_state = ControlState::new();

        while *is_running_clone.lock().unwrap() {
            println!("---------------------------------------------------------------");
            apply_fan_curve(&fan_data.get("left_fan"), &fan_data.get("right_fan"), &driver, &mut ctrl_state);
        }
        println!("{}", "🛑 风扇控制线程已停止".red());
        println!("---------------------------------------------------------------");
    });
}

#[tauri::command]
pub fn stop_fan_control(state: State<FanControlState>) {
    // 先释放锁，再 spawn，避免持锁期间 spawn
    {
        let mut is_running = state.is_running.lock().unwrap();
        *is_running = false;
    }
    thread::spawn(move || {
        // 等待控制线程退出后再重置，避免竞争
        thread::sleep(Duration::from_millis(CONTROL_SLEEP_MS + 1500));
        fan_reset();
    });
    println!("{}", "✅ 风扇控制已停止".green());
}
