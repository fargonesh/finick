use freya::prelude::*;
use ui::*;

#[derive(Clone, Debug, PartialEq)]
pub struct UserAccount {
    pub username: String,
    pub uid: u32,
    pub shell: String,
}

pub fn parse_passwd_users(content: &str) -> Vec<UserAccount> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let fields: Vec<&str> = line.split(':').collect();
            if fields.len() >= 7 {
                let username = fields[0].to_string();
                let uid = fields[2].parse::<u32>().ok()?;
                let shell = fields[6].to_string();

                if (1000..60000).contains(&uid) {
                    return Some(UserAccount { username, uid, shell });
                }
            }
            None
        })
        .collect()
}

#[derive(PartialEq)]
struct Accounts;

impl Component for Accounts {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let mut users = use_state(|| Vec::<UserAccount>::new());
        let mut loaded = use_state(|| false);

        if !*loaded.read() {
            loaded.set(true);
            let mut users_state = users.clone();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<UserAccount>>();

            tokio::spawn(async move {
                if let Ok(content) = tokio::fs::read_to_string("/etc/passwd").await {
                    let parsed = parse_passwd_users(&content);
                    let _ = tx.send(parsed);
                }
            });

            freya::prelude::spawn(async move {
                if let Some(user_list) = rx.recv().await {
                    users_state.set(user_list);
                }
            });
        }

        let user_list = users.read().clone();

        rect()
            .child(page_header("Accounts", "Manage user accounts and view system users."))
            .child(
                rect()
                    .margin((0., 0., 16., 0.))
                    .child(
                        label()
                            .font_size(16.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_primary)
                            .margin((0., 0., 12., 0.))
                            .text("Human Users (UID 1000 - 59999)")
                    )
            )
            .child({
                if user_list.is_empty() {
                    rect()
                        .padding(24.)
                        .corner_radius(12.)
                        .background(t.bg_card)
                        .border(Border::new().width(1.).fill(t.border_card))
                        .center()
                        .child(
                            label()
                                .font_size(14.)
                                .color(t.text_secondary)
                                .text(if !*loaded.read() {
                                    "Loading users..."
                                } else {
                                    "No human users found."
                                }),
                        )
                        .into_element()
                } else {
                    rect()
                        .children(user_list.into_iter().map(|user| {
                            let initial = user
                                .username
                                .chars()
                                .next()
                                .map(|c| c.to_uppercase().to_string())
                                .unwrap_or_else(|| "U".to_string());

                            rect()
                                .margin((0., 0., 12., 0.))
                                .padding(16.)
                                .corner_radius(12.)
                                .background(t.bg_card)
                                .border(Border::new().width(1.).fill(t.border_card))
                                .horizontal()
                                .cross_align(Alignment::Center)
                                .child(
                                    rect()
                                        .width(Size::px(48.))
                                        .height(Size::px(48.))
                                        .corner_radius(24.)
                                        .background(t.primary_accent)
                                        .center()
                                        .margin((0., 16., 0., 0.))
                                        .child(
                                            label()
                                                .font_size(20.)
                                                .font_weight(FontWeight::BOLD)
                                                .color(t.bg_base)
                                                .text(initial),
                                        ),
                                )
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .child(
                                            label()
                                                .font_size(16.)
                                                .font_weight(FontWeight::BOLD)
                                                .color(t.text_primary)
                                                .text(user.username.clone()),
                                        )
                                        .child(
                                            rect()
                                                .horizontal()
                                                .spacing(8.)
                                                .margin((4., 0., 0., 0.))
                                                .child(
                                                    label()
                                                        .font_size(13.)
                                                        .color(t.text_secondary)
                                                        .text(format!("UID: {}", user.uid)),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(13.)
                                                        .color(t.text_muted)
                                                        .text("•"),
                                                )
                                                .child(
                                                    label()
                                                        .font_size(13.)
                                                        .color(t.text_secondary)
                                                        .text(format!("Shell: {}", user.shell)),
                                                ),
                                        ),
                                )
                                .into_element()
                        }))
                        .into_element()
                }
            })
    }
}
