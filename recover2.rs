use std::fs;

fn main() {
    let content = fs::read_to_string("scripts.txt").unwrap();
    if let Some(last) = content.lines().last() {
        // Find the start of the file content
        let marker = "cat << 'EOF' > apps/settings/src/main.rs\\n";
        if let Some(start) = last.find(marker) {
            let rest = &last[start + marker.len()..];
            if let Some(end) = rest.find("\\nEOF\\n") {
                let file_content = &rest[..end];
                // Replace escaped newlines and quotes
                let unescaped = file_content.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\").replace("\\t", "\t");
                fs::write("apps/settings/src/main.rs.recovered", unescaped).unwrap();
            }
        }
    }
}
