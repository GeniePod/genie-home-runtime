use crate::AutomationTickResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerWindow {
    pub from_hh_mm: String,
    pub to_hh_mm: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerCatchUpPolicy {
    pub mode: SchedulerCatchUpMode,
    pub max_ticks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerCatchUpMode {
    SkipMissed,
    RunDueTicks,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerRunResult {
    pub window: SchedulerWindow,
    pub policy: SchedulerCatchUpPolicy,
    pub ticks_checked: usize,
    pub tick_results: Vec<AutomationTickResult>,
}

impl Default for SchedulerCatchUpPolicy {
    fn default() -> Self {
        Self {
            mode: SchedulerCatchUpMode::RunDueTicks,
            max_ticks: 120,
        }
    }
}

pub fn enumerate_hh_mm_window(
    window: &SchedulerWindow,
    max_ticks: usize,
) -> Result<Vec<String>, SchedulerWindowError> {
    if max_ticks == 0 {
        return Ok(Vec::new());
    }
    let from = parse_hh_mm(&window.from_hh_mm)?;
    let to = parse_hh_mm(&window.to_hh_mm)?;
    let mut cursor = from;
    let mut ticks = Vec::new();
    for _ in 0..1440 {
        cursor = (cursor + 1) % 1440;
        ticks.push(format_hh_mm(cursor));
        if cursor == to || ticks.len() >= max_ticks {
            break;
        }
    }
    Ok(ticks)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchedulerWindowError {
    #[error("invalid HH:MM value: {0}")]
    InvalidTime(String),
}

fn parse_hh_mm(value: &str) -> Result<u16, SchedulerWindowError> {
    let Some((hh, mm)) = value.split_once(':') else {
        return Err(SchedulerWindowError::InvalidTime(value.into()));
    };
    let Ok(hh) = hh.parse::<u16>() else {
        return Err(SchedulerWindowError::InvalidTime(value.into()));
    };
    let Ok(mm) = mm.parse::<u16>() else {
        return Err(SchedulerWindowError::InvalidTime(value.into()));
    };
    if hh >= 24 || mm >= 60 || value.len() != 5 {
        return Err(SchedulerWindowError::InvalidTime(value.into()));
    }
    Ok(hh * 60 + mm)
}

fn format_hh_mm(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerates_same_day_window_exclusive_start_inclusive_end() {
        let ticks = enumerate_hh_mm_window(
            &SchedulerWindow {
                from_hh_mm: "22:58".into(),
                to_hh_mm: "23:00".into(),
            },
            10,
        )
        .unwrap();

        assert_eq!(ticks, vec!["22:59", "23:00"]);
    }

    #[test]
    fn enumerates_midnight_wrap() {
        let ticks = enumerate_hh_mm_window(
            &SchedulerWindow {
                from_hh_mm: "23:58".into(),
                to_hh_mm: "00:01".into(),
            },
            10,
        )
        .unwrap();

        assert_eq!(ticks, vec!["23:59", "00:00", "00:01"]);
    }

    #[test]
    fn respects_tick_limit() {
        let ticks = enumerate_hh_mm_window(
            &SchedulerWindow {
                from_hh_mm: "10:00".into(),
                to_hh_mm: "11:00".into(),
            },
            2,
        )
        .unwrap();

        assert_eq!(ticks, vec!["10:01", "10:02"]);
    }
}
