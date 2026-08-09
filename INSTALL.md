<!--
SPDX-FileCopyrightText: 2024 Topola contributors

SPDX-License-Identifier: MIT
-->

# Installing Topola

## Building and installing Topola from source

### Prerequisites

Building Topola from source requires
[Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) and
[Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
to be installed on your system. Follow the instructions in the above links to
obtain these.

### Obtaining the source

From your command line, clone the
[repository](https://codeberg.org/topola/topola):

    git clone https://codeberg.org/topola/topola.git

### Preparing to build

Change your working directory to your clone of Topola's repository:

    cd topola

### Command-line application

Topola has a command-line application (CLI) written with the help of the
[`clap`](https://docs.rs/clap/latest/clap/) library.

#### Installation from source

(Topola can be also built and run without installation. If you do not want to install
new software on your system, skip now to the [*Debug build*](#debug-build)
subsection.)

Run the following command to build and install the Topola's command-line
application:

    cargo install --locked --path crates/topola-cli

You can then invoke the application from your terminal as `topola`. For example,

    topola --help

will display an explanation of the available command-line options and arguments.

#### Debug build

If you do not want to install new software on your system, or are interested
in developing or debugging Topola, you need to build a debug executable of the
Topola's command-line application inside your working directory by running

    cargo build -p topola-cli

Once built, you can invoke the debug executable by writing `cargo run -p
topola-cli --` in the same place and with same arguments as you would write an
installed `topola` command.

#### Autorouting example

As an example, running the following commands will autoroute a KiCad
project of a simple THT diode bridge rectifier:

    cd tests/unilayer/tht_diode_bridge_rectifier/
    topola tht_diode_bridge_rectifier.dsn

(Obviously, to use the debug executable, replace the second command with
`cargo run -p topola-cli -- tht_diode_bridge_rectifier.dsn`.)

By default, the output filename is the input filename with extension
changed to `ses`: `tht_diode_bridge_rectifier.ses`.

##### Viewing the results

You can view the results of the autorouting in KiCad if you have it
installed. First, open the layout in the KiCad PCB Editor:

    pcbnew tht_diode_bridge_rectifier.kicad_pcb

Then choose *File > Import > Specctra Session...* from the menu bar.
In the newly opened file dialog, choose the file named
*tht_diode_bridge_rectifier.ses*. This will load the autorouted traces.

### Egui graphical user interface application

Topola has a graphical user interface (GUI) application written using the
[`egui`](https://github.com/emilk/egui/) library and its paired
[`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe) framework.
For displaying dialog boxes, Topola's GUI application uses the
[Rusty File Dialogs](https://docs.rs/rfd/latest/rfd/) library.

**_NOTE:_** Due to technical constraints, these libraries have multiple runtime dependencies
that are not managed by the Rust's package manager, Cargo, and as such, it is
impossible to determine whether these dependencies have been satisfied during
compilation, but only once the application has been launched. Unfortunately,
because of that, Topola may crash on startup or have file selection dialog not
appear on some systems due to unsatisfied runtime dependencies. If you encounter
any problems, read the
[*Troubleshooting unmanaged runtime dependencies*](#troubleshooting-unmanaged-runtime-dependencies)
subsection.

#### Native installation from source

(Topola can be also built and run without installation. If you do not want to
install new software on your system, skip now to the
[*Native debug build*](#native-debug-build) subsection.)

The following command will build and install the Topola's GUI application:

    cargo install --locked --path crates/topola-egui

You can then launch the application from your terminal by running

    topola-egui

#### Native debug build

If you do not want to install new software on your system, or are interested in
debugging or developing Topola, you can build a debug executable of the Topola's
GUI application inside your working directory by running

    cargo build -p topola-egui

Once built, you can launch the application from the debug executable with the
following command:

    cargo run -p topola-egui

#### Running Topola in Web browser

Topola's GUI application can be built to run in a Web browser using
[Trunk](https://trunkrs.dev/). If you have
[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 
on your system, you can install Trunk from binary with

    cargo binstall trunk

Alternatively, you can build Trunk from source by running

    cargo install trunk

To build and open the Topola's GUI application in your browser, run

    trunk serve

#### Troubleshooting unmanaged runtime dependencies

##### Crash on startup

If the Topola's GUI application crashes on startup (no window is shown),
necessary libraries for graphics and windowing (such as X11 and Wayland) may
be missing. Note that running `ldd` on the `topola-egui` executable does not
show these, as they are loaded dynamically (via some `dlopen`-like mechanism)
on startup.

##### No file selection dialog appears

If no file selection dialog appears when trying to open a file, then this is
most likely because you do not have
[Zenity](https://gitlab.gnome.org/GNOME/zenity) installed on your system. In
this case, an error similar to the following should be emitted in your terminal:

```
[2025-01-01T01:16:17Z ERROR rfd::backend::xdg_desktop_portal] pick_file error No such file or directory (os error 2)
```

Zenity is needed because it is used by the default `xdg-portal` backend of
Rusty File Dialogs. As an alternative, you can try its other backend, `gtk3`, by
building `topola-egui` with the following command:

    cargo build -p topola-egui --release --no-default-features --features gtk3

### Automated tests

Topola has automated tests to make sure its basic functionalities work.
To execute these, run

    cargo test

### Contracts

(The feature described in this section is currently used only for debugging and
is enabled only when using nightly Rust. If you are not interested in debugging,
you can skip this section altogether.)

When trying to locate the source of a bug, it may be helpful to enable
[contracts](https://en.wikipedia.org/wiki/Design_by_contract) (yes, this
Wikipedia article needs improvement), which are nothing else but somewhat
enchanced assertions.

Regrettably, the [contracts](https://docs.rs/contracts/latest/contracts/)
library which we have been using enforces post-conditions using closures.
Borrow semantics of these closures cause compile errors for some function
signatures, which is a deal-breaking limitation. Hence, to bypass this problem,
we maintain our own fork of this library to use `try` blocks instead. Our
fork's [repository](https://codeberg.org/topola/contracts-try) is available
on Codeberg.

However, `try` blocks are not present in stable Rust yet, so to be able to
proceed you need to set up your toolchain to use a nightly version of Rust.

#### Nightly Rust

To use nightly Rust, run the following command:

    rustup override set nightly

You can go back to stable with

    rustup override unset
