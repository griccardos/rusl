//hide windows console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

use iced::{
    executor, widget::scrollable, widget::Button, widget::Column, widget::Row, widget::Space, widget::Text, widget::TextInput, window::icon,
    Application, Color, Command, Element, Length, Settings, Theme,
};

use librusl::{
    fileinfo::FileInfo,
    manager::{Manager, SearchResult},
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
}

#[derive(Debug, Clone)]
pub enum Message {
    FindPressed,
    NameChanged(String),
    ContentsChanged(String),
    DirectoryChanged(String),
    OpenDirectory,
    CheckExternal,
}

pub fn main() {
    let mut sets = Settings::default();
    sets.default_text_size = 20.;
    sets.antialiasing = true;
    let image = image::load_from_memory_with_format(include_bytes!("icons/icon.png"), image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();
    let (wid, hei) = image.dimensions();
    let icon = image.into_raw();
    sets.window.icon = Some(icon::from_rgba(icon, wid, hei).unwrap());
    App::run(sets).unwrap();
}

impl Application for App {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        let (s, r) = channel();
        let d = Self {
            name: "".to_string(),
            contents: "".to_string(),
            message: "".to_string(),
            directory: ".".to_string(),
            results: vec![],
            manager: Manager::new(s),
            receiver: r,
        };
        (d, Command::none())
    }

    fn title(&self) -> String {
        "rusl".into()
    }

    fn view(&self) -> Element<Self::Message> {
        let name = TextInput::new("Find file name", &self.name).padding(4).on_input(Message::NameChanged);
        let contents = TextInput::new("Find contents", &self.contents)
            .on_input(Message::ContentsChanged)
            .padding(4);
        let dir = TextInput::new("", &self.directory).on_input(Message::DirectoryChanged).padding(4);
        let res = Column::with_children(
            self.results
                .iter()
                .map(|x| {
                    let max = 100;
                    let maxlen = 200;
                    let mut content = x.content(max, maxlen);
                    if x.matches.len() > max {
                        content.push_str(&format!("\nand {} other lines", x.matches.len() - max));
                    }
                    Row::new()
                        .spacing(10)
                        .push(
                            Column::new()
                                .push(Text::new(&x.path).style(Color::from_rgb8(100, 200, 100)))
                                .push(Text::new(content).width(Length::Fill)),
                        )
                        .into()
                })
                .collect(),
        );
        let res = scrollable::Scrollable::new(res);
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
                    .push(Text::new("File contents").width(Length::Fixed(100.)))
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
                    .align_items(iced::Alignment::End)
                    .push(Button::new(Text::new("Find")).on_press(Message::FindPressed))
                    .push(Text::new(&self.message)),
            )
            .push(res)
            .into()
    }
    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match message {
            Message::FindPressed => {
                self.results.clear();
                self.message = "Searching...".to_string();
                self.manager.search(Search {
                    dir: self.directory.clone(),
                    name_text: self.name.clone(),
                    contents_text: self.contents.clone(),
                })
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
                if let Ok(res) = self.receiver.try_recv() {
                    match res {
                        SearchResult::FinalResults(res) => {
                            self.message = format!("Found {} items in {:.2}s", res.data.len(), res.duration.as_secs_f64());
                            self.results = res.data.iter().take(1000).cloned().collect();
                            if res.data.len() > 1000 {
                                self.results.push(FileInfo {
                                    path: format!("...and {} others", res.data.len() - 1000),
                                    matches: vec![],
                                    ext: "".into(),
                                    name: "".into(),
                                    is_folder: false,
                                    plugin: None,
                                });
                            }
                        }
                        SearchResult::InterimResult(res) => {
                            if self.results.len() < 1000 {
                                self.results.push(res)
                            }
                        }
                        SearchResult::SearchErrors(_) => { /*todo show errors*/ }
                    }
                }
            }
            Message::OpenDirectory => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.directory = path.to_string_lossy().to_string()
                }
            }
        }
        Command::none()
    }
    fn theme(&self) -> Theme {
        Theme::Dark
    }
    fn subscription(&self) -> iced::Subscription<Self::Message> {
        //keep looking for external messages.
        //this is a hack and polls receiver.
        //TODO: notify gui only if necessary (once results received) - dont know if possible with ICED
        iced::time::every(Duration::from_millis(10)).map(|_| Message::CheckExternal)
    }
}
