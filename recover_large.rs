use std::fs;

fn main() {
    let content = fs::read_to_string("/home/flora/.gemini/antigravity-cli/brain/913f48f5-6726-4b85-9a5d-643dc5a2c0d6/.system_generated/logs/transcript_full.jsonl").unwrap();
    let mut largest = String::new();
    let marker = r#"cat << 'EOF' > apps/settings/src/main.rs\n"#;
    for line in content.lines() {
        if let Some(start) = line.find(marker) {
            let rest = &line[start + marker.len()..];
            if let Some(end) = rest.find(r#"\nEOF\n"#) {
                let file_content = &rest[..end];
                if file_content.len() > largest.len() {
                    largest = file_content.to_string();
                }
            }
        }
    }
    let unescaped = largest.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\").replace("\\t", "\t");
    fs::write("apps/settings/src/main.rs.recovered", unescaped).unwrap();
}
