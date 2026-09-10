use freya::prelude::*;
use std::process::Command;
use ui::*;

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

    let preferred_languages = vec![
        LanguageItem {
            name: display_language.clone(),
            code: lang_code.clone(),
            is_primary: true,
        },
        LanguageItem {
            name: "English (United Kingdom)".to_string(),
            code: "en_GB.UTF-8".to_string(),
            is_primary: false,
        },
    ];

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
            LanguageItem {
                name: "English (United States)".to_string(),
                code: "en_US.UTF-8".to_string(),
                is_primary: true,
            },
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

        rect()
            .width(Size::fill())
            .child(page_header(
                "Language & Region",
                "Manage system language, regional formats, and localization.",
            ))
            // Primary Display Language Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .child(
                                label()
                                    .font_size(13.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_secondary)
                                    .text("DISPLAY LANGUAGE"),
                            )
                            .child(secondary_button("Refresh", {
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
                    )
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
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
                                            .text("🌐"),
                                    ),
                            )
                            .child(
                                rect()
                                    .child(
                                        label()
                                            .font_size(18.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(t.text_primary)
                                            .text(current.display_language.clone()),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((2., 0., 0., 0.))
                                            .text(format!("System default locale: {}", current.display_code)),
                                    ),
                            ),
                    )
                    .child(
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::SEMI_BOLD)
                            .color(t.text_secondary)
                            .margin((4., 0., 10., 0.))
                            .text("PREFERRED LANGUAGES"),
                    )
                    .children(current.preferred_languages.iter().map(|lang| {
                        let is_prim = lang.is_primary;
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .padding((10., 12.))
                            .margin((0., 0., 6., 0.))
                            .corner_radius(8.)
                            .background(if is_prim { t.bg_active } else { t.bg_base })
                            .border(Border::new().width(1.).fill(if is_prim { t.primary_accent } else { t.border_subtle }))
                            .child(
                                rect()
                                    .child(
                                        label()
                                            .font_size(14.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text(lang.name.clone()),
                                    )
                                    .child(
                                        label()
                                            .font_size(12.)
                                            .color(t.text_secondary)
                                            .text(lang.code.clone()),
                                    ),
                            )
                            .child(
                                rect()
                                    .padding((4., 8.))
                                    .corner_radius(4.)
                                    .background(if is_prim { t.primary_accent } else { t.bg_card })
                                    .child(
                                        label()
                                            .font_size(11.)
                                            .font_weight(FontWeight::BOLD)
                                            .color(if is_prim { t.bg_base } else { t.text_secondary })
                                            .text(if is_prim { "PRIMARY" } else { "SECONDARY" }),
                                    ),
                            )
                            .into_element()
                    })),
            )
            // Regional Formats & Standards Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_secondary)
                            .margin((0., 0., 14., 0.))
                            .text("REGIONAL FORMATS & PREVIEWS"),
                    )
                    // Format Preview Box
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((12., 14.))
                            .margin((0., 0., 14., 0.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .child(
                                label()
                                    .font_size(12.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_secondary)
                                    .margin((0., 0., 8., 0.))
                                    .text("EXAMPLE FORMATTING"),
                            )
                            .child(
                                rect()
                                    .horizontal()
                                    .spacing(24.)
                                    .child(
                                        rect()
                                            .child(label().font_size(12.).color(t.text_secondary).text("Date"))
                                            .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(current.date_example.clone())),
                                    )
                                    .child(
                                        rect()
                                            .child(label().font_size(12.).color(t.text_secondary).text("Number"))
                                            .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(current.number_example.clone())),
                                    )
                                    .child(
                                        rect()
                                            .child(label().font_size(12.).color(t.text_secondary).text("Currency"))
                                            .child(label().font_size(14.).font_weight(FontWeight::SEMI_BOLD).color(t.text_primary).text(current.currency_example.clone())),
                                    ),
                            ),
                    )
                    // Measurement Units Toggle
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Measurement System"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((2., 0., 0., 0.))
                                            .text(if use_metric {
                                                "Metric System (Celsius, Kilometers, Kilograms)"
                                            } else {
                                                "US / Imperial System (Fahrenheit, Miles, Pounds)"
                                            }),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(use_metric)
                                    .on_toggle({
                                        let mut m = is_metric;
                                        move |_| {
                                            let next = !*m.read();
                                            m.set(next);
                                        }
                                    }),
                            ),
                    )
                    // First Day of Week Row
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((10., 14.))
                            .margin((0., 0., 6., 0.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(
                                label()
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text_primary)
                                    .text("First Day of Week"),
                            )
                            .child(
                                label()
                                    .font_size(14.)
                                    .color(t.text_secondary)
                                    .text(current.first_day_of_week.clone()),
                            ),
                    )
                    // Region Locale Row
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((10., 14.))
                            .margin((0., 0., 6., 0.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(
                                label()
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text_primary)
                                    .text("Regional Formats Locale"),
                            )
                            .child(
                                label()
                                    .font_size(14.)
                                    .color(t.text_secondary)
                                    .text(current.region_locale.clone()),
                            ),
                    ),
            )
            // Keyboard & Input Sources Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        label()
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(t.text_secondary)
                            .margin((0., 0., 14., 0.))
                            .text("INPUT & KEYBOARD LAYOUT"),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding((10., 14.))
                            .margin((0., 0., 12., 0.))
                            .corner_radius(8.)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .child(
                                label()
                                    .font_size(14.)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .color(t.text_primary)
                                    .text("Active Keyboard Layout"),
                            )
                            .child(
                                label()
                                    .font_size(14.)
                                    .color(t.text_secondary)
                                    .text(current.keyboard_layout.clone()),
                            ),
                    )
                    // Spell Check Toggle
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .margin((0., 0., 14., 0.))
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Spell Checking"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((2., 0., 0., 0.))
                                            .text("Highlight misspelled words across desktop applications"),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(use_spell_check)
                                    .on_toggle({
                                        let mut sc = spell_check;
                                        move |_| {
                                            let next = !*sc.read();
                                            sc.set(next);
                                        }
                                    }),
                            ),
                    )
                    // Auto-Correction Toggle
                    .child(
                        rect()
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .width(Size::fill())
                            .content(Content::Flex)
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        label()
                                            .font_size(15.)
                                            .font_weight(FontWeight::SEMI_BOLD)
                                            .color(t.text_primary)
                                            .text("Automatic Capitalization"),
                                    )
                                    .child(
                                        label()
                                            .font_size(13.)
                                            .color(t.text_secondary)
                                            .margin((2., 0., 0., 0.))
                                            .text("Capitalize the first word of each sentence automatically"),
                                    ),
                            )
                            .child(
                                Switch::new()
                                    .toggled(use_auto_correct)
                                    .on_toggle({
                                        let mut ac = auto_correct;
                                        move |_| {
                                            let next = !*ac.read();
                                            ac.set(next);
                                        }
                                    }),
                            ),
                    ),
            )
            // Available Locales Card
            .child(
                rect()
                    .width(Size::fill())
                    .margin((0., 0., 16., 0.))
                    .padding(16.)
                    .corner_radius(12.)
                    .background(t.bg_card)
                    .border(Border::new().width(1.).fill(t.border_card))
                    .overflow(Overflow::Clip)
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .main_align(Alignment::SpaceBetween)
                            .cross_align(Alignment::Center)
                            .margin((0., 0., 12., 0.))
                            .child(
                                label()
                                    .font_size(13.)
                                    .font_weight(FontWeight::BOLD)
                                    .color(t.text_secondary)
                                    .text("INSTALLED LOCALES & PACKS"),
                            )
                            .child(
                                label()
                                    .font_size(12.)
                                    .color(t.text_dim)
                                    .text(if total_locales > 0 {
                                        format!("{total_locales} installed")
                                    } else {
                                        "Loading locales...".to_string()
                                    }),
                            ),
                    )
                    .child(
                        rect()
                            .width(Size::fill())
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(8.)
                            .padding((6., 10.))
                            .margin((0., 0., 12., 0.))
                            .corner_radius(RADIUS_PILL)
                            .background(t.bg_base)
                            .border(Border::new().width(1.).fill(t.border_subtle))
                            .child(icon(SEARCH, 13., t.text_dim))
                            .child(
                                rect()
                                    .width(Size::flex(1.))
                                    .child(
                                        Input::new(search_query)
                                            .background(Color::TRANSPARENT)
                                            .border_fill(Color::TRANSPARENT)
                                            .focus_background(Color::TRANSPARENT)
                                            .focus_border_fill(Color::TRANSPARENT)
                                            .placeholder("Search installed locales..."),
                                    ),
                            )
                            .content(Content::Flex),
                    )
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
                                                .background(t.bg_base)
                                                .border(Border::new().width(1.).fill(t.border_subtle))
                                                .child(
                                                    rect()
                                                        .child(
                                                            label()
                                                                .font_size(13.)
                                                                .font_weight(FontWeight::SEMI_BOLD)
                                                                .color(t.text_primary)
                                                                .text(friendly_name.clone()),
                                                        )
                                                        .child(
                                                            label()
                                                                .font_size(11.)
                                                                .color(t.text_secondary)
                                                                .text(code.clone()),
                                                        ),
                                                )
                                                .child(
                                                    rect()
                                                        .padding((3., 8.))
                                                        .corner_radius(4.)
                                                        .background(t.bg_card)
                                                        .child(
                                                            label()
                                                                .font_size(10.)
                                                                .font_weight(FontWeight::BOLD)
                                                                .color(t.accent_green)
                                                                .text("INSTALLED"),
                                                        ),
                                                ),
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
            )
    }
}
