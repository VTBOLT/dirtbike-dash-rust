# Dirtbike Dash Rust
<h4 style="display: inline"> New dirtbike dashboard, now programmed in Rust</h4>

## Implemented Features

- Reads virtualized can network on latop and places results within a data structure for later use. Temporarily prints to console
- State of charge may be completed. In any case, its ready for me to manually supply initial data and start testing

## TODO Features

- Read actual can data on a raspberry pi
- Redo GPS in human code
- Slint frontend but that's up to Cayden
- Finish up SOC

## Setup Instructions

### Build instructions

If you want to build this project locally, rust does work fundamentally different.
First, rust requires manual installation, instructions for which can be found [here](https://rust-lang.org/tools/install/).
<br>
Ensure that cargo is added to PATH and restart visual studio or your console before attempting to build. 
You will also have to clone the repo and manually cd into `./dirtbike-dash` before running a build command.

<details>
<summary><h3 style="display: inline">All Non-Linux Systems</h3></summary>

- To build on systems other than linux, simply pass `cargo build` when cd'd into the directory.
- Note that, because this program is built for a raspberry pi, it is build around certain linux packages that are not supported, so testing will likely not work

</details>

---

<details>
<summary><h3 style="display: inline">Only Windows</h3></summary>

- To build on windows, your options are either to build in visual studio, or build in WSL.
- I don't recommend Visual Studio and cannot help, probably use WSL
- For WSL, windows, rust, and VS Code each give guides for installing and using their software in WSL, follow those.
- You probably still won't be able to build with features, but otherwise follow the Linux instructions inside the WSL window.

</details>

---

<details>
<summary><h3 style="display: inline">Linux Systems</h3></summary>

- To build on linux systems without dependencies, simply pass `cargo build` or `cargo run` as before. Note that running without dependencies will not work
- To build with dependencies, pass `cargo build --features "can gps sim"`. Descriptions of each optional feature will be below
- The list of dependencies can be found in `./dirtbike-dash/Cargo.toml` as well as the descriptions below, though they will have to be manually acquired through your package manager

</details>

### Optional Features

#### Can / SocketCAN

This is the protocol that reads from the CAN network.
<br>
Requires the can and socketcan packages to be installed locally. As far as I am aware, these packages do not exist for linux and mac unfortunately.
<br>
That said, they are mandatory for most of the capabilities of the dashboard. Unlucky

---

#### GPS / GPSD

This is the protocol that handles gps data. It is the most optional of the optional features for testing, so if you can't get it to work like I couldn't for about two days, don't include it as a feature argument and move on.
<br>
Should require both the gpsd and gpsd_proto packages to be installed. Should not require gpsd_client, but if you are having build issues within `gps.rs`, try installing it and let me know to update.

---

#### Sim

Sim is the dedicated testing argument.
<br>
It requires access to a virtual can port, which will have to be set up independently. I am not aware of setup instruction for non-linux systems if possible at all, but vcan setup instructions are located at the head of `./dirtbike-dash/src/sim.rs`.
<br>
For Linux systems, simply run `bash PATH/vcan.sh` in your terminal, where PATH is the path to this repo. It contains the few lines needed to set it up automatically
<br>
Sim does not support simulated data for gps and only simulates data. It does not collect or manage any actual data.

---

#### SOC

SOC, or state of charge, tells us how much battery is left of the total capacity
<br>
Notice that it will only ever be an estimate, getting exact values is impossible since batteries aren't perfectly consistent when new, let alone after losing some health
<br>
A more in-depth rundown of how it is calculated can be found on the wiki soon

---

### Project by:

- Blake Gaither (All Backend -GPS)
- Cayden Cubbin (Frontend)
- Max Lupariello (GPS)
