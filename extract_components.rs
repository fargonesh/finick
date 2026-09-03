use std::fs;

fn extract_component(content: &str, start_marker: &str) -> Option<String> {
    if let Some(start) = content.rfind(start_marker) {
        let rest = &content[start..];
        // The component ends with a closing brace at the end of the `impl Component for X` block.
        // Or it ends when the next `struct ` or `pub struct` appears, or end of file.
        // Actually, let's just find the first `#[derive(PartialEq)]` or ```` (markdown block end)
        if let Some(end) = rest.find("```") {
            return Some(rest[..end].trim().to_string());
        }
    }
    None
}

fn main() {
    let content = fs::read_to_string("/home/flora/.gemini/antigravity-cli/brain/913f48f5-6726-4b85-9a5d-643dc5a2c0d6/.system_generated/logs/transcript_full.jsonl").unwrap();
    // In JSON, newlines are escaped. Let's unescape first to make searching easier.
    let unescaped = content.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\");

    let sound = extract_component(&unescaped, "struct Sound;\n\nimpl Component for Sound").unwrap_or_else(|| extract_component(&unescaped, "struct Sound;\nimpl Component for Sound").unwrap_or_default());
    let displays = extract_component(&unescaped, "struct Displays;\n\nimpl Component for Displays").unwrap_or_else(|| extract_component(&unescaped, "struct Displays;\nimpl Component for Displays").unwrap_or_default());
    let power = extract_component(&unescaped, "struct Power;\n\nimpl Component for Power").unwrap_or_else(|| extract_component(&unescaped, "struct Power;\nimpl Component for Power").unwrap_or_default());
    let input = extract_component(&unescaped, "struct Input;\n\nimpl Component for Input").unwrap_or_else(|| extract_component(&unescaped, "struct Input;\nimpl Component for Input").unwrap_or_default());
    let general = extract_component(&unescaped, "struct General;\n\nimpl Component for General").unwrap_or_else(|| extract_component(&unescaped, "struct General;\nimpl Component for General").unwrap_or_default());

    fs::write("sound.rs", sound).unwrap();
    fs::write("displays.rs", displays).unwrap();
    fs::write("power.rs", power).unwrap();
    fs::write("input.rs", input).unwrap();
    fs::write("general.rs", general).unwrap();
}
