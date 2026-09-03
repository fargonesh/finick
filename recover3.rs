use std::fs;

fn main() {
    let content = fs::read_to_string("last_json.json").unwrap();
    if let Some(start) = content.find(r#"cat << 'EOF' > apps/settings/src/main.rs\n"#) {
        let rest = &content[start + 42..];
        if let Some(end) = rest.find(r#"\nEOF"#) {
            let file_content = &rest[..end];
            let unescaped = file_content.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\").replace("\\t", "\t");
            fs::write("apps/settings/src/main.rs.recovered", unescaped).unwrap();
        }
    }
}
