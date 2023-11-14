rusl

### Iced

`App` holds required fields for GUI, including widget `States`

We create a Message channel between for `librusl` to search, and return results.

Currently, we cannot change gui immediately when results are ready, so we use a `Subscriber` to poll the results every few ms until it sees a change, which then updates the GUI. 

TODO:
- [ ] remove poll, and update once change. (We need to create a subscriber)
- [ ] add syntax match highlighting