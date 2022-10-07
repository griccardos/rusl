use std::{sync::mpsc, thread::spawn, time::Instant};

use druid::{
    im::Vector,
    piet::TextStorage,
    text::{Attribute, RichText},
    widget::{Button, Checkbox, Controller, Either, Flex, Label, List, RawLabel, Scroll, TextBox},
    AppDelegate, AppLauncher, Code, Color, Command, Data, Env, Event, EventCtx, FontFamily, FontWeight, Handled, Lens, Selector, Target, Widget,
    WidgetExt, WindowDesc,
};
use regex::{Regex, RegexBuilder};

use crate::{
    fileinfo::FileInfo,
    manager::{Manager, SearchResult},
    search::Search,
};

pub const SEARCH: Selector = Selector::new("search");
pub const STOP: Selector = Selector::new("stop");
pub const RESULTS: Selector<SearchResult> = Selector::new("results");
pub const UPDATEMESSAGE: Selector<String> = Selector::new("message");
pub const EXPORT: Selector = Selector::new("export");
pub const EXPORTSINGLE: Selector<String> = Selector::new("exportsingle");

#[derive(Data, Clone, Lens)]
struct AppState {
    text_name: String,
    text_contents: String,
    dir: String,
    message: RichText,
    visible: Vector<RichText>,
    find_name: String,
    searching: bool,
    data: Vector<String>,
    start: Instant,
    show_settings: bool,
    //settings
    name_case_sensitive: bool,
    name_same_filesystem: bool,
    name_follow_links: bool,
    name_ignore_dot: bool,
    content_case_sensitive: bool,
}

pub fn run_druid() {
    let (s, r) = mpsc::channel::<SearchResult>();
    let man = Manager::new(s);
    let ops = &man.options;

    let data = AppState {
        text_name: "".to_string(),
        text_contents: "".to_string(),
        dir: ops.last_dir.to_string(),
        message: RichText::new("Ready to search".into()),
        data: Vector::new(),
        visible: Vector::new(),
        start: Instant::now(),
        show_settings: false,
        searching: false,
        find_name: String::from("Find"),
        //settings
        name_case_sensitive: ops.name.case_sensitive,
        name_same_filesystem: ops.name.same_filesystem,
        name_follow_links: ops.name.follow_links,
        name_ignore_dot: ops.name.ignore_dot,
        content_case_sensitive: ops.content.case_sensitive,
    };
    let delegate = Delegate { manager: man };

    let main_window = WindowDesc::new(ui_builder()).title("Rusl").window_size((800.0, 800.0)).resizable(true);
    let app = AppLauncher::with_window(main_window).delegate(delegate).log_to_console();
    let sink = app.get_external_handle();

    spawn(move || loop {
        let mess = r.recv();
        if mess.is_err() {
            break;
        }
        let mess = mess.unwrap();

        sink.submit_command(RESULTS, mess, Target::Auto).expect("Sent results to sink");
    });

    app.launch(data).expect("Run druid window");
}

fn ui_builder() -> impl Widget<AppState> {
    let tname = TextBox::new()
        .with_placeholder("Regex file name search e.g. ^mai.*rs$ or r.st or ^best")
        .controller(TextBoxController {})
        .lens(AppState::text_name)
        .expand_width();
    let tcontents = TextBox::new()
        .with_placeholder("Regex content search e.g. str.{2}g")
        .controller(TextBoxController {})
        .lens(AppState::text_contents)
        .expand_width();
    let tdir = TextBox::new()
        .controller(TextBoxController {})
        .fix_width(300.)
        .lens(AppState::dir)
        .expand_width();
    let butfind = Button::dynamic(|state: &AppState, _env| state.find_name.clone())
        .on_click(|ctx, data, _env| {
            if data.find_name == "Find" {
                ctx.submit_command(SEARCH);
            } else {
                ctx.submit_command(STOP);
            }
        })
        .fix_size(80., 40.);
    let butset = Button::new("⚙️")
        .on_click(|_ctx: &mut EventCtx, data: &mut AppState, _env: &Env| data.show_settings = !data.show_settings)
        .fix_size(40., 40.);
    let butclip = Button::new("Clipboard")
        .on_click(|ctx, _data, _env| ctx.submit_command(EXPORT))
        .fix_size(85., 40.);
    let lmessage = RawLabel::new().lens(AppState::message).padding(5.0).center().expand_width();
    let butfolder = Button::new("📁")
        .on_click(|_ctx, data: &mut AppState, _env| {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                data.dir = folder.to_string_lossy().to_string();
            }
        })
        .fix_size(40., 40.);

    let list = Scroll::new(
        List::new(|| {
            RawLabel::new().padding(1.0).on_click(|ctx, data: &mut RichText, _env| {
                let mut filename = data.as_str().split_once(' ').unwrap().1;
                if filename.contains('>') {
                    //if we are showing content too...
                    filename = filename.split_once('>').unwrap().0;
                }

                ctx.submit_command(Command::new(EXPORTSINGLE, filename.to_string(), Target::Auto))
            })
        })
        .lens(AppState::visible)
        .padding(10.),
    )
    .background(Color::rgba8(50, 50, 50, 255))
    .expand();

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(Label::new("File name").padding(5.0).fix_width(100.))
                .with_flex_child(tname, 1.0),
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("Contents").padding(5.0).fix_width(100.))
                .with_flex_child(tcontents, 1.0),
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("Directory").padding(5.0).fix_width(100.))
                .with_child(butfolder)
                .with_flex_child(tdir, 1.0),
        )
        .with_child(
            Flex::row()
                .with_flex_spacer(1.)
                .with_child(butfind)
                .with_flex_spacer(1.)
                .with_child(butset)
                .with_child(butclip)
                .with_spacer(10.),
        )
        .with_child(settings_panel())
        .with_child(lmessage)
        .with_flex_child(list, 1.0)
}
fn settings_panel() -> impl Widget<AppState> {
    Either::new(
        |data: &AppState, _env| data.show_settings,
        Flex::column()
            .with_child(Label::new("Name Settings").align_left().padding(10.))
            .with_child(Checkbox::new("Case sensitive").lens(AppState::name_case_sensitive).align_left())
            .with_child(Checkbox::new("Same filesystem").lens(AppState::name_same_filesystem).align_left())
            .with_child(Checkbox::new("Ignore hidden (dot)").lens(AppState::name_ignore_dot).align_left())
            .with_child(Checkbox::new("Follow links").lens(AppState::name_follow_links).align_left())
            .with_child(Label::new("Content Settings").align_left().padding(10.))
            .with_child(Checkbox::new("Case sensitive").lens(AppState::content_case_sensitive).align_left())
            .padding(10.),
        Flex::column(),
    )
    .background(Color::rgb8(30, 30, 30))
    .padding(10.)
}

//on enter
pub struct TextBoxController;
impl Controller<String, TextBox<String>> for TextBoxController {
    fn event(&mut self, child: &mut TextBox<String>, ctx: &mut EventCtx, event: &Event, data: &mut String, env: &Env) {
        if let Event::KeyDown(key) = event {
            if key.code == Code::Enter || key.code == Code::NumpadEnter {
                ctx.submit_command(SEARCH);
            }
        }
        child.event(ctx, event, data, env)
    }
}

pub struct Delegate {
    manager: Manager,
}
impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        ctx: &mut druid::DelegateCtx,
        _target: druid::Target,
        cmd: &druid::Command,
        data: &mut AppState,
        _env: &druid::Env,
    ) -> druid::Handled {
        if cmd.is(STOP) {
            self.manager.stop();
            data.message = rich("Cancelled", Color::YELLOW);
            data.find_name = String::from("Find");
        }
        if cmd.is(SEARCH) {
            //early exit if invalid dir
            if !self.manager.dir_is_valid(&data.dir) {
                data.message = rich("Invalid directory", Color::rgb8(200, 100, 100));
                return Handled::Yes;
            }

            //early exit if no input
            if data.text_name.is_empty() && data.text_contents.is_empty() {
                data.message = rich("Nothing to search for", Color::rgb8(200, 100, 100));
                return Handled::Yes;
            }
            data.find_name = String::from("Stop");

            data.visible.clear();
            data.data.clear();

            data.message = rich("Searching...", Color::YELLOW);
            //set options
            let mut ops = self.manager.options.clone();
            ops.name.case_sensitive = data.name_case_sensitive;
            ops.name.follow_links = data.name_follow_links;
            ops.name.same_filesystem = data.name_same_filesystem;
            ops.content.case_sensitive = data.content_case_sensitive;
            ops.name.ignore_dot = data.name_ignore_dot;

            self.manager.set_options(ops);

            data.start = Instant::now();
            self.manager.search(Search {
                name_text: data.text_name.clone(),
                contents_text: data.text_contents.clone(),
                dir: data.dir.clone(),
            });
            return Handled::Yes;
        }

        if let Some(mess) = cmd.get(UPDATEMESSAGE) {
            data.message = RichText::new(mess.clone().into());
            return Handled::Yes;
        }

        if cmd.is(EXPORT) {
            self.manager.export(data.data.iter().map(|x| x.to_string()).collect());
            ctx.submit_command(Command::new(UPDATEMESSAGE, "Copied to clipboard".to_string(), Target::Auto));
            return Handled::Yes;
        }
        if let Some(line) = cmd.get(EXPORTSINGLE) {
            self.manager.export(vec![line.clone()]);
            ctx.submit_command(Command::new(UPDATEMESSAGE, "Copied to clipboard".to_string(), Target::Auto));

            return Handled::Yes;
        }

        if let Some(results) = cmd.get(RESULTS) {
            const MAX_OUT: usize = 1000;

            if let SearchResult::InterimResult(fi) = results {
                if data.visible.len() < MAX_OUT {
                    let sym = if fi.is_folder { "📁" } else { "📝" };
                    let string = format!("{} {}> {}", sym, fi.path.clone(), fi.content.trim());
                    data.visible.push_back(RichText::new(string.into()));
                    //TODO this takes long. fix
                    //data.visible.push_back(highlight_result(fi, re_name.clone(), re_content.clone()));
                }
            } else if let SearchResult::FinalResults(results) = results {
                let re_name = RegexBuilder::new(&data.text_name).case_insensitive(!data.name_case_sensitive).build();
                let re_content = RegexBuilder::new(&data.text_contents)
                    .case_insensitive(!data.content_case_sensitive)
                    .build();

                data.data = results.data.iter().map(|x| x.path.to_string()).collect();

                //create visible
                //limited to fixed number of lines
                //add colour and highlighting
                data.find_name = String::from("Find");
                data.visible = results
                    .data
                    .iter()
                    .take(MAX_OUT)
                    .map(|x| highlight_result(x, re_name.clone(), re_content.clone()))
                    .collect();
                if results.data.len() > MAX_OUT {
                    data.visible
                        .push_back(RichText::new(format!("...AND {} others", results.data.len() - 1000).into()));
                }

                let filecount = results.data.iter().filter(|x| !x.is_folder).count();
                let foldercount = results.data.len() - filecount;
                let mut string = format!("Found {filecount} files");
                if foldercount > 0 {
                    string += &format!(" and {foldercount} folders");
                }
                string += &format!(" in {:.3}s", data.start.elapsed().as_secs_f64());

                data.message = RichText::new(string.into());
            }
            return Handled::Yes;
        }

        druid::Handled::No
    }

    fn window_removed(&mut self, _id: druid::WindowId, _data: &mut AppState, _env: &druid::Env, _ctx: &mut druid::DelegateCtx) {
        //options are set on search, so we save them
        self.manager.save_and_quit();
    }
}

fn highlight_result(x: &FileInfo, re_name: Result<Regex, regex::Error>, re_content: Result<Regex, regex::Error>) -> RichText {
    let sym = if x.is_folder { "📁" } else { "📝" };
    let symlen = sym.as_bytes().len();
    let mut full = sym.to_string();
    full.push(' ');
    full.push_str(&x.path);
    let mut rich = if !x.content.is_empty() {
        full.push_str("> ");
        let start = full.len();
        full.push_str(x.content.trim());
        let mut rich = rich(&full, Color::WHITE);
        rich.add_attribute(start..full.len(), Attribute::text_color(Color::rgb8(50, 255, 55)));
        rich.add_attribute(0..symlen, Attribute::FontFamily(FontFamily::MONOSPACE));
        rich
    } else {
        rich(&full, Color::WHITE)
    };
    //highlight matches in name:
    if let Ok(re) = &re_name {
        if let Some(ranges) = re.captures(&x.name) {
            let start = x.path.len() - x.name.len();
            for cap in ranges.iter().flatten() {
                let mut range = cap.range();
                range.end += start + symlen + 1;
                range.start += start + symlen + 1;

                if range.end <= x.path.len() + symlen + 1 {
                    rich.add_attribute(range.clone(), Attribute::Weight(FontWeight::BOLD));
                    rich.add_attribute(range.clone(), Attribute::text_color(Color::rgb8(80, 122, 80)));
                } else {
                    eprintln!("{range:?} is out of range of {}", x.path.len());
                }
            }
        }
    }
    //highlight matches in content:
    if !x.content.is_empty() {
        if let Ok(re) = &re_content {
            for cap in re.captures_iter(&x.content) {
                if let Some(mat) = cap.get(0) {
                    let mut range = mat.range();
                    if range.end <= x.content.len() {
                        range.end += x.path.len() + 2 + symlen + 1;
                        range.start += x.path.len() + 2 + symlen + 1;
                        rich.add_attribute(range.clone(), Attribute::Weight(FontWeight::BOLD));
                        rich.add_attribute(range.clone(), Attribute::text_color(Color::rgb8(80, 80, 162)));
                    } else {
                        eprintln!("{range:?} is out of range of {}", x.content.len());
                    }
                }
            }
        }
    }
    rich
}

fn rich(str: &str, col: Color) -> RichText {
    RichText::new(str.into()).with_attribute(.., Attribute::text_color(col))
}
