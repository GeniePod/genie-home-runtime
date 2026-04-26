use anyhow::Result;
use genie_home_core::{RuntimeRequest, demo_runtime, demo_turn_on_kitchen_command};
use std::io::{Read, Write};

const DEFAULT_SOCKET_PATH: &str = "/tmp/genie-home-runtime.sock";

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1).map(String::as_str).unwrap_or("status") {
        "status" => print_status()?,
        "demo" => run_demo()?,
        "entities" => list_entities()?,
        "evaluate" => handle_json_request(false)?,
        "execute" => handle_json_request(true)?,
        "serve" => serve(
            args.get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_SOCKET_PATH),
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

COMMANDS:
    status    Print demo runtime status
    demo      Run an in-memory safety/action demo
    entities  Print demo entity graph
    evaluate  Read a HomeCommand JSON from stdin and evaluate without executing
    execute   Read a HomeCommand JSON from stdin and execute if allowed
    serve     Serve RuntimeRequest JSON over a Unix socket
    request   Send RuntimeRequest JSON from stdin to a Unix socket
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

#[cfg(unix)]
fn serve(socket_path: &str) -> Result<()> {
    use std::os::unix::net::UnixListener;
    use std::path::Path;

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
    eprintln!("genie-home-runtime listening on {}", path.display());

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut input = String::new();
                stream.read_to_string(&mut input)?;
                let output = runtime.handle_request_json(&input);
                stream.write_all(output.as_bytes())?;
                stream.write_all(b"\n")?;
            }
            Err(err) => {
                eprintln!("connection error: {err}");
            }
        }
    }

    Ok(())
}

#[cfg(not(unix))]
fn serve(_socket_path: &str) -> Result<()> {
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
