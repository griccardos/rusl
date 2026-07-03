//hide windows console
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use librusl::{
    extended::ExtendedTrait,
    fileinfo::FileInfo,
    manager::{Manager, SearchResult},
    options::{FTypes, Sort},
    search::Search,
};

const MAX_DETAIL_LINES: usize = 100;

const BOLD: Font = Font {
    family: Family::SansSerif,
    weight: Bold,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
struct GuiOptions {
    theme: String,
    show_settings: bool,
    display_limit: usize,
}

impl GuiOptions {
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
    fn load() -> GuiOptions {
        if let Some(file) = GuiOptions::get_gui_config_path()
            && let Ok(data) = std::fs::read_to_string(&file)
            && let Ok(opts) = toml::from_str(&data)
        {
            return opts;
        }
        GuiOptions {
            theme: Theme::TokyoNight.name().to_string(),
            show_settings: false,
            display_limit: 1000,
        }
    }

    fn save(&self) {
        if let Some(file) = GuiOptions::get_gui_config_path()
            && let Ok(toml) = toml::to_string_pretty(self)
        {
            let _ = std::fs::write(&file, toml);
        }
    }

    fn theme(&self) -> Theme {
        Theme::ALL.iter().find(|t| t.name() == self.theme).cloned().unwrap_or(Theme::TokyoNight)
    }

    fn set_theme(&mut self, theme: &Theme) {
        self.theme = theme.to_string();
    }
}

struct App {
    name: String,
    contents: String,
    directory: String,
    display_results: Vec<FileInfo>,
    full_results: Vec<FileInfo>,
    manager: Manager,
    receiver: Receiver<SearchResult>,
    message: String,
    found: usize,
    searching: bool,
    errors: Vec<String>,
    showing_errors: bool,
    searched_count: usize,
    interim_count: usize,
    gui_options: GuiOptions,
    size_compare: String,
    size_value: String,
    show_clipboard_copied: bool,
    expanded: HashSet<String>,
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
    ClearClipboardMessage,
    ToggleExpand(String),
    ToggleExpandAll,
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
    DisplayLimit(String),
    SizeOperator(String),
    SizeValue(String),
}

pub fn main() {
    let image = image::load_from_memory_with_format(include_bytes!("icons/icon.png"), image::ImageFormat::Png)
        .unwrap()
        .into_rgba8();
    let (wid, hei) = image.dimensions();
    let icon = image.into_raw();

    iced::application(App::new, App::update, App::view)
        .theme(|app: &App| app.gui_options.theme())
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
        let gui_options = GuiOptions::load();
        let ops = man.get_options();

        let d = Self {
            name: "".to_string(),
            contents: "".to_string(),
            message: "".to_string(),
            directory: man.get_options().last_dir.clone(),
            display_results: vec![],
            full_results: vec![],
            manager: man,
            receiver: r,
            found: 0,
            searching: false,
            errors: vec![],
            showing_errors: false,
            searched_count: 0,
            interim_count: 0,
            gui_options,
            size_compare: ops.size.operator.as_str().to_string(),
            size_value: "".to_string(),
            show_clipboard_copied: false,
            expanded: HashSet::new(),
        };
        (d, focus_next())
    }

    fn save_gui_options(&self) {
        self.gui_options.save();
    }

    fn view(&self) -> Element<'_, Message> {
        let name = TextInput::new("Regex file name search e.g. ^mai\\.*rs$ or b.st or ^best", &self.name)
            .padding(4)
            .on_input(Message::NameChanged)
            .on_submit(Message::FindPressed);
        let contents = TextInput::new("Regex content search e.g. str.{2}g", &self.contents)
            .on_input(Message::ContentsChanged)
            .padding(4)
            .on_submit(Message::FindPressed);
        let copied_label = if self.show_clipboard_copied { "Copied to clipboard" } else { "" };
        let clipboard = if self.display_results.is_empty() {
            Container::new(Text::new(""))
        } else {
            Container::new(
                Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(Button::new(Text::new("Clipboard")).on_press(Message::CopyAllToClipboard))
                    .push(Text::new(copied_label).color(Color::from_rgb8(150, 150, 150))),
            )
        };
        let dir = TextInput::new("", &self.directory)
            .on_input(Message::DirectoryChanged)
            .padding(Padding::default().horizontal(8).vertical(4));

        let res = Column::with_children(
            self.display_results
                .iter()
                .map(|x| -> Element<'_, Message> {
                    let maxlen = 200;

                    let mut rts: Vec<Span> = vec![];
                    let mut start = 0;
                    //directory
                    rts.push(span(&x.path[0..x.path.len() - x.name.len()]));
                    for r in &x.ranges {
                        if start < r.start {
                            rts.push(span(&x.name[start..r.start]).font(BOLD));
                        }
                        rts.push(span(&x.name[r.start..r.end]).color(Color::from_rgb8(200, 100, 100)).font(BOLD));
                        start = r.end;
                    }
                    if start < x.name.len() {
                        rts.push(span(&x.name[start..]).font(BOLD));
                    }
                    // add plugin label span if present
                    if let Some(plug) = &x.plugin {
                        let plugin_label = format!(" ({})", plug.name());
                        rts.push(span(plugin_label).color(Color::from_rgb8(18, 110, 171)));
                    }
                    let rt = rich_text(rts);

                    let is_expanded = self.expanded.contains(&x.path);
                    let has_matches = !x.matches.is_empty();
                    let show_arrow = !self.contents.is_empty() && has_matches;
                    let arrow: Element<'_, Message> = if x.path.starts_with("...") || !show_arrow {
                        container(text!(" ")).width(Length::Fixed(12.)).align_x(Alignment::Center).into()
                    } else {
                        let arrow_text = if is_expanded { "▼" } else { "▶" };
                        mouse_area(container(text!("{}", arrow_text)).width(Length::Fixed(12.)).align_x(Alignment::Center))
                            .on_press(Message::ToggleExpand(x.path.clone()))
                            .interaction(mouse::Interaction::Pointer)
                            .into()
                    };

                    let icon = if x.path.starts_with("...") {
                        text!("")
                    } else if x.is_folder {
                        text!("📁")
                    } else {
                        text!("📝")
                    };
                    let icon = container(icon);
                    let show_size = self.size_compare != "None" && !self.size_value.is_empty();
                    let size_label = if show_size {
                        Container::new(text!("{}", format_size(x.file_size)).color(Color::from_rgb8(150, 150, 150)))
                            .width(Length::Fixed(60.))
                            .align_x(Alignment::End)
                    } else {
                        Container::new(text!(""))
                    };
                    let row = Row::new().spacing(5).push(icon).push(size_label).push(rt);
                    let row = mouse_area(row)
                        .on_press(Message::CopySingleToClipboard(x.path.clone()))
                        .interaction(mouse::Interaction::Pointer);
                    let row = Row::new().spacing(4).push(arrow).push(row);

                    let mut col = Column::new().push(row);

                    //content matches (only when expanded)
                    if is_expanded {
                        for cline in x.matches.iter().take(MAX_DETAIL_LINES) {
                            let line_font = BOLD;
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
                                let match_font = BOLD;
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
                        if x.matches.len() > MAX_DETAIL_LINES {
                            col = col.push(
                                Text::new(format!("... and {} more", x.matches.len() - MAX_DETAIL_LINES)).color(Color::from_rgb8(200, 200, 200)),
                            );
                        }
                    }
                    Row::new().spacing(10).push(col).into()
                })
                .collect::<Vec<_>>(),
        );

        let res = scrollable(res).width(Length::Fill);

        let ops = self.manager.get_options();
        let sets = if self.gui_options.show_settings {
            Some(
                Column::new()
                    .push(Text::new("Name settings").font(BOLD))
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
                    .push(Text::new("Content settings").font(BOLD))
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
                    .push(Text::new("Size").font(BOLD))
                    .push({
                        let operators = vec!["None".to_string(), ">=".to_string(), "<".to_string(), "=".to_string()];
                        Row::new()
                            .align_y(Alignment::Center)
                            .push(Text::new("Operator").width(Length::Fixed(100.)))
                            .push(pick_list(operators, Some(self.size_compare.clone()), |s| {
                                Message::Settings(SettingsMessage::SizeOperator(s))
                            }))
                    })
                    .push(Space::new().height(Length::Fixed(2.)))
                    .push({
                        let is_active = self.size_compare != "None";
                        let input = TextInput::new(if is_active { "e.g. 500M, 20k, 5G, 1234" } else { "" }, &self.size_value)
                            .padding(4)
                            .width(Length::Fixed(150.));
                        let input = if is_active {
                            input.on_input(|s| Message::Settings(SettingsMessage::SizeValue(s)))
                        } else {
                            input
                        };
                        Row::new()
                            .align_y(Alignment::Center)
                            .push(Text::new("Value").width(Length::Fixed(100.)))
                            .push(input)
                    })
                    .push(Space::new().height(Length::Fixed(10.)))
                    .push(Text::new("Appearance").font(BOLD))
                    .push({
                        let selected = self.gui_options.theme();
                        Row::new()
                            .align_y(Alignment::Center)
                            .push(Text::new("Theme").width(Length::Fixed(100.)))
                            .push(pick_list(Theme::ALL.to_vec(), Some(selected), Message::ThemeSelected))
                            .push(Space::new().width(Length::Fixed(10.)))
                            .push(Button::new(Text::new("⟳")).on_press(Message::CycleTheme))
                    })
                    .push(Space::new().height(Length::Fixed(10.)))
                    .push(Text::new("Results").font(BOLD))
                    .push(
                        Row::new()
                            .align_y(Alignment::Center)
                            .push(Text::new("Display limit").width(Length::Fixed(100.)))
                            .push(
                                TextInput::new("", &self.gui_options.display_limit.to_string())
                                    .on_input(|s| Message::Settings(SettingsMessage::DisplayLimit(s)))
                                    .padding(4)
                                    .width(Length::Fixed(80.)),
                            ),
                    )
                    .push(Space::new().height(Length::Fixed(4.)))
                    .push(
                        Row::new()
                            .align_y(Alignment::Center)
                            .push(Text::new("Sort").width(Length::Fixed(100.)))
                            .push({
                                let sort = ops.sort;
                                Row::new()
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
                            }),
                    ),
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
                    .push(Space::new().width(Length::Fixed(10.)))
                    .push(Button::new(Text::new("📂")).height(28).on_press(Message::OpenDirectory))
                    .push(dir),
            )
            .push(
                Row::new()
                    .spacing(20)
                    .align_y(Alignment::Center)
                    .push(Button::new(Text::new("Settings")).on_press(Message::ToggleSettings))
                    .push(text("File Types:").font(BOLD))
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
                    ),
            )
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
                let error_btn = if error_count > 0 {
                    let c: Element<'_, Message> = Container::new(Button::new(Text::new(label.to_string())).on_press(Message::ToggleErrors))
                        .style(container::rounded_box)
                        .into();
                    c
                } else {
                    let c: Element<'_, Message> = Container::new(Text::new("")).into();
                    c
                };
                let expand_btn = if self.contents.is_empty() || self.display_results.is_empty() {
                    let c: Element<'_, Message> = Container::new(Text::new("")).into();
                    c
                } else {
                    let any_expanded = self.display_results.iter().any(|x| self.expanded.contains(&x.path));
                    let expand_label = if any_expanded { "Collapse all" } else { "Expand all" };
                    let c: Element<'_, Message> = Container::new(Button::new(Text::new(expand_label)).on_press(Message::ToggleExpandAll)).into();
                    c
                };
                let showing = if self.display_results.len() > 0 {
                    let mut display_text = format!("Showing {}", self.display_results.len().formato("N0"));
                    let others = self.full_results.len() - self.display_results.len();
                    if others > 0 {
                        display_text.push_str(&format!(". There are {} more", others.formato("N0")));
                    }

                    text(display_text)
                } else {
                    text("")
                };

                let c: Element<'_, Message> = Row::new()
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .push(error_btn)
                    .push(expand_btn)
                    .push(showing)
                    .into();
                c
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
                    self.display_results.clear();
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
                            //we take while path+num lines <display_limit
                            let mut display_lines = 0;
                            self.display_results.clear();
                            for rd in &res.data {
                                self.display_results.push(rd.clone());
                                display_lines += 1 + rd.matches.len().min(MAX_DETAIL_LINES);
                                if display_lines >= self.gui_options.display_limit {
                                    break;
                                }
                            }
                            self.full_results = res.data;
                            if !self.contents.is_empty() {
                                self.expanded = self.display_results.iter().map(|x| x.path.clone()).collect();
                            } else {
                                self.expanded.clear();
                            }

                            let filecount = self.full_results.iter().filter(|x| !x.is_folder).count();
                            let foldercount = self.full_results.len() - filecount;
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
                            let line_count = self.display_results.iter().map(|x| x.matches.len()).sum::<usize>();
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
                                if self.interim_count < self.gui_options.display_limit {
                                    self.display_results.push(res.clone());
                                    if !res.matches.is_empty() && !self.contents.is_empty() {
                                        self.expanded.insert(res.path.clone());
                                    }
                                }
                                let this_count = 1 + res.matches.len().min(MAX_DETAIL_LINES); //this is file+number of details
                                self.full_results.push(res);
                                self.interim_count += this_count;
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
                let text = self.full_results.iter().map(|x| x.path.clone()).collect::<Vec<_>>().join("\n");
                self.show_clipboard_copied = true;
                return Task::batch(vec![
                    iced::clipboard::write(text),
                    Task::perform(
                        async {
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                        },
                        |_| Message::ClearClipboardMessage,
                    ),
                ]);
            }
            Message::CopySingleToClipboard(path) => {
                self.show_clipboard_copied = true;
                return Task::batch(vec![
                    iced::clipboard::write(path),
                    Task::perform(
                        async {
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                        },
                        |_| Message::ClearClipboardMessage,
                    ),
                ]);
            }
            Message::ClearClipboardMessage => self.show_clipboard_copied = false,

            Message::ToggleExpand(path) => {
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
            }
            Message::ToggleExpandAll => {
                let others_label = self.display_results.last().map_or(false, |a| a.path.starts_with("...and")) as usize;
                let total = self.display_results.len() - others_label;
                let count_expanded = self.expanded.len();
                if count_expanded < total {
                    self.expanded = self.display_results.iter().map(|x| x.path.clone()).collect();
                } else {
                    self.expanded.clear();
                }
            }
            Message::ToggleErrors => {
                self.showing_errors = !self.showing_errors;
            }
            Message::ToggleSettings => {
                self.gui_options.show_settings = !self.gui_options.show_settings;
                self.save_gui_options();
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
                    SettingsMessage::DisplayLimit(val) => {
                        if let Ok(limit) = val.parse::<usize>()
                            && limit > 0
                        {
                            self.gui_options.display_limit = limit;
                            self.save_gui_options();
                        }
                    }
                    SettingsMessage::SizeOperator(val) => {
                        self.size_compare = val.clone();
                        ops.size.operator = val.into();
                        if ops.size.operator == librusl::options::SizeCompare::None {
                            self.size_value = String::new();
                            ops.size.bytes = 0;
                        }
                    }
                    SettingsMessage::SizeValue(val) => {
                        self.size_value = val.clone();
                        if let Ok(bytes) = parse_size(&val) {
                            ops.size.bytes = bytes;
                        }
                    }
                }
                self.manager.set_options(ops);
                self.manager.save();
            }
            Message::CycleTheme => {
                let idx = Theme::ALL.iter().position(|t| *t == self.gui_options.theme()).unwrap_or(0);
                self.gui_options.set_theme(&Theme::ALL[(idx + 1) % Theme::ALL.len()]);
                self.save_gui_options();
            }
            Message::ThemeSelected(theme) => {
                self.gui_options.set_theme(&theme);
                self.save_gui_options();
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

fn parse_size(input: &str) -> Result<u64, ()> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(0);
    }
    let (num_str, suffix) = if let Some(pos) = input.find(|c: char| c.is_alphabetic()) {
        (&input[..pos], &input[pos..])
    } else {
        (input, "")
    };
    let num: f64 = num_str.parse().map_err(|_| ())?;
    let multiplier: u64 = match suffix.to_lowercase().as_str() {
        "k" => 1_024,
        "m" => 1_024 * 1_024,
        "g" => 1_024 * 1_024 * 1_024,
        "" => 1,
        _ => return Err(()),
    };
    Ok((num * multiplier as f64) as u64)
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return String::new();
    }
    const KB: f64 = 1_024.0;
    const MB: f64 = KB * 1_024.0;
    const GB: f64 = MB * 1_024.0;
    let b = bytes as f64;

    if b >= GB {
        format!("{:.1}G", b / GB)
    } else if b >= MB {
        format!("{:.1}M", b / MB)
    } else if b >= KB {
        format!("{:.1}K", b / KB)
    } else {
        format!("{}", bytes)
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

use std::{
    collections::HashSet,
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

use formato::Formato;
use iced::{
    Color, Element, Font, Length, Padding, Subscription, Task, Theme,
    alignment::Alignment,
    event,
    font::{Family, Stretch, Style, Weight::Bold},
    keyboard::{Event, Key, key::Named},
    mouse,
    theme::Base,
    widget::{
        Button, Column, Container, Row, Space, Text, TextInput, container, mouse_area,
        operation::{focus_next, focus_previous},
        pick_list, radio, rich_text, scrollable, span, text,
        text::Span,
    },
    window::{self, icon},
};
