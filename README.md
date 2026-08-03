# PBS - Pipewire Browser Soundpad

PBS is a lightweight, zero-latency virtual microphone router that routes your physical microphone and selected browser output (e.g., Brave) to a single virtual microphone.

## 🚀 How it Works
1. Creates a virtual microphone device (`pbs_virtual_mic`).
2. Automatically links your physical mic and selected application to the virtual microphone.
3. Your application output remains routed to your default headphones/speakers so you can still hear it.
4. Auto-saves configuration to `~/.config/pbs/config.json` and auto-restores links when devices reconnect or apps are restarted.

---

## 📦 AppImage (Recommended)

PBS is packaged as a portable **AppImage** that runs on any modern Linux distribution without installation.

### How to Build the AppImage
If you wish to rebuild the AppImage yourself, run:
```bash
./build_appimage.sh
```
This downloads `appimagetool` and outputs `PBS_Soundpad-x86_64.AppImage` in the root folder.

### How to Run the AppImage
1. Make it executable (if not already):
   ```bash
   chmod +x PBS_Soundpad-x86_64.AppImage
   ```
2. Launch it from the terminal or your application launcher:
   ```bash
   ./PBS_Soundpad-x86_64.AppImage
   ```

---

## 🖥️ System Tray Icon & Web UI
* **System Tray:** When running, PBS creates a microphone icon in your system tray. 
  * Click the tray icon to open the dashboard in your default browser.
  * Right-click to open the context menu ("Open Dashboard" or "Quit").
* **Web UI Dashboard:** Accessible at [http://localhost:9876](http://localhost:9876).
  * Now dynamically suggests any application currently producing audio (e.g. Brave, Chrome, Discord, Spotify) in the Target Application input.

For detailed architecture and systemd background service configuration, see [pbs_soundpad_guide.md](./pbs_soundpad_guide.md).
