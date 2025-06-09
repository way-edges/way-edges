# Command line usage

```
Hidden widget on the screen edges

Usage: way-edges [OPTIONS] [COMMAND]

Commands:
  schema     print json schema of the configurations to the stdout
  daemon     (deprecated) run daemon. There can only be one daemon at a time
  togglepin  toggle pin of a widget under certain group. format: <group_name>:<widget_name>
  reload     reload widget configuration
  quit       close daemon
  help       Print this message or the help of the given subcommand(s)

Options:
  -d, --mouse-debug                    print the mouse button key to the log when press and release
  -c, --config-path <CONFIG_PATH>
  -i, --ipc-namespace <IPC_NAMESPACE>
  -h, --help                           Print help
  -V, --version                        Print version
```

## Shell completion

Dynamic completion, which can process your configuration file and return you the namespaces of widgets dynamically.

Only works for bash
