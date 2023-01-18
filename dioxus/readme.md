# rusl

## Dioxus 
(https://dioxuslabs.com/)

- Create a `Manager` for search, and keep reference to it with `use_ref`
- `onclick` asks `Manager` to do actual search based on input boxes
- `use_coroutine` runs async in background and tries to check if we received any messages, then updates messages, and UI if we did. if not we sleep a bit and check again. I'm sure there is a better way to await messages rather than keep polling...

## Todo
- [ ] open directory selector
- [ ] highlight content matches 
- [ ] copy to clipboard