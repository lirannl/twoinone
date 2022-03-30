use json::{self, JsonValue};
use regex::Regex;
use simple_error::{bail, simple_error};
use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    path::Path,
    process::Command,
};

#[link(name = "c")]
extern "C" {
    fn geteuid() -> u32;
}

fn switch_devices(devices: &JsonValue, target_action: &str) -> Result<(), Box<dyn Error>> {
    let is_root = unsafe { geteuid() == 0 };
    for device in devices.members() {
        let dev = device
            .as_str()
            .ok_or(simple_error!("Couldn't parse device."))?
            .into();
        // Throw an error (and exit) if any device is not of the right shape
        if !Regex::new("^/sys/bus/[\\w-]+/drivers/[\\w-]+/[^W\"]+")?.is_match(dev) {
            bail!(format!("Invalid device {dev}! Must be of shape /sys/bus/{{bus}}/drivers/{{driver}}/{{device}}"));
        }
        let device_path = Path::new(dev);
        let target = format!(
            "{}/{target_action}",
            device_path.parent().and_then(|p| { p.to_str() }).unwrap()
        );
        println!(
            "{target_action}ing {:?} {} {}",
            device_path.file_name().unwrap(),
            if target_action == "bind" {"to"} else {"from"},
            target
        );
        writeln!(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(if is_root {
                    target
                } else {
                    target_action.to_string()
                })?,
            "{}",
            device_path.file_name().and_then(|p| p.to_str()).unwrap()
        )
        .unwrap_or_else(|e: io::Error| {
            if !(e.to_string().starts_with("No such device")
                || e.to_string().starts_with("Device or resource busy"))
            {
                eprintln!("{:?}", e);
            }
        });
    }
    Ok(())
}

fn smart_execute(command: &JsonValue) -> Result<(), Box<dyn Error>> {
    let command = command
        .as_str()
        .ok_or(simple_error!("Command must be a string"))?;
    if unsafe { geteuid() != 0 } || command.starts_with("sudo ") {
        Command::new("bash")
            .arg("-c")
            .arg(command)
            .spawn()?;
    } else {
        Command::new("sudo")
            .arg("-u")
            .arg(env::var("ORIG_USER")?)
            .arg(format!(
                "DBUS_SESSION_BUS_ADDRESS={}",
                env::var("DBUS_SESSION_BUS_ADDRESS")?
            ))
            .arg("bash")
            .arg("-c")
            .arg(command)
            .spawn()?;
    }
    Ok(())
}

fn determine_current_mode(devices: &JsonValue) -> Result<bool, Box<dyn Error>> {
    let firstdev = devices[0]
        .as_str()
        .ok_or(simple_error!("Device is not a string"))?;
    if !Regex::new("^/sys/bus/[\\w-]+/drivers/[\\w-]+/[^W\"]+")?.is_match(firstdev) {
        bail!(format!("Invalid device {firstdev}! Must be of shape /sys/bus/{{bus}}/drivers/{{driver}}/{{device}}"));
    }
    Ok(Path::new(firstdev).exists())
}

fn app() -> Result<(), Box<dyn Error>> {
    println!("{:?}",env::var("TARGET_MODE"));
    let (devices, laptop_commands, tablet_commands) = match fs::read_to_string("/etc/twoinone.json")
    {
        Ok(config_file) => {
            let config = json::parse(config_file.as_str())?;
            let devices = config["devices"].clone();
            let laptop_commands = config["laptop_commands"].clone();
            let tablet_commands = config["tablet_commands"].clone();
            (Some(devices), laptop_commands, tablet_commands)
        }
        Err(msg) => {
            eprintln!(
                "Couldn't read /etc/twoinone.json due to the following error: {}",
                msg.to_string()
            );
            (
                None,
                JsonValue::from(JsonValue::Null),
                JsonValue::from(JsonValue::Null),
            )
        }
    };
    match devices {
        Some(devices) => {
            let target_mode = match env::var("TARGET_MODE") {
                Ok(val) if val == "laptop" => Ok(true),
                Ok(val) if val == "tablet" => Ok(false),
                Ok(val) if val != "" => Err(simple_error!("Invalid mode argument")),
                _  => Ok(!determine_current_mode(&devices)?),
            }?;
            if !target_mode {
                for command in tablet_commands.members() {
                    smart_execute(command)?;
                }
            }
            switch_devices(&devices, if target_mode { "bind" } else { "unbind" })?;
            if target_mode {
                for command in laptop_commands.members() {
                    smart_execute(command)?;
                }
            }
        }
        None => {
            eprintln!("No devices specified!");
        }
    }
    Ok(())
}

fn main() {
    app().unwrap_or_else(|e| eprintln!("{:#}", e))
}
