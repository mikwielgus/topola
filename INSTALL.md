# Installing Topola

## Installing Topola from Source

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

The following example will autoroute a KiCad project of a simple THT
diode bridge rectifier:

```
cd tests/single_layer/data/tht_diode_bridge_rectifier/
topola tht_diode_bridge_rectifier.dsn tht_diode_bridge_rectifier.ses autoroute_all.cmd
```

##### Viewing the results

You can view the results of the autorouting in KiCad if you have it
installed. First, open the layout in the KiCad PCB Editor:

```
pcbnew tht_diode_bridge_rectifier.kicad_pcb
```

Then choose *File > Import > Specctra Session...* from the menu bar.
In the newly opened file dialog, choose the file named
*tht_diode_bridge_rectifier.ses*. This will load the autorouted traces.

### Egui GUI application

The following command will build and install Topola's Egui graphical
user interface application:

    cargo install --locked --path . --features egui --bin egui

You can then invoke the application from your terminal by running

```
topola-egui
```

#### Running Topola in a Web browser

Topola's Egui application can be built to run in a Web browser using
[Trunk](https://trunkrs.dev/), which will be installed with the
following command:

    cargo binstall trunk

To build and open Topola in your browser, run

    trunk serve

### Automated tests

Topola has automated tests to make sure its basic functionalities work.
To execute these, run

    cargo test

### Contracts

When trying to locate the source of a bug, it may be helpful to enable
[contracts](https://en.wikipedia.org/wiki/Design_by_contract) (yes, this
Wikipedia article needs improvement), which are nothing else but
slightly enchanced assertions.

Unfortunately, the
[contracts](https://docs.rs/contracts/latest/contracts/) library which
we have been using enforces post-conditions via closures, which have
deal-breaking limitations. To bypass these we have forked and modified it
to use `try` blocks instead. The fork is vendored in the
[vendored/contracts/](vendored/contracts/) directory.

However, `try` blocks aren't present in stable Rust versions yet, so to
use these you need to set up your toolchain to use a nightly version of
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

Of course, you can enable contracts for any build target. For example,
the following command will build the Egui application with debug profile
and contracts enabled:

    cargo build --features egui --bin topola-egui --no-default-features
