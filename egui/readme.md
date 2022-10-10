rusl

### EGUI

`AppState` holds the required fields for the GUI.
Using the `librusl` result channel, we spawn a background thread, that when receives results, asks the GUI to repaint.