# CLI Module

Handles all command-line interface parsing, validation, and help display for the ezkvm tool.

## Structure

- `mod.rs` - Module exports and public API
- `options.rs` - CLI option structures and types
- `parser.rs` - Command-line argument parsing logic
- `help.rs` - Help message and usage display

## Syntax

Command syntax looks like this:
```
ezkvm <action>[ --<option>[:<args>]]*
```
Where:
- `<action>` can be:
  - `import`, valid options: `--input:type=<type>[,<args>]`
  - `export`, valid options: `--output:type=<type>[,<args>]`
  - `convert`, valid options: `--input:type=<type>[,<args>]`, `--output:type=<type>[,<args>]`
  - `show-runtime <name>`
  - `start <name>`
  - `stop <name>`
  - `reset <name>`
  - `shutdown <name>`
- `<name>` is the name of the referenced virtual machine
- `<args>` is a comma separated list of `<key>=<value>` arguments
- `<key>` is a unique name identifying the argument
- `<value>` is the value of the argument

And the following options are defined:
- `--input:type=<type>[,<args>]` describes the source of an import action.
  - `<type>`: is one of:
    - `ezkvm`: this is the native ezkvm format, it supports the following argument keys:
      - `config`: points to the ezkvm vm config file
      - `profiles`: points to the directory containing the profiles that belongs with this config file
    - `proxmox`: this is the proxmox config format, it supports the following argument keys:
      - `config`: points to the proxmox vm config file
      - `storage`: points to the file that describes the storage configuration that belongs with this config file

- `--output:type=<type>[,<args>]` describes the destination of an export action.
  - `<type>`: is one of:
    - `ezkvm`: this is the native ezkvm format, it supports the following argument keys:
      - `destination`: path to the destination folder where the exported files are written
    - `proxmox`: this is the native ezkvm format, it supports the following argument keys:
      - `destination`: path to the destination folder where the exported files are written
