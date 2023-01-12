# tts-script-tool

Attach and update scripts in Tabletop Simulator via the command line.

This tool exists as an alternative method of using scripts within Tabletop Simulator.
It attaches scripts to objects using ingame tags. This way multiple objects can share the same script.
This allows for a clean projects structure where the amount of scripts you have is independent from the objects ingame.
It also allows for easy integeration of tools like [TypescriptToLua](https://typescripttolua.github.io/).

## Usage

Run `ttsst.exe help` for a list of commands:

```
Attach and update scripts in Tabletop Simulator via the command line.

Usage: ttsst.exe <COMMAND>

Commands:
  attach  Attach script to object
  reload  Update scripts and reload save
  backup  Backup current save
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

## Attaching and Reloading

By running `ttsst.exe attach <Path> <Guid>` you can attach that script to an object with that Guid.
When attaching a script, the object will get a new tag added that points to the script in the Format of
`scripts/<File>.ttslua`. If you edit a file, you can reload the scripts for every object by running
`ttsst.exe reload <Path>`, with `<Path>` being the directory to your scripts. Objects will pull the newest
version of their attached script recursively depending on their tag. Ingame objects can only have one valid tag.
