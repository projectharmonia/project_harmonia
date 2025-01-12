# Project Harmonia

A work-in-progress life simulation game made with [Bevy](https://bevyengine.org).

![](https://lemmy.ml/pictrs/image/cbdfc19d-0f34-4fec-ae21-598c58648b09.png)

<img style="height: 150px;" src="https://lemmy.ml/pictrs/image/6c42fe21-6b9c-416a-b578-8bb7e3ef0f31.png"> <img style="height: 150px;" src="https://lemmy.ml/pictrs/image/cbf70dd4-07ad-4f0a-8239-7b7bb50bc8b2.png"> <img style="height: 150px;" src="https://lemmy.ml/pictrs/image/d13e57da-9d8c-4351-85ff-782a86ee84de.png">

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

The license for other assets and their authors are listed in their manifest files next to them.
