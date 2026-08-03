// ─── Application State ────────────────────────────────────────────────────────
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

// ─── Toast ────────────────────────────────────────────────────────────────────
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

// ─── Animated Signal Flow Canvas ──────────────────────────────────────────────
const canvas = document.getElementById('flow-canvas');
const ctx    = canvas.getContext('2d');

// Bezier cubic interpolation
function bezierPoint(t, p0, p1, p2, p3) {
    const mt = 1 - t;
    return {
        x: mt*mt*mt*p0.x + 3*mt*mt*t*p1.x + 3*mt*t*t*p2.x + t*t*t*p3.x,
        y: mt*mt*mt*p0.y + 3*mt*mt*t*p1.y + 3*mt*t*t*p2.y + t*t*t*p3.y
    };
}

// Particle pool — each line gets several staggered dots
function makeParticles(count) {
    return Array.from({ length: count }, (_, i) => ({
        t: i / count,
        speed: 0.006 + Math.random() * 0.003
    }));
}

const particles = {
    mic: makeParticles(5),
    app: makeParticles(5)
};

// Node layout constants (will be scaled to canvas size each frame)
// All values are in "design units" where canvas width = 560, height = 190
const DW = 560, DH = 190;
const NODE_W = 150, NODE_H = 52, NODE_R = 9;

// Node centres in design space
const NC = {
    mic:  { x: 95,  y: 58 },
    app:  { x: 95,  y: 142 },
    vmic: { x: 465, y: 100 }
};

// Bezier control points for the two paths
function pathFor(from, to) {
    const mx = (from.x + to.x) / 2;
    return {
        p0: from,
        p1: { x: mx, y: from.y },
        p2: { x: mx, y: to.y },
        p3: to
    };
}

// Convert design-space point → canvas pixel
function scale(pt, W, H) {
    return { x: pt.x * W / DW, y: pt.y * H / DH };
}

// Rounded rectangle path helper
function roundRect(ctx, x, y, w, h, r) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + w - r, y);
    ctx.arcTo(x + w, y,     x + w, y + r,     r);
    ctx.lineTo(x + w, y + h - r);
    ctx.arcTo(x + w, y + h, x + w - r, y + h, r);
    ctx.lineTo(x + r, y + h);
    ctx.arcTo(x,     y + h, x,     y + h - r, r);
    ctx.lineTo(x,    y + r);
    ctx.arcTo(x,     y,     x + r, y,          r);
    ctx.closePath();
}

function drawNode(ctx, cx, cy, W, H, label, name, active) {
    const sc  = pt => scale(pt, W, H);
    const c   = sc({ x: cx, y: cy });
    const nw  = NODE_W * W / DW;
    const nh  = NODE_H * H / DH;
    const nr  = NODE_R  * W / DW;
    const x   = c.x - nw / 2;
    const y   = c.y - nh / 2;

    // Shadow
    ctx.save();
    ctx.shadowColor = active ? 'rgba(25,135,84,0.25)' : 'rgba(0,0,0,0.06)';
    ctx.shadowBlur  = active ? 14 : 6;

    // Box fill
    roundRect(ctx, x, y, nw, nh, nr);
    ctx.fillStyle = active ? '#f0fdf4' : '#f8f9fa';
    ctx.fill();

    // Border
    ctx.strokeStyle = active ? '#198754' : '#ced4da';
    ctx.lineWidth   = active ? 2 : 1.5;
    ctx.stroke();
    ctx.restore();

    // Left accent bar
    const barW = 4 * W / DW;
    roundRect(ctx, x, y + nh * 0.2, barW, nh * 0.6, barW / 2);
    ctx.fillStyle = active ? '#198754' : '#dee2e6';
    ctx.fill();

    // Label text
    const fs1 = Math.max(9, 11 * W / DW);
    ctx.font      = `500 ${fs1}px system-ui, sans-serif`;
    ctx.fillStyle = '#6c757d';
    ctx.textAlign = 'center';
    ctx.fillText(label, c.x, y + nh * 0.38);

    // Name text
    const fs2 = Math.max(10, 13 * W / DW);
    ctx.font      = `700 ${fs2}px system-ui, sans-serif`;
    ctx.fillStyle = active ? '#198754' : '#495057';
    // Truncate if needed
    let display = name || '—';
    while (display.length > 3 && ctx.measureText(display).width > nw * 0.82) {
        display = display.slice(0, -4) + '…';
    }
    ctx.fillText(display, c.x, y + nh * 0.72);
}

function drawPath(ctx, from, to, active, color, W, H) {
    const s = pt => scale(pt, W, H);
    const { p0, p1, p2, p3 } = pathFor(
        { x: from.x + NODE_W / 2, y: from.y },
        { x: to.x   - NODE_W / 2, y: to.y   }
    );

    ctx.beginPath();
    ctx.moveTo(...Object.values(s(p0)));
    ctx.bezierCurveTo(
        ...Object.values(s(p1)),
        ...Object.values(s(p2)),
        ...Object.values(s(p3))
    );

    if (active) {
        ctx.save();
        ctx.shadowColor = color + '66';
        ctx.shadowBlur  = 8;
        ctx.strokeStyle = color;
        ctx.lineWidth   = 2.5 * W / DW;
        ctx.stroke();
        ctx.restore();
    } else {
        ctx.setLineDash([6 * W / DW, 8 * W / DW]);
        ctx.strokeStyle = '#ced4da';
        ctx.lineWidth   = 1.5 * W / DW;
        ctx.stroke();
        ctx.setLineDash([]);
    }
}

function drawArrowHead(ctx, from, to, active, color, W, H) {
    if (!active) return;
    const s   = pt => scale(pt, W, H);
    const { p3 } = pathFor(
        { x: from.x + NODE_W / 2, y: from.y },
        { x: to.x   - NODE_W / 2, y: to.y   }
    );
    // Tangent at t=0.98
    const a = bezierPoint(0.98, ...['p0','p1','p2','p3'].map(k =>
        pathFor({ x: from.x + NODE_W/2, y: from.y }, { x: to.x - NODE_W/2, y: to.y })[k]
    ));
    const b = s(p3);
    const angle = Math.atan2(b.y - scale(a, W, H).y, b.x - scale(a, W, H).x);
    const size  = 7 * W / DW;
    ctx.save();
    ctx.shadowColor = color + '88';
    ctx.shadowBlur  = 6;
    ctx.fillStyle   = color;
    ctx.beginPath();
    ctx.translate(b.x, b.y);
    ctx.rotate(angle);
    ctx.moveTo(0, 0);
    ctx.lineTo(-size * 2, -size);
    ctx.lineTo(-size * 2,  size);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
}

function drawParticles(ctx, ptList, from, to, active, color, W, H) {
    if (!active) return;
    const path = pathFor(
        { x: from.x + NODE_W / 2, y: from.y },
        { x: to.x   - NODE_W / 2, y: to.y   }
    );
    const points = [path.p0, path.p1, path.p2, path.p3];
    const r = 4 * W / DW;

    ptList.forEach(p => {
        p.t += p.speed;
        if (p.t > 1) p.t -= 1;

        const pos = bezierPoint(p.t, ...points);
        const sc  = scale(pos, W, H);

        // Fade in/out at ends
        const fade = Math.min(p.t * 6, (1 - p.t) * 6, 1);

        ctx.save();
        ctx.globalAlpha = fade * 0.85;
        ctx.beginPath();
        ctx.arc(sc.x, sc.y, r, 0, Math.PI * 2);
        ctx.fillStyle   = color;
        ctx.shadowColor = color;
        ctx.shadowBlur  = 10;
        ctx.fill();
        ctx.restore();
    });
}

function drawFrame() {
    // Resize canvas to match CSS pixels (HiDPI aware)
    const rect = canvas.getBoundingClientRect();
    const dpr  = window.devicePixelRatio || 1;
    const W    = rect.width;
    const H    = Math.round(W * DH / DW); // maintain aspect ratio

    if (canvas.width  !== Math.round(W * dpr) ||
        canvas.height !== Math.round(H * dpr)) {
        canvas.width  = Math.round(W * dpr);
        canvas.height = Math.round(H * dpr);
        canvas.style.height = H + 'px';
        ctx.scale(dpr, dpr);
    }

    ctx.clearRect(0, 0, W, H);

    // Background
    ctx.fillStyle = '#f8f9fa';
    ctx.fillRect(0, 0, W, H);

    const micActive  = appState.enabled && appState.mic_connected;
    const appActive  = appState.enabled && appState.app_connected;
    const vmicActive = appState.enabled;

    const micColor = '#0d6efd';
    const appColor = '#6f42c1';

    // Paths
    drawPath(ctx, NC.mic, NC.vmic, micActive, micColor, W, H);
    drawPath(ctx, NC.app, NC.vmic, appActive, appColor, W, H);

    // Particles
    drawParticles(ctx, particles.mic, NC.mic, NC.vmic, micActive, micColor, W, H);
    drawParticles(ctx, particles.app, NC.app, NC.vmic, appActive, appColor, W, H);

    // Arrowheads
    drawArrowHead(ctx, NC.mic, NC.vmic, micActive, micColor, W, H);
    drawArrowHead(ctx, NC.app, NC.vmic, appActive, appColor, W, H);

    // Nodes (draw over paths)
    drawNode(ctx, NC.mic.x,  NC.mic.y,  W, H, 'Physical Mic',   appState.selected_mic  || '—', appState.mic_connected);
    drawNode(ctx, NC.app.x,  NC.app.y,  W, H, 'Target App',     appState.selected_app  || '—', appState.app_connected);
    drawNode(ctx, NC.vmic.x, NC.vmic.y, W, H, 'PBS Virtual Mic','pbs_virtual_mic',              vmicActive);

    requestAnimationFrame(drawFrame);
}

// Kick off the animation loop
drawFrame();

// ─── API ──────────────────────────────────────────────────────────────────────
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
        toggleBtn.className   = 'btn btn-danger px-4';
    } else {
        toggleBtn.textContent = 'Enable';
        toggleBtn.className   = 'btn btn-success px-4';
    }

    // Mic dropdown
    const micSelect = document.getElementById('mic-select');
    if (refillMics) {
        micSelect.innerHTML = '<option value="">— No microphone selected —</option>';
        (appState.available_mics || []).forEach(mic => {
            const opt = document.createElement('option');
            opt.value       = mic.name;
            opt.textContent = mic.description;
            micSelect.appendChild(opt);
        });
        if (appState.selected_mic && !appState.available_mics.some(m => m.name === appState.selected_mic)) {
            const opt = document.createElement('option');
            opt.value       = appState.selected_mic;
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
            opt.value       = app;
            opt.textContent = app;
            appSelect.appendChild(opt);
        });
        if (appState.selected_app && !(appState.available_apps || []).includes(appState.selected_app)) {
            const opt = document.createElement('option');
            opt.value       = appState.selected_app;
            opt.textContent = `${appState.selected_app} (not active)`;
            appSelect.appendChild(opt);
        }
        appSelect.value = appState.selected_app || '';
    }

    // Mic volume slider
    const micSlider = document.getElementById('mic-volume');
    if (document.activeElement !== micSlider)
        micSlider.value = appState.selected_mic_volume ?? 1.0;
    document.getElementById('mic-volume-val').textContent =
        `${Math.round(micSlider.value * 100)}%`;

    // App volume slider
    const appSlider = document.getElementById('app-volume');
    if (document.activeElement !== appSlider)
        appSlider.value = appState.selected_volume ?? 1.0;
    document.getElementById('app-volume-val').textContent =
        `${Math.round(appSlider.value * 100)}%`;

    checkChanged();
}

function checkChanged() {
    const micVal = document.getElementById('mic-select').value;
    const appVal = document.getElementById('app-select').value;
    const micVol = parseFloat(document.getElementById('mic-volume').value);
    const appVol = parseFloat(document.getElementById('app-volume').value);

    const changed =
        micVal !== (appState.selected_mic    || '') ||
        appVal !== (appState.selected_app    || '') ||
        Math.abs(micVol - (appState.selected_mic_volume ?? 1.0)) > 0.01 ||
        Math.abs(appVol - (appState.selected_volume      ?? 1.0)) > 0.01;

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
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body:    JSON.stringify(payload)
        });
        if (res.ok) {
            showToast('Settings saved!');
            await fetchStatus();
        } else {
            showToast('Failed to save settings', 'error');
        }
    } catch {
        showToast('Connection error', 'error');
    }
}

async function togglePower() {
    const nextState = !appState.enabled;
    const payload = {
        physical_mic:  appState.selected_mic,
        target_app:    appState.selected_app,
        target_volume: appState.selected_volume      ?? 1.0,
        mic_volume:    appState.selected_mic_volume  ?? 1.0,
        enabled:       nextState
    };

    try {
        const res = await fetch('/api/settings', {
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body:    JSON.stringify(payload)
        });
        if (res.ok) {
            showToast(nextState ? 'PBS enabled' : 'PBS disabled');
            await fetchStatus();
        } else {
            showToast('Failed', 'error');
        }
    } catch {
        showToast('Connection error', 'error');
    }
}

// ─── Event listeners ──────────────────────────────────────────────────────────
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

// ─── Init ─────────────────────────────────────────────────────────────────────
fetchStatus();
setInterval(fetchStatus, 2000);
