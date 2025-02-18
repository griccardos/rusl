//hide windows console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

use iced::{
    event,
    keyboard::{key::Named, Event, Key},
    widget::{
        self, checkbox, container, mouse_area, radio, rich_text, scrollable, span, text, text::Span, tooltip, Button, Column, Container, Row, Space,
        Text, TextInput,
    },
    window::{self, icon},
    Color, Element, Font, Length, Subscription, Task, Theme,
};

//use iced_core::{text::Span, window};
use librusl::{
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
    CopyToClipboard(Vec<String>),
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

    iced::application(App::title, App::update, App::view)
        .theme(|_| Theme::TokyoNight)
        .subscription(App::subscription)
        .window(window::Settings {
            icon: Some(icon::from_rgba(icon, wid, hei).unwrap()),
            ..Default::default()
        })
        .run_with(App::new)
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
        };
        (d, widget::focus_next())
    }

    fn title(&self) -> String {
        "rusl".into()
    }

    fn view(&self) -> Element<Message> {
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
            Container::new(
                Button::new(Text::new("Clipboard")).on_press(Message::CopyToClipboard(self.results.iter().map(|x| x.path.clone()).collect())),
            )
        };
        let dir = TextInput::new("", &self.directory).on_input(Message::DirectoryChanged).padding(4);

        let res = Column::with_children(
            self.results
                .iter()
                .map(|x| {
                    let max = 100;
                    let maxlen = 200;

                    let mut rts: Vec<Span> = vec![];
                    let mut start = 0;
                    //directory
                    rts.push(span(&x.path[0..x.path.len() - &x.name.len()]));
                    for r in &x.ranges {
                        //if not in range, print up to range
                        if start < r.start {
                            rts.push(span(&x.name[start..r.start]));
                        }
                        //now print the range
                        rts.push({
                            let mut font = Font::default();
                            font.weight = iced::font::Weight::Bold;
                            span(&x.name[r.start..r.end]).color(Color::from_rgb8(200, 100, 100)).font(font)
                        });
                        start = r.end;
                    }
                    if start < x.name.len() {
                        rts.push(span(&x.name[start..]));
                    }
                    let rt = rich_text(rts);
                    let icon = if x.path.starts_with("...") {
                        text!("")
                    } else if x.is_folder {
                        text!("D")
                    } else {
                        text!("F")
                    }; //does not support unicode yet

                    let icon = tooltip(
                        mouse_area(icon).on_press(Message::CopyToClipboard(vec![x.path.clone()])),
                        container("Click to copy path to clipboard").padding(10).style(container::rounded_box),
                        tooltip::Position::Right,
                    );

                    let row = Row::new().spacing(10).push(icon).push(rt);

                    let mut col = Column::new().push(row);

                    //content matches
                    for cline in x.matches.iter().take(max) {
                        let mut cspans: Vec<Span> = vec![span(format!("{}: ", cline.line)).color(Color::from_rgb8(17, 122, 13))];
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
                            cspans.push(span(text[range.start..range.end].to_owned()).color(Color::from_rgb8(255, 0, 0)));
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

        let res = scrollable(res);

        Column::new()
            .padding(10)
            .spacing(10)
            .push(
                Row::new()
                    .push(Text::new("File name").width(Length::Fixed(100.)))
                    .push(Space::new(iced::Length::Fixed(10.), iced::Length::Shrink))
                    .push(name),
            )
            .push(
                Row::new()
                    .push(Text::new("Contents").width(Length::Fixed(100.)))
                    .push(Space::new(iced::Length::Fixed(10.), iced::Length::Shrink))
                    .push(contents),
            )
            .push(
                Row::new()
                    .push(Text::new("Directory").width(Length::Fixed(100.)))
                    .push(Button::new(Text::new("+")).on_press(Message::OpenDirectory))
                    .push(Space::new(iced::Length::Fixed(10.), iced::Length::Shrink))
                    .push(dir),
            )
            .push(
                Row::new()
                    .spacing(15)
                    .push(Button::new(Text::new("Settings")).on_press(Message::ToggleSettings))
                    .push_maybe(if self.show_settings {
                        let ops = self.manager.get_options();
                        Some(
                            Column::new()
                                .push(text("Name settings"))
                                .push(
                                    checkbox("Case sensitive", ops.name.case_sensitive)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::NameCaseSensitive)),
                                )
                                .push(
                                    checkbox("Same filesystem", ops.name.same_filesystem)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::NameSameFilesystem)),
                                )
                                .push(
                                    checkbox("Ignore hidden", ops.name.ignore_dot)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::NameIgnoreHidden)),
                                )
                                .push(
                                    checkbox("Use gitignore", ops.name.use_gitignore)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::NameUseGitignore)),
                                )
                                .push(
                                    checkbox("Follow links", ops.name.follow_links)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::NameFollowSymlinks)),
                                )
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
                                .push(text("Content settings"))
                                .push(
                                    checkbox("Case sensitive", ops.content.case_sensitive)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::ContentCaseSensitive)),
                                )
                                .push(
                                    checkbox("Extended file types", ops.content.extended)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::ContentExtendedFiletypes)),
                                )
                                .push(
                                    checkbox("Literal match (non regex)", ops.content.nonregex)
                                        .on_toggle(|_| Message::Settings(SettingsMessage::ContentLiteralMatch)),
                                ),
                        )
                    } else {
                        None
                    }),
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
            .push(res)
            .into()
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FindPressed => {
                if self.searching {
                    self.manager.stop();
                    self.message = format!("Found {} items. Stopped", self.found);

                    self.searching = false;
                } else {
                    self.results.clear();
                    self.searching = true;
                    self.found = 0;
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
                            self.message = format!("Found {} items in {:.2}s", res.data.len(), res.duration.as_secs_f64());
                            //self.results = res.data.iter().take(1000).cloned().collect();
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
                            self.found += 1;
                            self.message = format!("Found {}, searching...", self.found);
                        }
                        SearchResult::SearchErrors(_) => {}
                        SearchResult::SearchCount(_) => {}
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
                return if modifiers.shift() {
                    widget::focus_previous()
                } else {
                    widget::focus_next()
                };
            }
            Message::Event(iced::Event::Window(iced::window::Event::CloseRequested)) => {
                self.manager.save_and_quit();
            }

            Message::CopyToClipboard(str) => {
                self.manager.export(str);
                self.message = "Copied to clipboard".to_string();
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
