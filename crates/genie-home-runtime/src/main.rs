use anyhow::Result;
use genie_home_core::{
    Capability, CommandOrigin, Entity, EntityId, EntityState, HomeAction, HomeActionKind,
    HomeCommand, HomeRuntime, TargetSelector,
};

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str).unwrap_or("status") {
        "status" => print_status()?,
        "demo" => run_demo()?,
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
    help      Show this help"
    );
}

fn print_status() -> Result<()> {
    let runtime = HomeRuntime::with_default_policy();
    println!("{}", serde_json::to_string_pretty(&runtime.status())?);
    Ok(())
}

fn run_demo() -> Result<()> {
    let id = EntityId::new("light.kitchen")?;
    let mut runtime = HomeRuntime::with_default_policy();
    runtime.upsert_entity(
        Entity::new(id.clone(), "Kitchen Light")
            .with_area("kitchen")
            .with_state(EntityState::Off)
            .with_capability(Capability::Power),
    );

    let command = HomeCommand::new(
        CommandOrigin::Voice,
        HomeAction {
            target: TargetSelector::exact(id),
            kind: HomeActionKind::TurnOn,
            value: None,
        },
    );
    let decision = runtime.execute(command);

    println!("{}", serde_json::to_string_pretty(&decision)?);
    Ok(())
}
