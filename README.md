# tts-script-tool

**ttsst** is an experimental command-line interface (CLI) tool designed for managing scripts in [Tabletop Simulator](https://www.tabletopsimulator.com/).
It offers an alternative approach to working with Lua scripts and XML UI within the game.

## Usage

Run `ttsst.exe help` to display a list of available commands:

```txt
Attach and update scripts in Tabletop Simulator via the command line.

Usage: ttsst.exe [OPTIONS] <COMMAND>

Commands:
  attach   Attach a Lua script or XML UI to object(s)
  detach   Detach Lua script and XML UI from object(s)
  reload   Reload script path(s)
  console  Mirror Tabletop Simulator messages to the console
  watch    Watch script path(s) and reload on change
  backup   Create a backup of the current save as a JSON file
  help     Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Verbosity level (use up to 2 times for more detailed output)
  -h, --help        Print help
  -V, --version     Print version
```

When using **ttsst**, keep these key concepts in mind:

### Attaching

To attach a Lua or XML file to an in-game object, use the command: `ttsst attach <File> <GUID(s)>`.
If no GUIDs are provided, a selection prompt will appear, listing all objects in the save file.
By default, hidden objects like Zones are excluded, but you can include them using the `--all` or `-a` flag.

For example, running `ttsst attach ./Foo.lua 4f6ab0` will attach the `Foo.lua` file to an object with the GUID `4f6ab0`.
In-game, this object will have the `lua/Foo.lua` tag. Objects can have only one Lua and one XML tag, respectively.

### Reloading

If you make changes to an attached file and want them to update in-game, execute: `ttsst reload <Path(s)>`.
If `<Path>` is a directory, all files within it will be reloaded. If `<Path>` is a file, only that file will be reloaded.
By default, **ttsst** uses the current working directory as the path.

### Detaching

To remove all Lua and XML tags and scripts from one or more objects, use the command: `ttsst detach <GUID(s)>.`

### Console & Watching

To mirror messages from Tabletop Simulator to the console, use the command: `ttsst console`.
If you want to hot-reload files upon changes, you can watch them using `ttsst watch <Path(s)>`.
