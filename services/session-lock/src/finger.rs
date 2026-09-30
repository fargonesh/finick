//! Fingerprint unlock via fprintd on the system bus.
//!
//! Runs on its own thread with the blocking zbus API so D-Bus roundtrips
//! never stall the Wayland event loop. Reports to the main loop over a
//! calloop channel; a match takes the same unlock path as a correct password.
//! Absent device / no enrolled fingers / claim failure all degrade to
//! `Gone` (password-only), never an error exit.

use crate::BgEvent;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FingerEvent {
    /// Enrolled fingers found, verification running.
    Ready,
    /// Finger matched: main loop should unlock.
    Match,
    /// Done without match: sensor rearmed, user may retry.
    NoMatch,
    /// Transient scan hint (retry-scan, finger-not-centered, ...).
    Retry,
    /// No fingerprint path on this machine (no device, none enrolled, denied).
    Gone,
}

const SERVICE: &str = "net.reactivated.Fprint";
const MANAGER_PATH: &str = "/net/reactivated/Fprint/Manager";
const MANAGER_IFACE: &str = "net.reactivated.Fprint.Manager";
const DEVICE_IFACE: &str = "net.reactivated.Fprint.Device";

fn send(tx: &calloop::channel::Sender<BgEvent>, ev: FingerEvent) -> bool {
    tx.send(BgEvent::Finger(ev)).is_ok()
}

pub fn spawn(tx: calloop::channel::Sender<BgEvent>, username: String) {
    std::thread::spawn(move || run(tx, username));
}

fn run(tx: calloop::channel::Sender<BgEvent>, username: String) {
    use zbus::{blocking::{Connection, Proxy}, zvariant::OwnedObjectPath};

    let gone = |tx: &calloop::channel::Sender<BgEvent>| {
        send(tx, FingerEvent::Gone);
    };
    let conn = match Connection::system() {
        Ok(c) => c,
        Err(_) => return gone(&tx),
    };
    let manager = match Proxy::new(&conn, SERVICE, MANAGER_PATH, MANAGER_IFACE) {
        Ok(p) => p,
        Err(_) => return gone(&tx),
    };
    let device_path: OwnedObjectPath = match manager.call("GetDefaultDevice", &()) {
        Ok(p) => p,
        Err(_) => match manager.call::<_, _, Vec<OwnedObjectPath>>("GetDevices", &()) {
            Ok(mut devs) if !devs.is_empty() => devs.remove(0),
            _ => return gone(&tx),
        },
    };
    let device = match Proxy::new(&conn, SERVICE, device_path, DEVICE_IFACE) {
        Ok(p) => p,
        Err(_) => return gone(&tx),
    };
    let enrolled: Vec<String> = match device.call("ListEnrolledFingers", &(username.as_str(),)) {
        Ok(v) => v,
        Err(_) => return gone(&tx),
    };
    if enrolled.is_empty() {
        return gone(&tx);
    }
    if device.call::<_, _, ()>("Claim", &(username.as_str(),)).is_err() {
        return gone(&tx);
    }
    if !send(&tx, FingerEvent::Ready) {
        release(&device);
        return;
    }
    loop {
        let _ = device.call::<_, _, ()>("VerifyStop", &());
        if device.call::<_, _, ()>("VerifyStart", &(username.as_str(),)).is_err() {
            break;
        }
        let signals = match device.receive_signal("VerifyStatus") {
            Ok(s) => s,
            Err(_) => break,
        };
        let mut restart = false;
        for msg in signals {
            let (result, done): (String, bool) = match msg.body().deserialize() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if result == "verify-match" {
                send(&tx, FingerEvent::Match);
                release(&device);
                return;
            }
            if done {
                // verify-no-match (or unknown error): rearm for another try.
                if !send(&tx, FingerEvent::NoMatch) {
                    release(&device);
                    return;
                }
                restart = true;
                break;
            }
            if !send(&tx, FingerEvent::Retry) {
                release(&device);
                return;
            }
        }
        if !restart {
            break; // signal stream ended unexpectedly
        }
    }
    release(&device);
    send(&tx, FingerEvent::Gone);
}

fn release(device: &zbus::blocking::Proxy) {
    let _ = device.call::<_, _, ()>("VerifyStop", &());
    let _ = device.call::<_, _, ()>("Release", &());
}
