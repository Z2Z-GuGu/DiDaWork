//! 闹钟相关命令。所有写操作都会重新校验并广播最新列表。

use tauri::{AppHandle, Emitter, State};

use crate::domain::alarm::{validate, Alarm, ValidationError};
use crate::domain::ringtone;
use crate::events;
use crate::infra::id;
use crate::state::SharedState;

#[tauri::command]
pub fn list_alarms(state: State<'_, SharedState>) -> Vec<Alarm> {
    state.alarms()
}

/// 新建或更新一组闹钟：id 为空视为新建。
#[tauri::command]
pub fn save_alarm(
    app: AppHandle,
    state: State<'_, SharedState>,
    alarm: Alarm,
) -> Result<Vec<Alarm>, ValidationError> {
    let mut alarm = alarm;
    if alarm.id.trim().is_empty() {
        alarm.id = id::next("alarm");
    }
    // 归一化铃声，避免配置文件里出现不存在的 id。
    alarm.ringtone_in = ringtone::normalize(&alarm.ringtone_in).to_string();
    alarm.ringtone_out = ringtone::normalize(&alarm.ringtone_out).to_string();

    let mut list = state.alarms();
    validate(&alarm, &list)?;

    match list.iter_mut().find(|item| item.id == alarm.id) {
        Some(slot) => *slot = alarm,
        None => list.push(alarm),
    }
    Ok(persist(&app, &state, list))
}

#[tauri::command]
pub fn delete_alarm(app: AppHandle, state: State<'_, SharedState>, id: String) -> Vec<Alarm> {
    let list: Vec<Alarm> = state
        .alarms()
        .into_iter()
        .filter(|alarm| alarm.id != id)
        .collect();
    persist(&app, &state, list)
}

/// 开关某个闹钟。开启时需要重新校验重叠，否则关掉再开就能绕过约束。
#[tauri::command]
pub fn set_alarm_enabled(
    app: AppHandle,
    state: State<'_, SharedState>,
    id: String,
    enabled: bool,
) -> Result<Vec<Alarm>, ValidationError> {
    let mut list = state.alarms();
    let Some(index) = list.iter().position(|alarm| alarm.id == id) else {
        return Ok(list);
    };
    let mut candidate = list[index].clone();
    candidate.enabled = enabled;
    validate(&candidate, &list)?;
    list[index] = candidate;
    Ok(persist(&app, &state, list))
}

fn persist(app: &AppHandle, state: &SharedState, mut list: Vec<Alarm>) -> Vec<Alarm> {
    list.sort_by_key(|alarm| alarm.clock_in.minutes());
    let saved = state.replace_alarms(list);
    let _ = app.emit(events::ALARMS_CHANGED, &saved);
    saved
}
