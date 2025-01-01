<!--
SPDX-FileCopyrightText: 2024 Topola contributors

SPDX-License-Identifier: MIT
-->

# Installing Topola

## Building and installing Topola from source

### Prerequisites

Building Topola from source requires
[git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) and
[cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
to be installed on your system. Follow the instructions in the above links to
obtain these.

### Obtaining the source

Clone the [repository](https://codeberg.org/topola/topola):

    git clone https://codeberg.org/topola/topola.git

### Preparing to build

Change your working directory to your clone of Topola's repository:

    cd topola

### Command-line application

Topola has a command-line application written the help of the
[clap](https://docs.rs/clap/latest/clap/) library.

(If you do not want to install new software on your system, skip now to the
[Debug build](#debug-build) subsection.)

Run the following command to build and install the Topola's command-line
application:

    cargo install --locked --path crates/topola-cli

You can then invoke the application from your terminal as `topola`.

#### Debug build

If you do not want to install new software on your system, or are interested in
debugging or developing Topola, you can build a debug executable of the Topola's
command-line application inside your working directory by running

    cargo build -p topola-cli

Once built, you can invoke the debug executable by replacing the `topola`
command with `cargo run -p topola-cli -- `.

#### Autorouting example

As an example, running the following commands will autoroute a KiCad
project of a simple THT diode bridge rectifier:

    cd tests/single_layer/tht_diode_bridge_rectifier/
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
[egui](https://github.com/emilk/egui/) library and its paired
[eframe](https://github.com/emilk/egui/tree/master/crates/eframe) framework.

(If you do not want to install new software on your system, skip now to the
[Debug build](#debug-build-1) subsection.)

The following command will build and install the Topola's GUI application:

    cargo install --locked --path crates/topola-egui

You can then start the application from your terminal by running

    topola-egui

#### Debug build

If you do not want to install new software on your system, or are interested in
debugging or developing Topola, you can build a debug executable of the Topola's
GUI application inside your working directory by running

    cargo build -p topola-egui

Once built, you can start the application from the debug executable with the
following command:

    cargo run -p topola-egui

#### Native run-time dependencies

On Linux and BSDs, the `egui` depends on the native graphics libraries (X11, Wayland),
and requires [GNOME `zenity`](https://gitlab.gnome.org/GNOME/zenity) when using `xdg-portal`
(which is the default) as the backend for [`rfd`](https://docs.rs/rfd) (which we use for file chooser dialogs).

If the Topola GUI crashes on startup (no window is shown), necessary graphics libraries (X11, Wayland)
might be missing. Note that running `ldd` on the `topola-egui` executable doesn't show these,
they are loaded dynamically (via some `dlopen`-like mechanism) on startup.

If no file chooser dialog is shown (e.g. when trying to Open a DSN file),
and, if `topola-egui` is started from a terminal, an error message like:
```
[2025-01-01T01:16:17Z ERROR rfd::backend::xdg_desktop_portal] pick_file error No such file or directory (os error 2)
```
is emitted (which should only happen on Linux/BSDs and similar environments), then
[GNOME `zenity`](https://gitlab.gnome.org/GNOME/zenity) is not installed, but is required
for the file choser to work.
Alternatively, one might try the alternative `gtk3` backend for `rfd` by enabling the `gtk3`
feature of `topola-egui`, e.g.
```
cargo build -p topola-egui --release --no-default-features --features disable_contracts --features gtk3
```
This is mostly interesting for people who want to package Topola, and allow exposing features
(e.g. Gentoo Linux / Portage)

#### Running Topola in Web browser

Topola's GUI application can be built to and run in a Web browser using
[Trunk](https://trunkrs.dev/). If you have
[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 
on your system, you can install Trunk from binary with

    cargo binstall trunk

Alternatively, you can build Trunk from source by running

    cargo install trunk

To build and open Topola in your browser, run

    trunk serve

This will work both with and without having the GUI application
installed.

### Automated tests

Topola has automated tests to make sure its basic functionalities work.
To execute these, run

    cargo test

Automated tests are run in `debug` profile.

### Contracts

(The feature described in this section is enabled only when using nightly Rust
and only under `debug` profile. If you are not interested in debugging, you can
skip this section altogether.)

When trying to locate the source of a bug, it may be helpful to enable
[contracts](https://en.wikipedia.org/wiki/Design_by_contract) (yes, this
Wikipedia article needs improvement), which are nothing else but
somewhat enchanced assertions.

Unfortunately, the
[contracts](https://docs.rs/contracts/latest/contracts/) library which
we have been using enforces post-conditions via closures, which have
deal-breaking limitations. To bypass these we have forked and modified
it to use `try` blocks instead. The fork is vendored in the
[vendored/contracts/](vendored/contracts/) directory.

However, `try` blocks are not present in stable Rust yet, so to use
these you need to set up your toolchain to use a nightly version of
Rust.

#### Nightly Rust

To use nightly Rust, run the following command:

    rustup override set nightly

You can go back to stable with

    rustup override unset

#### Enabling contracts

To enable contracts, simply add a `--no-default-features` switch. This
switches off a default feature that prevents contracts from executing.
For example, to build tests with contracts, simply run

    cargo test --no-default-features

Of course, you can enable contracts for any build target. For instance,
the following command will build the Egui application with debug profile
and contracts enabled:

    cargo build -p topola-egui --no-default-features
