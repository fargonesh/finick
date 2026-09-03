use std::fs;

fn main() {
    let mut content = fs::read_to_string("libs/system/src/lib.rs").unwrap();
    content = content.replace("pub struct DisplayInfo", "#[derive(Clone, Debug, PartialEq)] pub struct DisplayInfo");
    content = content.replace("pub struct InputDevice", "#[derive(Clone, Debug, PartialEq)] pub struct InputDevice");
    content = content.replace("|d|", "|d: &BluetoothDevice|");
    content = content.replace("let mut in_monitor = false;", "");
    fs::write("libs/system/src/lib.rs", content).unwrap();
}
