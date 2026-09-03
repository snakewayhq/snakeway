---
title: Editor Support
---

When you write Snakeway configuration by hand, a mistake normally surfaces when the proxy refuses to start.
The language server moves that feedback into your editor.
It underlines most of the errors `snakeway config check` reports, completes field names and keyword values, shows
documentation and constraints on hover, and navigates between labeled blocks and their references.

## Starting the server

The server ships inside the `snakeway` binary.

```sh
snakeway lsp
```

The command speaks the Language Server Protocol over standard input and output, so you register it in your editor rather
than run it yourself.
One registration covers every Snakeway config file.
The server picks the right schema for each file from its path.

- `snakeway.hcl` is served as the entry point.
- Files matching your `include.devices` glob are served as device documents.
- Files matching your `include.ingresses` glob are served as ingress documents.

The server reads the globs from the `snakeway.hcl` nearest to the file you open, so a customized include pattern routes
correctly without any editor configuration.
Without a `snakeway.hcl`, the conventional `device.d/*.hcl` and `ingress.d/*.hcl` layout is served, so a new project
gets feedback before its entry point exists.

## Registering in a JetBrains IDE

A JetBrains IDE, such as RustRover or IntelliJ IDEA, attaches a language server through the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin.

1. Install the LSP4IJ plugin.
   It requires IDE version 2024.2 or later.
2. Open the language server settings and click `+` to open the New Language Server dialog.
   The dialog is also reachable from the LSP console menu.
3. On the Server tab, name the server `Snakeway` and set the command to `snakeway lsp`.
   The command must be on your `PATH`, or give its absolute path.
4. On the Mappings tab, add a file name pattern mapping for `*.hcl` with language id `hcl`.

Open a Snakeway config file and the diagnostics appear.
An unrelated HCL file is left alone, with one warning in the LSP console naming it.

## Registering in VS Code

VS Code attaches a language server through an extension, so the setup takes two extensions and three settings.

1. Install an HCL grammar extension, such as HashiCorp's `HCL`, so `.hcl` files have the `hcl` language id.
2. Install `Generic LSP Client (v2)` (`zsol.vscode-glspc`).
3. Point the client at the server in your `settings.json`:

```json
{
  "glspc.server.command": "snakeway",
  "glspc.server.commandArguments": [
    "lsp"
  ],
  "glspc.server.languageId": [
    "hcl"
  ]
}
```

The command must be on your `PATH`, or give its absolute path.
The client starts the server for every HCL file.
The server routes each file by its path.
A Snakeway config file gets its schema.
An unrelated HCL file is left alone, with one warning in the `Generic LSP Client` output channel.

Any other client that can launch a language server over standard input and output works the same way.
The command is `snakeway lsp`, and the file pattern is `*.hcl` under your config directory.

## Limitations

The server checks each file on its own, so a few `snakeway config check` rules stay outside the editor.
Run `snakeway config check` before a deployment.

- Rules that read the whole configuration: a duplicate device across files, a device that requires the identity device,
  a duplicate service name across ingress files, and a listener that is not unique.
- Rules that read the host, such as a certificate file that must exist or a data directory that must be present.
  These paths resolve against your editor's working directory, so the editor can accept a path that the CLI rejects.

If a file matches no schema, the server leaves it alone and writes one warning to the editor's log naming the file.
A file that is already open keeps its schema when you change the include globs.
Reopen the file to route it again.
