use crate::plug::struct_set::FanControlMode;

pub struct ControlState {
    pub fan_cache: [i64; 2],
    pub ema_cpu: f64,
    pub ema_gpu: f64,
    pub ema_target_left: f64,
    pub ema_target_right: f64,
    pub active_mode: FanControlMode,
    pub office_cooldown_ticks: u32,
    pub gaming_hot_ticks: u32,
    pub cpu_hysteresis_anchor_temp: i64,
    pub gpu_hysteresis_anchor_temp: i64,
    pub alert_cpu_high_count: u32,
    pub alert_gpu_high_count: u32,
    pub alert_cpu_recover_count: u32,
    pub alert_gpu_recover_count: u32,
    pub cpu_alert_active: bool,
    pub gpu_alert_active: bool,
    pub force_max_fan: bool,
}

impl ControlState {
    pub fn new(mode: FanControlMode) -> Self {
        ControlState {
            fan_cache: [0i64; 2],
            ema_cpu: 0.0,
            ema_gpu: 0.0,
            ema_target_left: 0.0,
            ema_target_right: 0.0,
            active_mode: mode,
            office_cooldown_ticks: 0,
            gaming_hot_ticks: 0,
            cpu_hysteresis_anchor_temp: 0,
            gpu_hysteresis_anchor_temp: 0,
            alert_cpu_high_count: 0,
            alert_gpu_high_count: 0,
            alert_cpu_recover_count: 0,
            alert_gpu_recover_count: 0,
            cpu_alert_active: false,
            gpu_alert_active: false,
            force_max_fan: false,
        }
    }
}
