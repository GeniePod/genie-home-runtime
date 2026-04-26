use crate::{HomeActionKind, HomeRuntime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub ok: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Error,
    Warning,
}

impl ValidationReport {
    pub fn new(issues: Vec<ValidationIssue>) -> Self {
        Self {
            ok: !issues
                .iter()
                .any(|issue| issue.severity == ValidationSeverity::Error),
            issues,
        }
    }
}

pub fn validate_runtime(runtime: &HomeRuntime) -> ValidationReport {
    let mut issues = Vec::new();

    for entity in runtime.graph().entities() {
        if let Some(device_id) = &entity.device_id
            && runtime.device(device_id).is_none()
        {
            issues.push(error(
                "entity_device_missing",
                format!(
                    "entity {} references missing device {}",
                    entity.id, device_id
                ),
            ));
        }
    }

    for scene in runtime.scenes() {
        let Some(scene_entity) = runtime.graph().get(&scene.id) else {
            issues.push(error(
                "scene_entity_missing",
                format!("scene {} has no backing scene entity", scene.id),
            ));
            continue;
        };
        if !scene_entity
            .capabilities
            .contains(&crate::Capability::SceneActivation)
        {
            issues.push(error(
                "scene_entity_missing_capability",
                format!(
                    "scene entity {} lacks scene_activation capability",
                    scene.id
                ),
            ));
        }
        if scene.actions.is_empty() {
            issues.push(warning(
                "scene_empty",
                format!("scene {} has no actions", scene.id),
            ));
        }
        for action in &scene.actions {
            if runtime.graph().get(&action.target.entity_id).is_none() {
                issues.push(error(
                    "scene_action_target_missing",
                    format!(
                        "scene {} references missing target {}",
                        scene.id, action.target.entity_id
                    ),
                ));
            }
        }
    }

    for automation in runtime.automations() {
        if automation.actions.is_empty() {
            issues.push(warning(
                "automation_empty",
                format!("automation {} has no actions", automation.id),
            ));
        }
        match &automation.trigger {
            crate::AutomationTrigger::TimeOfDay { hh_mm } if !valid_hh_mm(hh_mm) => {
                issues.push(error(
                    "automation_invalid_time",
                    format!(
                        "automation {} has invalid time trigger {}",
                        automation.id, hh_mm
                    ),
                ));
            }
            _ => {}
        }
        for action in &automation.actions {
            if runtime.graph().get(&action.target.entity_id).is_none() {
                issues.push(error(
                    "automation_action_target_missing",
                    format!(
                        "automation {} references missing target {}",
                        automation.id, action.target.entity_id
                    ),
                ));
            }
            if action.kind == HomeActionKind::ActivateScene
                && runtime
                    .scenes()
                    .all(|scene| scene.id != action.target.entity_id)
            {
                issues.push(error(
                    "automation_scene_missing",
                    format!(
                        "automation {} activates missing scene {}",
                        automation.id, action.target.entity_id
                    ),
                ));
            }
        }
    }

    ValidationReport::new(issues)
}

fn valid_hh_mm(value: &str) -> bool {
    let Some((hh, mm)) = value.split_once(':') else {
        return false;
    };
    let Ok(hh) = hh.parse::<u8>() else {
        return false;
    };
    let Ok(mm) = mm.parse::<u8>() else {
        return false;
    };
    hh < 24 && mm < 60 && value.len() == 5
}

fn error(code: &str, message: String) -> ValidationIssue {
    ValidationIssue {
        severity: ValidationSeverity::Error,
        code: code.into(),
        message,
    }
}

fn warning(code: &str, message: String) -> ValidationIssue {
    ValidationIssue {
        severity: ValidationSeverity::Warning,
        code: code.into(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Automation, AutomationTrigger, EntityId, demo_runtime};

    #[test]
    fn demo_runtime_validates_cleanly() {
        let runtime = demo_runtime();
        let report = validate_runtime(&runtime);

        assert!(report.ok);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn reports_invalid_automation_time() {
        let mut runtime = demo_runtime();
        runtime.upsert_automation(Automation::new(
            "automation.bad",
            "Bad",
            AutomationTrigger::TimeOfDay {
                hh_mm: "99:99".into(),
            },
        ));

        let report = validate_runtime(&runtime);

        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "automation_invalid_time")
        );
    }

    #[test]
    fn reports_missing_scene_action_target() {
        let mut runtime = demo_runtime();
        let missing = EntityId::new("light.missing").unwrap();
        let scene_id = EntityId::new("scene.movie_night").unwrap();
        runtime.upsert_scene(crate::Scene::new(scene_id, "Broken").with_action(
            crate::HomeAction {
                target: crate::TargetSelector::exact(missing),
                kind: crate::HomeActionKind::TurnOn,
                value: None,
            },
        ));

        let report = validate_runtime(&runtime);

        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "scene_action_target_missing")
        );
    }
}
