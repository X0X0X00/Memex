# Installing Memex

## macOS

1. Download `Memex_0.1.0_aarch64.zip` (Apple Silicon) from the Releases page.
2. Double-click the zip to extract `Memex.app`.
3. Drag **Memex.app** into your Applications folder.
4. **First launch (important):** macOS will block the app because it isn't code-signed.
   Right-click `Memex.app` in Applications → **Open** → confirm in the dialog.
   After this once, you can launch normally.

If you see "app is damaged and can't be opened":

```sh
xattr -dr com.apple.quarantine /Applications/Memex.app
```

## Windows

1. Download `Memex_0.1.0_x64-setup.exe` from Releases.
2. Windows SmartScreen will warn that the publisher is unknown. Click
   **More info** → **Run anyway**.

## Linux

Download either the `.deb` or the `.AppImage` from Releases. AppImage is a
single executable: `chmod +x Memex_*.AppImage && ./Memex_*.AppImage`.

## Why no code-signing?

Code-signing certificates cost real money ($99/yr for Apple, $300+/yr for
Windows EV certs) and Memex is a free, open-source side project. The
workarounds above are one-time per platform.

## Build from source

Memex never makes a network request. To verify the binary against the source:

```sh
git clone <repo>
cd Memex
npm install
npm run tauri build
```

Output is written to `src-tauri/target/release/bundle/`.
