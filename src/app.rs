use cosmic::app::Core;
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{
    destroy_popup, get_popup,
};
use cosmic::iced::{window::Id, Alignment, Length, Subscription};
use cosmic::widget::{self, settings, text};
use cosmic::{executor, Application, Element, Task};
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_ID: &str = "com.ali.WhenPrayerApplet";
const FETCH_INTERVAL_SECS: u64 = 120;

#[derive(Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct NextPrayer {
    pub name: String,
    pub remaining: String,
    pub time: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PrayerTime {
    pub name: String,
    pub time: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub show_name: bool,
    pub show_time: bool,
    pub show_countdown: bool,
    pub show_current_period: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_name: true,
            show_time: true,
            show_countdown: false,
            show_current_period: false,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum PopupPage {
    #[default]
    Main,
    Settings,
}

#[derive(Default)]
pub struct AppModel {
    core: Core,
    popup: Option<Id>,
    page: PopupPage,
    next_prayer: Option<NextPrayer>,
    all_prayers: Vec<PrayerTime>,
    prayer_date: String,
    config: Config,
    remaining_secs: Option<u64>,
    alarm_running: bool,
    cached_current_name: Option<String>,
    cached_formatted_date: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    OpenSettings,
    CloseSettings,
    DataUpdated(Option<NextPrayer>, Vec<PrayerTime>),
    SetDate(String),
    Tick,
    ToggleShowName,
    ToggleShowTime,
    ToggleShowCountdown,
    ToggleShowCurrentPeriod,
    ToggleAlarm,
    AlarmStatus(bool),
}

impl Application for AppModel {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config = load_config();
        let mut app = AppModel {
            core,
            config,
            ..Default::default()
        };
        app.next_prayer = fetch_next_prayer();
        app.all_prayers = fetch_all_prayers();
        app.prayer_date = fetch_date();
        if let Some(ref next) = app.next_prayer {
            app.remaining_secs = parse_remaining(&next.remaining);
        }
        app.cached_current_name = app.current_prayer_name();
        app.cached_formatted_date = format_date(&app.prayer_date);
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let (_, panel_h) = self.core.applet.suggested_window_size();
        let h = panel_h.get() as f32;

        let icon_name = "weather-clear-night-symbolic";

        let icon = widget::icon(widget::icon::from_name(icon_name).symbolic(true).into())
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0));

        let mut parts: Vec<Element<'_, Self::Message>> = vec![icon.into()];

        if let Some(ref next) = self.next_prayer {
            if self.config.show_current_period
                && let Some(ref current) = self.cached_current_name
            {
                parts.push(
                    text::body(format!(" {}", current))
                        .size(10.0)
                        .into(),
                );
                parts.push(text::body("→").size(10.0).into());
            }

            if self.config.show_name {
                parts.push(
                    text::body(format!(" {}", next.name))
                        .size(10.0)
                        .font(cosmic::font::semibold())
                        .class(cosmic::theme::Text::Accent)
                        .into(),
                );
            }

            if self.config.show_countdown {
                let display = self
                    .remaining_secs
                    .map(format_remaining)
                    .unwrap_or_default();
                if !display.is_empty() {
                    parts.push(
                        widget::container(text::body(format!(" {}", display)).size(10.0))
                            .padding([0.0, 4.0])
                            .into(),
                    );
                }
            } else if self.config.show_time {
                parts.push(
                    text::body(format!(" {}", next.time))
                        .size(10.0)
                        .into(),
                );
            }
        }

        let has_text = parts.len() > 1;
        let row = widget::row::with_children(parts)
            .align_y(Alignment::Center)
            .spacing(if has_text { 4.0 } else { 0.0 });

        let pill_height = h * 0.85;
        let pill_radius = pill_height / 2.0;

        let content = widget::container(row)
            .height(Length::Fixed(pill_height))
            .width(Length::Shrink)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .padding([0.0, pill_radius / 2.0]);

        let btn = widget::button::custom(self.core.applet.autosize_window(content))
            .on_press(Message::TogglePopup)
            .class(cosmic::theme::Button::AppletIcon);

        btn.into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let mut list =
            widget::list_column().style(cosmic::theme::Container::Transparent);

        if self.page == PopupPage::Settings {
            let back = widget::button::icon(
                widget::icon::from_name("go-previous-symbolic").symbolic(true),
            )
            .on_press(Message::CloseSettings);

            list = list.add(
                widget::container(widget::settings::item_row(vec![
                    back.into(),
                    widget::text::heading("Settings")
                        .width(Length::Fill)
                        .into(),
                ]))
                .width(Length::Fill)
                .padding([2, 0, 2, 0]),
            );

            list = list.add(
                settings::item::builder("Show prayer name")
                    .toggler(self.config.show_name, |_| Message::ToggleShowName),
            );
            list = list.add(
                settings::item::builder("Show prayer time")
                    .toggler(self.config.show_time && !self.config.show_countdown, |_| {
                        Message::ToggleShowTime
                    }),
            );
            list = list.add(
                settings::item::builder("Show countdown")
                    .toggler(self.config.show_countdown, |_| Message::ToggleShowCountdown),
            );
            list = list.add(
                settings::item::builder("Show current period")
                    .toggler(self.config.show_current_period, |_| {
                        Message::ToggleShowCurrentPeriod
                    }),
            );

            let alarm_label = if self.alarm_running {
                "Alarm daemon: running"
            } else {
                "Alarm daemon: stopped"
            };
            list = list.add(
                settings::item::builder(alarm_label)
                    .toggler(self.alarm_running, |_| Message::ToggleAlarm),
            );
        } else {
            let gear = widget::button::icon(
                widget::icon::from_name("preferences-system-symbolic").symbolic(true),
            )
            .on_press(Message::OpenSettings);

            list = list.add(
                widget::container(widget::settings::item_row(vec![
                    widget::text::heading(self.cached_formatted_date.as_str())
                        .width(Length::Fill)
                        .into(),
                    gear.into(),
                ]))
                .width(Length::Fill)
                .padding([2, 0, 2, 0]),
            );

            if self.all_prayers.is_empty() {
                list = list.add(widget::settings::item_row(vec![
                    widget::text::body("No prayer times available").into(),
                ]));
            } else {
                let next_name = self.next_prayer.as_ref().map(|n| n.name.as_str());
                for prayer in &self.all_prayers {
                    let is_next = next_name == Some(prayer.name.as_str());
                    if is_next {
                        list = list.add(
                            widget::settings::item_row(vec![
                                widget::text::body(format!("→ {}", prayer.name))
                                    .width(Length::Fill)
                                    .into(),
                                widget::text::body(&prayer.time).into(),
                            ])
                            .padding([6, 8]),
                        );
                    } else {
                        list = list.add(
                            widget::settings::item_row(vec![
                                widget::text::body(prayer.name.as_str())
                                    .width(Length::Fill)
                                    .into(),
                                widget::text::body(&prayer.time).into(),
                            ])
                            .padding([6, 8]),
                        );
                    }
                }
            }
        }

        self.core.applet.popup_container(list)
            .max_width(320.0)
            .min_width(260.0)
            .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subs: Vec<Subscription<Self::Message>> = vec![];

        subs.push(Subscription::run(|| {
            cosmic::iced::stream::channel(
                4,
                move |mut channel: futures::channel::mpsc::Sender<_>| async move {
                    let mut interval =
                        tokio::time::interval(tokio::time::Duration::from_secs(
                            FETCH_INTERVAL_SECS,
                        ));
                    loop {
                        interval.tick().await;
                        let next = fetch_next_prayer_async().await;
                        let all = fetch_all_prayers_async().await;
                        _ = channel.send(Message::DataUpdated(next, all)).await;
                        let date = fetch_date_async().await;
                        _ = channel.send(Message::SetDate(date)).await;
                        let running = check_alarm_daemon().await;
                        _ = channel.send(Message::AlarmStatus(running)).await;
                    }
                },
            )
        }));

        if self.config.show_countdown {
            subs.push(Subscription::run(|| {
                cosmic::iced::stream::channel(
                    5,
                    move |mut channel: futures::channel::mpsc::Sender<_>| async move {
                        let mut interval =
                            tokio::time::interval(tokio::time::Duration::from_secs(1));
                        loop {
                            interval.tick().await;
                            _ = channel.send(Message::Tick).await;
                        }
                    },
                )
            }));
        }

        Subscription::batch(subs)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    self.page = PopupPage::Main;
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                    self.page = PopupPage::Main;
                }
            }
            Message::OpenSettings => {
                self.page = PopupPage::Settings;
            }
            Message::CloseSettings => {
                self.page = PopupPage::Main;
            }
            Message::DataUpdated(next, all) => {
                self.next_prayer = next;
                self.all_prayers = all;
                if let Some(ref next) = self.next_prayer {
                    self.remaining_secs = parse_remaining(&next.remaining);
                } else {
                    self.remaining_secs = None;
                }
                self.update_cached_strings();
            }
            Message::SetDate(date) => {
                self.prayer_date = date;
                self.cached_formatted_date = format_date(&self.prayer_date);
            }
            Message::Tick => {
                if let Some(secs) = self.remaining_secs.as_mut()
                    && *secs > 0 {
                    *secs -= 1;
                }
                self.update_cached_strings();
            }
            Message::ToggleShowName => {
                self.config.show_name = !self.config.show_name;
                save_config(&self.config);
            }
            Message::ToggleShowTime => {
                self.config.show_time = !self.config.show_time;
                if self.config.show_countdown {
                    self.config.show_countdown = false;
                }
                save_config(&self.config);
            }
            Message::ToggleShowCountdown => {
                self.config.show_countdown = !self.config.show_countdown;
                if self.config.show_countdown {
                    self.config.show_time = false;
                }
                save_config(&self.config);
            }
            Message::ToggleShowCurrentPeriod => {
                self.config.show_current_period = !self.config.show_current_period;
                save_config(&self.config);
            }
            Message::ToggleAlarm => {
                if self.alarm_running {
                    let _ = std::process::Command::new("pkill")
                        .args(["-f", "when alarm"])
                        .output();
                    self.alarm_running = false;
                } else {
                    let _ = std::process::Command::new("sh")
                        .args(["-c", "nohup when alarm --daemon > /dev/null 2>&1 &"])
                        .output();
                    self.alarm_running = true;
                }
            }
            Message::AlarmStatus(running) => {
                self.alarm_running = running;
            }
        }
        Task::none()
    }
}

impl AppModel {
    fn update_cached_strings(&mut self) {
        self.cached_current_name = self.current_prayer_name();
    }

    fn current_time_str(&self) -> Option<String> {
        let next = self.next_prayer.as_ref()?;
        let secs = self.remaining_secs?;
        let h: u8 = next.time[..2].parse().ok()?;
        let m: u8 = next.time[3..5].parse().ok()?;
        let next_total = h as u64 * 3600 + m as u64 * 60;
        let cur_total = next_total.saturating_sub(secs) % 86400;
        let ch = (cur_total / 3600) as u8;
        let cm = ((cur_total % 3600) / 60) as u8;
        Some(format!("{:02}:{:02}", ch, cm))
    }

    fn current_prayer_name(&self) -> Option<String> {
        let current = self.current_time_str()?;
        let prayers = &self.all_prayers;
        if prayers.is_empty() {
            return None;
        }
        let idx = prayers.iter().rposition(|p| p.time.as_str() <= current.as_str());
        let name = match idx {
            Some(i) => prayers[i].name.as_str(),
            None => prayers.last().map(|p| p.name.as_str()).unwrap(),
        };
        Some(name.to_string())
    }

}

fn parse_remaining(s: &str) -> Option<u64> {
    let mut secs = 0u64;
    let mut num = 0u64;
    for c in s.chars() {
        match c {
            '0'..='9' => num = num * 10 + (c as u64 - '0' as u64),
            'h' => {
                secs += num * 3600;
                num = 0;
            }
            'm' => {
                secs += num * 60;
                num = 0;
            }
            's' => {
                secs += num;
                num = 0;
            }
            _ => {}
        }
    }
    Some(secs)
}

fn format_remaining(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {:02}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

async fn check_alarm_daemon() -> bool {
    tokio::process::Command::new("pgrep")
        .args(["-f", "when alarm"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn config_dir() -> PathBuf {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config")
        });
    config_home.join("when-cosmic-applet")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn load_config() -> Config {
    let path = config_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &Config) {
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = fs::write(config_path(), json);
    }
}

fn fetch_next_prayer() -> Option<NextPrayer> {
    let output = std::process::Command::new("when")
        .args(["next", "--json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn fetch_all_prayers() -> Vec<PrayerTime> {
    let Ok(output) = std::process::Command::new("when")
        .args(["all", "--json"])
        .output()
    else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

async fn fetch_next_prayer_async() -> Option<NextPrayer> {
    let output = tokio::process::Command::new("when")
        .args(["next", "--json"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

async fn fetch_all_prayers_async() -> Vec<PrayerTime> {
    let Ok(output) = tokio::process::Command::new("when")
        .args(["all", "--json"])
        .output()
        .await
    else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

fn cache_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".cache/when/prayer-times.json")
}

fn fetch_date() -> String {
    let path = cache_path();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    #[derive(Deserialize)]
    struct Cache {
        date: String,
    }
    serde_json::from_str::<Cache>(&content)
        .map(|c| c.date)
        .unwrap_or_default()
}

async fn fetch_date_async() -> String {
    let path = cache_path();
    let content = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    #[derive(Deserialize)]
    struct Cache {
        date: String,
    }
    serde_json::from_str::<Cache>(&content)
        .map(|c| c.date)
        .unwrap_or_default()
}

fn format_date(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return date.to_string();
    }
    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };
    let day = parts[2].trim_start_matches('0');
    format!("{} {}, {}", month, day, parts[0])
}
