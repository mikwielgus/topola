# Installing Topola

## Building and installing Topola from source

Note that running any of the below commands that start with `cargo
install` will install a Topola binary on your system. We assume this is
what most people coming here want. If you want to build and run Topola
without installing it, skip these particular commands and follow the
subsections named *Building and running without installing*.

By default, the installed version will have a `release` profile, whereas
without installing the `debug` profile will be used by default.

### Prerequisites

Building Topola from source requires
[git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) and
[cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
to be installed on your system. Follow the instructions in above links
to obtain these.

### Obtaining the source

Clone the [repository](https://codeberg.org/topola/topola):

    git clone https://codeberg.org/topola/topola.git

### Preparing to build

Change your working directory to your clone of Topola's repository:

    cd topola

### Command-line application

Run the following command to build and install Topola's command-line
application:

    cargo install --locked --path . --features cli

The application will now be invokable from your terminal as `topola`.

#### Autorouting example

As an example, running the following commands will autoroute a KiCad
project of a simple THT diode bridge rectifier:

```
cd tests/single_layer/tht_diode_bridge_rectifier/
topola tht_diode_bridge_rectifier.dsn
```

By default, the output filename is the input filename with extension
changed to `ses`: `tht_diode_bridge_rectifier.ses`.

##### Viewing the results

You can view the results of the autorouting in KiCad if you have it
installed. First, open the layout in the KiCad PCB Editor:

```
pcbnew tht_diode_bridge_rectifier.kicad_pcb
```

Then choose *File > Import > Specctra Session...* from the menu bar.
In the newly opened file dialog, choose the file named
*tht_diode_bridge_rectifier.ses*. This will load the autorouted traces.

#### Building and running without installing

If you chose not to install the command-line application, you can build
and run it without installing by replacing the `topola` command with
`cargo run --features cli --`. Running the above autorouting example is
then as follows:

```
cd tests/single_layer/tht_diode_bridge_rectifier/
cargo run --features cli -- tht_diode_bridge_rectifier.dsn
```

Viewing the results is obviously the same.

### Egui graphical user interface application

Topola has a graphical user interface (GUI) application written using
the [egui](https://github.com/emilk/egui/) library and its paired
[eframe](https://github.com/emilk/egui/tree/master/crates/eframe)
framework.

The following command will build and install Topola's GUI application:

    cargo install --locked --path . --features egui --bin topola-egui

You can then invoke the application from your terminal by running

```
topola-egui
```

#### Building and running without installing

If you chose not to install the GUI application, you can build and run
it without installing by running

```
cargo run --features egui --bin topola-egui`
```

instead of the above `topola-egui` command.

#### Running Topola in a Web browser

Topola's GUI application can be built to and run in a Web browser using
[Trunk](https://trunkrs.dev/), which will be installed with the
following command:

    cargo binstall trunk

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

The feature described in this section works only in `debug` profile. If
you are not interested in debugging, you can skip it altogether.

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

    cargo build --features egui --bin topola-egui --no-default-features
