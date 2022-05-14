use fancy_regex::Regex;
use json::{self, JsonValue};
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

/**
 * Given a path with "*" wildcards, replace the wildcard with the first existing file that matches the pattern.
 */
fn expand_wildcards(orig: &str) -> Result<String, Box<dyn Error>> {
    let mut expanded = orig.to_string();
    // Query for parts with wildcards
    let part_re = Regex::new(r"(?<=/)[-\w:*.]+").unwrap();
    for part in part_re.find_iter(orig) {
        let part_m = part?;
        let part = &orig[part_m.start()..part_m.end()];
        if !part.contains('*') {
            continue;
        }
        // Query for devices on provided bus
        let bus_devices_path = Regex::new(r"^/sys/bus/[-\w:*.]+")?
            .find(&expanded)?
            .unwrap()
            .as_str()
            .to_owned()
            + "/devices";
        let mut file_names = fs::read_dir(bus_devices_path)?
            .filter_map(|f| f.ok())
            .map(|e| e.file_name().into_string())
            .filter_map(|s| s.ok());
        let part_re = Regex::new(&format!("^{}$", part.replace("*", ".*"))).unwrap();
        let file_name = file_names
            .find(|n| part_re.is_match(n).unwrap())
            .ok_or(simple_error!("No match for {}", part))?;
        expanded = expanded.replace(part, &file_name);
    }
    Ok(expanded)
}

fn switch_devices(devices: Vec<String>, target_action: &str) -> Result<(), Box<dyn Error>> {
    let is_root = unsafe { geteuid() == 0 };
    for device in devices {
        let device = device.as_str();
        if !Regex::new("^/sys/bus/[-\\w\"]+/drivers/[-\\w\"]+/[-\\w\"]+")?.is_match(device)? {
            bail!(format!("Invalid device {device}! Must be of shape /sys/bus/{{bus}}/drivers/{{driver}}/{{device}}"));
        }
        let device_path = Path::new(device);
        let target = format!(
            "{}/{target_action}",
            device_path.parent().and_then(|p| { p.to_str() }).unwrap()
        );
        println!(
            "{target_action}ing {:?} {} {}",
            device_path.file_name().unwrap(),
            if target_action == "bind" {
                "to"
            } else {
                "from"
            },
            Regex::new(r".*(?=/[-\w:*.]+$)")?
                .find(&target)?
                .ok_or(simple_error!("Invalid target"))?
                .as_str()
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
        Command::new("bash").arg("-c").arg(command).spawn()?;
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

fn determine_current_mode(devices: &Vec<String>) -> Result<bool, Box<dyn Error>> {
    let firstdev = &devices[0];
    Ok(Path::new(firstdev.as_str()).exists())
}

fn app() -> Result<(), Box<dyn Error>> {
    let (devices, laptop_commands, tablet_commands) = match fs::read_to_string("/etc/twoinone.json")
    {
        Ok(config_file) => {
            let config = json::parse(config_file.as_str())?;
            let devices = config["devices"].clone();
            let laptop_commands = config["laptop_commands"].clone();
            let tablet_commands = config["tablet_commands"].clone();
            if !devices.is_array() {
                bail!("No \"devices\" array specified in config file");
            }
            (devices, laptop_commands, tablet_commands)
        }
        Err(msg) => {
            eprintln!(
                "Couldn't read /etc/twoinone.json due to the following error: {}",
                msg.to_string()
            );
            (
                JsonValue::from(JsonValue::Null),
                JsonValue::from(JsonValue::Null),
                JsonValue::from(JsonValue::Null),
            )
        }
    };
    let devices = devices
        .members()
        .map(|d| match d.as_str() {
            Some(d) => expand_wildcards(d),
            None => bail!("Device must be a string"),
        })
        .filter_map(|d| d.ok())
        .collect::<Vec<_>>();

    let target_mode = match env::var("TARGET_MODE") {
        Ok(val) if val == "laptop" => Ok(true),
        Ok(val) if val == "tablet" => Ok(false),
        Ok(val) if val != "" => Err(simple_error!("Invalid mode argument")),
        _ => Ok(!determine_current_mode(&devices)?),
    }?;
    if !target_mode {
        for command in tablet_commands.members() {
            smart_execute(command)?;
        }
    }

    switch_devices(devices, if target_mode { "bind" } else { "unbind" })?;

    if target_mode {
        for command in laptop_commands.members() {
            smart_execute(command)?;
        }
    }
    Ok(())
}

fn main() {
    app().unwrap_or_else(|e| eprintln!("{:#}", e))
}
