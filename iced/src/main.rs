//hide windows console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

use formato::Formato;
use iced::{
    Color, Element, Font, Length, Subscription, Task, Theme,
    alignment::Alignment,
    event,
    keyboard::{Event, Key, key::Named},
    theme::Base,
    widget::{
        Button, Column, Container, Row, Space, Text, TextInput, button, container, mouse_area,
        operation::{focus_next, focus_previous},
        pick_list, radio, rich_text, scrollable, span, text,
        text::Span,
        tooltip,
    },
    window::{self, icon},
};

//use iced_core::{text::Span, window};
use librusl::{
    extended::ExtendedTrait,
    fileinfo::FileInfo,
    manager::{Manager, SearchResult},
    options::{FTypes, Sort},
    search::Search,
};

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
struct GuiOptions {
    theme: String,
}

impl GuiOptions {
    fn to_theme(&self) -> Theme {
        Theme::ALL.iter().find(|t| t.name() == self.theme).cloned().unwrap_or(Theme::TokyoNight)
    }

    fn from_theme(theme: &Theme) -> Self {
        Self {
            theme: theme.name().to_string(),
        }
    }
}

fn get_gui_config_path() -> Option<String> {
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("rusl");
        if std::fs::create_dir_all(&dir).is_ok() {
            dir.push("gui_config.toml");
            return dir.to_str().map(|s| s.to_string());
        }
    }
    None
}

fn load_gui_options() -> GuiOptions {
    if let Some(file) = get_gui_config_path()
        && let Ok(data) = std::fs::read_to_string(&file)
            && let Ok(opts) = toml::from_str(&data) {
                return opts;
            }
    GuiOptions {
        theme: "TokyoNight".to_string(),
    }
}

fn save_gui_options(opts: &GuiOptions) {
    if let Some(file) = get_gui_config_path()
        && let Ok(toml) = toml::to_string_pretty(opts) {
            let _ = std::fs::write(&file, toml);
        }
}

struct App {
    name: String,
    contents: String,
    directory: String,
    results: Vec<FileInfo>,
    manager: Manager,
    receiver: Receiver<SearchResult>,
    message: String,
    found: usize,
    searching: bool,
    show_settings: bool,
    errors: Vec<String>,
    showing_errors: bool,
    searched_count: usize,
    interim_count: usize,
    current_theme: Theme,
}

#[derive(Debug, Clone)]
pub enum Message {
    FindPressed,
    NameChanged(String),
    ContentsChanged(String),
    DirectoryChanged(String),
    OpenDirectory,
    CheckExternal,
    Event(iced::event::Event),
    CopyAllToClipboard,
    CopySingleToClipboard(String),
    ToggleErrors,
    ToggleSettings,
    Settings(SettingsMessage),
    CycleTheme,
    ThemeSelected(Theme),
}
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    NameCaseSensitive,
    NameSameFilesystem,
    NameIgnoreHidden,
    NameUseGitignore,
    NameFollowSymlinks,
    ContentCaseSensitive,
    ContentExtendedFiletypes,
    ContentLiteralMatch,
    NameType(FTypes),
    SortType(Sort),
}

pub fn main() {
    let image = image::load_from_memory_with_format(include_bytes!("icons/icon.png"), image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();
    let (wid, hei) = image.dimensions();
    let icon = image.into_raw();

    iced::application(App::new, App::update, App::view)
        .theme(|app: &App| app.current_theme.clone())
        .subscription(App::subscription)
        .window(window::Settings {
            icon: Some(icon::from_rgba(icon, wid, hei).unwrap()),
            ..Default::default()
        })
        .run()
        .expect("Could not run app");
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let (s, r) = channel();
        let man = Manager::new(s);
        let gui_opts = load_gui_options();

        let d = Self {
            name: "".to_string(),
            contents: "".to_string(),
            message: "".to_string(),
            directory: man.get_options().last_dir.clone(),
            results: vec![],
            manager: man,
            receiver: r,
            found: 0,
            searching: false,
            show_settings: false,
            errors: vec![],
            showing_errors: false,
            searched_count: 0,
            interim_count: 0,
            current_theme: gui_opts.to_theme(),
        };
        (d, focus_next())
    }

    fn view(&self) -> Element<'_, Message> {
        let name = TextInput::new("Find file name", &self.name)
            .padding(4)
            .on_input(Message::NameChanged)
            .on_submit(Message::FindPressed);
        let contents = TextInput::new("Find contents", &self.contents)
            .on_input(Message::ContentsChanged)
            .padding(4)
            .on_submit(Message::FindPressed);
        let clipboard = if self.results.is_empty() {
            Container::new(Text::new(""))
        } else {
            Container::new(Button::new(Text::new("Clipboard")).on_press(Message::CopyAllToClipboard))
        };
        let dir = TextInput::new("", &self.directory).on_input(Message::DirectoryChanged).padding(4);

        let res = Column::with_children(
            self.results
                .iter()
                .map(|x| -> Element<'_, Message> {
                    let max = 100;
                    let maxlen = 200;

                    let mut rts: Vec<Span> = vec![];
                    let mut start = 0;
                    //directory
                    rts.push(span(&x.path[0..x.path.len() - x.name.len()]));
                    for r in &x.ranges {
                        if start < r.start {
                            rts.push(span(&x.name[start..r.start]).font(Font {
                                weight: iced::font::Weight::Bold,
                                ..Font::default()
                            }));
                        }
                        rts.push(span(&x.name[r.start..r.end]).color(Color::from_rgb8(200, 100, 100)).font(Font {
                            weight: iced::font::Weight::Bold,
                            ..Font::default()
                        }));
                        start = r.end;
                    }
                    if start < x.name.len() {
                        rts.push(span(&x.name[start..]).font(Font {
                            weight: iced::font::Weight::Bold,
                            ..Font::default()
                        }));
                    }
                    // add plugin label span if present
                    if let Some(plug) = &x.plugin {
                        let plugin_label = format!(" ({})", plug.name());
                        rts.push(span(plugin_label).color(Color::from_rgb8(18, 110, 171)));
                    }
                    let rt = rich_text(rts);
                    let icon = if x.path.starts_with("...") {
                        text!("")
                    } else if x.is_folder {
                        text!("📁")
                    } else {
                        text!("📝")
                    };
                    let icon = tooltip(
                        mouse_area(icon).on_press(Message::CopySingleToClipboard(x.path.clone())),
                        container("Click to copy path to clipboard").padding(10).style(container::rounded_box),
                        tooltip::Position::Right,
                    );
                    let row = Row::new().spacing(10).push(icon).push(rt);

                    let mut col = Column::new().push(row);

                    //content matches
                    for cline in x.matches.iter().take(max) {
                        let line_font = Font {
                            weight: iced::font::Weight::Bold,
                            ..Font::default()
                        };
                        let mut cspans: Vec<Span> = vec![span(format!("{}: ", cline.line)).color(Color::from_rgb8(17, 122, 13)).font(line_font)];
                        let mut last = 0;
                        //careful of char boudaries
                        let mut cutoff = cline.content.len().min(maxlen);
                        while cutoff > 0 && cutoff != cline.content.len() && !cline.content.is_char_boundary(cutoff) {
                            cutoff -= 1;
                        }
                        let text = &cline.content[..cutoff];

                        for range in &cline.ranges {
                            if range.start > text.len() || range.end > text.len() {
                                break;
                            }
                            cspans.push(span(text[last..range.start].to_owned()).color(Color::from_rgb8(200, 200, 200)));
                            let match_font = Font {
                                weight: iced::font::Weight::Bold,
                                ..Font::default()
                            };
                            cspans.push(
                                span(text[range.start..range.end].to_owned())
                                    .color(Color::from_rgb8(255, 0, 0))
                                    .font(match_font),
                            );
                            last = range.end;
                        }
                        cspans.push(span(text[last..].to_string()).color(Color::from_rgb8(200, 200, 200)));
                        let content = rich_text(cspans);
                        col = col.push(content);
                    }
                    if x.matches.len() > max {
                        col = col.push(Text::new(format!("... and {} more", x.matches.len() - max)).color(Color::from_rgb8(200, 200, 200)));
                    }
                    // if !content.is_empty() {
                    //     let details = Text::new(content).width(Length::Fill).color(Color::from_rgb8(200, 200, 200));
                    //     col = col.push(details);
                    // }
                    Row::new().spacing(10).push(col).into()
                })
                .collect::<Vec<_>>(),
        );

        let res = scrollable(res).width(Length::Fill);

        let ops = self.manager.get_options();
        let sets = if self.show_settings {
            Some(
                Column::new()
                    .push(Text::new("Name settings").font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Font::default()
                    }))
                    .push(
                        checkbox("Case sensitive", ops.name.case_sensitive)
                            .on_toggle(|| Message::Settings(SettingsMessage::NameCaseSensitive))
                            .into_widget(),
                    )
                    .push(
                        checkbox("Same filesystem", ops.name.same_filesystem)
                            .on_toggle(|| Message::Settings(SettingsMessage::NameSameFilesystem))
                            .into_widget(),
                    )
                    .push(
                        checkbox("Ignore hidden", ops.name.ignore_dot)
                            .on_toggle(|| Message::Settings(SettingsMessage::NameIgnoreHidden))
                            .into_widget(),
                    )
                    .push(
                        checkbox("Use gitignore", ops.name.use_gitignore)
                            .on_toggle(|| Message::Settings(SettingsMessage::NameUseGitignore))
                            .into_widget(),
                    )
                    .push(
                        checkbox("Follow links", ops.name.follow_links)
                            .on_toggle(|| Message::Settings(SettingsMessage::NameFollowSymlinks))
                            .into_widget(),
                    )
                    .push(Space::new().height(Length::Fixed(10.)))
                    .push(Text::new("Content settings").font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Font::default()
                    }))
                    .push(
                        checkbox("Case sensitive", ops.content.case_sensitive)
                            .on_toggle(|| Message::Settings(SettingsMessage::ContentCaseSensitive))
                            .into_widget(),
                    )
                    .push(
                        checkbox("Extended file types", ops.content.extended)
                            .on_toggle(|| Message::Settings(SettingsMessage::ContentExtendedFiletypes))
                            .into_widget(),
                    )
                    .push(
                        checkbox("Literal match (non regex)", ops.content.nonregex)
                            .on_toggle(|| Message::Settings(SettingsMessage::ContentLiteralMatch))
                            .into_widget(),
                    )
                    .push(Space::new().height(Length::Fixed(10.)))
                    .push(Text::new("Appearance").font(Font {
                        weight: iced::font::Weight::Bold,
                        ..Font::default()
                    }))
                    .push({
                        let selected = Some(self.current_theme.clone());
                        Row::new()
                            .align_y(Alignment::Center)
                            .push(Text::new("Theme").width(Length::Fixed(100.)))
                            .push(pick_list(Theme::ALL.to_vec(), selected, Message::ThemeSelected))
                            .push(Space::new().width(Length::Fixed(10.)))
                            .push(Button::new(Text::new("⟳")).on_press(Message::CycleTheme))
                    }),
            )
        } else {
            None
        };

        Column::new()
            .padding(10)
            .spacing(10)
            .push(
                Row::new()
                    .push(Text::new("File name").width(Length::Fixed(100.)))
                    .push(Space::new().width(Length::Fixed(10.)))
                    .push(name),
            )
            .push(
                Row::new()
                    .push(Text::new("Contents").width(Length::Fixed(100.)))
                    .push(Space::new().width(Length::Fixed(10.)))
                    .push(contents),
            )
            .push(
                Row::new()
                    .push(Text::new("Directory").width(Length::Fixed(100.)))
                    .push(Button::new(Text::new("📂")).on_press(Message::OpenDirectory))
                    .push(Space::new().width(Length::Fixed(10.)))
                    .push(dir),
            )
            .push(
                Row::new().push(
                    Row::new()
                        .push(radio("All", FTypes::All, Some(ops.name.file_types), |_| {
                            Message::Settings(SettingsMessage::NameType(FTypes::All))
                        }))
                        .push(radio("Files", FTypes::Files, Some(ops.name.file_types), |_| {
                            Message::Settings(SettingsMessage::NameType(FTypes::Files))
                        }))
                        .push(radio("Folders", FTypes::Directories, Some(ops.name.file_types), |_| {
                            Message::Settings(SettingsMessage::NameType(FTypes::Directories))
                        }))
                        .spacing(10),
                ),
            )
            .push({
                let sort = ops.sort;
                Row::new()
                    .push(text("Sort Results"))
                    .push(radio("None", Sort::None, Some(sort), |_| {
                        Message::Settings(SettingsMessage::SortType(Sort::None))
                    }))
                    .push(radio("Path", Sort::Path, Some(sort), |_| {
                        Message::Settings(SettingsMessage::SortType(Sort::Path))
                    }))
                    .push(radio("Name", Sort::Name, Some(sort), |_| {
                        Message::Settings(SettingsMessage::SortType(Sort::Name))
                    }))
                    .push(radio("Ext", Sort::Extension, Some(sort), |_| {
                        Message::Settings(SettingsMessage::SortType(Sort::Extension))
                    }))
                    .spacing(10)
            })
            .push(Row::new().push(Button::new(Text::new("Settings")).on_press(Message::ToggleSettings)))
            .push(sets)
            .push(
                Row::new()
                    .spacing(15)
                    .align_y(Alignment::Center)
                    .push(if self.searching {
                        Button::new(Container::new(Text::new("Stop")).align_x(Alignment::Center))
                            .width(80)
                            .on_press(Message::FindPressed)
                    } else {
                        Button::new(Container::new(Text::new("Find")).align_x(Alignment::Center))
                            .width(80)
                            .on_press(Message::FindPressed)
                            .style(button::secondary)
                    })
                    .push(Text::new(&self.message))
                    .push(clipboard),
            )
            .push({
                let error_count = self.errors.len();
                let label = if self.showing_errors {
                    "Show results"
                } else if error_count == 1 {
                    "1 error"
                } else {
                    &format!("{} errors", error_count)
                };
                if error_count > 0 {
                    let c: Element<'_, Message> = Container::new(Button::new(Text::new(label.to_string())).on_press(Message::ToggleErrors))
                        .style(container::rounded_box)
                        .into();
                    c
                } else {
                    let c: Element<'_, Message> = Container::new(Text::new("")).into();
                    c
                }
            })
            .push(if self.showing_errors && !self.errors.is_empty() {
                let c: Element<'_, Message> = scrollable(Column::with_children(
                    self.errors
                        .iter()
                        .map(|e| Text::new(e).color(Color::from_rgb8(255, 100, 100)).into())
                        .collect::<Vec<_>>(),
                ))
                .into();
                c
            } else {
                let c: Element<'_, Message> = res.into();
                c
            })
            .into()
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FindPressed => {
                if self.name.is_empty() && self.contents.is_empty() {
                    self.message = "Nothing to search for".to_string();
                    return Task::none();
                }
                if self.searching {
                    self.manager.stop();
                    self.message = format!("Found {} items. Stopped", self.interim_count);

                    self.searching = false;
                } else {
                    self.results.clear();
                    self.errors.clear();
                    self.showing_errors = false;
                    self.searching = true;
                    self.found = 0;
                    self.interim_count = 0;
                    self.searched_count = 0;
                    self.message = "Searching...".to_string();
                    self.manager.search(&Search {
                        dir: self.directory.clone(),
                        name_text: self.name.clone(),
                        contents_text: self.contents.clone(),
                    })
                }
            }
            Message::NameChanged(nn) => self.name = nn,
            Message::ContentsChanged(con) => self.contents = con,
            Message::DirectoryChanged(dir) => {
                self.directory = dir.clone();
                if !self.manager.dir_is_valid(&dir) {
                    self.message = "Invalid directory".to_string();
                } else {
                    self.message = "".to_string();
                }
            }
            Message::CheckExternal => {
                while let Ok(res) = self.receiver.try_recv() {
                    match res {
                        SearchResult::FinalResults(res) => {
                            self.searching = false;
                            let data_len = res.data.len();
                            let display_count = data_len.min(1000);
                            self.results = res.data.into_iter().take(display_count).collect();
                            if data_len > 1000 {
                                self.results.push(FileInfo {
                                    path: format!("...and {} others", data_len - 1000),
                                    matches: vec![],
                                    ext: "".into(),
                                    name: "".into(),
                                    is_folder: false,
                                    plugin: None,
                                    ranges: vec![],
                                });
                            }
                            let filecount = self.results.iter().filter(|x| !x.is_folder).count();
                            let foldercount = self.results.len() - filecount;
                            let mut msg = String::new();
                            if filecount == 0 && foldercount == 0 {
                                msg.push_str("Nothing found");
                            } else {
                                msg.push_str("Found");
                            }
                            if filecount > 0 {
                                msg += &format!(" {} file", filecount.formato("N0"));
                                if filecount > 1 {
                                    msg.push('s');
                                }
                            }
                            if foldercount > 0 {
                                msg += &format!(" {} folder", foldercount.formato("N0"));
                                if foldercount > 1 {
                                    msg.push('s');
                                }
                            }
                            if filecount > 0 && foldercount > 0 {
                                msg += &format!(" {} total", (filecount + foldercount).formato("N0"));
                            }
                            let line_count = self.results.iter().map(|x| x.matches.len()).sum::<usize>();
                            if line_count > 0 {
                                msg += &format!(" with {} lines", line_count.formato("N0"));
                            }
                            msg += &format!(" in {:.3}s", res.duration.as_secs_f64());
                            if res.stopped {
                                msg += " (stopped)";
                            }
                            self.message = msg;
                        }
                        SearchResult::InterimResult(res) => {
                            //only pick up messages if searching (have not found final result) so we dont update ui unnecessarily
                            if self.searching {
                                if self.results.len() < 1000 {
                                    self.results.push(res)
                                }
                                self.interim_count += 1;
                                self.found += 1;
                                self.message = format!(
                                    "Found {} in {} files and folders. Searching...",
                                    self.interim_count.formato("N0"),
                                    self.searched_count.formato("N0")
                                );
                            }
                        }
                        SearchResult::SearchErrors(errs) => {
                            self.errors.extend(errs);
                        }
                        SearchResult::SearchCount(count) => {
                            if self.searching {
                                self.searched_count = count;
                                self.message = format!(
                                    "Found {} in {} files and folders. Searching...",
                                    self.interim_count.formato("N0"),
                                    self.searched_count.formato("N0")
                                );
                            }
                        }
                    }
                }
                if let Err(std::sync::mpsc::TryRecvError::Disconnected) = self.receiver.try_recv() {
                    return Task::none();
                }
            }
            Message::OpenDirectory => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.directory = path.to_string_lossy().to_string()
                }
            }
            Message::Event(iced::Event::Keyboard(Event::KeyPressed {
                key: Key::Named(Named::Tab),
                modifiers,
                ..
            })) => {
                return if modifiers.shift() { focus_previous() } else { focus_next() };
            }
            Message::Event(iced::Event::Window(iced::window::Event::CloseRequested)) => {
                self.manager.save_and_quit();
            }

            Message::CopyAllToClipboard => {
                let text = self.results.iter().map(|x| x.path.clone()).collect::<Vec<_>>().join("\n");
                self.message = "Copied to clipboard".to_string();
                return iced::clipboard::write(text);
            }
            Message::CopySingleToClipboard(path) => {
                self.message = "Copied to clipboard".to_string();
                return iced::clipboard::write(path);
            }
            Message::ToggleErrors => {
                self.showing_errors = !self.showing_errors;
            }
            Message::ToggleSettings => {
                self.show_settings = !self.show_settings;
            }
            Message::Settings(ms) => {
                let mut ops = self.manager.get_options().clone();
                match ms {
                    SettingsMessage::ContentCaseSensitive => ops.content.case_sensitive = !ops.content.case_sensitive,
                    SettingsMessage::ContentExtendedFiletypes => ops.content.extended = !ops.content.extended,
                    SettingsMessage::ContentLiteralMatch => ops.content.nonregex = !ops.content.nonregex,
                    SettingsMessage::NameCaseSensitive => ops.name.case_sensitive = !ops.name.case_sensitive,
                    SettingsMessage::NameFollowSymlinks => ops.name.follow_links = !ops.name.follow_links,
                    SettingsMessage::NameIgnoreHidden => ops.name.ignore_dot = !ops.name.ignore_dot,
                    SettingsMessage::NameSameFilesystem => ops.name.same_filesystem = !ops.name.same_filesystem,
                    SettingsMessage::NameType(nt) => ops.name.file_types = nt,
                    SettingsMessage::NameUseGitignore => ops.name.use_gitignore = !ops.name.use_gitignore,
                    SettingsMessage::SortType(sort) => ops.sort = sort,
                }
                self.manager.set_options(ops);
                self.manager.save();
            }
            Message::CycleTheme => {
                let idx = Theme::ALL.iter().position(|t| *t == self.current_theme).unwrap_or(0);
                self.current_theme = Theme::ALL[(idx + 1) % Theme::ALL.len()].clone();
                save_gui_options(&GuiOptions::from_theme(&self.current_theme));
            }
            Message::ThemeSelected(theme) => {
                self.current_theme = theme;
                save_gui_options(&GuiOptions::from_theme(&self.current_theme));
            }
            Message::Event(_) => {}
        }

        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        Subscription::batch(vec![
            //keep looking for external messages.
            //this is a hack and polls receiver.
            //TODO: notify gui only if necessary (once results received) - dont know if possible with ICED
            iced::time::every(Duration::from_millis(100)).map(|_| Message::CheckExternal),
            //keyboard events
            event::listen().map(Message::Event),
        ])
    }
}

pub struct MyCheckbox<'a> {
    label: &'a str,
    checked: bool,
    callback: Option<Box<dyn Fn() -> Message>>,
}

impl<'a> MyCheckbox<'a> {
    pub fn on_toggle(mut self, callback: impl Fn() -> Message + 'static) -> Self {
        self.callback = Some(Box::new(callback));
        self
    }

    pub fn into_widget(self) -> iced::widget::Row<'a, Message> {
        let cb = self.callback.unwrap();
        let checked = self.checked;
        iced::widget::Row::new()
            .align_y(Alignment::Center)
            .height(Length::Fixed(30.))
            .push(iced::widget::checkbox(checked).on_toggle(move |_| cb()))
            .push(Space::new().width(Length::Fixed(20.)))
            .push(iced::widget::text(self.label))
    }
}

pub fn checkbox<'a>(label: &'a str, checked: bool) -> MyCheckbox<'a> {
    MyCheckbox {
        label,
        checked,
        callback: None,
    }
}
