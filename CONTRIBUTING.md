## Prerequisites

Building Topola from source requires [git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git) and [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) to be installed on your system.

## Installation
	
Clone the [repository](https://codeberg.org/topola/topola):

	git clone https://codeberg.org/topola/topola.git
	
Topola currently requires nightly Rust to build. Enable it with

	rustup override set nightly
		
### egui port
	
After cloning, change your working directory to your clone of Topola's repository:

    cd topola

Then build the project with

	cargo build --features egui --bin topola-egui	
	
Finally, run Topola by executing

	cargo run --features egui --bin topola-egui

#### Running Topola in a Web browser

Topola can be built to run in a Web browser using [Trunk](https://trunkrs.dev/), which will be installed with the following command:

    cargo binstall trunk

To build and open Topola in your browser, run

    trunk serve
	
### SDL2 demo

For shorter build times you may use the SDL2 demo instead of the egui port:

	cargo build --features sdl2 --bin topola-sdl2-demo	
	cargo run --features sdl2 --bin topola-sdl2-demo

The downside is that the SDL2 demo's user interface is highly incomplete.

## Contributing

Topola project accepts contributions using pull requests. For a step-by-step guide on how to use these, refer to Codeberg's [documentation](https://docs.codeberg.org/collaborating/pull-requests-and-git-flow/).