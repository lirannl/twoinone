# Twoinone
Software built to allow convertible 2 in 1s to switch between operation modes manually. 

Twoinone differs from other similar software (such as tablet-mode) by working on a kernel level (as opposed to a libinput level), and therefore, desktop environments with tablet modes (such as gnome and KDE), will actually detect tablet mode properly (gnome will allow auto-rotate on supported devices, and enable the on-screen keyboard. KDE will enable auto-rotate if the setting to only do so in tablet mode is enabled, and will scale the system tray icons up).

Enter "twoinone -h" for help.
Comes with a desktop icon and a cli.

# Installation
In the main folder, with rust installed, run "cargo build -r", then, the following commands in a root shell (or using sudo):
```bash
cp twoinone "/usr/bin/twoinone"
chmod 755 "/usr/bin/twoinone"
cp target/release/twoinone "/usr/share/twoinone/twoinone"
chmod 755 "/usr/share/twoinone/twoinone"
cp env_sanitizer "/usr/share/twoinone/env_sanitizer"
chmod 755 "/usr/share/twoinone/env_sanitizer"
cp twoinone.sudoers "/etc/sudoers.d/twoinone"
chmod 600 "/etc/sudoers.d/twoinone"
cp twoinone.group "/usr/lib/sysusers.d/twoinone.conf"
chmod 644 "/usr/lib/sysusers.d/twoinone.conf"
cp twoinone.json "/usr/share/twoinone/twoinone.json"
chmod 644 "/usr/share/twoinone/twoinone.json"
cp two-in-one.svg "/usr/share/icons/hicolor/scalableapps/two-in-one.svg"
chmod 644 "/usr/share/icons/hicolor/scalable/appstwo-in-one.svg"
cp twoinone.desktop "/usr/share/applications/twoinonedesktop"
chmod 644 "/usr/share/applications/twoinone.desktop"
```
Note: twoinone depends on libc, sudo, and bash to function.

Alternatively, an AUR package is available under 'twoinone'.

# Configuration
The application looks for its configuration on /etc/twoinone.json, parsing a valid json (regardless of spacing).
The configuration consists of the following fields -
- "devices" (required): A list of device paths to disable (and re-enable). Must be of shape "/sys/bus/{bus}/drivers/{driver}/{device}" (hint: use ls to check which devices are available, and using a root shell, echo device names to the kernel to see which bus address corresponds to which device). This should be populated with the bus paths of the mouse, keyboard, and any other devices that should be (virtually) disconnected in tablet mode.
- "tablet_commands": A list of commands that will be executed *by the user* (unless sudo is used - which will not require a password) right before twoinone switches to tablet mode.

- "laptop_commands": A list of commands that will be executed *by the user* (unless sudo is used - which will not require a password) right after twoinone switches to laptop mode.

An example config file is available in twoinone.json. Note that "devices" **must** be populated with at least one device (more are optional, though they must still conform to the same pattern mentioned above) for twoinone to work.