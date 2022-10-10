rusl

Rust GUI interface for ripgrep / file searcher / content searcher

It aims to be a slim frontend for file and content search with the heavy lifting done by ripgrep and other libraries

This project started out to evaluate the maturity of some GUI frameworks in rust. See further down for comments on the GUIs.
Each GUI has its own branch. Currently main branch is Druid.

Why the name rusl? Well, its the sound made when you are searching through leaves or papers for something. Also its 75% of the letters in rust!

## Druid
(https://github.com/linebender/druid)

`AppState` holds GUI state, including a list of visible results

`Delegate` sends and receives commands from the GUI. It also holds instance of `Manager`. Initiating a search from the GUI, will be picked up by the delegate, and initiate a search. 

We also create a `sink` which is an external handle to our gui, which allows us to send commands to the GUI. 
We move this to another thread, and await messages from our `Manager`. 
For each search result message, we send a command, and `Delegate` will update the results in the GUI.