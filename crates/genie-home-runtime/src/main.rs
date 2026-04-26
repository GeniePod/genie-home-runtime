use anyhow::Result;
use genie_home_core::{RuntimeRequest, demo_runtime, demo_turn_on_kitchen_command};
use std::io::Read;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str).unwrap_or("status") {
        "status" => print_status()?,
        "demo" => run_demo()?,
        "entities" => list_entities()?,
        "evaluate" => handle_json_request(false)?,
        "execute" => handle_json_request(true)?,
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

COMMANDS:
    status    Print empty runtime status
    demo      Run an in-memory safety/action demo
    entities  Print demo entity graph
    evaluate  Read a HomeCommand JSON from stdin and evaluate without executing
    execute   Read a HomeCommand JSON from stdin and execute if allowed
    help      Show this help"
    );
}

fn print_status() -> Result<()> {
    let runtime = demo_runtime();
    println!("{}", serde_json::to_string_pretty(&runtime.status())?);
    Ok(())
}

fn run_demo() -> Result<()> {
    let mut runtime = demo_runtime();
    let decision = runtime.execute(demo_turn_on_kitchen_command());

    println!("{}", serde_json::to_string_pretty(&decision)?);
    Ok(())
}

fn list_entities() -> Result<()> {
    let mut runtime = demo_runtime();
    let response = runtime.handle_request(RuntimeRequest::ListEntities);
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
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
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
