use crate::plug::constants::{FAN_RAMP_STEP, MIN_FAN_SPEED};

pub fn ramp_speed_internal(
    cache: i64,
    target: i64,
    max_step_up: i64,
    max_step_down: i64,
    min_speed: i64,
) -> i64 {
    let min_speed = min_speed.clamp(0, 100);
    let target = target.clamp(0, 100);

    if cache == 0 {
        if target == 0 {
            0
        } else {
            target.max(min_speed)
        }
    } else if target > cache {
        let high = (cache + max_step_up).min(100);
        target.clamp(cache, high)
    } else if target < cache {
        let low = if target == 0 {
            0
        } else {
            (cache - max_step_down).max(min_speed)
        };
        target.clamp(low, cache)
    } else {
        cache
    }
}

#[allow(dead_code)]
pub fn ramp_speed(cache: i64, target: i64) -> i64 {
    ramp_speed_internal(cache, target, FAN_RAMP_STEP, FAN_RAMP_STEP, MIN_FAN_SPEED)
}
