rusl

Rust GUI interface for ripgrep / file searcher / content searcher

It aims to be a slim frontend for file and content search with the heavy lifting done by ripgrep and other libraries

This project started out to evaluate the maturity of some GUI frameworks in rust. See further down for comments on the GUIs.
Saved each GUI in its own branch. Currently master is Druid


## Objectives
- [x] File name search
- [x]  File content search (currently only first match is shown)
- [x] Combination of the above 2
- [x] Highlight match in both name and content
- [x] GUI
- [x] Export results to clipboard
- [ ] Cross platform tested
    - [x] Windows 10
    - [x] Arch Linux
    - [ ] OSX
- [ ] Click on individual result to copy to clipboard
- [ ] Option to show all content matches in a file
- [ ] Alternate GUI libraries 
    - [x] Druid
    - [x] Slint
    - [x] EGUI
    - [ ] Dioxus

This project relies heavily on ripgrep and BurntSushi's libraries. 
> Shout out to BurntSushi for the awesome work (https://github.com/BurntSushi)  

## Contributions
Contributions are welcome. You are also welcome to add a new GUI frontend in a new branch. 

## GUI Libraries
As stated, this was a small project to test out some of the existing libraries.

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
- Limited widgets, unsure how to create new ones
- No auto formatting of slint files
- Basic functionality missing e.g.:
    - Changing font colours for text in some widgets
    - Changing background colours
- Alignment not always working
- Linux behind in development

### Dioxus 
(https://dioxuslabs.com/)

I heard a lot of good things about dioxus, so I tried setting that up. However having never used react or something similar, I struggled a bit. I wasn't quite sure of the difference in use_state, use_future, use_hook, and when one would use each case.
To be fair, I did not spend a lot of time on this, so I left it for someone to contribute to the dioxus branch so we can get something going there. 


### Druid
(https://github.com/linebender/druid)
Next up was Druid. I have used it in a personal project previously, and feel like it has a lot of potential. First off, it *is* more complicated than Slint, and there are a few complications like Data, Lens, Delegates etc., but the hurdles are surmountable. Also there is a lot of boilerplate to set up a basic widget.

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
- Flex is finnicky, sometimes cannot get it to do exactly what I want


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

---

