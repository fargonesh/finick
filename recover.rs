use std::fs;

fn main() {
    let content = fs::read_to_string("/home/flora/.gemini/antigravity-cli/brain/913f48f5-6726-4b85-9a5d-643dc5a2c0d6/.system_generated/logs/transcript_full.jsonl").unwrap();
    let mut outputs = Vec::new();
    for line in content.lines() {
        if line.contains("cat << 'EOF' > apps/settings/src/main.rs") {
            outputs.push(line.to_string());
        }
    }
    fs::write("scripts.txt", outputs.join("\n")).unwrap();
}
