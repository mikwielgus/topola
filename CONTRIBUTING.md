# Contributing to Topola

*Anyone* can contribute to Topola, including you.

Contributions can be of any kind: documentation, organization,
tutorials, blog posts, bug reports, issues, feature requests, feature
implementations, pull requests, helping to manage issues, etc.. Many of
these tasks do not require specialized programming knowledge, or any
programming at all.

## Chat

You are encouraged to join our [Matrix
chatroom](https://matrix.to/#/%23topola:tchncs.de) or [IRC
channel](https://webchat.oftc.net/?channels=#topola) to talk with us
before you contribute. Both chatrooms are bridged, so it does not matter
which one you join.

## Reporting issues

If you believe you have found a defect in Topola or its documentation,
please report it on our [issue
tracker](https://codeberg.org/topola/topola/issues).

Under normal operation, crashes and panics are always considered
reportable bugs.

## Writing code

We welcome code from anyone regardless of skill or experience level.
We're friendly to newcomers. We will help you with your contribution if
there are any problems.

Topola accepts contributions via pull requests. For a step-by-step guide
on how to use these, refer to Codeberg's
[documentation](https://docs.codeberg.org/collaborating/pull-requests-and-git-flow/).

Before you submit a pull request, make sure Topola actually builds with
your changes. Follow the build instructions from the next section.

### Building

#### Prerequisites

Building Topola from source requires
[git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) and
[cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
to be installed on your system. Follow the instructions in above links
to obtain these.

#### Obtaining the source

Clone the [repository](https://codeberg.org/topola/topola):

    git clone https://codeberg.org/topola/topola.git

#### Preparing to build

Change your working directory to your clone of Topola's repository:

    cd topola

#### Egui port

Build the project with

    cargo build --features "egui,disable_contracts" --bin topola-egui

Finally, run Topola by executing

    cargo run --features "egui,disable_contracts" --bin topola-egui

##### Running Topola in a Web browser

Topola can be built to run in a Web browser using
[Trunk](https://trunkrs.dev/), which will be installed with the
following command:

    cargo binstall trunk

To build and open Topola in your browser, run

    trunk serve

#### SDL2 demo

Optionally, for shorter build times you may build the SDL2 demo instead
of the Egui port:

    cargo build --features sdl2 --bin topola-sdl2-demo	
    cargo run --features sdl2 --bin topola-sdl2-demo

The downside is that the SDL2 demo's user interface is highly incomplete.


#### Automated tests

Topola has automated tests to make sure its basic functionalities work.
To execute these, run

    cargo test --features disable_contracts

#### Contracts

When trying to locate the source of a bug, it may be helpful to enable
[contracts](https://en.wikipedia.org/wiki/Design_by_contract) (yes, this
Wikipedia article needs improvement), which are nothing else but
slightly enchanced assertions.

Unfortunately, the
[contracts](https://docs.rs/contracts/latest/contracts/) library which
we have been using enforces post-conditions via closures, which have
numerous limitations. To bypass these we have forked and modified it to
use `try` blocks instead. The fork is vendored in the
[vendored/contracts/](vendored/contracts/) directory.

However, `try` blocks aren't present in stable Rust versions yet, so to
use these you need to set up your toolchain to use a nightly version of
Rust.

##### Nightly Rust

To use nightly Rust, run the following command:

    rustup override set nightly

You can go back to stable with

    rustup override unset

##### Enabling contracts

To enable contracts, simply remove the `disable_contracts` feature from
commands. For example, to build tests with contracts, simply run

    cargo test
