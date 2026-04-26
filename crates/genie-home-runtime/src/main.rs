use anyhow::Result;
use genie_home_core::{
    AuditEntry, Automation, ConnectivityReport, Device, Entity, EntityId, HardwareInterface,
    HomeRuntime, MockHardwareBus, RuntimeEvent, RuntimeRequest, RuntimeResponse, RuntimeSnapshot,
    Scene, SchedulerCatchUpPolicy, SchedulerWindow, StateReport, build_home_assistant_import_plan,
    build_home_assistant_migration_report, default_hardware_inventory, default_mcp_surface,
    demo_runtime, demo_turn_on_kitchen_command, domain_support_matrix,
    mock_turn_on_thread_lamp_command, parse_home_assistant_entities_json,
    run_mock_home_assistant_port, service_specs,
};
use rusqlite::{Connection, params, types::Type};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::path::Path;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_SOCKET_PATH: &str = "/tmp/genie-home-runtime.sock";
const DEFAULT_AUDIT_LOG_PATH: &str = "/tmp/genie-home-runtime-audit.jsonl";
const DEFAULT_EVENT_LOG_PATH: &str = "/tmp/genie-home-runtime-events.jsonl";
const DEFAULT_STATE_DB_PATH: &str = "/tmp/genie-home-runtime-state.sqlite3";

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str).unwrap_or("status") {
        "status" => print_status()?,
        "validate" => validate_demo_runtime()?,
        "demo" => run_demo()?,
        "entities" => list_entities()?,
        "devices" => list_devices()?,
        "services" => list_services()?,
        "domains" => list_domains()?,
        "hardware" => print_hardware_inventory()?,
        "events" => list_demo_events()?,
        "scenes" => list_scenes()?,
        "snapshot" => print_snapshot()?,
        "restore-snapshot" => handle_restore_snapshot()?,
        "automations" => list_automations()?,
        "automation-tick" => {
            run_automation_tick(args.get(2).map(String::as_str).unwrap_or("23:00"))?
        }
        "scheduler-window" => run_scheduler_window(
            args.get(2).map(String::as_str).unwrap_or("22:58"),
            args.get(3).map(String::as_str).unwrap_or("23:01"),
        )?,
        "evaluate" => handle_json_request(false)?,
        "execute" => handle_json_request(true)?,
        "call-service" => handle_service_call()?,
        "upsert-scene" => handle_upsert_scene()?,
        "delete-scene" => handle_delete_scene(args.get(2).map(String::as_str))?,
        "upsert-automation" => handle_upsert_automation()?,
        "delete-automation" => handle_delete_automation(args.get(2).map(String::as_str))?,
        "apply-state-report" => handle_state_report()?,
        "mock-hardware-demo" => print_mock_hardware_demo()?,
        "ha-mock-port-demo" => print_ha_mock_port_demo()?,
        "connectivity-demo" => print_connectivity_demo()?,
        "ha-compat-report" => print_ha_compat_report(args.get(2).map(String::as_str))?,
        "ha-import-plan" => print_ha_import_plan(args.get(2).map(String::as_str))?,
        "mcp-manifest" => print_mcp_manifest()?,
        "mcp-stdio" => serve_mcp_stdio()?,
        "support-bundle" => print_support_bundle(
            args.get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_AUDIT_LOG_PATH),
            args.get(3)
                .map(String::as_str)
                .unwrap_or(DEFAULT_STATE_DB_PATH),
            args.get(4)
                .map(String::as_str)
                .unwrap_or(DEFAULT_EVENT_LOG_PATH),
        )?,
        "serve" => serve(
            args.get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_SOCKET_PATH),
            args.get(3)
                .map(String::as_str)
                .unwrap_or(DEFAULT_AUDIT_LOG_PATH),
            args.get(4)
                .map(String::as_str)
                .unwrap_or(DEFAULT_STATE_DB_PATH),
            args.get(5)
                .map(String::as_str)
                .unwrap_or(DEFAULT_EVENT_LOG_PATH),
        )?,
        "request" => request(
            args.get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_SOCKET_PATH),
        )?,
        "help" | "--help" | "-h" => print_help(),
        other => {
            anyhow::bail!("unknown command: {other}");
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        "\
genie-home-runtime

USAGE:
    genie-home-runtime <COMMAND>
    genie-home-runtime ha-compat-report [HA_STATES_JSON|-]
    genie-home-runtime ha-import-plan [HA_STATES_JSON|-]
    genie-home-runtime support-bundle [AUDIT_LOG] [STATE_DB] [EVENT_LOG]
    genie-home-runtime serve [SOCKET] [AUDIT_LOG] [STATE_DB] [EVENT_LOG]
    genie-home-runtime request [SOCKET]

COMMANDS:
    status    Print demo runtime status
    validate  Validate demo runtime invariants
    demo      Run an in-memory safety/action demo
    devices   Print demo device registry
    entities  Print demo entity graph
    services  Print supported HA-style domain services
    domains   Print implemented and planned home domain support
    hardware  Print runtime hardware/protocol support boundaries
    events    Print demo runtime events
    scenes    Print demo scenes
    snapshot  Print a versioned demo runtime snapshot
    restore-snapshot  Read a RuntimeSnapshot JSON from stdin and validate/restore it
    automations  Print demo automations
    automation-tick  Run demo automations for HH:MM
    scheduler-window  Run catch-up scheduler window FROM_HH:MM TO_HH:MM
    evaluate  Read a HomeCommand JSON from stdin and evaluate without executing
    execute   Read a HomeCommand JSON from stdin and execute if allowed
    call-service  Read a ServiceCall JSON from stdin and execute if allowed
    upsert-scene  Read a Scene JSON from stdin and validate/install it
    delete-scene  Delete a scene definition by entity id
    upsert-automation  Read an Automation JSON from stdin and validate/install it
    delete-automation  Delete an automation definition by id
    apply-state-report  Read a StateReport JSON from stdin and apply entity states
    mock-hardware-demo  Run a deterministic mock hardware discovery/state/action demo
    ha-mock-port-demo  Run mock hardware through Home Assistant migration/import
    connectivity-demo  Print a sample GenieOS connectivity report request
    ha-compat-report  Print a Home Assistant migration compatibility report
    ha-import-plan  Print a Genie connectivity import plan from Home Assistant states
    mcp-manifest  Print the local MCP-facing tool/resource manifest
    mcp-stdio  Serve a local JSON-RPC MCP-style stdio bridge
    support-bundle  Print local JSON diagnostics for support
    serve     Serve RuntimeRequest JSON over a Unix socket
    request   Send RuntimeRequest JSON from stdin to a Unix socket
    help      Show this help"
    );
}

fn print_status() -> Result<()> {
    let runtime = demo_runtime();
    print_stdout_line(&serde_json::to_string_pretty(&runtime.status())?)
}

fn validate_demo_runtime() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::Validate);
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn run_demo() -> Result<()> {
    let mut runtime = demo_runtime();
    let decision = runtime.execute(demo_turn_on_kitchen_command());

    print_stdout_line(&serde_json::to_string_pretty(&decision)?)
}

fn list_entities() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ListEntities);
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn list_devices() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ListDevices);
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn list_services() -> Result<()> {
    print_stdout_line(&serde_json::to_string_pretty(&service_specs())?)
}

fn list_domains() -> Result<()> {
    let response = RuntimeResponse::Domains {
        domains: domain_support_matrix(),
    };
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn print_hardware_inventory() -> Result<()> {
    let response = RuntimeResponse::HardwareInventory {
        inventory: default_hardware_inventory(),
    };
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn list_demo_events() -> Result<()> {
    let mut runtime = demo_runtime();
    runtime.execute(demo_turn_on_kitchen_command());
    let response = runtime.handle_request(RuntimeRequest::Events { limit: Some(20) });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn list_scenes() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ListScenes);
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn print_snapshot() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ExportSnapshot);
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_restore_snapshot() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let snapshot: RuntimeSnapshot = serde_json::from_str(&input)?;
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ImportSnapshot { snapshot });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_service_call() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let call = serde_json::from_str(&input)?;
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::CallService { call });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_upsert_scene() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let scene = serde_json::from_str(&input)?;
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::UpsertScene { scene });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_delete_scene(scene_id: Option<&str>) -> Result<()> {
    let Some(scene_id) = scene_id else {
        anyhow::bail!("delete-scene requires a scene entity id")
    };
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::DeleteScene {
        scene_id: EntityId::new(scene_id)?,
    });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_upsert_automation() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let automation = serde_json::from_str(&input)?;
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::UpsertAutomation { automation });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_delete_automation(automation_id: Option<&str>) -> Result<()> {
    let Some(automation_id) = automation_id else {
        anyhow::bail!("delete-automation requires an automation id")
    };
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::DeleteAutomation {
        automation_id: automation_id.into(),
    });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_state_report() -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let report: StateReport = serde_json::from_str(&input)?;
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ApplyStateReport { report });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn print_mock_hardware_demo() -> Result<()> {
    let mut hardware = MockHardwareBus::reference_home();
    let discovery_report = hardware.discovery_report();
    let initial_state_report = hardware.poll_state();
    let command = mock_turn_on_thread_lamp_command();
    let command_result = hardware.apply_command(&command);
    let final_state_report = command_result.state_report.clone();

    print_stdout_line(&serde_json::to_string_pretty(&serde_json::json!({
        "schema": "genie.home.mock_hardware_demo.v1",
        "source": hardware.source(),
        "devices": hardware.entities().count(),
        "discovery_request": RuntimeRequest::ApplyConnectivityReport {
            report: discovery_report,
        },
        "initial_state_request": RuntimeRequest::ApplyStateReport {
            report: initial_state_report,
        },
        "command": command,
        "command_result": command_result,
        "final_state_request": final_state_report.map(|report| RuntimeRequest::ApplyStateReport {
            report,
        }),
    }))?)
}

fn print_ha_mock_port_demo() -> Result<()> {
    print_stdout_line(&serde_json::to_string_pretty(
        &run_mock_home_assistant_port(),
    )?)
}

fn list_automations() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ListAutomations);
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn run_automation_tick(now_hh_mm: &str) -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::RunAutomationTick {
        now_hh_mm: now_hh_mm.into(),
    });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn run_scheduler_window(from_hh_mm: &str, to_hh_mm: &str) -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::RunSchedulerWindow {
        window: SchedulerWindow {
            from_hh_mm: from_hh_mm.into(),
            to_hh_mm: to_hh_mm.into(),
        },
        policy: SchedulerCatchUpPolicy::default(),
    });
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn handle_json_request(execute: bool) -> Result<()> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let command = serde_json::from_str(&input)?;
    let request = if execute {
        RuntimeRequest::Execute { command }
    } else {
        RuntimeRequest::Evaluate { command }
    };
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(request);
    print_stdout_line(&serde_json::to_string_pretty(&response)?)
}

fn print_support_bundle(
    audit_log_path: &str,
    state_db_path: &str,
    event_log_path: &str,
) -> Result<()> {
    let bundle = build_support_bundle(audit_log_path, state_db_path, event_log_path)?;
    print_stdout_line(&serde_json::to_string_pretty(&bundle)?)
}

fn print_ha_compat_report(path: Option<&str>) -> Result<()> {
    let input = read_path_or_stdin(path)?;
    let records = parse_home_assistant_entities_json(&input).map_err(anyhow::Error::msg)?;
    let report = build_home_assistant_migration_report(records);
    print_stdout_line(&serde_json::to_string_pretty(&report)?)
}

fn print_ha_import_plan(path: Option<&str>) -> Result<()> {
    let input = read_path_or_stdin(path)?;
    let records = parse_home_assistant_entities_json(&input).map_err(anyhow::Error::msg)?;
    let plan = build_home_assistant_import_plan(records);
    print_stdout_line(&serde_json::to_string_pretty(&plan)?)
}

fn print_connectivity_demo() -> Result<()> {
    let report = ConnectivityReport::esp32c6_thread_demo()?;
    let request = RuntimeRequest::ApplyConnectivityReport { report };
    print_stdout_line(&serde_json::to_string_pretty(&request)?)
}

fn print_mcp_manifest() -> Result<()> {
    print_stdout_line(&serde_json::to_string_pretty(&default_mcp_surface())?)
}

fn serve_mcp_stdio() -> Result<()> {
    let stdin = std::io::stdin();
    let mut runtime = demo_runtime();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_mcp_stdio_message(&mut runtime, &line);
        print_stdout_line(&serde_json::to_string(&response)?)?;
    }
    Ok(())
}

fn handle_mcp_stdio_message(runtime: &mut HomeRuntime, input: &str) -> serde_json::Value {
    let Ok(request) = serde_json::from_str::<serde_json::Value>(input) else {
        return mcp_error(serde_json::Value::Null, -32700, "parse error");
    };
    let id = request
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let method = request
        .get("method")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let params = request
        .get("params")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    match method {
        "initialize" => mcp_result(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "genie-home-runtime",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {},
                    "resources": {}
                }
            }),
        ),
        "tools/list" => mcp_result(
            id,
            serde_json::json!({
                "tools": default_mcp_surface().tools
            }),
        ),
        "resources/list" => mcp_result(
            id,
            serde_json::json!({
                "resources": default_mcp_surface().resources
            }),
        ),
        "tools/call" => match mcp_tool_to_runtime_request(&params) {
            Ok(request) => {
                let response = runtime.handle_request(request);
                mcp_result(
                    id,
                    serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string(&response).unwrap_or_else(|err| {
                                serde_json::json!({"type":"error","error":err.to_string()}).to_string()
                            })
                        }],
                        "structuredContent": response
                    }),
                )
            }
            Err(err) => mcp_error(id, -32602, &err),
        },
        _ => mcp_error(id, -32601, "method not found"),
    }
}

fn mcp_result(id: serde_json::Value, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn mcp_error(id: serde_json::Value, code: i64, message: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn mcp_tool_to_runtime_request(params: &serde_json::Value) -> Result<RuntimeRequest, String> {
    let name = params
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "tools/call requires params.name".to_string())?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    match name {
        "home.status" => Ok(RuntimeRequest::Status),
        "home.validate" => Ok(RuntimeRequest::Validate),
        "home.list_entities" => Ok(RuntimeRequest::ListEntities),
        "home.list_devices" => Ok(RuntimeRequest::ListDevices),
        "home.list_services" => Ok(RuntimeRequest::ListServices),
        "home.list_domains" => Ok(RuntimeRequest::ListDomains),
        "home.hardware_inventory" => Ok(RuntimeRequest::HardwareInventory),
        "home.list_scenes" => Ok(RuntimeRequest::ListScenes),
        "home.list_automations" => Ok(RuntimeRequest::ListAutomations),
        "home.audit" => Ok(RuntimeRequest::Audit {
            limit: arguments
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
        }),
        "home.events" => Ok(RuntimeRequest::Events {
            limit: arguments
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as usize),
        }),
        "home.evaluate" => Ok(RuntimeRequest::Evaluate {
            command: required_argument(arguments, "command")?,
        }),
        "home.execute" => Ok(RuntimeRequest::Execute {
            command: required_argument(arguments, "command")?,
        }),
        "home.call_service" => Ok(RuntimeRequest::CallService {
            call: required_argument(arguments, "call")?,
        }),
        "home.upsert_scene" => Ok(RuntimeRequest::UpsertScene {
            scene: required_argument(arguments, "scene")?,
        }),
        "home.delete_scene" => Ok(RuntimeRequest::DeleteScene {
            scene_id: required_argument(arguments, "scene_id")?,
        }),
        "home.upsert_automation" => Ok(RuntimeRequest::UpsertAutomation {
            automation: required_argument(arguments, "automation")?,
        }),
        "home.delete_automation" => Ok(RuntimeRequest::DeleteAutomation {
            automation_id: required_argument(arguments, "automation_id")?,
        }),
        "home.apply_connectivity_report" => Ok(RuntimeRequest::ApplyConnectivityReport {
            report: required_argument(arguments, "report")?,
        }),
        "home.apply_state_report" => Ok(RuntimeRequest::ApplyStateReport {
            report: required_argument(arguments, "report")?,
        }),
        "home.run_automation_tick" => Ok(RuntimeRequest::RunAutomationTick {
            now_hh_mm: required_argument(arguments, "now_hh_mm")?,
        }),
        _ => Err(format!("unsupported tool: {name}")),
    }
}

fn required_argument<T: serde::de::DeserializeOwned>(
    arguments: serde_json::Value,
    key: &str,
) -> Result<T, String> {
    let value = arguments
        .get(key)
        .cloned()
        .ok_or_else(|| format!("missing required argument: {key}"))?;
    serde_json::from_value(value).map_err(|err| format!("invalid argument {key}: {err}"))
}

fn read_path_or_stdin(path: Option<&str>) -> Result<String> {
    match path {
        Some("-") | None => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            Ok(input)
        }
        Some(path) => Ok(std::fs::read_to_string(path)?),
    }
}

fn print_stdout_line(output: &str) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    if let Err(err) = stdout
        .write_all(output.as_bytes())
        .and_then(|_| stdout.write_all(b"\n"))
    {
        if err.kind() == ErrorKind::BrokenPipe {
            return Ok(());
        }
        return Err(err.into());
    }
    Ok(())
}

#[cfg(unix)]
fn serve(
    socket_path: &str,
    audit_log_path: &str,
    state_db_path: &str,
    event_log_path: &str,
) -> Result<()> {
    use std::os::unix::net::UnixListener;

    let path = Path::new(socket_path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let listener = UnixListener::bind(path)?;
    let mut runtime = demo_runtime();
    let mut state_store = SqliteStateStore::open(state_db_path)?;
    let restored_devices = state_store.load_devices()?;
    if restored_devices.is_empty() {
        state_store.save_devices(runtime.devices())?;
    } else {
        for device in restored_devices {
            runtime.upsert_device(device);
        }
    }
    let restored_entities = state_store.load_entities()?;
    if restored_entities.is_empty() {
        state_store.save_entities(runtime.graph().entities())?;
    } else {
        for entity in restored_entities {
            runtime.upsert_entity(entity);
        }
    }
    let restored_scenes = state_store.load_scenes()?;
    if restored_scenes.is_empty() {
        state_store.save_scenes(runtime.scenes())?;
    } else {
        for scene in restored_scenes {
            runtime.upsert_scene(scene);
        }
    }
    let restored_automations = state_store.load_automations()?;
    if restored_automations.is_empty() {
        state_store.save_automations(runtime.automations())?;
    } else {
        for automation in restored_automations {
            runtime.upsert_automation(automation);
        }
    }
    let restored = load_audit_entries(audit_log_path)?;
    runtime.restore_audit_entries(restored);
    let restored_events = load_event_entries(event_log_path)?;
    runtime.restore_events(restored_events);
    eprintln!("genie-home-runtime listening on {}", path.display());
    eprintln!("audit log: {}", audit_log_path);
    eprintln!("event log: {}", event_log_path);
    eprintln!("state db: {}", state_db_path);
    eprintln!("entity count: {}", runtime.graph().len());
    eprintln!("restored audit entries: {}", runtime.audit_len());
    eprintln!("restored runtime events: {}", runtime.event_len());

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut input = String::new();
                if let Err(err) = stream.read_to_string(&mut input) {
                    eprintln!("request read error: {err}");
                    continue;
                }
                let audit_start = runtime.audit_len();
                let event_start = runtime.event_len();
                let response = handle_runtime_request(&mut runtime, &input);
                if response_persists_entities(&response) {
                    state_store.save_devices(runtime.devices())?;
                    state_store.save_entities(runtime.graph().entities())?;
                    state_store.save_scenes(runtime.scenes())?;
                    state_store.save_automations(runtime.automations())?;
                }
                let output = serialize_runtime_response(&response);
                append_new_audit_entries(audit_log_path, runtime.audit_since(audit_start))?;
                append_new_event_entries(event_log_path, runtime.events_since(event_start))?;
                if let Err(err) = write_socket_response(&mut stream, &output) {
                    eprintln!("response write error: {err}");
                }
            }
            Err(err) => {
                eprintln!("connection error: {err}");
            }
        }
    }

    Ok(())
}

fn write_socket_response(stream: &mut impl Write, output: &str) -> Result<()> {
    match stream
        .write_all(output.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
    {
        Ok(()) => Ok(()),
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::BrokenPipe | ErrorKind::ConnectionReset
            ) =>
        {
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

#[cfg(not(unix))]
fn serve(
    _socket_path: &str,
    _audit_log_path: &str,
    _state_db_path: &str,
    _event_log_path: &str,
) -> Result<()> {
    anyhow::bail!("Unix socket runtime API is only supported on Unix targets")
}

#[cfg(unix)]
fn request(socket_path: &str) -> Result<()> {
    use std::os::unix::net::UnixStream;

    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(input.as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut output = String::new();
    stream.read_to_string(&mut output)?;
    print!("{output}");
    Ok(())
}

#[cfg(not(unix))]
fn request(_socket_path: &str) -> Result<()> {
    anyhow::bail!("Unix socket runtime API is only supported on Unix targets")
}

fn append_new_audit_entries(path: &str, entries: &[AuditEntry]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    let path = Path::new(path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for entry in entries {
        serde_json::to_writer(&mut file, entry)?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn append_new_event_entries(path: &str, entries: &[RuntimeEvent]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    let path = Path::new(path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for entry in entries {
        serde_json::to_writer(&mut file, entry)?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn load_audit_entries(path: &str) -> Result<Vec<AuditEntry>> {
    let path = Path::new(path);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry = serde_json::from_str(line)
            .map_err(|err| anyhow::anyhow!("invalid audit log line {}: {err}", index + 1))?;
        entries.push(entry);
    }
    Ok(entries)
}

fn load_event_entries(path: &str) -> Result<Vec<RuntimeEvent>> {
    let path = Path::new(path);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry = serde_json::from_str(line)
            .map_err(|err| anyhow::anyhow!("invalid event log line {}: {err}", index + 1))?;
        entries.push(entry);
    }
    Ok(entries)
}

fn build_support_bundle(
    audit_log_path: &str,
    state_db_path: &str,
    event_log_path: &str,
) -> Result<serde_json::Value> {
    let audit = load_audit_entries(audit_log_path)?;
    let events = load_event_entries(event_log_path)?;
    let devices = load_devices_from_state_db(state_db_path)?;
    let entities = load_entities_from_state_db(state_db_path)?;
    let scenes = load_scenes_from_state_db(state_db_path)?;
    let automations = load_automations_from_state_db(state_db_path)?;
    let hardware = default_hardware_inventory();
    let domains = domain_support_matrix();
    let mut recent_audit = audit.iter().rev().take(20).cloned().collect::<Vec<_>>();
    recent_audit.reverse();
    let mut recent_events = events.iter().rev().take(50).cloned().collect::<Vec<_>>();
    recent_events.reverse();
    let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;

    Ok(serde_json::json!({
        "schema": "genie.home.support_bundle.v1",
        "generated_at": generated_at,
        "runtime": {
            "package": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        },
        "paths": {
            "audit_log": audit_log_path,
            "audit_log_exists": Path::new(audit_log_path).exists(),
            "event_log": event_log_path,
            "event_log_exists": Path::new(event_log_path).exists(),
            "state_db": state_db_path,
            "state_db_exists": Path::new(state_db_path).exists(),
        },
        "counts": {
            "devices": devices.len(),
            "entities": entities.len(),
            "scenes": scenes.len(),
            "automations": automations.len(),
            "audit_entries": audit.len(),
            "recent_audit_entries": recent_audit.len(),
            "event_entries": events.len(),
            "recent_event_entries": recent_events.len(),
        },
        "devices": devices,
        "entities": entities,
        "scenes": scenes,
        "automations": automations,
        "domains": domains,
        "hardware": hardware,
        "recent_audit": recent_audit,
        "recent_events": recent_events,
    }))
}

fn load_devices_from_state_db(path: &str) -> Result<Vec<Device>> {
    if !Path::new(path).exists() {
        return Ok(Vec::new());
    }
    SqliteStateStore::open(path)?.load_devices()
}

fn load_scenes_from_state_db(path: &str) -> Result<Vec<Scene>> {
    if !Path::new(path).exists() {
        return Ok(Vec::new());
    }
    SqliteStateStore::open(path)?.load_scenes()
}

fn load_automations_from_state_db(path: &str) -> Result<Vec<Automation>> {
    if !Path::new(path).exists() {
        return Ok(Vec::new());
    }
    SqliteStateStore::open(path)?.load_automations()
}

fn load_entities_from_state_db(path: &str) -> Result<Vec<Entity>> {
    if !Path::new(path).exists() {
        return Ok(Vec::new());
    }
    SqliteStateStore::open(path)?.load_entities()
}

fn handle_runtime_request(
    runtime: &mut genie_home_core::HomeRuntime,
    input: &str,
) -> RuntimeResponse {
    match serde_json::from_str::<RuntimeRequest>(input) {
        Ok(request) => runtime.handle_request(request),
        Err(err) => RuntimeResponse::Error {
            error: format!("invalid runtime request: {err}"),
        },
    }
}

fn serialize_runtime_response(response: &RuntimeResponse) -> String {
    serde_json::to_string(response).unwrap_or_else(|err| {
        serde_json::json!({
            "type": "error",
            "error": format!("failed to serialize runtime response: {err}")
        })
        .to_string()
    })
}

fn response_persists_entities(response: &RuntimeResponse) -> bool {
    matches!(response, RuntimeResponse::Command { result } if result.executed)
        || matches!(response, RuntimeResponse::ServiceCall { result } if result.executed > 0)
        || matches!(response, RuntimeResponse::ConfigChanged { result } if result.changed)
        || matches!(response, RuntimeResponse::SnapshotApplied { result } if result.changed)
        || matches!(response, RuntimeResponse::ConnectivityApplied { result } if result.entities_upserted > 0)
        || matches!(response, RuntimeResponse::StateApplied { result } if result.entities_updated > 0)
        || matches!(response, RuntimeResponse::AutomationTick { result } if result.actions_executed > 0)
}

struct SqliteStateStore {
    conn: Connection,
}

impl SqliteStateStore {
    fn open(path: &str) -> Result<Self> {
        let path = Path::new(path);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            "\
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY NOT NULL,
                json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY NOT NULL,
                json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS scenes (
                id TEXT PRIMARY KEY NOT NULL,
                json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS automations (
                id TEXT PRIMARY KEY NOT NULL,
                json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(Self { conn })
    }

    fn load_devices(&self) -> Result<Vec<Device>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM devices ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            serde_json::from_str(&json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
            })
        })?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(row?);
        }
        Ok(devices)
    }

    fn load_scenes(&self) -> Result<Vec<Scene>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM scenes ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            serde_json::from_str(&json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
            })
        })?;

        let mut scenes = Vec::new();
        for row in rows {
            scenes.push(row?);
        }
        Ok(scenes)
    }

    fn load_automations(&self) -> Result<Vec<Automation>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM automations ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            serde_json::from_str(&json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
            })
        })?;

        let mut automations = Vec::new();
        for row in rows {
            automations.push(row?);
        }
        Ok(automations)
    }

    fn load_entities(&self) -> Result<Vec<Entity>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM entities ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            serde_json::from_str(&json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    fn save_devices<'a>(&mut self, devices: impl IntoIterator<Item = &'a Device>) -> Result<()> {
        let updated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let tx = self.conn.transaction()?;
        for device in devices {
            let json = serde_json::to_string(device)?;
            tx.execute(
                "\
                INSERT INTO devices (id, json, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    json = excluded.json,
                    updated_at = excluded.updated_at
                ",
                params![device.id.as_str(), json, updated_at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn save_scenes<'a>(&mut self, scenes: impl IntoIterator<Item = &'a Scene>) -> Result<()> {
        let updated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let tx = self.conn.transaction()?;
        for scene in scenes {
            let json = serde_json::to_string(scene)?;
            tx.execute(
                "\
                INSERT INTO scenes (id, json, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    json = excluded.json,
                    updated_at = excluded.updated_at
                ",
                params![scene.id.as_str(), json, updated_at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn save_automations<'a>(
        &mut self,
        automations: impl IntoIterator<Item = &'a Automation>,
    ) -> Result<()> {
        let updated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let tx = self.conn.transaction()?;
        for automation in automations {
            let json = serde_json::to_string(automation)?;
            tx.execute(
                "\
                INSERT INTO automations (id, json, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    json = excluded.json,
                    updated_at = excluded.updated_at
                ",
                params![automation.id, json, updated_at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn save_entities<'a>(&mut self, entities: impl IntoIterator<Item = &'a Entity>) -> Result<()> {
        let updated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let tx = self.conn.transaction()?;
        for entity in entities {
            let json = serde_json::to_string(entity)?;
            tx.execute(
                "\
                INSERT INTO entities (id, json, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    json = excluded.json,
                    updated_at = excluded.updated_at
                ",
                params![entity.id.as_str(), json, updated_at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genie_home_core::EntityState;

    fn temp_db_path(name: &str) -> String {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "genie-home-runtime-{name}-{}-{}.sqlite3",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn sqlite_state_store_persists_entities() {
        let path = temp_db_path("entities");
        let mut store = SqliteStateStore::open(&path).unwrap();
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());

        store.save_devices(runtime.devices()).unwrap();
        store.save_entities(runtime.graph().entities()).unwrap();
        store.save_scenes(runtime.scenes()).unwrap();
        store.save_automations(runtime.automations()).unwrap();
        let devices = store.load_devices().unwrap();
        let entities = store.load_entities().unwrap();
        let scenes = store.load_scenes().unwrap();
        let automations = store.load_automations().unwrap();
        let kitchen = entities
            .iter()
            .find(|entity| entity.id.as_str() == "light.kitchen")
            .unwrap();

        assert_eq!(devices.len(), 2);
        assert_eq!(scenes.len(), 1);
        assert_eq!(automations.len(), 1);
        assert_eq!(kitchen.state, EntityState::On);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn runtime_request_handler_keeps_error_shape() {
        let mut runtime = demo_runtime();
        let response = handle_runtime_request(&mut runtime, "{not json");

        let RuntimeResponse::Error { error } = response else {
            panic!("expected error response");
        };
        assert!(error.contains("invalid runtime request"));
    }

    #[test]
    fn support_bundle_reports_counts_without_network() {
        let db_path = temp_db_path("support");
        let audit_path = temp_db_path("support-audit-jsonl");
        let event_path = temp_db_path("support-event-jsonl");
        let mut store = SqliteStateStore::open(&db_path).unwrap();
        let mut runtime = demo_runtime();
        runtime.execute(demo_turn_on_kitchen_command());
        store.save_devices(runtime.devices()).unwrap();
        store.save_entities(runtime.graph().entities()).unwrap();
        store.save_scenes(runtime.scenes()).unwrap();
        store.save_automations(runtime.automations()).unwrap();
        append_new_audit_entries(&audit_path, runtime.audit()).unwrap();
        append_new_event_entries(&event_path, runtime.events()).unwrap();

        let bundle = build_support_bundle(&audit_path, &db_path, &event_path).unwrap();

        assert_eq!(bundle["schema"], "genie.home.support_bundle.v1");
        assert_eq!(bundle["counts"]["devices"], 2);
        assert_eq!(bundle["counts"]["entities"], 3);
        assert_eq!(bundle["counts"]["scenes"], 1);
        assert_eq!(bundle["counts"]["automations"], 1);
        assert_eq!(bundle["counts"]["audit_entries"], 1);
        assert_eq!(bundle["counts"]["event_entries"], 1);
        assert_eq!(bundle["recent_audit"].as_array().unwrap().len(), 1);
        assert_eq!(bundle["recent_events"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_file(db_path);
        let _ = std::fs::remove_file(audit_path);
        let _ = std::fs::remove_file(event_path);
    }

    #[test]
    fn ha_compat_report_maps_common_domains() {
        let input = r#"[
            {"entity_id":"light.kitchen","state":"on","attributes":{"friendly_name":"Kitchen Light"}},
            {"entity_id":"climate.hallway","state":"70","attributes":{}},
            {"entity_id":"camera.driveway","state":"streaming","attributes":{}}
        ]"#;

        let records = parse_home_assistant_entities_json(input).unwrap();
        let report = build_home_assistant_migration_report(records);

        assert_eq!(report.counts.total, 3);
        assert_eq!(report.counts.mappable, 1);
        assert_eq!(report.counts.manual_review, 1);
        assert_eq!(report.counts.unsupported, 1);
    }

    #[test]
    fn connectivity_apply_response_triggers_state_persistence() {
        let response = RuntimeResponse::ConnectivityApplied {
            result: genie_home_core::ConnectivityApplyResult {
                source: "test".into(),
                devices_seen: 1,
                entities_upserted: 1,
            },
        };

        assert!(response_persists_entities(&response));
    }

    #[test]
    fn state_apply_response_triggers_state_persistence() {
        let response = RuntimeResponse::StateApplied {
            result: genie_home_core::StateApplyResult {
                source: "test".into(),
                updates_seen: 1,
                entities_updated: 1,
                unknown_entities: Vec::new(),
            },
        };

        assert!(response_persists_entities(&response));
    }

    #[test]
    fn automation_tick_response_triggers_state_persistence() {
        let response = RuntimeResponse::AutomationTick {
            result: genie_home_core::AutomationTickResult {
                now_hh_mm: "23:00".into(),
                automations_checked: 1,
                automations_triggered: 1,
                actions_executed: 1,
                blocked: Vec::new(),
            },
        };

        assert!(response_persists_entities(&response));
    }

    #[test]
    fn service_call_response_triggers_state_persistence() {
        let response = RuntimeResponse::ServiceCall {
            result: genie_home_core::ServiceCallResult {
                domain: "light".into(),
                service: "turn_on".into(),
                targets: 1,
                executed: 1,
                results: Vec::new(),
            },
        };

        assert!(response_persists_entities(&response));
    }

    #[test]
    fn config_change_response_triggers_state_persistence() {
        let response = RuntimeResponse::ConfigChanged {
            result: genie_home_core::ConfigChangeResult {
                resource: genie_home_core::ConfigResource::Automation,
                id: "automation.test".into(),
                changed: true,
                validation: None,
            },
        };

        assert!(response_persists_entities(&response));
    }

    #[test]
    fn snapshot_apply_response_triggers_state_persistence() {
        let response = RuntimeResponse::SnapshotApplied {
            result: genie_home_core::SnapshotApplyResult {
                changed: true,
                devices: 1,
                entities: 1,
                scenes: 0,
                automations: 0,
                validation: genie_home_core::ValidationReport {
                    ok: true,
                    issues: Vec::new(),
                },
            },
        };

        assert!(response_persists_entities(&response));
    }

    #[test]
    fn mcp_manifest_has_home_tools() {
        let surface = default_mcp_surface();

        assert!(surface.tools.iter().any(|tool| tool.name == "home.status"));
        assert!(surface.tools.iter().any(|tool| tool.name == "home.execute"));
        assert!(
            surface
                .tools
                .iter()
                .any(|tool| tool.name == "home.upsert_scene")
        );
    }

    #[test]
    fn mcp_stdio_lists_tools_and_calls_status() {
        let mut runtime = demo_runtime();
        let list = handle_mcp_stdio_message(
            &mut runtime,
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#,
        );
        let call = handle_mcp_stdio_message(
            &mut runtime,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"home.status","arguments":{}}}"#,
        );

        assert_eq!(list["jsonrpc"], "2.0");
        assert!(
            list["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|tool| tool["name"] == "home.status")
        );
        assert_eq!(call["result"]["structuredContent"]["type"], "status");
    }
}
