use std::fs;

fn main() {
    let mut code = fs::read_to_string("apps/settings/src/main.rs").unwrap();
    // I can just replace all occurrences of:
    //    }
    // }
    // with:
    //    }
    code = code.replace("    }\n}\n\nfn view_", "    }\n\nfn view_");
    code = code.replace("    }\n}\n\n#[derive(Clone, Debug, PartialEq)]\npub struct UserAccount", "    }\n\n#[derive(Clone, Debug, PartialEq)]\npub struct UserAccount");
    code = code.replace("            )\n    }\n}", "            )\n    }");
    code = code.replace("            )\n        )\n    }\n}", "            )\n        )\n    }");
    
    // QuickActions is the last one before Accounts
    code = code.replace("    fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text(\"Quick Actions\")) } }\n", "    fn render(&self) -> impl IntoElement { rect().child(label().color(use_app_theme().text_primary).text(\"Quick Actions\")) }\n");

    fs::write("apps/settings/src/main.rs", code).unwrap();
}
