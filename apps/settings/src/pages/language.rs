use {freya::prelude::*, std::process::Command, ui::*};

#[derive(Clone, Debug, PartialEq)]
pub struct LanguageItem {
    pub name: String,
    pub code: String,
    pub is_primary: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LanguageRegionInfo {
    pub display_language: String,
    pub display_code: String,
    pub region_locale: String,
    pub keyboard_layout: String,
    pub measurement_units: String,
    pub first_day_of_week: String,
    pub number_example: String,
    pub currency_example: String,
    pub date_example: String,
    pub installed_locales: Vec<String>,
    pub preferred_languages: Vec<LanguageItem>,
}

fn locale_code_to_friendly_name(code: &str) -> String {
    let clean = code.split('.').next().unwrap_or(code);
    match clean {
        "en_US" => "English (United States)".to_string(),
        "en_GB" => "English (United Kingdom)".to_string(),
        "en_AU" => "English (Australia)".to_string(),
        "en_CA" => "English (Canada)".to_string(),
        "es_ES" => "Spanish (Spain)".to_string(),
        "es_MX" => "Spanish (Mexico)".to_string(),
        "fr_FR" => "French (France)".to_string(),
        "fr_CA" => "French (Canada)".to_string(),
        "de_DE" => "German (Germany)".to_string(),
        "it_IT" => "Italian (Italy)".to_string(),
        "pt_BR" => "Portuguese (Brazil)".to_string(),
        "pt_PT" => "Portuguese (Portugal)".to_string(),
        "ja_JP" => "Japanese (Japan)".to_string(),
        "zh_CN" => "Chinese (Simplified)".to_string(),
        "zh_TW" => "Chinese (Traditional)".to_string(),
        "ko_KR" => "Korean (South Korea)".to_string(),
        "ru_RU" => "Russian (Russia)".to_string(),
        "nl_NL" => "Dutch (Netherlands)".to_string(),
        "sv_SE" => "Swedish (Sweden)".to_string(),
        "pl_PL" => "Polish (Poland)".to_string(),
        "C" | "POSIX" => "POSIX / C (System Default)".to_string(),
        other => {
            if other.is_empty() {
                "English (United States)".to_string()
            } else {
                format!("Locale ({})", other)
            }
        }
    }
}

pub fn fetch_language_info() -> LanguageRegionInfo {
    let mut lang_code = "en_US.UTF-8".to_string();
    let mut region_locale = "en_US.UTF-8".to_string();
    let mut keyboard_layout = "us (English US)".to_string();
    let mut measurement_units = "Metric (Celsius, km)".to_string();
    let first_day_of_week = "Monday".to_string();
    let mut date_example = "Wednesday, September 2, 2026".to_string();

    // 1. Query `locale`
    if let Ok(output) = Command::new("locale").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("LANG=") {
                let clean = val.trim_matches('"').trim();
                if !clean.is_empty() {
                    lang_code = clean.to_string();
                }
            } else if let Some(val) = line.strip_prefix("LC_TIME=") {
                let clean = val.trim_matches('"').trim();
                if !clean.is_empty() {
                    region_locale = clean.to_string();
                }
            } else if let Some(val) = line.strip_prefix("LC_MEASUREMENT=") {
                let clean = val.trim_matches('"').trim();
                if clean.contains("US") || clean.contains("en_US") {
                    measurement_units = "US Customary (Fahrenheit, miles)".to_string();
                }
            }
        }
    }

    // 2. Query `localectl status`
    if let Ok(output) = Command::new("localectl").arg("status").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("System Locale:") {
                for part in rest.split_whitespace() {
                    if let Some(v) = part.strip_prefix("LANG=") {
                        let clean = v.trim_matches('"').trim();
                        if !clean.is_empty() {
                            lang_code = clean.to_string();
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("X11 Layout:") {
                let l = rest.trim();
                if !l.is_empty() {
                    keyboard_layout = format!("{} ({})", l, match l {
                        "us" => "English US",
                        "gb" | "uk" => "English UK",
                        "es" => "Spanish",
                        "fr" => "French",
                        "de" => "German",
                        "jp" => "Japanese",
                        other => other,
                    });
                }
            }
        }
    }

    // 3. Query current date format sample
    if let Ok(output) = Command::new("date").arg("+%A, %B %d, %Y").output() {
        let d = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !d.is_empty() {
            date_example = d;
        }
    }

    // 4. Query installed locales via `locale -a`
    let mut installed_locales = Vec::new();
    if let Ok(output) = Command::new("locale").arg("-a").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let item = line.trim();
            if !item.is_empty() && item != "C" && item != "POSIX" {
                installed_locales.push(item.to_string());
            }
        }
    }

    if installed_locales.is_empty() {
        installed_locales = vec![
            "en_US.utf8".to_string(),
            "en_GB.utf8".to_string(),
            "es_ES.utf8".to_string(),
            "de_DE.utf8".to_string(),
            "fr_FR.utf8".to_string(),
            "ja_JP.utf8".to_string(),
        ];
    }

    let display_language = locale_code_to_friendly_name(&lang_code);

    let preferred_languages =
        vec![LanguageItem { name: display_language.clone(), code: lang_code.clone(), is_primary: true }, LanguageItem {
            name: "English (United Kingdom)".to_string(),
            code: "en_GB.UTF-8".to_string(),
            is_primary: false,
        }];

    LanguageRegionInfo {
        display_language,
        display_code: lang_code,
        region_locale,
        keyboard_layout,
        measurement_units,
        first_day_of_week,
        number_example: "1,234,567.89".to_string(),
        currency_example: "$1,234.56 USD".to_string(),
        date_example,
        installed_locales,
        preferred_languages,
    }
}

pub fn fast_initial_language_info() -> LanguageRegionInfo {
    LanguageRegionInfo {
        display_language: "English (United States)".to_string(),
        display_code: "en_US.UTF-8".to_string(),
        region_locale: "en_US.UTF-8".to_string(),
        keyboard_layout: "us (English US)".to_string(),
        measurement_units: "Metric (Celsius, km)".to_string(),
        first_day_of_week: "Monday".to_string(),
        number_example: "1,234,567.89".to_string(),
        currency_example: "$1,234.56 USD".to_string(),
        date_example: "Wednesday, September 2, 2026".to_string(),
        installed_locales: Vec::new(),
        preferred_languages: vec![
            LanguageItem { name: "English (United States)".to_string(), code: "en_US.UTF-8".to_string(), is_primary: true },
            LanguageItem {
                name: "English (United Kingdom)".to_string(),
                code: "en_GB.UTF-8".to_string(),
                is_primary: false,
            },
        ],
    }
}

#[derive(PartialEq)]
pub struct Language;

impl Component for Language {
    fn render(&self) -> impl IntoElement {
        let t = use_app_theme();

        let mut lang_info = use_state(fast_initial_language_info);
        let search_query = use_state(String::new);
        let is_metric = use_state(|| true);
        let spell_check = use_state(|| true);
        let auto_correct = use_state(|| false);

        use_hook(move || {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let data = fetch_language_info();
                let _ = tx.send(data);
            });

            freya::prelude::spawn(async move {
                if let Some(data) = rx.recv().await {
                    lang_info.set(data);
                }
            });
        });

        let current = lang_info.read().clone();
        let use_metric = *is_metric.read();
        let use_spell_check = *spell_check.read();
        let use_auto_correct = *auto_correct.read();

        let q = search_query.read().trim().to_lowercase();
        let filtered_locales: Vec<(String, String)> = current
            .installed_locales
            .iter()
            .filter_map(|loc| {
                let name = locale_code_to_friendly_name(loc);
                if q.is_empty() || loc.to_lowercase().contains(&q) || name.to_lowercase().contains(&q) {
                    Some((name, loc.clone()))
                } else {
                    None
                }
            })
            .collect();
        let total_locales = current.installed_locales.len();
        let filtered_count = filtered_locales.len();

        FadeSlideIn::new().child(
            rect()
                .width(Size::fill())
                .vertical()
                .spacing(GAP)
                .child(page_head(
                    GLOBE,
                    "Language and region",
                    "Manage system language, regional formats, and localization.",
                ))
                .child(
                    tile()
                        .child(tile_head(
                            Some(GLOBE),
                            "Display language",
                            Some(secondary_button("Refresh", {
                                let mut info = lang_info;
                                move || {
                                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                                    std::thread::spawn(move || {
                                        let data = fetch_language_info();
                                        let _ = tx.send(data);
                                    });
                                    freya::prelude::spawn(async move {
                                        if let Some(data) = rx.recv().await {
                                            info.set(data);
                                        }
                                    });
                                }
                            })),
                        ))
                        .child(hero_identity(
                            "🌐",
                            current.display_language.clone(),
                            format!("System default locale: {}", current.display_code),
                        ))
                        .child(field_label("Preferred languages"))
                        .children(current.preferred_languages.iter().map(|lang| {
                            let is_prim = lang.is_primary;
                            setting_row(
                                lang.name.clone(),
                                Some(lang.code.clone()),
                                false,
                                status_chip(if is_prim { "Primary" } else { "Secondary" }, is_prim, None),
                            )
                            .into_element()
                        })),
                )
                .child(
                    tile()
                        .child(tile_head(None, "Regional formats", None::<String>))
                        // Format Preview Box
                        .child(
                            rect()
                                .width(Size::fill())
                                .padding((12., 14.))
                                .margin((0., 0., 14., 0.))
                                .corner_radius(8.)
                                .background(t.panel_raised)
                                .border(Border::new().width(1.).fill(t.border))
                                .child(field_label("Example formatting"))
                                .child(
                                    rect()
                                        .horizontal()
                                        .spacing(24.)
                                        .child(
                                            rect().child(label().font_size(12.).color(t.text_dim).text("Date")).child(
                                                label()
                                                    .font_size(14.)
                                                    .font_weight(FontWeight::SEMI_BOLD)
                                                    .color(t.text)
                                                    .text(current.date_example.clone()),
                                            ),
                                        )
                                        .child(
                                            rect()
                                                .child(label().font_size(12.).color(t.text_dim).text("Number"))
                                                .child(
                                                    label()
                                                        .font_size(14.)
                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                        .color(t.text)
                                                        .text(current.number_example.clone()),
                                                ),
                                        )
                                        .child(
                                            rect()
                                                .child(label().font_size(12.).color(t.text_dim).text("Currency"))
                                                .child(
                                                    label()
                                                        .font_size(14.)
                                                        .font_weight(FontWeight::SEMI_BOLD)
                                                        .color(t.text)
                                                        .text(current.currency_example.clone()),
                                                ),
                                        ),
                                ),
                        )
                        .child(setting_row(
                            "Measurement system",
                            Some(if use_metric {
                                "Metric System (Celsius, Kilometers, Kilograms)"
                            } else {
                                "US / Imperial System (Fahrenheit, Miles, Pounds)"
                            }),
                            false,
                            pill_switch(use_metric, {
                                let mut m = is_metric;
                                move |v| m.set(v)
                            }),
                        ))
                        .child(setting_row(
                            "First day of week",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(current.first_day_of_week.clone()),
                        ))
                        .child(setting_row(
                            "Regional formats locale",
                            None::<String>,
                            true,
                            label().font_size(14.).color(t.text_dim).text(current.region_locale.clone()),
                        )),
                )
                .child(
                    tile()
                        .child(tile_head(None, "Input and keyboard", None::<String>))
                        .child(setting_row(
                            "Active keyboard layout",
                            None::<String>,
                            false,
                            label().font_size(14.).color(t.text_dim).text(current.keyboard_layout.clone()),
                        ))
                        .child(setting_row(
                            "Spell checking",
                            Some("Highlight misspelled words across desktop applications"),
                            true,
                            pill_switch(use_spell_check, {
                                let mut sc = spell_check;
                                move |v| sc.set(v)
                            }),
                        ))
                        .child(setting_row(
                            "Automatic capitalization",
                            Some("Capitalize the first word of each sentence automatically"),
                            true,
                            pill_switch(use_auto_correct, {
                                let mut ac = auto_correct;
                                move |v| ac.set(v)
                            }),
                        )),
                )
                .child(
                    tile()
                        .child(tile_head(
                            None,
                            "Installed locales",
                            Some(label().font_size(12.).color(t.text_dim).text(if total_locales > 0 {
                                format!("{total_locales} installed")
                            } else {
                                "Loading locales...".to_string()
                            })),
                        ))
                        .child(search_field(search_query.into(), "Search installed locales...", None))
                        .child({
                            if total_locales == 0 {
                                rect()
                                    .width(Size::fill())
                                    .padding(24.)
                                    .center()
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_dim)
                                            .text("Discovering installed system locales..."),
                                    )
                                    .into_element()
                            } else if filtered_count == 0 {
                                rect()
                                    .width(Size::fill())
                                    .padding(24.)
                                    .center()
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_dim)
                                            .text(format!("No language packs matching \"{}\"", search_query.read())),
                                    )
                                    .into_element()
                            } else {
                                VirtualScrollView::new_with_data(
                                    (t, filtered_locales),
                                    |item, (t, items): &(AppTheme, Vec<(String, String)>)| {
                                        let (friendly_name, code) = &items[item.index];
                                        rect()
                                            .key(item.index)
                                            .width(Size::fill())
                                            .height(Size::px(item.size))
                                            .padding((3., 0.))
                                            .child(
                                                rect()
                                                    .width(Size::fill())
                                                    .height(Size::fill())
                                                    .horizontal()
                                                    .main_align(Alignment::SpaceBetween)
                                                    .cross_align(Alignment::Center)
                                                    .padding((8., 12.))
                                                    .corner_radius(8.)
                                                    .background(t.panel_raised)
                                                    .border(Border::new().width(1.).fill(t.border))
                                                    .child(
                                                        rect()
                                                            .child(
                                                                label()
                                                                    .font_size(13.)
                                                                    .font_weight(FontWeight::SEMI_BOLD)
                                                                    .color(t.text)
                                                                    .text(friendly_name.clone()),
                                                            )
                                                            .child(
                                                                label()
                                                                    .font_size(11.)
                                                                    .color(t.text_dim)
                                                                    .text(code.clone()),
                                                            ),
                                                    )
                                                    .child(status_chip("Installed", true, None)),
                                            )
                                            .into()
                                    },
                                )
                                .length(filtered_count)
                                .item_size(56.)
                                .height(Size::px(320.))
                                .into_element()
                            }
                        }),
                ),
        )
    }
}
