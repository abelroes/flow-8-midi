<div align="center">

<img alt="FLOW 8 MIDI Logo" src="./resources/flow_32x32.ico"><br>

# FLOW 8 MIDI Controller

A simple non-official cross-platform desktop MIDI controller for the [Behringer FLOW 8 mixer](https://www.behringer.com/behringer/product?modelCode=0603-AEW).

Made with 🦀 [Rust](https://www.rust-lang.org/), 🧊 [iced](https://iced.rs/) and 🎹 [midir](https://github.com/Boddlnagg/midir).

<img alt="FLOW 8 MIDI Controller" src="./resources/screenshot.png" width="100%">

</div>

## Features

- All channels Level, Mute and Solo commands
- Level commands for Main and Monitor busses

## How to Use

1. Download the FLOW 8 MIDI Controller
2. Connect your FLOW 8 device to your PC
3. Open the application
4. Use the GUI to control the parameters

## Known Issues

- Your device needs to be connected for the application to work
- No confirmation for the Phantom Power switch (use it at your own risk)

## Download

## Disclaimers

- This application is not official. Any damage, misuse or act that avoids warranty is not our responsibility. Use it at your own risk.
- Differently from the official bluetooth mobile app, this application can't reflect commands from the device. Meaning: if you change a fader in the app, you won't see it reflected in this application.
- Only basic MIDI commands are implemented (for now - this is a work in progress).
- Current and future implementations are limited by the [FLOW 8 MIDI Implementation](https://mediadl.musictribe.com/media/PLM/data/docs/P0DNM/QSG_BE_0603-AEW_FLOW-8_WW.pdf#page=23)
- Later, I found [another solution](https://ikarusstore.com/community/articulo-27-control-your-behringer-flow-8-via-windows-mac-controla-tu-behringer-flow-8-via-windows-y-mac) for controlling this unit. Give it a try and use what is best for you!

## License

## Donations
