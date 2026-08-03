let appState = {
    enabled: false,
    selected_mic: "",
    selected_app: "",
    selected_volume: 1.0,
    selected_mic_volume: 1.0,
    available_mics: [],
    available_apps: [],
    mic_connected: false,
    app_connected: false,
    default_sink_name: ""
};

function showToast(message, type = 'success') {
    const container = document.getElementById('toast-container');
    const el = document.createElement('div');
    el.className = `toast-msg${type === 'error' ? ' error' : ''}`;
    el.textContent = message;
    container.appendChild(el);
    setTimeout(() => {
        el.style.transition = 'opacity 0.3s';
        el.style.opacity = '0';
        setTimeout(() => el.remove(), 350);
    }, 3000);
}

async function fetchStatus() {
    try {
        const res = await fetch('/api/status');
        if (!res.ok) throw new Error('API error');
        const data = await res.json();

        const micListChanged = JSON.stringify(appState.available_mics) !== JSON.stringify(data.available_mics);
        const appListChanged = JSON.stringify(appState.available_apps) !== JSON.stringify(data.available_apps);

        appState = data;
        updateUI(micListChanged, appListChanged);
    } catch (e) {
        console.error('fetchStatus failed:', e);
    }
}

function updateUI(refillMics = false, refillApps = false) {
    // Toggle button
    const toggleBtn = document.getElementById('master-toggle');
    if (appState.enabled) {
        toggleBtn.textContent = 'Disable';
        toggleBtn.className = 'btn btn-danger px-4';
    } else {
        toggleBtn.textContent = 'Enable';
        toggleBtn.className = 'btn btn-success px-4';
    }

    // Mic dropdown
    const micSelect = document.getElementById('mic-select');
    if (refillMics) {
        micSelect.innerHTML = '<option value="">— No microphone selected —</option>';
        (appState.available_mics || []).forEach(mic => {
            const opt = document.createElement('option');
            opt.value = mic.name;
            opt.textContent = mic.description;
            micSelect.appendChild(opt);
        });
        if (appState.selected_mic && !appState.available_mics.some(m => m.name === appState.selected_mic)) {
            const opt = document.createElement('option');
            opt.value = appState.selected_mic;
            opt.textContent = `${appState.selected_mic} (not found)`;
            micSelect.appendChild(opt);
        }
        micSelect.value = appState.selected_mic || '';
    }

    // App dropdown
    const appSelect = document.getElementById('app-select');
    if (refillApps) {
        appSelect.innerHTML = '<option value="">— No application selected —</option>';
        (appState.available_apps || []).forEach(app => {
            if (!app) return;
            const opt = document.createElement('option');
            opt.value = app;
            opt.textContent = app;
            appSelect.appendChild(opt);
        });
        if (appState.selected_app && !(appState.available_apps || []).includes(appState.selected_app)) {
            const opt = document.createElement('option');
            opt.value = appState.selected_app;
            opt.textContent = `${appState.selected_app} (not active)`;
            appSelect.appendChild(opt);
        }
        appSelect.value = appState.selected_app || '';
    }

    // Mic volume slider
    const micSlider = document.getElementById('mic-volume');
    const micVolLabel = document.getElementById('mic-volume-val');
    if (document.activeElement !== micSlider) {
        micSlider.value = appState.selected_mic_volume ?? 1.0;
    }
    micVolLabel.textContent = `${Math.round(micSlider.value * 100)}%`;

    // App volume slider
    const appSlider = document.getElementById('app-volume');
    const appVolLabel = document.getElementById('app-volume-val');
    if (document.activeElement !== appSlider) {
        appSlider.value = appState.selected_volume ?? 1.0;
    }
    appVolLabel.textContent = `${Math.round(appSlider.value * 100)}%`;

    // Signal flow boxes
    const flowMic = document.getElementById('flow-mic');
    document.getElementById('flow-mic-name').textContent = appState.selected_mic || '—';
    flowMic.className = appState.mic_connected ? 'flow-box connected' : 'flow-box disconnected';

    const flowApp = document.getElementById('flow-app');
    document.getElementById('flow-app-name').textContent = appState.selected_app || '—';
    flowApp.className = appState.app_connected ? 'flow-box connected' : 'flow-box disconnected';

    document.getElementById('flow-vmic').className = appState.enabled ? 'flow-box connected' : 'flow-box disconnected';

    checkChanged();
}

function checkChanged() {
    const micVal    = document.getElementById('mic-select').value;
    const appVal    = document.getElementById('app-select').value;
    const micVol    = parseFloat(document.getElementById('mic-volume').value);
    const appVol    = parseFloat(document.getElementById('app-volume').value);

    const changed =
        micVal !== (appState.selected_mic || '') ||
        appVal !== (appState.selected_app || '') ||
        Math.abs(micVol - (appState.selected_mic_volume ?? 1.0)) > 0.01 ||
        Math.abs(appVol - (appState.selected_volume ?? 1.0)) > 0.01;

    document.getElementById('save-btn').disabled = !changed;
}

async function saveSettings() {
    const payload = {
        physical_mic:  document.getElementById('mic-select').value,
        target_app:    document.getElementById('app-select').value,
        target_volume: parseFloat(document.getElementById('app-volume').value),
        mic_volume:    parseFloat(document.getElementById('mic-volume').value),
        enabled:       appState.enabled
    };

    try {
        const res = await fetch('/api/settings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });
        if (res.ok) {
            showToast('Settings saved!');
            await fetchStatus();
        } else {
            showToast('Failed to save settings', 'error');
        }
    } catch (e) {
        showToast('Connection error', 'error');
    }
}

async function togglePower() {
    const nextState = !appState.enabled;
    const payload = {
        physical_mic:  appState.selected_mic,
        target_app:    appState.selected_app,
        target_volume: appState.selected_volume ?? 1.0,
        mic_volume:    appState.selected_mic_volume ?? 1.0,
        enabled:       nextState
    };

    try {
        const res = await fetch('/api/settings', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });
        if (res.ok) {
            showToast(nextState ? 'PBS enabled' : 'PBS disabled');
            await fetchStatus();
        } else {
            showToast('Failed', 'error');
        }
    } catch (e) {
        showToast('Connection error', 'error');
    }
}

// Event listeners
document.getElementById('master-toggle').addEventListener('click', togglePower);
document.getElementById('save-btn').addEventListener('click', saveSettings);
document.getElementById('mic-select').addEventListener('change', checkChanged);
document.getElementById('app-select').addEventListener('change', checkChanged);

const micSlider = document.getElementById('mic-volume');
micSlider.addEventListener('input', () => {
    document.getElementById('mic-volume-val').textContent = `${Math.round(micSlider.value * 100)}%`;
    checkChanged();
});

const appSlider = document.getElementById('app-volume');
appSlider.addEventListener('input', () => {
    document.getElementById('app-volume-val').textContent = `${Math.round(appSlider.value * 100)}%`;
    checkChanged();
});

// Init
fetchStatus();
setInterval(fetchStatus, 2000);
