rusl
Rust GUI interface for ripgrep / file searcher / content searcher

## Druid
(https://github.com/linebender/druid)

`AppState` holds GUI state, including a list of visible results

`Delegate` sends and receives commands from the GUI. It also holds instance of `Manager`. Initiating a search from the GUI, will be picked up by the delegate, and initiate a search. 

We also create a `sink` which is an external handle to our gui, which allows us to send commands to the GUI. 
We move this to another thread, and await messages from our `Manager`. 
For each search result message, we send a command, and `Delegate` will update the results in the GUI.