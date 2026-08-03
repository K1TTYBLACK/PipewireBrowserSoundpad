# PBS - Pipewire Browser Soundpad

PBS is a lightweight, zero-latency virtual microphone router that routes your physical microphone and selected browser output to a single virtual microphone.

## Yes it is VibeCoded
I can't code sh*t since AI took over the world and I'm no longer a software engineer, but I tried to use best practices and understanding to make it as light as possible: No Bloat, No Electron for UI, written on Rust. AppImage 1.5MB, takes about 2.5MB of your RAM.

## 🚀 How it Works
1. Creates a virtual microphone device (`pbs_virtual_mic`).
2. Links your physical mic and selected application to the virtual microphone.
3. Your application output remains routed to your default headphones/speakers so you can still hear it.
4. Auto-saves configuration to `~/.config/pbs/config.json` and auto-restores links when devices reconnect or apps are restarted.

---

## 🖥️ System Tray Icon & Web UI
* When running, PBS creates a microphone icon in your system tray. 
* Accessible at [http://localhost:9876](http://localhost:9876).
* If you dont see your browser in the list, play any audio it it and it will show up.

---

## 📦 AppImage (Recommended)

PBS is packaged as a portable **AppImage** . I recommend installing it with GearLever.

### Build
```bash
./build_appimage.sh
```

### Run AppImage if not using GearLever:
   ```bash
   chmod +x PBS_Soundpad-x86_64.AppImage
   ./PBS_Soundpad-x86_64.AppImage
   ```




