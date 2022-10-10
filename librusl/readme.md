rusl Manager and tools

Library with the Manager and helper files which acts as a service layer to the GUI's

`Manager` - Orchestrate search
`Search` - Search query
`FileInfo` - Stores results of searches
`Options` - Options for search
`rgtools` - Ripgrep helper to assist with search 

### Manager
Spawns search in background thread. Takes a channel that it can send results on.