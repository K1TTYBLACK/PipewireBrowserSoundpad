# PBS - Pipewire Browser Soundpad

PBS is a lightweight, zero-latency virtual microphone router that routes your physical microphone and selected browser output to a single virtual microphone.

I personnaly use it to play some music from Youtube/Spotify while hanging out with friends on VRChat, but you can use it for anything you want. 

## Recommendations
* If you play music make sure to set sensitivity in Discord, VRChat or other apps to lowest so audio doesn't cuts off.
* Set **PBS Virtual Mic** as default mic. Why would you? This way it will be prioritised. PBS only spawns virtual mic when app is running. So when not running, your system will fallback to main mic. And on launch it will be already ready to use without changing system inputs all the time.
* https://www.myinstants.com/ whould be useful for soundboards.


## Yes, it is VibeCoded
I can't code sh*t since AI took over the world and I'm no longer a software engineer, but I tried to use best practices and understanding to make it as light as possible: No Bloat, No Electron for UI, written on Rust. AppImage 1.5MB, takes about 2.5MB of your RAM.

## 🚀 How it Works
* Settings are accessible at [http://localhost:9876](http://localhost:9876) or via icon in system tray.
* Links your physical mic and selected application to the virtual microphone.
* If you dont see your browser in the list, play any audio it it and it will show up.
* Auto-saves configuration to `~/.config/pbs/config.json` and auto-restores links when devices reconnect or apps are restarted.


---

## 📦 Installation

[https://github.com/K1TTYBLACK/PipewireBrowserSoundpad/releases](**📥 DOWNLOAD HERE**)

PBS is packaged as a portable **AppImage**. I recommend installing it with [https://flathub.org/en/apps/it.mijorus.gearlever](GearLever).

### Build AppImage
```bash
./build_appimage.sh
```

### Run AppImage if not using GearLever:
   ```bash
   chmod +x PBS_Soundpad-x86_64.AppImage
   ./PBS_Soundpad-x86_64.AppImage
   ```

---

## Feel free to contribute/request features/report bugs.
