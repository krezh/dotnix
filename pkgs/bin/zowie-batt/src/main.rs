//! Read the battery of a BenQ ZOWIE DW mouse, via its receiver (04a5:800a)
//! or, while the cable is plugged into this machine, the mouse itself (04a5:800b).
//!
//! Protocol (reversed from the Dongle_52820_Rx_DW and Mouse_Tx_S2 firmware;
//! both expose the same vendor interface):
//!   OUT report 0x08, 15 bytes: [0xb4, cmd, 0...]
//!   IN  report 0x09, 15 bytes: [data..., 0xc8 = ok | 0xe6 = error]
//!   cmd 0x15 -> [V_hi, V_lo, power, 0xc8]; V in 10 mV.
//!   `power` comes only from the wired mouse: 2 = power on its P0.0 input (not the
//!   USB cable; likely the dock contacts), 1 = USB enumerated, 0 = battery. Neither
//!   device reports charging vs. full; the receiver always sends 0 because the
//!   mouse's RF battery packet carries the voltage alone.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

const RECEIVER_ID: &str = "000004A5:0000800A";
const WIRED_MOUSE_ID: &str = "000004A5:0000800B";
const REPLY_TIMEOUT: Duration = Duration::from_millis(500);
const CMD_BATTERY: u8 = 0x15;
const STATUS_OK: u8 = 0xc8;

const HELP: &str = "\
Read the battery of a BenQ ZOWIE DW wireless mouse.

Talks to the mouse directly while its cable is plugged into this machine,
otherwise to its receiver.

Usage: zowie-batt [--json]
       zowie-batt <CMD>...

Arguments:
  <CMD>...    Send raw command bytes (hex like 0x15, or decimal) and print the replies

Options:
  --json      Print the battery reading as one JSON object:
              device, connection (wireless|wired), power (null when wireless;
              usb|external|battery when wired), voltage_mv, percent
  -h, --help  Print this help

Exit status: 0 on success, 2 when no receiver or wired mouse is present, 1 on any
other error.

The percentage is a rough estimate from the voltage and reads high while charging.
Neither the mouse nor the receiver reports charging vs. full.
";

type Report = [u8; 16];

/// Where the reply comes from; ordered by preference.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    /// Cabled mouse: answers directly and is the only source of the charging state.
    WiredMouse,
    /// Receiver: relays the last voltage the mouse sent over RF.
    Receiver,
}

/// Finds the vendor-page (0xFF03) command interface, preferring the wired mouse.
fn find_node() -> Option<(PathBuf, Source)> {
    let mut dirs: Vec<_> = fs::read_dir("/sys/class/hidraw")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs.into_iter()
        .filter_map(|dir| {
            let uevent = fs::read_to_string(dir.join("device/uevent")).ok()?;
            let source = if uevent.contains(WIRED_MOUSE_ID) {
                Source::WiredMouse
            } else if uevent.contains(RECEIVER_ID) {
                Source::Receiver
            } else {
                return None;
            };
            let desc = fs::read(dir.join("device/report_descriptor")).ok()?;
            let name = dir.file_name()?;
            desc.starts_with(&[0x06, 0x03, 0xff])
                .then(|| (Path::new("/dev").join(name), source))
        })
        .min_by_key(|&(_, source)| source)
}

struct Device {
    dev: File,
    replies: Receiver<Report>,
}

impl Device {
    fn open(path: &Path) -> io::Result<Self> {
        let dev = OpenOptions::new().read(true).write(true).open(path)?;
        let mut reader = dev.try_clone()?;
        let (tx, replies) = mpsc::channel();
        // hidraw reads block; a reader thread lets the caller time out via the channel.
        thread::spawn(move || {
            let mut buf = [0u8; 64];
            while let Ok(n @ 1..) = reader.read(&mut buf) {
                if n >= 16 && buf[0] == 0x09 {
                    let mut report = [0u8; 16];
                    report.copy_from_slice(&buf[..16]);
                    if tx.send(report).is_err() {
                        break;
                    }
                }
            }
        });
        Ok(Self { dev, replies })
    }

    /// Sends `cmd` and waits for its reply. The receiver drops a reply when its
    /// previous IN report was never collected, so the first command after an
    /// idle period can go unanswered: resend a few times.
    fn query(&mut self, cmd: u8) -> io::Result<Option<Report>> {
        while self.replies.try_recv().is_ok() {} // discard late replies to earlier sends
        let mut request = [0u8; 16];
        request[..3].copy_from_slice(&[0x08, 0xb4, cmd]);
        for _ in 0..3 {
            self.dev.write_all(&request)?;
            if let Ok(reply) = self.replies.recv_timeout(REPLY_TIMEOUT) {
                return Ok(Some(reply));
            }
        }
        Ok(None)
    }
}

/// Rough single-cell LiPo resting-voltage curve (mV, %); an estimate, not from firmware.
const CURVE: [(u32, u32); 10] = [
    (4200, 100), (4100, 90), (4000, 80), (3900, 65), (3800, 50),
    (3750, 40), (3700, 30), (3650, 20), (3550, 10), (3400, 0),
];

fn estimate_pct(mv: u32) -> u32 {
    if mv >= CURVE[0].0 {
        return 100;
    }
    CURVE
        .windows(2)
        .find(|w| mv >= w[1].0)
        .map_or(0, |w| {
            let ((hv, hp), (lv, lp)) = (w[0], w[1]);
            lp + ((mv - lv) * (hp - lp) + (hv - lv) / 2) / (hv - lv)
        })
}

fn parse_cmd(arg: &str) -> Option<u8> {
    match arg.strip_prefix("0x").or_else(|| arg.strip_prefix("0X")) {
        Some(hex) => u8::from_str_radix(hex, 16).ok(),
        None => arg.parse().ok(),
    }
}

fn hex(report: &Report) -> String {
    report.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

enum Failure {
    /// Neither the receiver nor a wired mouse is plugged in; exit status 2.
    NoDevice,
    Error(String),
}

impl From<String> for Failure {
    fn from(msg: String) -> Self {
        Self::Error(msg)
    }
}

impl From<&str> for Failure {
    fn from(msg: &str) -> Self {
        Self::Error(msg.into())
    }
}

fn run() -> Result<(), Failure> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        return Ok(());
    }
    let (flags, cmds): (Vec<String>, Vec<String>) =
        args.into_iter().partition(|a| a.starts_with('-'));
    let json = match flags.as_slice() {
        [] => false,
        [flag] if flag == "--json" && cmds.is_empty() => true,
        [flag] if flag == "--json" => return Err("--json does not apply to raw commands".into()),
        [flag, ..] => return Err(format!("unknown option: {flag}").into()),
    };
    let cmds = cmds
        .iter()
        .map(|arg| parse_cmd(arg).ok_or_else(|| format!("invalid command byte: {arg}")))
        .collect::<Result<Vec<u8>, _>>()?;

    let (node, source) = find_node().ok_or(Failure::NoDevice)?;
    let mut device = Device::open(&node).map_err(|e| format!("{}: {e}", node.display()))?;
    let io_err = |e: io::Error| format!("{}: {e}", node.display());

    if !cmds.is_empty() {
        for cmd in cmds {
            match device.query(cmd).map_err(io_err)? {
                Some(reply) => println!("cmd {cmd:#04x}: {}", hex(&reply)),
                None => println!("cmd {cmd:#04x}: timeout"),
            }
        }
        return Ok(());
    }

    let reply = device
        .query(CMD_BATTERY)
        .map_err(io_err)?
        .ok_or("no reply from device")?;
    if reply[4] != STATUS_OK {
        return Err(format!("bad response: {}", hex(&reply)).into());
    }
    let mv = u32::from(u16::from_be_bytes([reply[1], reply[2]])) * 10;
    let pct = estimate_pct(mv);
    // The receiver never knows the mouse's power source.
    let (connection, power) = match source {
        Source::Receiver => ("wireless", None),
        Source::WiredMouse => match reply[3] {
            2 => ("wired", Some("external")),
            1 => ("wired", Some("usb")),
            _ => ("wired", Some("battery")),
        },
    };

    if json {
        // `node` is always /dev/hidrawN, so it needs no JSON escaping.
        let power = power.map_or("null".into(), |p| format!("\"{p}\""));
        println!(
            "{{\"device\":\"{}\",\"connection\":\"{connection}\",\"power\":{power},\"voltage_mv\":{mv},\"percent\":{pct}}}",
            node.display()
        );
    } else {
        let power = match power {
            None => String::new(),
            Some("usb") => ", on USB power".into(),
            Some(p) => format!(", on {p} power"),
        };
        println!("{}: battery {mv} mV (~{pct}%), {connection}{power}", node.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::NoDevice) => {
            eprintln!("zowie-batt: no ZOWIE receiver or wired DW mouse found");
            ExitCode::from(2)
        }
        Err(Failure::Error(e)) => {
            eprintln!("zowie-batt: {e}");
            ExitCode::FAILURE
        }
    }
}
