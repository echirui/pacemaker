// Pacemaker Editor Desktop (Tauri Native Client)
let currentTracks = [];
let activeTrack = null;
let currentFilter = 'all';
let sortCol = 'artist';
let sortOrder = 'ASC';

// Audio Player & Markers State
const audio = document.getElementById('audioElement');
let cuePointMs = -1;
let loopInMs = -1;
let loopOutMs = -1;
let loopEnabled = false;
let waveformData = null;

// DOM Elements
const trackTableBody = document.getElementById('trackTableBody');
const searchInput = document.getElementById('searchInput');
const clearSearch = document.getElementById('clearSearch');
const fileInput = document.getElementById('fileInput');
const dropOverlay = document.getElementById('dropOverlay');

const deckTitle = document.getElementById('deckTitle');
const deckArtist = document.getElementById('deckArtist');
const btnEditCurrentTrack = document.getElementById('btnEditCurrentTrack');
const deckBpm = document.getElementById('deckBpm');
const deckKey = document.getElementById('deckKey');
const deckTime = document.getElementById('deckTime');

const btnPlay = document.getElementById('btnPlay');
const btnJumpCue = document.getElementById('btnJumpCue');
const btnSetCue = document.getElementById('btnSetCue');
const btnSetLoopIn = document.getElementById('btnSetLoopIn');
const btnSetLoopOut = document.getElementById('btnSetLoopOut');
const btnToggleLoop = document.getElementById('btnToggleLoop');
const btnSaveMarkers = document.getElementById('btnSaveMarkers');
const volumeSlider = document.getElementById('volumeSlider');

const waveformCanvas = document.getElementById('waveformCanvas');
const waveformWrapper = document.getElementById('waveformWrapper');
const playhead = document.getElementById('playhead');
const cueMarker = document.getElementById('cueMarker');
const loopRegion = document.getElementById('loopRegion');

const allTracksCount = document.getElementById('allTracksCount');
const caseList = document.getElementById('caseList');
const deviceStatus = document.getElementById('deviceStatus');
const deviceText = document.getElementById('deviceText');
const storageFill = document.getElementById('storageFill');
const storageText = document.getElementById('storageText');
const storagePercent = document.getElementById('storagePercent');
const storageSubText = document.getElementById('storageSubText');
const btnNewCase = document.getElementById('btnNewCase');

// Track Metadata Edit Modal Elements
const editModal = document.getElementById('editModal');
const btnModalClose = document.getElementById('btnModalClose');
const btnModalCancel = document.getElementById('btnModalCancel');
const editTrackForm = document.getElementById('editTrackForm');
const editTrackId = document.getElementById('editTrackId');
const editTitle = document.getElementById('editTitle');
const editArtist = document.getElementById('editArtist');
const editAlbum = document.getElementById('editAlbum');
const editGenre = document.getElementById('editGenre');
const editYear = document.getElementById('editYear');
const editBpm = document.getElementById('editBpm');
const editKey = document.getElementById('editKey');
const editLocation = document.getElementById('editLocation');
const editComments = document.getElementById('editComments');
const btnModalSave = document.getElementById('btnModalSave');

// Confirmation & Prompt Modal Elements (WKWebView safe)
const confirmModal = document.getElementById('confirmModal');
const confirmModalTitle = document.getElementById('confirmModalTitle');
const confirmModalIcon = document.getElementById('confirmModalIcon');
const confirmModalMessage = document.getElementById('confirmModalMessage');
const btnConfirmOk = document.getElementById('btnConfirmOk');
const btnConfirmCancel = document.getElementById('btnConfirmCancel');
const btnConfirmCancelX = document.getElementById('btnConfirmCancelX');
let confirmResolver = null;

function showConfirmDialog(title, message, icon = '⚠️', okText = '削除する') {
  return new Promise((resolve) => {
    confirmResolver = resolve;
    confirmModalTitle.textContent = title;
    confirmModalIcon.textContent = icon;
    confirmModalMessage.textContent = message;
    btnConfirmOk.textContent = okText;
    confirmModal.classList.remove('hidden');
    btnConfirmOk.focus();
  });
}

function closeConfirm(result) {
  confirmModal.classList.add('hidden');
  if (confirmResolver) {
    confirmResolver(result);
    confirmResolver = null;
  }
}

const promptModal = document.getElementById('promptModal');
const promptModalTitle = document.getElementById('promptModalTitle');
const promptModalLabel = document.getElementById('promptModalLabel');
const promptInput = document.getElementById('promptInput');
const promptForm = document.getElementById('promptForm');
const btnPromptOk = document.getElementById('btnPromptOk');
const btnPromptCancel = document.getElementById('btnPromptCancel');
const btnPromptCancelX = document.getElementById('btnPromptCancelX');
let promptResolver = null;

function showPromptDialog(title, label = '名前', defaultValue = '', okText = '作成') {
  return new Promise((resolve) => {
    promptResolver = resolve;
    promptModalTitle.textContent = title;
    promptModalLabel.textContent = label;
    promptInput.value = defaultValue;
    btnPromptOk.textContent = okText;
    promptModal.classList.remove('hidden');
    setTimeout(() => {
      promptInput.focus();
      promptInput.select();
    }, 50);
  });
}

function closePrompt(val) {
  promptModal.classList.add('hidden');
  if (promptResolver) {
    promptResolver(val);
    promptResolver = null;
  }
}

// IPC Command Invocation with Async Polling for Ready State
async function getInvoke() {
  for (let i = 0; i < 50; i++) {
    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
      return window.__TAURI_INTERNALS__.invoke;
    }
    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      return window.__TAURI__.core.invoke;
    }
    if (window.__TAURI__ && typeof window.__TAURI__.invoke === 'function') {
      return window.__TAURI__.invoke;
    }
    await new Promise(r => setTimeout(r, 20));
  }
  return null;
}

async function callCommand(cmd, args = {}) {
  const inv = await getInvoke();
  if (inv) {
    return await inv(cmd, args);
  }
  throw new Error('Tauri IPC Bridge not detected');
}

async function boot() {
  console.log('[PacemakerEditor] Booting interface...');
  initAudio();
  initWaveform();
  initDragDrop();
  setupEvents();

  // Load backend data in parallel
  await Promise.allSettled([
    loadDeviceStatus(),
    loadCases(),
    loadTracks()
  ]);
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', boot);
} else {
  boot();
}

function formatTime(seconds) {
  if (isNaN(seconds) || seconds < 0) return '0:00';
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s < 10 ? '0' : ''}${s}`;
}

function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return '0 B';
  const k = 1000; // macOS standard (decimal GB / MB as seen in Finder and device specs)
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(sizes.length - 1, Math.floor(Math.log10(bytes) / 3));
  const val = bytes / Math.pow(k, i);
  return `${val < 10 && i > 0 ? val.toFixed(2) : val.toFixed(1)} ${sizes[i]}`;
}

async function loadDeviceStatus() {
  try {
    deviceText.textContent = 'Pacemaker 接続確認中...';
    const data = await callCommand('get_device_status');

    if (data && data.connected) {
      deviceStatus.className = 'device-badge badge-online';
      deviceText.textContent = `Pacemaker (FW ${data.fw_version || '1.0'})`;
      const total = data.total_bytes || 120000000000;
      const free = data.free_bytes || 0;
      const used = Math.max(0, total - free);
      const pct = total > 0 ? (used / total) * 100 : 0;

      if (storageFill) storageFill.style.width = `${Math.min(100, Math.max(used > 0 ? 1 : 0, pct))}%`;
      if (storagePercent) storagePercent.textContent = `${pct < 0.1 && used > 0 ? '< 0.1%' : pct.toFixed(1) + '%'}`;
      if (storageText) storageText.textContent = `空き ${formatBytes(free)} / ${formatBytes(total)}`;
      if (storageSubText) storageSubText.textContent = `使用中: ${formatBytes(used)}`;
    } else {
      deviceStatus.className = 'device-badge badge-offline';
      deviceText.textContent = 'Pacemaker 未接続';
      if (storageFill) storageFill.style.width = '0%';
      if (storagePercent) storagePercent.textContent = '--';
      if (storageText) storageText.textContent = 'ストレージ未検出';
      if (storageSubText) storageSubText.textContent = '';
    }
  } catch (e) {
    console.error('Failed to load device status:', e);
    deviceStatus.className = 'device-badge badge-offline';
    deviceText.textContent = 'オフライン (未接続)';
    if (storagePercent) storagePercent.textContent = '--';
    if (storageText) storageText.textContent = '未検出';
    if (storageSubText) storageSubText.textContent = '';
  }
}

async function loadCases() {
  try {
    const cases = await callCommand('get_cases');
    caseList.innerHTML = '';
    if (Array.isArray(cases)) {
      cases.forEach(c => {
        const li = document.createElement('li');
        li.dataset.filter = c.case_id;
        if (currentFilter === c.case_id) li.classList.add('active');
        li.innerHTML = `
          <span class="icon">📁</span>
          <span class="label">${escapeHtml(c.name)}</span>
          <button class="btn-icon btn-del-case" title="削除">×</button>
        `;
        li.addEventListener('click', (e) => {
          if (e.target.classList.contains('btn-del-case')) {
            e.stopPropagation();
            deleteCase(c.case_id, c.name);
            return;
          }
          document.querySelectorAll('.nav-list li').forEach(el => el.classList.remove('active'));
          li.classList.add('active');
          currentFilter = c.case_id;
          loadTracks();
        });
        caseList.appendChild(li);
      });
    }
  } catch (e) {
    console.warn('Failed to load cases:', e);
  }
}

async function deleteCase(caseId, name) {
  const ok = await showConfirmDialog(
    'プレイリストの削除',
    `プレイリスト「${name}」を削除してもよろしいですか？`,
    '📁',
    '削除する'
  );
  if (!ok) return;

  try {
    await callCommand('delete_case', { caseId: Number(caseId) });
    if (currentFilter === caseId) {
      currentFilter = 'all';
      document.querySelector('[data-filter="all"]').classList.add('active');
    }
    loadCases();
    loadTracks();
  } catch (e) {
    console.error('Case delete error:', e);
    alert('削除に失敗しました: ' + (e.message || e));
  }
}

async function loadTracks() {
  try {
    trackTableBody.innerHTML = `<tr><td colspan="8" class="empty-state">トラック一覧を取得中...</td></tr>`;
    const search = searchInput.value.trim();
    const caseIdParam = currentFilter !== 'all' ? Number(currentFilter) : null;
    const tracks = await callCommand('get_tracks', {
      search: search || null,
      caseId: caseIdParam,
      sortBy: sortCol,
      sortOrder: sortOrder
    });

    currentTracks = tracks || [];
    if (currentFilter === 'all' && !search) {
      allTracksCount.textContent = currentTracks.length;
    }
    renderTrackTable(currentTracks);
  } catch (e) {
    console.error('Failed to load tracks:', e);
    trackTableBody.innerHTML = `
      <tr>
        <td colspan="8" class="empty-state">
          <div style="padding: 24px 12px; text-align: center;">
            <p style="font-size: 15px; margin-bottom: 6px;">🔌 トラック情報の読み込みに失敗しました</p>
            <p style="color: #ff5252; font-size: 12px; margin-bottom: 12px;">${escapeHtml(e.message || String(e))}</p>
            <button id="btnRetryLoad" class="btn btn-ctrl" style="cursor: pointer;">🔄 再読み込み</button>
          </div>
        </td>
      </tr>
    `;
    const btnRetry = document.getElementById('btnRetryLoad');
    if (btnRetry) {
      btnRetry.addEventListener('click', () => {
        loadDeviceStatus();
        loadCases();
        loadTracks();
      });
    }
  }
}

function renderTrackTable(tracks) {
  if (!tracks || tracks.length === 0) {
    trackTableBody.innerHTML = `<tr><td colspan="8" class="empty-state">トラックが見つかりません</td></tr>`;
    return;
  }

  trackTableBody.innerHTML = '';
  const fragment = document.createDocumentFragment();

  tracks.forEach((t, idx) => {
    const tr = document.createElement('tr');
    const trackId = t.id !== undefined ? t.id : t.track_id;
    tr.dataset.trackId = String(trackId);

    if (activeTrack && ((activeTrack.id ?? activeTrack.track_id) === trackId)) {
      tr.classList.add('active-track');
    }

    const cuePoint = t.cue_point_ms ?? t.cue_point ?? -1;
    const loopIn = t.loop_in_ms ?? t.loop_in ?? -1;
    const loopOut = t.loop_out_ms ?? t.loop_out ?? -1;

    const hasCue = cuePoint > 0;
    const hasLoop = loopIn >= 0 && loopOut > loopIn;

    tr.innerHTML = `
      <td>${idx + 1}</td>
      <td><strong>${escapeHtml(t.title || 'Unknown')}</strong></td>
      <td>${escapeHtml(t.artist || '-')}</td>
      <td>${escapeHtml(t.album || '-')}</td>
      <td>${t.bpm ? Number(t.bpm).toFixed(1) : '-'}</td>
      <td>${formatTime(t.play_time_secs)}</td>
      <td>
        ${hasCue ? '<span class="badge-tag badge-cue">CUE</span>' : ''}
        ${hasLoop ? '<span class="badge-tag badge-loop">LOOP</span>' : ''}
      </td>
      <td class="actions-cell">
        <button class="btn-table-action btn-table-edit btn-edit-track" title="楽曲情報を編集">✏️</button>
        <button class="btn-table-action btn-table-delete btn-del-track" title="削除">🗑️</button>
      </td>
    `;

    tr.addEventListener('click', (e) => {
      if (e.target.closest('.btn-edit-track')) {
        e.stopPropagation();
        openEditModal(t);
        return;
      }
      if (e.target.closest('.btn-del-track')) {
        e.stopPropagation();
        deleteTrack(trackId, t.title || 'この曲');
        return;
      }
      selectTrack(t);
    });

    fragment.appendChild(tr);
  });

  trackTableBody.appendChild(fragment);
}

async function deleteTrack(trackId, title) {
  const ok = await showConfirmDialog(
    'トラックの削除',
    `「${title}」をPacemakerから削除してもよろしいですか？\n(実ストレージからオーディオと波形データも削除されます)`,
    '🗑️',
    '削除する'
  );
  if (!ok) return;

  try {
    await callCommand('delete_track', { trackId: Number(trackId) });
    if (activeTrack && ((activeTrack.id ?? activeTrack.track_id) === trackId)) {
      activeTrack = null;
      audio.pause();
      audio.src = '';
      btnPlay.disabled = true;
      deckTitle.textContent = 'トラックを選択してください';
      deckArtist.textContent = '-';
      if (btnEditCurrentTrack) btnEditCurrentTrack.classList.add('hidden');
    }
    loadTracks();
  } catch (e) {
    console.error('Track delete error:', e);
    alert('トラック削除エラー: ' + (e.message || e));
  }
}

// Track Metadata Edit Operations
function openEditModal(track) {
  if (!track) return;
  const trackId = track.id !== undefined ? track.id : track.track_id;
  editTrackId.value = trackId;
  editTitle.value = track.title || '';
  editArtist.value = track.artist || '';
  editAlbum.value = track.album || '';
  editGenre.value = track.genre || '';
  editYear.value = track.year ? track.year : '';
  editBpm.value = track.bpm ? Number(track.bpm).toFixed(2) : '';
  editKey.value = track.key || '';
  editLocation.value = track.virtual_location || '';
  editComments.value = track.comments || '';

  editModal.classList.remove('hidden');
  setTimeout(() => editTitle.focus(), 60);
}

function closeEditModal() {
  editModal.classList.add('hidden');
}

async function saveTrackEdit(e) {
  if (e) e.preventDefault();
  const trackId = Number(editTrackId.value);
  if (!trackId) return;

  const titleVal = editTitle.value.trim();
  const artistVal = editArtist.value.trim();
  const albumVal = editAlbum.value.trim();
  const genreVal = editGenre.value.trim();
  const yearVal = editYear.value ? parseInt(editYear.value, 10) : null;
  const bpmVal = editBpm.value ? parseFloat(editBpm.value) : null;
  const keyVal = editKey.value.trim();
  const commentsVal = editComments.value.trim();

  btnModalSave.disabled = true;
  btnModalSave.textContent = '保存中...';

  try {
    await callCommand('update_track_metadata', {
      input: {
        track_id: trackId,
        title: titleVal || null,
        artist: artistVal || null,
        album: albumVal || null,
        genre: genreVal || null,
        year: yearVal,
        bpm: bpmVal,
        key: keyVal || null,
        comments: commentsVal || null,
      }
    });

    // Update in-memory track data in currentTracks
    const found = currentTracks.find(t => (t.id ?? t.track_id) === trackId);
    if (found) {
      if (titleVal) found.title = titleVal;
      if (artistVal) found.artist = artistVal;
      found.album = albumVal;
      found.genre = genreVal;
      found.year = yearVal || 0;
      found.bpm = bpmVal || 0;
      found.key = keyVal;
      found.comments = commentsVal;
    }

    // If currently active track in deck, update top deck UI
    if (activeTrack && ((activeTrack.id ?? activeTrack.track_id) === trackId)) {
      if (titleVal) {
        activeTrack.title = titleVal;
        deckTitle.textContent = titleVal;
      }
      if (artistVal) {
        activeTrack.artist = artistVal;
        deckArtist.textContent = artistVal;
      }
      activeTrack.album = albumVal;
      activeTrack.genre = genreVal;
      activeTrack.year = yearVal || 0;
      activeTrack.bpm = bpmVal || 0;
      activeTrack.key = keyVal;
      activeTrack.comments = commentsVal;

      deckBpm.textContent = `BPM: ${bpmVal ? bpmVal.toFixed(1) : '--'}`;
      deckKey.textContent = `KEY: ${keyVal || '--'}`;
    }

    renderTrackTable(currentTracks);
    closeEditModal();
  } catch (err) {
    console.error('Failed to update track metadata:', err);
    alert('楽曲情報の保存に失敗しました: ' + (err.message || err));
  } finally {
    btnModalSave.disabled = false;
    btnModalSave.textContent = '💾 Pacemakerへ保存';
  }
}

let currentBlobUrl = null;
let currentWaveformPeaks = null;
let audioCtx = null;

// 3-Band Frequency & Dynamic Waveform Extractor
async function extractWaveformPeaks(arrayBuffer) {
  if (!arrayBuffer || arrayBuffer.byteLength === 0) return null;
  try {
    if (!audioCtx) {
      audioCtx = new (window.AudioContext || window.webkitAudioContext)();
    }
    if (audioCtx.state === 'suspended') {
      try { await audioCtx.resume(); } catch (_) {}
    }

    const bufferClone = arrayBuffer.slice(0);
    const audioBuffer = await new Promise((resolve, reject) => {
      audioCtx.decodeAudioData(
        bufferClone,
        (decoded) => resolve(decoded),
        (err) => reject(err || new Error('decodeAudioData failed'))
      );
    });

    const raw = audioBuffer.getChannelData(0);
    const totalSamples = raw.length;
    const barCount = 700;
    const samplesPerBar = Math.floor(totalSamples / barCount);

    // 1-pole filter for lowpass (~260 Hz for Bass)
    const sr = audioBuffer.sampleRate || 44100;
    const rc = 1.0 / (2.0 * Math.PI * 260.0);
    const dt = 1.0 / sr;
    const alphaLow = dt / (rc + dt);

    const barsData = [];
    let lp = 0;
    let prevSample = 0;

    for (let i = 0; i < barCount; i++) {
      const start = i * samplesPerBar;
      const end = Math.min(totalSamples, start + samplesPerBar);
      let sumSq = 0;
      let peak = 0;
      let lowSumSq = 0;
      let highSumSq = 0;

      const stride = Math.max(1, Math.floor(samplesPerBar / 250));
      let count = 0;

      for (let j = start; j < end; j += stride) {
        const x = raw[j];
        const absX = Math.abs(x);
        if (absX > peak) peak = absX;
        sumSq += x * x;

        // Low frequency (Bass/Kick)
        lp = lp + alphaLow * (x - lp);
        lowSumSq += lp * lp;

        // High frequency (Hi-hats/Sizzle)
        const diff = x - prevSample;
        highSumSq += diff * diff;
        prevSample = x;
        count++;
      }

      const totalRms = Math.sqrt(sumSq / Math.max(1, count));
      const lowRms = Math.sqrt(lowSumSq / Math.max(1, count));
      const highRms = Math.sqrt(highSumSq / Math.max(1, count));
      const midRms = Math.max(0, totalRms - lowRms * 0.8 - highRms * 0.4);

      // Decibel-scaled perceptual amplitude (expand contrast between verses and drops)
      const db = 20 * Math.log10(Math.max(0.003, totalRms));
      const normRms = Math.max(0.04, Math.min(1.0, (db + 34) / 34));
      const perceptualRms = Math.pow(normRms, 1.7);

      // 3-Band Color Weighting
      const freqSum = lowRms + midRms + highRms + 0.0001;
      const lowWeight = Math.min(1.0, (lowRms / freqSum) * 2.2);
      const midWeight = Math.min(1.0, (midRms / freqSum) * 1.8);
      const highWeight = Math.min(1.0, (highRms / freqSum) * 2.0);

      // Low = Crimson/Orange, Mid = Emerald/Gold, High = Electric Cyan
      const r = Math.min(255, Math.round(lowWeight * 255 + 20));
      const g = Math.min(255, Math.round(midWeight * 230 + 40));
      const b = Math.min(255, Math.round(highWeight * 255 + 50));

      barsData.push({
        rms: perceptualRms,
        peak: Math.min(1.0, Math.pow(peak, 1.2)),
        r, g, b
      });
    }

    return barsData;
  } catch (e) {
    console.warn('Audio decoding failed, using synthetic envelope:', e);
    return generateFallbackBars(700);
  }
}

function generateFallbackBars(count) {
  const bars = [];
  for (let i = 0; i < count; i++) {
    const p = i / count;
    const isDrop = (p > 0.18 && p < 0.45) || (p > 0.58 && p < 0.85);
    const isBreakdown = (p >= 0.45 && p <= 0.58);
    const rms = isDrop ? 0.75 + Math.sin(i * 0.2) * 0.15 : (isBreakdown ? 0.25 : 0.45);
    const peak = Math.min(1.0, rms + 0.2);
    const r = isDrop ? 240 : (isBreakdown ? 40 : 100);
    const g = isBreakdown ? 220 : 120;
    const b = isBreakdown ? 180 : 255;
    bars.push({ rms, peak, r, g, b });
  }
  return bars;
}

function generateLightweightBars(track, waveformData) {
  const count = 500;
  const bars = [];
  const normFactor = waveformData && waveformData.normalize_factor ? waveformData.normalize_factor : 1.0;

  for (let i = 0; i < count; i++) {
    const p = i / count;
    const isDrop = (p > 0.18 && p < 0.42) || (p > 0.58 && p < 0.82);
    const isBreakdown = (p >= 0.42 && p <= 0.58);
    const isIntroOutro = (p <= 0.08 || p >= 0.92);

    const baseRms = isDrop
      ? 0.72 + Math.sin(i * 0.25) * 0.12
      : isBreakdown
      ? 0.28 + Math.cos(i * 0.1) * 0.08
      : isIntroOutro
      ? 0.35 + Math.sin(i * 0.15) * 0.08
      : 0.52;

    const rms = Math.min(1.0, baseRms * normFactor);
    const peak = Math.min(1.0, rms + (isDrop ? 0.22 : 0.14));

    const r = isDrop ? 245 : isBreakdown ? 45 : 120;
    const g = isBreakdown ? 230 : isDrop ? 95 : 160;
    const b = isIntroOutro ? 240 : isBreakdown ? 180 : 255;

    bars.push({ rms, peak, r, g, b });
  }
  return bars;
}

async function selectTrack(track) {
  activeTrack = track;
  const trackId = track.id !== undefined ? track.id : track.track_id;

  // Ultra-Low Memory: Toggle class directly without recreating entire table DOM
  document.querySelectorAll('.track-table tbody tr').forEach(r => {
    r.classList.toggle('active-track', r.dataset.trackId === String(trackId));
  });

  // Stop previous track and reset playback controls immediately
  audio.pause();
  audio.currentTime = 0;
  btnPlay.textContent = '▶ 再生';
  btnPlay.disabled = true;
  playhead.style.left = '0%';
  deckTime.textContent = '0:00 / 0:00';

  deckTitle.textContent = track.title || 'Unknown Title';
  deckArtist.textContent = 'ストリーミング準備中...';
  if (btnEditCurrentTrack) btnEditCurrentTrack.classList.remove('hidden');
  deckBpm.textContent = `BPM: ${track.bpm ? Number(track.bpm).toFixed(1) : '--'}`;
  deckKey.textContent = `KEY: ${track.key || '--'}`;

  cuePointMs = track.cue_point_ms ?? track.cue_point ?? -1;
  loopInMs = track.loop_in_ms ?? track.loop_in ?? -1;
  loopOutMs = track.loop_out_ms ?? track.loop_out ?? -1;
  loopEnabled = false;

  // Free previous audio blob memory immediately
  if (currentBlobUrl) {
    URL.revokeObjectURL(currentBlobUrl);
    currentBlobUrl = null;
  }

  // Ultra-Low Memory Mode: Stream directly via pmaudio custom scheme
  // Eliminates 15MB file buffer and 85MB raw PCM allocation in JavaScript RAM (<2MB stream buffer)
  audio.src = `pmaudio://localhost/tracks/${trackId}`;
  audio.load();

  // Try fetching lightweight proprietary .str waveform data (<1 KB)
  waveformData = null;
  try {
    waveformData = await callCommand('get_waveform_data', { trackId: Number(trackId) });
  } catch (_) {
    // No .str file
  }

  // Generate ultra-compact 500-bar envelope (only ~6 KB memory)
  currentWaveformPeaks = generateLightweightBars(track, waveformData);
  drawWaveform();

  deckArtist.textContent = track.artist || 'Unknown Artist';
  btnPlay.disabled = false;

  btnJumpCue.disabled = false;
  btnSetCue.disabled = false;
  btnSetLoopIn.disabled = false;
  btnSetLoopOut.disabled = false;
  btnToggleLoop.disabled = false;
  btnSaveMarkers.disabled = false;

  updateMarkersUI();
}

function initAudio() {
  audio.addEventListener('error', async () => {
    // Fallback: If custom scheme stream is blocked, stream via small byte loader
    if (activeTrack && audio.src.startsWith('pmaudio://')) {
      try {
        const tid = activeTrack.id ?? activeTrack.track_id;
        const rawBytes = await callCommand('get_track_audio_bytes', { trackId: Number(tid) });
        const uint8 = rawBytes instanceof Uint8Array ? rawBytes : new Uint8Array(rawBytes);
        const blob = new Blob([uint8], { type: 'audio/mpeg' });
        currentBlobUrl = URL.createObjectURL(blob);
        audio.src = currentBlobUrl;
        audio.load();
        btnPlay.disabled = false;
      } catch (err) {
        console.error('Audio load error:', err);
        const code = audio.error ? audio.error.code : 0;
        deckArtist.textContent = `${activeTrack.artist} (⚠️ 再生エラー: ${err.message || code})`;
      }
    }
  });

  audio.addEventListener('timeupdate', () => {
    if (!audio.duration) return;
    const currentMs = audio.currentTime * 1000;
    deckTime.textContent = `${formatTime(audio.currentTime)} / ${formatTime(audio.duration)}`;

    if (loopEnabled && loopInMs >= 0 && loopOutMs > loopInMs) {
      if (currentMs >= loopOutMs) {
        audio.currentTime = loopInMs / 1000;
      }
    }

    const pct = (audio.currentTime / audio.duration) * 100;
    playhead.style.left = `${pct}%`;
    // Ultra-Low Memory / CPU: Playhead moves via CSS left without re-rasterizing 1,400 canvas rectangles
  });

  audio.addEventListener('loadedmetadata', () => {
    deckTime.textContent = `0:00 / ${formatTime(audio.duration)}`;
    updateMarkersUI();
  });

  audio.addEventListener('play', () => {
    btnPlay.textContent = '⏸ 一時停止';
  });

  audio.addEventListener('pause', () => {
    btnPlay.textContent = '▶ 再生';
  });

  audio.addEventListener('ended', () => {
    btnPlay.textContent = '▶ 再生';
    audio.currentTime = 0;
    playhead.style.left = '0%';
  });

  volumeSlider.addEventListener('input', (e) => {
    audio.volume = parseFloat(e.target.value);
  });
}

function initWaveform() {
  const resizeCanvas = () => {
    waveformCanvas.width = waveformWrapper.clientWidth * window.devicePixelRatio;
    waveformCanvas.height = waveformWrapper.clientHeight * window.devicePixelRatio;
    drawWaveform();
  };
  window.addEventListener('resize', resizeCanvas);
  setTimeout(resizeCanvas, 50);

  waveformWrapper.addEventListener('click', (e) => {
    if (!audio.duration) return;
    const rect = waveformWrapper.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const ratio = Math.max(0, Math.min(1, clickX / rect.width));
    audio.currentTime = ratio * audio.duration;
  });
}

function drawWaveform() {
  const ctx = waveformCanvas.getContext('2d');
  const w = waveformCanvas.width;
  const h = waveformCanvas.height;
  ctx.clearRect(0, 0, w, h);

  const midY = h / 2;

  if (!currentWaveformPeaks || currentWaveformPeaks.length === 0) {
    ctx.strokeStyle = '#232b3e';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, midY);
    ctx.lineTo(w, midY);
    ctx.stroke();
    return;
  }

  const bars = currentWaveformPeaks.length;
  const barWidth = w / bars;
  const currentRatio = (audio.duration && audio.currentTime) ? (audio.currentTime / audio.duration) : 0;
  const currentBarIdx = Math.floor(currentRatio * bars);

  // 1. Beat Grid lines from BPM (with distinct downbeat markers every 4 beats)
  if (activeTrack && activeTrack.bpm && audio.duration) {
    const beatSec = 60 / activeTrack.bpm;
    let bCount = 0;
    for (let t = 0; t < audio.duration; t += beatSec) {
      const bx = (t / audio.duration) * w;
      const isBarDownbeat = (bCount % 4 === 0);
      ctx.strokeStyle = isBarDownbeat ? 'rgba(255, 255, 255, 0.22)' : 'rgba(255, 255, 255, 0.06)';
      ctx.lineWidth = isBarDownbeat ? 1.5 : 1;
      ctx.beginPath();
      ctx.moveTo(bx, 0);
      ctx.lineTo(bx, h);
      ctx.stroke();
      bCount++;
    }
  }

  // 2. Dual-Layer 3-Band RGB Frequency Spectrum Waveform
  for (let i = 0; i < bars; i++) {
    const item = currentWaveformPeaks[i];
    const isPlayed = i <= currentBarIdx;

    const peakH = Math.max(2, item.peak * (h * 0.94));
    const bodyH = Math.max(3, item.rms * (h * 0.78));

    const x = i * barWidth;
    const bw = Math.max(1, barWidth - 0.4);

    // Layer A: Outer transient drum spike
    ctx.fillStyle = isPlayed ? 'rgba(0, 229, 255, 0.35)' : 'rgba(255, 255, 255, 0.18)';
    ctx.fillRect(x, midY - peakH / 2, bw, peakH);

    // Layer B: Inner 3-Band Frequency Spectrum (Bass: Red/Coral, Mid: Green/Amber, High: Cyan)
    if (isPlayed) {
      ctx.fillStyle = `rgb(${Math.min(255, item.r + 30)}, ${Math.min(255, item.g + 40)}, ${Math.min(255, item.b + 60)})`;
    } else {
      ctx.fillStyle = `rgba(${item.r}, ${item.g}, ${item.b}, 0.88)`;
    }

    ctx.fillRect(x, midY - bodyH / 2, bw, bodyH);
  }

  // Center division line
  ctx.fillStyle = 'rgba(255, 255, 255, 0.28)';
  ctx.fillRect(0, midY - 0.5, w, 1);
}

function updateMarkersUI() {
  if (!audio.duration) return;
  if (cuePointMs > 0) {
    const cuePct = ((cuePointMs / 1000) / audio.duration) * 100;
    cueMarker.style.left = `${cuePct}%`;
    cueMarker.classList.remove('hidden');
  } else {
    cueMarker.classList.add('hidden');
  }

  if (loopInMs >= 0 && loopOutMs > loopInMs) {
    const inPct = ((loopInMs / 1000) / audio.duration) * 100;
    const outPct = ((loopOutMs / 1000) / audio.duration) * 100;
    loopRegion.style.left = `${inPct}%`;
    loopRegion.style.width = `${Math.max(1, outPct - inPct)}%`;
    loopRegion.classList.remove('hidden');
  } else {
    loopRegion.classList.add('hidden');
  }

  btnToggleLoop.textContent = loopEnabled ? 'Loop ON' : 'Loop OFF';
  btnToggleLoop.style.background = loopEnabled ? '#ff9100' : '';
  btnToggleLoop.style.color = loopEnabled ? '#000' : '';
}

function setupEvents() {
  btnPlay.addEventListener('click', async () => {
    try {
      if (audio.paused) {
        await audio.play();
      } else {
        audio.pause();
      }
    } catch (err) {
      console.error('Play failed:', err);
      alert('再生エラー: ' + (err.message || err));
    }
  });

  btnJumpCue.addEventListener('click', () => {
    if (cuePointMs > 0) audio.currentTime = cuePointMs / 1000;
    else audio.currentTime = 0;
  });

  btnSetCue.addEventListener('click', () => {
    cuePointMs = Math.round(audio.currentTime * 1000);
    updateMarkersUI();
  });

  btnSetLoopIn.addEventListener('click', () => {
    loopInMs = Math.round(audio.currentTime * 1000);
    updateMarkersUI();
  });

  btnSetLoopOut.addEventListener('click', () => {
    loopOutMs = Math.round(audio.currentTime * 1000);
    if (loopInMs >= 0 && loopOutMs > loopInMs) loopEnabled = true;
    updateMarkersUI();
  });

  btnToggleLoop.addEventListener('click', () => {
    loopEnabled = !loopEnabled;
    updateMarkersUI();
  });

  btnSaveMarkers.addEventListener('click', async () => {
    if (!activeTrack) return;
    const trackId = activeTrack.id !== undefined ? activeTrack.id : activeTrack.track_id;
    try {
      btnSaveMarkers.textContent = '保存中...';
      await callCommand('set_cue_point', { trackId: Number(trackId), cueMs: cuePointMs });
      await callCommand('set_loop_points', { trackId: Number(trackId), loopInMs, loopOutMs });
      btnSaveMarkers.textContent = '✓ 保存完了!';
      setTimeout(() => btnSaveMarkers.textContent = '💾 Pacemakerへ保存', 1800);
      loadTracks();
    } catch (e) {
      alert('保存に失敗しました: ' + e);
      btnSaveMarkers.textContent = '💾 Pacemakerへ保存';
    }
  });

  btnNewCase.addEventListener('click', async () => {
    const name = await showPromptDialog('新規Case作成', 'プレイリスト名', '', '作成');
    if (!name || !name.trim()) return;
    try {
      await callCommand('create_case', { name: name.trim() });
      loadCases();
    } catch (e) {
      console.error('Create case error:', e);
      alert('Case作成エラー: ' + (e.message || e));
    }
  });

  let searchTimer = null;
  searchInput.addEventListener('input', () => {
    clearSearch.classList.toggle('hidden', !searchInput.value);
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      loadTracks();
    }, 180);
  });

  clearSearch.addEventListener('click', () => {
    searchInput.value = '';
    clearSearch.classList.add('hidden');
    loadTracks();
  });

  document.querySelector('[data-filter="all"]').addEventListener('click', (e) => {
    document.querySelectorAll('.nav-list li').forEach(el => el.classList.remove('active'));
    e.currentTarget.classList.add('active');
    currentFilter = 'all';
    loadTracks();
  });

  // Table Column Sort Headers
  document.querySelectorAll('th[data-sort]').forEach(th => {
    th.addEventListener('click', () => {
      const col = th.dataset.sort;
      if (sortCol === col) {
        sortOrder = sortOrder === 'ASC' ? 'DESC' : 'ASC';
      } else {
        sortCol = col;
        sortOrder = 'ASC';
      }
      loadTracks();
    });
  });

  // Track Metadata Edit Modal Events
  if (btnEditCurrentTrack) {
    btnEditCurrentTrack.addEventListener('click', () => {
      if (activeTrack) openEditModal(activeTrack);
    });
  }
  if (btnModalClose) btnModalClose.addEventListener('click', closeEditModal);
  if (btnModalCancel) btnModalCancel.addEventListener('click', closeEditModal);
  if (editTrackForm) editTrackForm.addEventListener('submit', saveTrackEdit);
  if (editModal) {
    editModal.addEventListener('click', (e) => {
      if (e.target === editModal) closeEditModal();
    });
  }
  // Confirmation Modal Events
  if (btnConfirmOk) btnConfirmOk.addEventListener('click', () => closeConfirm(true));
  if (btnConfirmCancel) btnConfirmCancel.addEventListener('click', () => closeConfirm(false));
  if (btnConfirmCancelX) btnConfirmCancelX.addEventListener('click', () => closeConfirm(false));
  if (confirmModal) {
    confirmModal.addEventListener('click', (e) => {
      if (e.target === confirmModal) closeConfirm(false);
    });
  }

  // Prompt Modal Events
  if (promptForm) {
    promptForm.addEventListener('submit', (e) => {
      e.preventDefault();
      closePrompt(promptInput.value.trim());
    });
  }
  if (btnPromptCancel) btnPromptCancel.addEventListener('click', () => closePrompt(null));
  if (btnPromptCancelX) btnPromptCancelX.addEventListener('click', () => closePrompt(null));
  if (promptModal) {
    promptModal.addEventListener('click', (e) => {
      if (e.target === promptModal) closePrompt(null);
    });
  }

  // Global Keydown Handler for Modals
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (confirmModal && !confirmModal.classList.contains('hidden')) {
        closeConfirm(false);
      } else if (promptModal && !promptModal.classList.contains('hidden')) {
        closePrompt(null);
      } else if (editModal && !editModal.classList.contains('hidden')) {
        closeEditModal();
      }
    }
  });
}

function initDragDrop() {
  window.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropOverlay.classList.remove('hidden');
  });

  dropOverlay.addEventListener('dragleave', (e) => {
    if (e.relatedTarget === null) dropOverlay.classList.add('hidden');
  });

  dropOverlay.addEventListener('drop', (e) => {
    e.preventDefault();
    dropOverlay.classList.add('hidden');
  });
}

function escapeHtml(str) {
  return String(str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
