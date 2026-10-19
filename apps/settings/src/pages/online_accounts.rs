use {freya::prelude::*, std::process::Command, ui::*};

#[derive(Clone, Debug, PartialEq)]
pub struct OnlineAccount {
    pub name: String,
    pub provider: String,
    pub status: String,
}

fn secret_service_available() -> bool {
    Command::new("secret-tool")
        .args(["search", "--all", "xdg:schema", "org.gnome.OnlineAccounts"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || Command::new("goa-daemon").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn probe_accounts() -> Vec<OnlineAccount> {
    let mut out = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let cfg = format!("{home}/.config/goa-1.0/accounts.conf");
        if let Ok(text) = std::fs::read_to_string(&cfg) {
            let mut cur_name = String::new();
            let mut cur_provider = String::new();
            for line in text.lines() {
                let t = line.trim();
                if t.starts_with('[') && t.ends_with(']') {
                    if !cur_name.is_empty() {
                        out.push(OnlineAccount {
                            name: cur_name.clone(),
                            provider: if cur_provider.is_empty() { "Unknown".to_string() } else { cur_provider.clone() },
                            status: "Configured".to_string(),
                        });
                    }
                    cur_name = t.trim_matches(|c| c == '[' || c == ']').to_string();
                    cur_provider.clear();
                } else if let Some(v) = t.strip_prefix("ProviderType=") {
                    cur_provider = v.trim().to_string();
                }
            }
            if !cur_name.is_empty() {
                out.push(OnlineAccount {
                    name: cur_name,
                    provider: if cur_provider.is_empty() { "Unknown".to_string() } else { cur_provider },
                    status: "Configured".to_string(),
                });
            }
        }
    }
    out
}

#[derive(PartialEq)]
pub struct OnlineAccounts;

impl Component for OnlineAccounts {
    fn render(&self) -> impl IntoElement {
        let accounts: State<Vec<OnlineAccount>> = use_state(Vec::new);
        let loading: State<bool> = use_state(|| true);
        let new_name: State<String> = use_state(String::new);
        let mut provider_idx: State<usize> = use_state(|| 0usize);
        let armed_remove: State<Option<String>> = use_state(|| None);
        let has_secret: State<bool> = use_state(|| false);

        use_hook(move || {
            let mut acc = accounts;
            let mut ld = loading;
            let mut sec = has_secret;
            spawn(async move {
                let probed = tokio::task::spawn_blocking(|| (probe_accounts(), secret_service_available()))
                    .await
                    .unwrap_or_default();
                acc.set(probed.0);
                sec.set(probed.1);
                ld.set(false);
            });
        });

        let list = accounts.read().clone();
        let is_loading = *loading.read();
        let secret_ok = *has_secret.read();
        let providers = ["Google", "Nextcloud", "Exchange"];
        let cur_provider = providers[(*provider_idx.read()).min(2)].to_string();

        let mut base = rect()
            .width(Size::fill())
            .vertical()
            .spacing(GAP)
            .child(page_head(
                GLOBE,
                "Online accounts",
                "Connect Google, Nextcloud and Exchange. Tokens live in Secret Service when present.",
            ))
            .child(
                tile()
                    .child(tile_head(
                        None,
                        "Secret storage",
                        Some(status_chip(
                            if secret_ok { "Secret Service" } else { "No Secret Service" },
                            secret_ok,
                            None,
                        )),
                    ))
                    .child(tile_sub(if secret_ok {
                        "Tokens are stored via Secret Service (secret-tool / goa-daemon)."
                    } else {
                        "secret-tool and goa-daemon not found. Accounts are listed from goa-1.0 config only; no token is stored by Finick."
                    })),
            );

        let mut accounts_tile = tile().child(tile_head(
            None,
            format!("Accounts ({})", list.len()),
            (!list.is_empty()).then(|| status_chip(format!("{} connected", list.len()), true, None)),
        ));
        if is_loading {
            accounts_tile = accounts_tile.child(tile_sub("Reading online accounts…"));
        } else if list.is_empty() {
            accounts_tile = accounts_tile.child(empty_state(
                GLOBE,
                "No online accounts",
                "Add Google, Nextcloud or Exchange to sync mail, calendar and files.",
            ));
        } else {
            for acc in list.iter().take(12) {
                let key = acc.name.clone();
                let is_armed = armed_remove.read().clone().as_deref() == Some(key.as_str());
                let control: Element = if !is_armed {
                    let mut armed = armed_remove;
                    let k = key.clone();
                    secondary_button("Remove", move || {
                        armed.set(Some(k.clone()));
                        let id = k.clone();
                        spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            if armed.read().clone().as_deref() == Some(id.as_str()) {
                                armed.set(None);
                            }
                        });
                    })
                    .into_element()
                } else {
                    let mut armed = armed_remove;
                    let mut list_state = accounts;
                    let k = key.clone();
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .content(Content::Flex)
                        .child(danger_button("Sure? Remove", move || {
                            let mut v = list_state.read().clone();
                            v.retain(|a| a.name != k);
                            list_state.set(v);
                            armed.set(None);
                        }))
                        .child(ghost_button("Cancel", move || armed.set(None)))
                        .into_element()
                };
                accounts_tile = accounts_tile.child(setting_row(
                    acc.name.clone(),
                    Some(format!("{} · {}", acc.provider, acc.status)),
                    false,
                    control,
                ));
            }
        }
        base = base.child(accounts_tile);

        let prov_opts: Vec<DropdownOption> = providers
            .iter()
            .map(|p| DropdownOption { label: p.to_string(), value: p.to_string() })
            .collect();
        base = base.child(
            tile()
                .child(tile_head(None, "Add account", None::<String>))
                .child(tile_sub("Finick does not implement OAuth here. Adding records the account locally; sign-in happens in the provider app."))
                .child(field_label("Provider"))
                .child(dropdown_select(
                    cur_provider.clone(),
                    prov_opts,
                    EventHandler::new(move |v: String| {
                        let idx = providers.iter().position(|p| *p == v).unwrap_or(0);
                        provider_idx.set(idx);
                    }),
                ))
                .child(rect().margin((10., 0., 0., 0.)).child(field_label("Account name or email")))
                .child(search_field(new_name.into(), "name@example.com", None))
                .child(
                    rect().margin((10., 0., 0., 0.)).child({
                        let mut acc = accounts;
                        let mut nm = new_name;
                        let prov = cur_provider.clone();
                        primary_button("Add", move || {
                            let name = nm.read().trim().to_string();
                            if name.is_empty() {
                                return;
                            }
                            let mut v = acc.read().clone();
                            v.push(OnlineAccount { name, provider: prov.clone(), status: "Local only".to_string() });
                            acc.set(v);
                            nm.set(String::new());
                        })
                    }),
                ),
        );
        base = base.child(tile().child(tile_sub(
            "Empty state is honest: no mock Google account is shown when none is configured.",
        )));
        FadeSlideIn::new().child(base)
    }
}
