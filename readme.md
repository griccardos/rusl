## rusl

### Rust GUI interface for ripgrep / file searcher / content searcher

rusl aims to be a slim frontend for file and content search with the heavy lifting done by ripgrep and other libraries. This project started out to evaluate the maturity of some GUI frameworks in rust. See further down for comments on the GUIs.

Each GUI Can be found in its own folder. To compile, cd into the folder, and cargo build.

Druid is currently the most feature complete and releases are Druid.

Why the name rusl? Well, it's the sound made when you are searching through leaves or papers for something. Also it's 75% of the letters in rust!

![image](https://user-images.githubusercontent.com/30464685/197233181-db1760dd-429f-48dc-b73a-6aea8f1c3743.png)


## Objectives
- [X] File name search
- [X] File content search with line numbers
- [X] Combination of the above 2
- [X] Highlight match in both name and content
- [X] Export results to clipboard
- [X] Cross platform tested
    - [X] Windows 10
    - [X] Arch Linux
    - [X] OSX
- [X] Click on individual result to copy to clipboard
- [ ] Autocomplete or dropdown history
- [ ] Alternate GUI libraries 
    - [X] Druid
    - [X] Slint
    - [X] EGUI
    - [X] Dioxus
    - [ ] fltk
    - [ ] Relm4
    - [X] Iced

This project relies heavily on ripgrep and BurntSushi's libraries. 
> Shout out to BurntSushi for the awesome work (https://github.com/BurntSushi)  

## How to build
- `git clone https://github.com/griccardos/rusl`
- `cd rusl`
- `cd druid` (or into other guis)
- `cargo build --release`
- `cd target/release`
- `./rusl` 
- Or copy to bin directory


Linux/Druid requires gtk+3; see [GTK installation page](https://www.gtk.org/docs/installations/linux/) . (On ubuntu-based distro, run sudo apt-get install libgtk-3-dev )

## Contributions
Contributions are welcome. You are also welcome to add a new GUI frontend in a new branch. 

## GUI Libraries
Each library is on a different branch. Change branch to select different UI.

Summary
----------
|                       |Druid   |Slint|EGUI  |ICED   |Dioxus|
|-                      |---:    |----:|---:  |----:  |--:   |
|Dependencies           |**171** |439  |243   |312    |337   |
|Build time cold (s)    |18      |40   |**16**|42     | 56   |
|Lines                  |403     |479  | 229  |**198**| 236  |
|File size windows (kb) |**4739**|6926 |7071  |10276  |7026  |

UI 
--------
|                       |Druid  |Slint  |EGUI   |ICED   |Dioxus |
|-                      |:-----:|:-----:|:-----:|:-----:|:-----:|
| - Toolbar icon        |&check;|&check;|&check;|&check;|&check;|
| - Window icon         |&cross;|&check;|&check;|&check;|&check;| 
| - Tab between fields  |&check;|&check;|&check;|&cross;|&check;| 
| - paste into textbox  |&check;|&check;|&check;|&check;|&check;| 

Functionality Implemented 
--------
|                       |Druid  |Slint  |EGUI     |ICED   |Dioxus   |
|-                      |:-----:|:---:  |:--:     |:---:  |:---:  |
| - Filename search     |&check;|&check;|&check;  |&check;|&check;|
| - Content search      |&check;|&check;|&check;  |&check;|&check;| 
| - Match Highlighting  |&check;|&cross;|&cross;  |&cross;|&cross;| 
| - Settings            |&check;|&check;|&check;  |&cross;|&cross;| 
| - Copy to clipboard   |&check;|&check;|&check;  |&cross;|&cross;| 
| - Windows             |&check;|&check;|&check;  |&check;|&check;| 
| - Linux               |&check;|&check;|&check;  |&check;|| 
| - Mac                 |&check;*1|&check;|&check;|&check;|| 

- *1 Currently gui update is slow on MacOS, but has been fixed in druid; awaiting new version

### Slint 
This project started off using Slint (https://slint-ui.com/). It was really easy to get up and running and within 2 days had something relatively complete (with a few concessions made). 
There were so many good things about the way it worked, and the easy of setting it up. Easy concepts such as events, callbacks, and communication between GUI and backend, made it very simple and straightforward to use.

Unfortunately when I tried compiling for Linux, I realised that some of the functionality did not work on Linux. 
Will keep an eye on this one.

#### Pros
- Easy to setup
- Easy concepts
- Separation of ui file from rust
- Easy communication between backend and frontend
- Some programmability capabilities in slint format saves a lot of backend work  
- Gui preview of slint files - Amazing!

#### Cons
- Limited widget configurability, (can create custom ones: https://docs.rs/slint/latest/slint/docs/recipes/index.html#custom-widgets)
    - Changing font colours for text in some widgets
    - Changing background colours
- No auto formatting of slint files (this is being worked on)
- Alignment not always working
- Linux behind in development

### Dioxus 
(https://dioxuslabs.com/)

Having never used react or something similar, I struggled a bit with hooks. However after a bit of time, and as the documentation became better, was able to get a solution
working. Needed to use async and `use_coroutine` to run in the background and check for results. Other than that it was easy to set up user interface if you are familiar with front end web development.

#### Pros
- few lines of code
- availability of thousands different web frameworks that can be used

#### Cons
- resizing of window has flickering due to use of webview
- hooks are perhaps more difficult to understand if one has never used React
- limited formatting in `rsx!` - would like to see formatter here

### Druid
(https://github.com/linebender/druid)
Next up was Druid. I have used it in a personal project previously, and feel like it has a lot of potential. First off, it is a little more complicated than Slint, and there are additional things to learn like Data, Lens etc. There is some boilerplate to set up a custom widget.

However it feels like one can do almost everything they need, and if there is no existing widget, it gives us the tools to build our own widgets.
I really like the feel of this.
My favourite in terms of professional look and feel.

#### Pros
- Easy to get up and running especially for simple apps
- Everything is configurable, and feels like a complete solution. E.g. has events, commands, delegates, widget extensions, rich text, etc.
- Fast startup and interaction

#### Cons
- Can be a bit verbose at times e.g. implementing a new widget
- Cannot change window icon, need to use another crate
- Cannot change titlebar icon
- Flex is finnicky, sometimes cannot always get it to do exactly what I want
- Errors are sometimes confusing


### EGUI
(https://github.com/emilk/egui)
This one is a real gem, so easy to use and setup. It is an immediate mode GUI, which means it gets repainted many times per second. It only took a few hours and had most of the functionality.
A lot less fighting with the borrow checker here, and still had mut references to self for all the modification and lookup to the app state that was required.
My favourite in terms of getting something up and running fast with almost all functionality.

#### Pros
- Fast to develop something
- Allows complex tasks easily e.g. interaction with app state
- Lots of widgets available including Grid
- Easy automatic interaction of switches, boxes, radio buttons with app state
- Title bar icon
- Dark/light mode
#### Cons
- Less native looking than others
- Repaints everything each time (may or may not be an issue, was not an issue here)


### ICED
(https://github.com/iced-rs/iced)
Nice library using Elm framework. Easy to get up and running. Uses winit for window, so one can set the icon for window itself in Windows

#### Pros
- Fast to develop something
- Easy to understand
- Title bar icon
- Dark/light mode
- Nice hover over effects
#### Cons
- Less native looking than others
- Cannot send message to update GUI easily. Currently using Subscriber to poll changes which is less than optimal
