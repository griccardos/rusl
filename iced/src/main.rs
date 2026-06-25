//hide windows console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

use formato::Formato;
use iced::{
    Color, Element, Font, Length, Subscription, Task, Theme, event,
    keyboard::{Event, Key, key::Named},
    widget::{
        Button, Column, Container, Row, Space, Text, TextInput, container, mouse_area,
        operation::{focus_next, focus_previous},
        radio, rich_text, scrollable, span, text,
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
    options::FTypes,
    search::Search,
};

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
}

pub fn main() {
    let image = image::load_from_memory_with_format(include_bytes!("icons/icon.png"), image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();
    let (wid, hei) = image.dimensions();
    let icon = image.into_raw();

    iced::application(App::new, App::update, App::view)
        .theme(Theme::TokyoNight)
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

        let sets =
            if self.show_settings {
                let ops = self.manager.get_options();
                Some(
                    Column::new()
                        .push(Text::new("Name settings"))
                        .push(Row::new().push(Text::new("Case sensitive")).push(
                            iced::widget::checkbox(ops.name.case_sensitive).on_toggle(|_| Message::Settings(SettingsMessage::NameCaseSensitive)),
                        ))
                        .push(Row::new().push(Text::new("Same filesystem")).push(
                            iced::widget::checkbox(ops.name.same_filesystem).on_toggle(|_| Message::Settings(SettingsMessage::NameSameFilesystem)),
                        ))
                        .push(
                            Row::new().push(Text::new("Ignore hidden")).push(
                                iced::widget::checkbox(ops.name.ignore_dot).on_toggle(|_| Message::Settings(SettingsMessage::NameIgnoreHidden)),
                            ),
                        )
                        .push(
                            Row::new().push(Text::new("Use gitignore")).push(
                                iced::widget::checkbox(ops.name.use_gitignore).on_toggle(|_| Message::Settings(SettingsMessage::NameUseGitignore)),
                            ),
                        )
                        .push(Row::new().push(Text::new("Follow links")).push(
                            iced::widget::checkbox(ops.name.follow_links).on_toggle(|_| Message::Settings(SettingsMessage::NameFollowSymlinks)),
                        ))
                        .push(
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
                        )
                        .push(Text::new("Content settings"))
                        .push(
                            Row::new().push(Text::new("Case sensitive")).push(
                                iced::widget::checkbox(ops.content.case_sensitive)
                                    .on_toggle(|_| Message::Settings(SettingsMessage::ContentCaseSensitive)),
                            ),
                        )
                        .push(Row::new().push(Text::new("Extended file types")).push(
                            iced::widget::checkbox(ops.content.extended).on_toggle(|_| Message::Settings(SettingsMessage::ContentExtendedFiletypes)),
                        ))
                        .push(Row::new().push(Text::new("Literal match (non regex)")).push(
                            iced::widget::checkbox(ops.content.nonregex).on_toggle(|_| Message::Settings(SettingsMessage::ContentLiteralMatch)),
                        )),
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
                    .push(Button::new(Text::new("+")).on_press(Message::OpenDirectory))
                    .push(Space::new().width(Length::Fixed(10.)))
                    .push(dir),
            )
            .push(
                Row::new()
                    .spacing(15)
                    .push(Button::new(Text::new("Settings")).on_press(Message::ToggleSettings))
                    .push(sets),
            )
            .push(
                Row::new()
                    .spacing(15)
                    //.align_items(iced::Alignment::End)
                    .push(if self.searching {
                        Button::new(Text::new("Stop")).on_press(Message::FindPressed)
                    } else {
                        Button::new(Text::new("Find")).on_press(Message::FindPressed)
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
                            let filecount = res.data.iter().filter(|x| !x.is_folder).count();
                            let foldercount = res.data.len() - filecount;
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
                            let line_count = res.data.iter().map(|x| x.matches.len()).sum::<usize>();
                            if line_count > 0 {
                                msg += &format!(" with {} lines", line_count.formato("N0"));
                            }
                            msg += &format!(" in {:.3}s", res.duration.as_secs_f64());
                            if res.stopped {
                                msg += " (stopped)";
                            }
                            self.message = msg;
                            if res.data.len() > 1000 {
                                self.results.push(FileInfo {
                                    path: format!("...and {} others", res.data.len() - 1000),
                                    matches: vec![],
                                    ext: "".into(),
                                    name: "".into(),
                                    is_folder: false,
                                    plugin: None,
                                    ranges: vec![],
                                });
                            }
                        }
                        SearchResult::InterimResult(res) => {
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
                        SearchResult::SearchErrors(errs) => {
                            self.errors.extend(errs);
                        }
                        SearchResult::SearchCount(count) => {
                            self.searched_count = count;
                            self.message = format!(
                                "Found {} in {} files and folders. Searching...",
                                self.interim_count.formato("N0"),
                                self.searched_count.formato("N0")
                            );
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
                    SettingsMessage::NameCaseSensitive => ops.name.case_sensitive = !ops.name.case_sensitive,
                    SettingsMessage::NameSameFilesystem => ops.name.same_filesystem = !ops.name.same_filesystem,
                    SettingsMessage::ContentCaseSensitive => ops.content.case_sensitive = !ops.content.case_sensitive,
                    SettingsMessage::NameIgnoreHidden => ops.name.ignore_dot = !ops.name.ignore_dot,
                    SettingsMessage::NameUseGitignore => ops.name.use_gitignore = !ops.name.use_gitignore,
                    SettingsMessage::NameFollowSymlinks => ops.name.follow_links = !ops.name.follow_links,
                    SettingsMessage::NameType(nt) => ops.name.file_types = nt,
                    SettingsMessage::ContentLiteralMatch => ops.content.nonregex = !ops.content.nonregex,
                    SettingsMessage::ContentExtendedFiletypes => ops.content.extended = !ops.content.extended,
                }
                self.manager.set_options(ops);
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
            iced::time::every(Duration::from_millis(10)).map(|_| Message::CheckExternal),
            //keyboard events
            event::listen().map(Message::Event),
        ])
    }
}

pub struct MyCheckbox {
    label: &'static str,
    checked: bool,
    callback: Option<fn() -> Message>,
}

impl MyCheckbox {
    pub fn on_toggle(mut self, callback: fn() -> Message) -> Self {
        self.callback = Some(callback);
        self
    }

    pub fn into_widget(self) -> iced::widget::Row<'static, Message> {
        let cb = self.callback.unwrap();
        iced::widget::Row::new()
            .push(iced::widget::text(self.label))
            .push(iced::widget::checkbox(self.checked).on_toggle(move |_| cb()))
    }
}

pub fn checkbox(label: &'static str, checked: bool) -> MyCheckbox {
    MyCheckbox {
        label,
        checked,
        callback: None,
    }
}
