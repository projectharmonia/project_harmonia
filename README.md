# Project Harmonia

A work-in-progress life simulation game made with [Bevy](https://bevyengine.org).

## Building

### Desktop

```bash
cargo build --release
```

### Mobile

Tested only on Android. It compiles and runs, but missing proper touch controls and gamepad support (`girls` doesn't support Android).

1. Install [xbuild](https://github.com/rust-mobile/xbuild)
2. Enable USB debugging on the device, connect it and allow access.
3. Get its ID:

```bash
x devices
```

3. Build:

```bash
x build --release --device <device ID> -p project_harmonia
```

## License

The code licensed under [GNU Affero General Public License v3.0](./COPYING).

Used [fonts](assets/base/fonts) licensed under [SIL Open Font License 1.1](./LICENSE-OFL).

The license for other assets and their authors are listed in their info files next to them.
