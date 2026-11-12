// Pacemaker Editor Desktop (Tauri Native Client)
let currentTracks = [];
let currentCases = [];
let activeTrack = null;
let currentFilter = 'all';
let sortCol = 'artist';
let sortOrder = 'ASC';

// Audio Player & Markers State
const audio = document.getElementById('audioElement');
let currentTrackLoadId = 0;
let currentBlobUrl = null;
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

// YouTube / YouTube Music Modal Elements
const btnOpenYtModal = document.getElementById('btnOpenYtModal');
const ytModal = document.getElementById('ytModal');
const btnYtModalClose = document.getElementById('btnYtModalClose');
const btnYtModalCloseBottom = document.getElementById('btnYtModalCloseBottom');
const ytSearchForm = document.getElementById('ytSearchForm');
const ytSearchInput = document.getElementById('ytSearchInput');
const btnYtSearch = document.getElementById('btnYtSearch');
const ytTargetCaseSelect = document.getElementById('ytTargetCaseSelect');
const ytResultsContainer = document.getElementById('ytResultsContainer');
const ytSearchResultsCount = document.getElementById('ytSearchResultsCount');

// Comparison Split-View Elements in YouTube Modal
const compareMatchBanner = document.getElementById('compareMatchBanner');
const compareMatchIcon = document.getElementById('compareMatchIcon');
const compareMatchText = document.getElementById('compareMatchText');
const leftTracksCountBadge = document.getElementById('leftTracksCountBadge');
const btnSortLeftTitle = document.getElementById('btnSortLeftTitle');
const btnSortLeftArtist = document.getElementById('btnSortLeftArtist');
const filterLeftInput = document.getElementById('filterLeftInput');
const leftTracksList = document.getElementById('leftTracksList');
const rightTracksCountBadge = document.getElementById('rightTracksCountBadge');
const btnSortRightRelevance = document.getElementById('btnSortRightRelevance');
const btnSortRightTitle = document.getElementById('btnSortRightTitle');
const btnSortRightArtist = document.getElementById('btnSortRightArtist');

// Comparison Split-View State
let leftSortKey = 'title'; // 'title' | 'artist'
let leftSortAsc = true;
let filterLeftQuery = '';
let currentYtResults = [];
let rightSortKey = 'relevance'; // 'relevance' | 'title' | 'artist'
let rightSortAsc = true;
let selectedRightTrack = null;
let lastAddedTrackTitle = null;

// YouTube Dedicated Auth Modal Elements
const btnOpenYtAuthModal = document.getElementById('btnOpenYtAuthModal');
const navOpenYtLogin = document.getElementById('navOpenYtLogin');
const btnOpenAuthFromImport = document.getElementById('btnOpenAuthFromImport');
const ytAuthModal = document.getElementById('ytAuthModal');
const btnYtAuthModalClose = document.getElementById('btnYtAuthModalClose');
const btnYtAuthModalCancel = document.getElementById('btnYtAuthModalCancel');
const btnModalClearAuth = document.getElementById('btnModalClearAuth');

// Badges & Labels
const headerAuthIcon = document.getElementById('headerAuthIcon');
const headerAuthText = document.getElementById('headerAuthText');
const headerAuthBadge = document.getElementById('headerAuthBadge');
const navAuthBadge = document.getElementById('navAuthBadge');
const ytImportAuthBadge = document.getElementById('ytImportAuthBadge');
const ytImportAuthDetail = document.getElementById('ytImportAuthDetail');
const modalAuthBadge = document.getElementById('modalAuthBadge');
const modalAuthDetail = document.getElementById('modalAuthDetail');

const cookieDropArea = document.getElementById('cookieDropArea');
const cookieDropLabel = document.getElementById('cookieDropLabel');
const cookieFileInput = document.getElementById('cookieFileInput');
const cookiePasteArea = document.getElementById('cookiePasteArea');
const btnSaveCookies = document.getElementById('btnSaveCookies');
const btnTestAuth = document.getElementById('btnTestAuth');
const ytBrowserSelect = document.getElementById('ytBrowserSelect');
const btnTestBrowser = document.getElementById('btnTestBrowser');
const btnLinkYtMusic = document.getElementById('btnLinkYtMusic');
const btnOpenBrowserYt = document.getElementById('btnOpenBrowserYt');
const ytAuthMsg = document.getElementById('ytAuthMsg');
let currentAuthStatus = null;

function openYtAuthModal() {
  if (ytAuthMsg) ytAuthMsg.classList.add('hidden');
  ytAuthModal.classList.remove('hidden');
  loadYoutubeAuthStatus();
}

function closeYtAuthModal() {
  ytAuthModal.classList.add('hidden');
}

async function loadYoutubeAuthStatus() {
  try {
    const status = await callCommand('get_youtube_auth_status');
    currentAuthStatus = status;
    const isLogged = status && status.is_logged_in;
    const badgeText = isLogged ? 'ログイン中' : '未ログイン';
    const badgeClass = isLogged ? 'nav-badge active' : 'nav-badge guest';
    const bannerBadgeClass = isLogged ? 'yt-auth-badge active' : 'yt-auth-badge guest';
    const bannerBadgeText = isLogged ? '🟢 ログイン中 (YouTube Music)' : '⚪ 未ログイン (ゲストモード)';
    const detailText = isLogged
      ? (status.details || 'ログインセッション有効 (非公開プレイリスト・高音質対応)')
      : '非公開プレイリストや高音質音源を取得するにはログインしてください';

    if (headerAuthBadge) {
      headerAuthBadge.className = badgeClass;
      headerAuthBadge.textContent = badgeText;
    }
    if (headerAuthIcon) {
      headerAuthIcon.textContent = isLogged ? '🟢' : '🔐';
    }
    if (headerAuthText) {
      headerAuthText.textContent = isLogged ? 'YouTube ログイン中' : 'YouTube ログイン設定';
    }
    if (navAuthBadge) {
      navAuthBadge.className = badgeClass;
      navAuthBadge.textContent = badgeText;
    }
    if (ytImportAuthBadge) {
      ytImportAuthBadge.className = bannerBadgeClass;
      ytImportAuthBadge.textContent = bannerBadgeText;
    }
    if (ytImportAuthDetail) {
      ytImportAuthDetail.textContent = detailText;
    }
    if (modalAuthBadge) {
      modalAuthBadge.className = bannerBadgeClass;
      modalAuthBadge.textContent = bannerBadgeText;
    }
    if (modalAuthDetail) {
      modalAuthDetail.textContent = detailText;
    }
    if (btnModalClearAuth) {
      btnModalClearAuth.classList.toggle('hidden', !isLogged);
    }
  } catch (err) {
    console.error('Failed to load YouTube auth status:', err);
  }
}

function showAuthFeedback(msg, isSuccess = true) {
  if (!ytAuthMsg) return;
  ytAuthMsg.className = `yt-auth-feedback ${isSuccess ? 'success' : 'error'}`;
  ytAuthMsg.textContent = msg;
  ytAuthMsg.classList.remove('hidden');
}

function initYoutubeAuthHandlers() {
  // Open / Close Auth Modal
  if (btnOpenYtAuthModal) btnOpenYtAuthModal.addEventListener('click', openYtAuthModal);
  if (navOpenYtLogin) navOpenYtLogin.addEventListener('click', openYtAuthModal);
  if (btnOpenAuthFromImport) {
    btnOpenAuthFromImport.addEventListener('click', () => {
      closeYtModal();
      openYtAuthModal();
    });
  }
  if (btnYtAuthModalClose) btnYtAuthModalClose.addEventListener('click', closeYtAuthModal);
  if (btnYtAuthModalCancel) btnYtAuthModalCancel.addEventListener('click', closeYtAuthModal);
  if (ytAuthModal) {
    ytAuthModal.addEventListener('click', (e) => {
      if (e.target === ytAuthModal) closeYtAuthModal();
    });
  }

  // Auth Tabs
  const authTabs = document.querySelectorAll('.yt-auth-tab');
  authTabs.forEach(tab => {
    tab.addEventListener('click', () => {
      authTabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      const targetId = tab.getAttribute('data-tab');
      document.querySelectorAll('.yt-tab-pane').forEach(pane => {
        pane.classList.toggle('hidden', pane.id !== targetId);
      });
      if (ytAuthMsg) ytAuthMsg.classList.add('hidden');
    });
  });

  // Cookie File input
  if (cookieFileInput) {
    cookieFileInput.addEventListener('change', async (e) => {
      const file = e.target.files && e.target.files[0];
      if (!file) return;
      try {
        const text = await file.text();
        cookiePasteArea.value = text;
        if (cookieDropLabel) cookieDropLabel.textContent = `選択中: ${file.name} (${(file.size / 1024).toFixed(1)} KB)`;
        await handleSaveCookieText(text);
      } catch (err) {
        showAuthFeedback(`ファイルの読み込みに失敗しました: ${err.message}`, false);
      }
    });
  }

  // Cookie Drop Area drag & drop
  if (cookieDropArea) {
    cookieDropArea.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.stopPropagation();
      cookieDropArea.classList.add('dragover');
    });
    cookieDropArea.addEventListener('dragleave', (e) => {
      e.preventDefault();
      e.stopPropagation();
      cookieDropArea.classList.remove('dragover');
    });
    cookieDropArea.addEventListener('drop', async (e) => {
      e.preventDefault();
      e.stopPropagation();
      cookieDropArea.classList.remove('dragover');
      const file = e.dataTransfer.files && e.dataTransfer.files[0];
      if (file) {
        try {
          const text = await file.text();
          cookiePasteArea.value = text;
          if (cookieDropLabel) cookieDropLabel.textContent = `読込: ${file.name} (${(file.size / 1024).toFixed(1)} KB)`;
          await handleSaveCookieText(text);
        } catch (err) {
          showAuthFeedback(`ドラッグされたファイルの読込に失敗しました: ${err.message}`, false);
        }
      }
    });
  }

  // Save Cookie Text
  async function handleSaveCookieText(text) {
    const val = (text || cookiePasteArea.value || '').trim();
    if (!val) {
      showAuthFeedback('Cookieの内容を入力または指定してください。', false);
      return;
    }
    showAuthFeedback('Cookieを保存して検証中...', true);
    try {
      await callCommand('save_youtube_cookies', { content: val });
      await loadYoutubeAuthStatus();
      showAuthFeedback('✓ Cookieを保存しました！YouTube Music ログインが有効化されました。', true);
    } catch (err) {
      showAuthFeedback(`保存失敗: ${err.message || String(err)}`, false);
    }
  }

  if (btnSaveCookies) {
    btnSaveCookies.addEventListener('click', () => handleSaveCookieText());
  }

  // Clear Cookies (Logout)
  if (btnModalClearAuth) {
    btnModalClearAuth.addEventListener('click', async () => {
      try {
        await callCommand('clear_youtube_cookies');
        if (cookiePasteArea) cookiePasteArea.value = '';
        if (cookieDropLabel) cookieDropLabel.textContent = 'cookies.txt をここにドラッグ＆ドロップ';
        await loadYoutubeAuthStatus();
        showAuthFeedback('ログアウトしました (ゲストモードに戻りました)。', true);
      } catch (err) {
        showAuthFeedback(`ログアウト処理失敗: ${err.message || String(err)}`, false);
      }
    });
  }

  // Test Authentication (Cookie file)
  if (btnTestAuth) {
    btnTestAuth.addEventListener('click', async () => {
      showAuthFeedback('⏳ YouTube Musicとの接続およびログイン検証中...', true);
      try {
        const res = await callCommand('test_youtube_auth', { method: 'cookies_file' });
        if (res.success) {
          showAuthFeedback(res.message, true);
        } else {
          showAuthFeedback(res.message, false);
        }
      } catch (err) {
        showAuthFeedback(`検証エラー: ${err.message || String(err)}`, false);
      }
    });
  }

  // Test Browser Authentication
  if (btnTestBrowser) {
    btnTestBrowser.addEventListener('click', async () => {
      const b = (ytBrowserSelect && ytBrowserSelect.value) || 'chrome';
      showAuthFeedback(`⏳ ${b} からのセッション取得をテスト中...`, true);
      try {
        const res = await callCommand('test_youtube_auth', { method: 'browser', browser: b });
        if (res.success) {
          showAuthFeedback(res.message, true);
        } else {
          showAuthFeedback(res.message, false);
        }
      } catch (err) {
        showAuthFeedback(`ブラウザ連携エラー: ${err.message || String(err)}`, false);
      }
    });
  }

  // Open YouTube Music in browser
  const openYtMusicWeb = () => {
    callCommand('open_in_browser', { url: 'https://music.youtube.com' }).catch(e => console.error(e));
  };
  if (btnLinkYtMusic) {
    btnLinkYtMusic.addEventListener('click', (e) => {
      e.preventDefault();
      openYtMusicWeb();
    });
  }
  if (btnOpenBrowserYt) {
    btnOpenBrowserYt.addEventListener('click', (e) => {
      e.preventDefault();
      openYtMusicWeb();
    });
  }
}

function updateYtModalDeviceStatus() {
  const el = document.getElementById('ytDeviceStatusBanner');
  if (!el) return;
  if (isDeviceConnected) {
    el.className = 'yt-device-status-banner online';
    el.innerHTML = '<span>🟢 Pacemaker 実機接続中 — ワンクリックで実機へ直接転送・登録できます</span>';
    if (ytTargetCaseSelect) ytTargetCaseSelect.disabled = false;
  } else {
    el.className = 'yt-device-status-banner offline';
    el.innerHTML = '<span>🔌 Pacemaker 実機未接続 — 楽曲の検索・試聴が可能です（実機転送にはUSB接続が必要です）</span>';
  }
}

function populateTargetCaseSelect() {
  if (!ytTargetCaseSelect) return;
  const currentVal = ytTargetCaseSelect.value;
  ytTargetCaseSelect.innerHTML = '<option value="">ライブラリ全体のみ (Case指定なし)</option>';
  if (Array.isArray(currentCases)) {
    currentCases.forEach(c => {
      const opt = document.createElement('option');
      opt.value = c.name;
      opt.textContent = `${c.name} (${c.track_count || 0}曲)`;
      ytTargetCaseSelect.appendChild(opt);
    });
  }
  if (currentVal) ytTargetCaseSelect.value = currentVal;
}

function normalizeSongTitle(title) {
  if (!title) return '';
  return String(title)
    .toLowerCase()
    .replace(/[\(\[\{【](?:official\s*(?:music\s*)?video|mv|official\s*audio|audio|lyrics?|lyric\s*video|remaster(?:ed)?|full\s*(?:song|album)|hd|4k|hq|music\s*video)[\)\]\}】]/gi, '')
    .replace(/(?:feat\.?|ft\.?)\s+[^\-\(\)\[\]]+/gi, '')
    .replace(/[^\p{L}\p{N}\s]/gu, '')
    .replace(/\s+/g, ' ')
    .trim();
}

async function openYtModal() {
  populateTargetCaseSelect();
  ytModal.classList.remove('hidden');
  loadYoutubeAuthStatus();
  updateYtModalDeviceStatus();

  // If tracks aren't loaded yet, try loading
  if (!currentTracks || currentTracks.length === 0) {
    if (isDeviceConnected) {
      await loadTracks();
    }
  }
  renderLeftCompareList();

  if (selectedRightTrack) {
    highlightMatchingLeftTracks(selectedRightTrack);
  } else if (compareMatchBanner && compareMatchIcon && compareMatchText) {
    compareMatchBanner.className = 'compare-match-banner';
    compareMatchIcon.textContent = '💡';
    compareMatchText.innerHTML = '右の「追加したい候補」から曲をクリックすると、左の既存ライブラリと同名曲・配置位置を自動比較して並べて表示します';
  }

  setTimeout(() => {
    if (ytSearchInput) ytSearchInput.focus();
  }, 60);
}

function closeYtModal() {
  ytModal.classList.add('hidden');
}

function renderLeftCompareList() {
  if (!leftTracksList) return;
  leftTracksList.innerHTML = '';

  let tracks = [...currentTracks];

  if (filterLeftQuery) {
    tracks = tracks.filter(t => {
      const title = (t.title || '').toLowerCase();
      const artist = (t.artist || '').toLowerCase();
      return title.includes(filterLeftQuery) || artist.includes(filterLeftQuery);
    });
  }

  tracks.sort((a, b) => {
    let valA = '';
    let valB = '';
    if (leftSortKey === 'artist') {
      valA = (a.artist || a.title || '').toLowerCase();
      valB = (b.artist || b.title || '').toLowerCase();
    } else {
      valA = (a.title || '').toLowerCase();
      valB = (b.title || '').toLowerCase();
    }
    const cmp = valA.localeCompare(valB, 'ja', { numeric: true, sensitivity: 'base' });
    return leftSortAsc ? cmp : -cmp;
  });

  if (leftTracksCountBadge) {
    leftTracksCountBadge.textContent = `${tracks.length}曲`;
  }

  if (tracks.length === 0) {
    leftTracksList.innerHTML = `
      <div class="yt-empty-state" style="padding: 30px 10px;">
        <span style="font-size: 22px;">📁</span>
        <span>${filterLeftQuery ? '条件に一致する楽曲が見つかりません' : '実機に登録されている楽曲がありません'}</span>
      </div>
    `;
    return;
  }

  const fragment = document.createDocumentFragment();
  tracks.forEach(t => {
    const card = document.createElement('div');
    card.className = 'left-track-card';
    const trackId = t.id !== undefined ? t.id : t.track_id;
    card.dataset.trackId = String(trackId);
    card.dataset.title = t.title || '';
    card.dataset.artist = t.artist || '';
    card.dataset.normTitle = normalizeSongTitle(t.title);
    card.dataset.sortVal = (leftSortKey === 'artist' ? (t.artist || t.title || '') : (t.title || '')).toLowerCase();

    const isJustAdded = lastAddedTrackTitle && (t.title === lastAddedTrackTitle || card.dataset.normTitle === normalizeSongTitle(lastAddedTrackTitle));
    if (isJustAdded) {
      card.classList.add('just-added');
    }

    const durationText = t.play_time_secs ? formatTime(t.play_time_secs) : '';
    const bpmText = t.bpm ? `${Math.round(t.bpm)} BPM` : '';
    const metaParts = [t.artist || 'Unknown Artist', durationText, bpmText].filter(Boolean).join(' • ');

    card.innerHTML = `
      <div class="left-track-icon">🎵</div>
      <div class="left-track-info">
        <div class="left-track-title" title="${escapeHtml(t.title)}">${escapeHtml(t.title)}</div>
        <div class="left-track-meta" title="${escapeHtml(metaParts)}">${escapeHtml(metaParts)}</div>
      </div>
      <div class="left-track-badges">
        ${isJustAdded ? '<span class="left-track-badge-match" style="background:rgba(0,230,118,0.25);border-color:#00e676;">🆕 追加完了</span>' : ''}
      </div>
    `;

    fragment.appendChild(card);
  });

  leftTracksList.appendChild(fragment);

  // If a candidate on the right is selected, re-highlight left matches!
  if (selectedRightTrack) {
    highlightMatchingLeftTracks(selectedRightTrack);
  }
}

function selectRightCandidate(item, rowEl) {
  selectedRightTrack = item;

  // Clear previous selected on right
  const prevSelected = ytResultsContainer ? ytResultsContainer.querySelectorAll('.yt-result-item.selected-candidate') : [];
  prevSelected.forEach(el => el.classList.remove('selected-candidate'));

  if (rowEl) {
    rowEl.classList.add('selected-candidate');
  }

  highlightMatchingLeftTracks(item);
}

function highlightMatchingLeftTracks(rightItem) {
  if (!leftTracksList || !rightItem) return;

  const cards = leftTracksList.querySelectorAll('.left-track-card');
  if (cards.length === 0) return;

  // Reset classes and badges
  cards.forEach(c => {
    c.classList.remove('matched-exact', 'matched-adjacent');
    const badgeWrap = c.querySelector('.left-track-badges');
    if (badgeWrap) {
      const badges = badgeWrap.querySelectorAll('.left-track-badge-match, .left-track-badge-adjacent');
      badges.forEach(b => {
        if (!b.textContent.includes('🆕')) b.remove();
      });
    }
  });

  const normRight = normalizeSongTitle(rightItem.title);
  const matchedCards = [];

  cards.forEach(c => {
    const leftNorm = c.dataset.normTitle || '';
    let isMatch = false;

    if (leftNorm && normRight) {
      if (leftNorm === normRight) {
        isMatch = true;
      } else if (leftNorm.length >= 4 && normRight.includes(leftNorm)) {
        isMatch = true;
      } else if (normRight.length >= 4 && leftNorm.includes(normRight)) {
        isMatch = true;
      }
    }

    if (isMatch) {
      matchedCards.push(c);
      c.classList.add('matched-exact');
      const badgeWrap = c.querySelector('.left-track-badges');
      if (badgeWrap && !badgeWrap.querySelector('.left-track-badge-match:not([style*="🆕"])')) {
        const badge = document.createElement('span');
        badge.className = 'left-track-badge-match';
        badge.textContent = '✓ 同名曲あり';
        badgeWrap.appendChild(badge);
      }
    }
  });

  if (matchedCards.length > 0) {
    const firstMatch = matchedCards[0];
    firstMatch.scrollIntoView({ behavior: 'smooth', block: 'center' });

    if (compareMatchBanner && compareMatchIcon && compareMatchText) {
      compareMatchBanner.className = 'compare-match-banner match-found';
      compareMatchIcon.textContent = '⚠️';
      const extraCount = matchedCards.length > 1 ? `（計${matchedCards.length}件）` : '';
      compareMatchText.innerHTML = `実機ライブラリに同名曲が見つかりました: <strong>「${escapeHtml(firstMatch.dataset.title)}」</strong> by ${escapeHtml(firstMatch.dataset.artist || 'Unknown')} ${extraCount} (重複登録にご注意ください)`;
    }
  } else {
    // No match: find adjacent items (前後の曲名) based on left list sort order
    const targetVal = (leftSortKey === 'artist' ? (rightItem.artist || rightItem.title || '') : rightItem.title).toLowerCase();

    let prevCard = null;
    let nextCard = null;

    cards.forEach(c => {
      const cardVal = c.dataset.sortVal || '';
      const cmp = cardVal.localeCompare(targetVal, 'ja', { numeric: true, sensitivity: 'base' });
      if (cmp < 0) {
        prevCard = c;
      } else if (cmp >= 0 && !nextCard) {
        nextCard = c;
      }
    });

    if (prevCard) {
      prevCard.classList.add('matched-adjacent');
      const badgeWrap = prevCard.querySelector('.left-track-badges');
      if (badgeWrap) {
        const b = document.createElement('span');
        b.className = 'left-track-badge-adjacent';
        b.textContent = '▲ 直前の曲';
        badgeWrap.appendChild(b);
      }
    }

    if (nextCard) {
      nextCard.classList.add('matched-adjacent');
      const badgeWrap = nextCard.querySelector('.left-track-badges');
      if (badgeWrap) {
        const b = document.createElement('span');
        b.className = 'left-track-badge-adjacent';
        b.textContent = '▼ 直後の曲';
        badgeWrap.appendChild(b);
      }
    }

    const scrollTo = prevCard || nextCard;
    if (scrollTo) {
      scrollTo.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }

    if (compareMatchBanner && compareMatchIcon && compareMatchText) {
      compareMatchBanner.className = 'compare-match-banner match-none';
      compareMatchIcon.textContent = '✨';
      const prevName = prevCard ? `「${escapeHtml(prevCard.dataset.title)}」` : '（先頭）';
      const nextName = nextCard ? `「${escapeHtml(nextCard.dataset.title)}」` : '（末尾）';
      compareMatchText.innerHTML = `実機に未登録の新曲です。ソート位置: <strong>${prevName}</strong> と <strong>${nextName}</strong> の間に配置されます`;
    }
  }
}

function sortAndRenderRightList() {
  if (!currentYtResults || currentYtResults.length === 0) {
    if (rightTracksCountBadge) rightTracksCountBadge.textContent = '0件';
    return;
  }

  let items = [...currentYtResults];

  if (rightSortKey === 'title') {
    items.sort((a, b) => {
      const cmp = (a.title || '').localeCompare(b.title || '', 'ja', { numeric: true, sensitivity: 'base' });
      return rightSortAsc ? cmp : -cmp;
    });
  } else if (rightSortKey === 'artist') {
    items.sort((a, b) => {
      const valA = (a.artist || a.title || '').toLowerCase();
      const valB = (b.artist || b.title || '').toLowerCase();
      const cmp = valA.localeCompare(valB, 'ja', { numeric: true, sensitivity: 'base' });
      return rightSortAsc ? cmp : -cmp;
    });
  }

  renderYtSearchResults(items);
}

async function handleYtSearch(e) {
  if (e) e.preventDefault();
  const query = (ytSearchInput.value || '').trim();
  if (!query) return;

  btnYtSearch.disabled = true;
  btnYtSearch.innerHTML = '<span class="spin">⏳</span> 検索中...';
  ytResultsContainer.innerHTML = `
    <div class="yt-empty-state">
      <span class="spin" style="font-size: 26px;">⏳</span>
      <span>「${escapeHtml(query)}」を YouTube Music から検索中...</span>
    </div>
  `;
  if (ytSearchResultsCount) ytSearchResultsCount.textContent = '';
  if (rightTracksCountBadge) rightTracksCountBadge.textContent = '検索中...';

  try {
    const res = await callCommand('search_youtube', { query, limit: 15 });
    btnYtSearch.disabled = false;
    btnYtSearch.innerHTML = '<span>🔍 検索</span>';

    if (!res || !res.results || res.results.length === 0) {
      currentYtResults = [];
      ytResultsContainer.innerHTML = `
        <div class="yt-empty-state">
          <span style="font-size: 28px;">🔍</span>
          <span>「${escapeHtml(query)}」に一致する楽曲が見つかりませんでした。<br>別のキーワードやアーティスト名をお試しください。</span>
        </div>
      `;
      if (ytSearchResultsCount) ytSearchResultsCount.textContent = '0 件';
      if (rightTracksCountBadge) rightTracksCountBadge.textContent = '0件';
      return;
    }

    currentYtResults = res.results;
    if (ytSearchResultsCount) ytSearchResultsCount.textContent = `${res.results.length} 件の楽曲が見つかりました`;
    if (rightTracksCountBadge) rightTracksCountBadge.textContent = `${res.results.length}件`;
    
    sortAndRenderRightList();
  } catch (err) {
    console.error('YouTube search failed:', err);
    btnYtSearch.disabled = false;
    btnYtSearch.innerHTML = '<span>🔍 検索</span>';
    currentYtResults = [];
    if (rightTracksCountBadge) rightTracksCountBadge.textContent = '0件';
    ytResultsContainer.innerHTML = `
      <div class="yt-empty-state">
        <span style="font-size: 28px; color: #ff5252;">⚠️</span>
        <span style="color: #ff5252;">検索エラー: ${escapeHtml(err.message || String(err))}</span>
      </div>
    `;
  }
}

function renderYtSearchResults(items) {
  ytResultsContainer.innerHTML = '';
  if (!items || items.length === 0) return;

  const fragment = document.createDocumentFragment();
  let firstRow = null;

  items.forEach((item, idx) => {
    const row = document.createElement('div');
    row.className = 'yt-result-item';
    row.dataset.id = item.id;

    if (selectedRightTrack && (selectedRightTrack.id === item.id || selectedRightTrack.url === item.url)) {
      row.classList.add('selected-candidate');
    }

    const thumbHtml = item.thumbnail_url 
      ? `<img src="${escapeHtml(item.thumbnail_url)}" class="yt-item-thumb" alt="" loading="lazy">`
      : `<div class="yt-item-thumb" style="display:flex;align-items:center;justify-content:center;background:#1a202c;color:#718096;font-size:18px;">🎵</div>`;

    row.innerHTML = `
      <div class="yt-item-thumb-wrap">
        ${thumbHtml}
        <span class="yt-item-duration">${escapeHtml(item.duration_formatted || '')}</span>
      </div>
      <div class="yt-item-info">
        <div class="yt-item-title" title="${escapeHtml(item.title)}">${escapeHtml(item.title)}</div>
        <div class="yt-item-artist" title="${escapeHtml(item.artist)}">${escapeHtml(item.artist)}</div>
      </div>
      <div class="yt-item-action">
        <button type="button" class="btn btn-sm btn-yt-add" data-id="${escapeHtml(item.id)}" data-url="${escapeHtml(item.url)}" data-title="${escapeHtml(item.title)}" title="Pacemaker 実機へダウンロード・転送">
          <span class="btn-add-icon">⬅️</span>
          <span class="btn-add-label">追加 (+ Add)</span>
        </button>
      </div>
    `;

    // Click candidate to select & compare
    row.addEventListener('click', (e) => {
      if (e.target.closest('.btn-yt-add')) return;
      selectRightCandidate(item, row);
    });

    const btnAdd = row.querySelector('.btn-yt-add');
    btnAdd.addEventListener('click', () => handleAddTrackToPacemaker(btnAdd, item));

    if (idx === 0) firstRow = row;
    fragment.appendChild(row);
  });

  ytResultsContainer.appendChild(fragment);

  // If nothing is selected yet, automatically select the first candidate so the comparison is immediately shown
  if (!selectedRightTrack && items.length > 0 && firstRow) {
    selectRightCandidate(items[0], firstRow);
  }
}

async function handleAddTrackToPacemaker(btn, item) {
  if (!isDeviceConnected) {
    await showConfirmDialog(
      '実機未接続',
      'Pacemaker 実機が接続されていません。\nUSBケーブルで実機を接続してから楽曲を追加してください。',
      '🔌',
      '了解',
      true
    );
    return;
  }

  const caseName = ytTargetCaseSelect ? ytTargetCaseSelect.value.trim() : null;
  const originalHtml = btn.innerHTML;
  btn.disabled = true;
  btn.className = 'btn btn-sm btn-yt-add loading';
  btn.innerHTML = '<span class="spin">⏳</span> 転送中...';

  try {
    await callCommand('import_youtube_track', {
      queryOrUrl: item.url || `https://www.youtube.com/watch?v=${item.id}`,
      caseName: caseName || null,
      authMethod: null,
      browserName: null,
    });

    btn.className = 'btn btn-sm btn-yt-add added';
    btn.innerHTML = '<span>✓ 追加完了</span>';

    if (ytSearchResultsCount) {
      ytSearchResultsCount.textContent = `✓ 「${item.title}」を Pacemaker に追加しました！`;
    }

    lastAddedTrackTitle = item.title;

    // Refresh background state and update left compare list
    await loadDeviceStatus();
    await loadCases();
    await loadTracks();
    renderLeftCompareList();

    // Scroll to the newly added track in the left list
    const addedCard = leftTracksList ? leftTracksList.querySelector('.left-track-card.just-added') : null;
    if (addedCard) {
      addedCard.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  } catch (err) {
    console.error('Import failed:', err);
    btn.disabled = false;
    btn.className = 'btn btn-sm btn-yt-add';
    btn.innerHTML = originalHtml;
    await showConfirmDialog(
      '追加エラー',
      `「${item.title}」の転送に失敗しました:\n${err.message || String(err)}`,
      '⚠️',
      '閉じる',
      true
    );
  }
}

// Confirmation & Prompt Modal Elements (WKWebView safe)
const confirmModal = document.getElementById('confirmModal');
const confirmModalTitle = document.getElementById('confirmModalTitle');
const confirmModalIcon = document.getElementById('confirmModalIcon');
const confirmModalMessage = document.getElementById('confirmModalMessage');
const btnConfirmOk = document.getElementById('btnConfirmOk');
const btnConfirmCancel = document.getElementById('btnConfirmCancel');
const btnConfirmCancelX = document.getElementById('btnConfirmCancelX');
let confirmResolver = null;

function showConfirmDialog(title, message, icon = '⚠️', okText = '削除する', hideCancel = false) {
  return new Promise((resolve) => {
    confirmResolver = resolve;
    confirmModalTitle.textContent = title;
    confirmModalIcon.textContent = icon;
    confirmModalMessage.textContent = message;
    btnConfirmOk.textContent = okText;
    if (hideCancel) {
      if (btnConfirmCancel) btnConfirmCancel.classList.add('hidden');
      if (btnConfirmCancelX) btnConfirmCancelX.classList.add('hidden');
      btnConfirmOk.className = 'btn btn-primary';
    } else {
      if (btnConfirmCancel) btnConfirmCancel.classList.remove('hidden');
      if (btnConfirmCancelX) btnConfirmCancelX.classList.remove('hidden');
      btnConfirmOk.className = 'btn btn-danger';
    }
    confirmModal.classList.remove('hidden');
    btnConfirmOk.focus();
  });
}

function closeConfirm(result) {
  confirmModal.classList.add('hidden');
  if (btnConfirmCancel) btnConfirmCancel.classList.remove('hidden');
  if (btnConfirmCancelX) btnConfirmCancelX.classList.remove('hidden');
  btnConfirmOk.className = 'btn btn-danger';
  btnConfirmOk.textContent = '削除する';
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

let isDeviceConnected = false;
let isCheckingDevice = false;

function resetDeckPlayer() {
  if (audio) {
    audio.pause();
    audio.src = '';
  }
  deckTitle.textContent = '実機が未接続です';
  deckArtist.textContent = '-';
  deckBpm.textContent = 'BPM: --';
  deckKey.textContent = 'KEY: --';
  deckTime.textContent = '0:00 / 0:00';
  btnPlay.disabled = true;
  btnJumpCue.disabled = true;
  btnSetCue.disabled = true;
  btnSetLoopIn.disabled = true;
  btnSetLoopOut.disabled = true;
  btnToggleLoop.disabled = true;
  btnSaveMarkers.disabled = true;
  if (btnEditCurrentTrack) btnEditCurrentTrack.classList.add('hidden');
  cueMarker.classList.add('hidden');
  loopRegion.classList.add('hidden');
  playhead.style.left = '0%';
  currentWaveformPeaks = null;
  drawWaveform();
  activeTrack = null;
}

function renderDisconnectedState() {
  trackTableBody.innerHTML = `
    <tr>
      <td colspan="8" class="empty-state">
        <div style="padding: 56px 16px; text-align: center; color: #90a4ae;">
          <div style="font-size: 48px; margin-bottom: 14px; opacity: 0.85;">🔌</div>
          <h3 style="font-size: 18px; color: #eceff1; margin-bottom: 8px; font-weight: 600;">Pacemaker 実機が接続されていません</h3>
          <p style="font-size: 13px; color: #78909c; max-width: 480px; margin: 0 auto 18px; line-height: 1.6;">
            Tonium Pacemaker 本体をUSBケーブルでMacに接続してください。<br>
            マスストレージ内の <code>.Pacemaker/music.db</code> を自動検出してライブラリを読み込みます。
          </p>
          <button id="btnCheckDevice" class="btn btn-ctrl" style="padding: 8px 20px; font-size: 13px; cursor: pointer; border: 1px solid #00bcd4; color: #00bcd4;">🔄 接続を再確認</button>
        </div>
      </td>
    </tr>
  `;
  const btn = document.getElementById('btnCheckDevice');
  if (btn) {
    btn.addEventListener('click', () => {
      loadDeviceStatus();
    });
  }
}

async function boot() {
  console.log('[PacemakerEditor] Booting interface...');
  initAudio();
  initWaveform();
  initDragDrop();
  setupEvents();

  // Initial check of physical device connection
  await loadDeviceStatus();
  await loadYoutubeAuthStatus();

  // Background hardware connection polling (plug / unplug auto-detection)
  setInterval(() => {
    loadDeviceStatus();
  }, 2500);
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
  if (isCheckingDevice) return;
  isCheckingDevice = true;

  try {
    const data = await callCommand('get_device_status');

    if (data && data.connected) {
      const wasDisconnected = !isDeviceConnected;
      isDeviceConnected = true;

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

      // If the device just got connected, trigger data loading
      if (wasDisconnected) {
        deckTitle.textContent = 'トラックを選択してください';
        await Promise.allSettled([loadCases(), loadTracks()]);
      }
    } else {
      const wasConnected = isDeviceConnected;
      isDeviceConnected = false;

      deviceStatus.className = 'device-badge badge-offline';
      deviceText.textContent = 'Pacemaker 未接続';
      if (storageFill) storageFill.style.width = '0%';
      if (storagePercent) storagePercent.textContent = '--';
      if (storageText) storageText.textContent = 'ストレージ未検出';
      if (storageSubText) storageSubText.textContent = '実機USBケーブルを接続してください';

      caseList.innerHTML = '<li class="empty-state-small" style="padding: 10px 14px; color: #78909c; font-size: 12px; font-style: italic;">実機未接続</li>';
      allTracksCount.textContent = '0';
      currentTracks = [];

      renderDisconnectedState();

      if (wasConnected) {
        resetDeckPlayer();
      }
    }
  } catch (e) {
    console.error('Failed to load device status:', e);
    isDeviceConnected = false;
    deviceStatus.className = 'device-badge badge-offline';
    deviceText.textContent = 'オフライン (未接続)';
    if (storagePercent) storagePercent.textContent = '--';
    if (storageText) storageText.textContent = '未検出';
    if (storageSubText) storageSubText.textContent = '';
    caseList.innerHTML = '<li class="empty-state-small" style="padding: 10px 14px; color: #78909c; font-size: 12px; font-style: italic;">実機未接続</li>';
    allTracksCount.textContent = '0';
    currentTracks = [];
    renderDisconnectedState();
    resetDeckPlayer();
  } finally {
    updateYtModalDeviceStatus();
    isCheckingDevice = false;
  }
}

async function loadCases() {
  if (!isDeviceConnected) {
    caseList.innerHTML = '<li class="empty-state-small" style="padding: 10px 14px; color: #78909c; font-size: 12px; font-style: italic;">実機未接続</li>';
    currentCases = [];
    populateTargetCaseSelect();
    return;
  }
  try {
    const cases = await callCommand('get_cases');
    currentCases = Array.isArray(cases) ? cases : [];
    populateTargetCaseSelect();
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
  if (!isDeviceConnected) {
    showConfirmDialog('実機未接続', 'Pacemaker 実機が接続されていません。', '🔌', '了解');
    return;
  }
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
  if (!isDeviceConnected) {
    currentTracks = [];
    allTracksCount.textContent = '0';
    renderDisconnectedState();
    return;
  }

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
  if (!isDeviceConnected) {
    renderDisconnectedState();
    return;
  }

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
      selectTrack(t, false);
    });

    tr.addEventListener('dblclick', (e) => {
      if (e.target.closest('.btn-edit-track') || e.target.closest('.btn-del-track')) {
        return;
      }
      selectTrack(t, true);
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

async function selectTrack(track, autoplay = false) {
  if (!isDeviceConnected || !track) return;
  const trackId = track.id !== undefined ? track.id : track.track_id;

  // If already active track and audio has loaded, handle immediate toggle or replay
  if (activeTrack && ((activeTrack.id ?? activeTrack.track_id) === trackId) && audio.src) {
    if (autoplay) {
      try {
        audio.currentTime = 0;
        await audio.play();
      } catch (err) {
        console.warn('Replay failed:', err);
      }
    }
    return;
  }

  activeTrack = track;
  const loadId = ++currentTrackLoadId;

  // Toggle active class directly without recreating entire table DOM
  document.querySelectorAll('.track-table tbody tr').forEach(r => {
    r.classList.toggle('active-track', r.dataset.trackId === String(trackId));
  });

  // Stop previous track and reset playback controls immediately
  audio.pause();
  audio.currentTime = 0;
  btnPlay.disabled = true;
  btnPlay.textContent = '▶ 再生';
  playhead.style.left = '0%';
  deckTime.textContent = '0:00 / 0:00';

  deckTitle.textContent = track.title || 'Unknown Title';
  deckArtist.textContent = autoplay ? '再生準備中...' : '音声 & 波形データを読み込み中...';
  if (btnEditCurrentTrack) btnEditCurrentTrack.classList.remove('hidden');
  deckBpm.textContent = `BPM: ${track.bpm ? Number(track.bpm).toFixed(1) : '--'}`;
  deckKey.textContent = `KEY: ${track.key || '--'}`;

  cuePointMs = track.cue_point_ms ?? track.cue_point ?? -1;
  loopInMs = track.loop_in_ms ?? track.loop_in ?? -1;
  loopOutMs = track.loop_out_ms ?? track.loop_out ?? -1;
  loopEnabled = false;

  btnJumpCue.disabled = false;
  btnSetCue.disabled = false;
  btnSetLoopIn.disabled = false;
  btnSetLoopOut.disabled = false;
  btnToggleLoop.disabled = false;
  btnSaveMarkers.disabled = false;
  updateMarkersUI();

  // Try fetching proprietary .str waveform data in background
  waveformData = null;
  try {
    waveformData = await callCommand('get_waveform_data', { trackId: Number(trackId) });
  } catch (_) {}

  if (loadId !== currentTrackLoadId) return;

  // Free previous audio blob memory
  if (currentBlobUrl) {
    URL.revokeObjectURL(currentBlobUrl);
    currentBlobUrl = null;
  }

  try {
    const rawBytes = await callCommand('get_track_audio_bytes', { trackId: Number(trackId) });
    if (loadId !== currentTrackLoadId) return;

    const uint8 = rawBytes instanceof Uint8Array ? rawBytes : new Uint8Array(rawBytes);
    const loc = track.virtual_location || '';
    const ext = loc.split('.').pop().toLowerCase();
    let mime = 'audio/mpeg';
    if (ext === 'm4a' || ext === 'mp4') mime = 'audio/mp4';
    else if (ext === 'wav') mime = 'audio/wav';
    else if (ext === 'flac') mime = 'audio/flac';
    else if (ext === 'aif' || ext === 'aiff') mime = 'audio/aiff';

    const blob = new Blob([uint8], { type: mime });
    currentBlobUrl = URL.createObjectURL(blob);
    audio.src = currentBlobUrl;
    audio.load();
    btnPlay.disabled = false;
    deckArtist.textContent = track.artist || 'Unknown Artist';

    if (autoplay) {
      audio.play().catch(playErr => {
        console.warn('Blob playback start error:', playErr);
      });
    }

    // Decode audio for waveform in background without blocking playback
    const arrayBuffer = uint8.buffer.slice(uint8.byteOffset, uint8.byteOffset + uint8.byteLength);
    extractWaveformPeaks(arrayBuffer).then(peaks => {
      if (loadId === currentTrackLoadId) {
        currentWaveformPeaks = peaks || generateLightweightBars(track, waveformData);
        drawWaveform();
      }
    }).catch(() => {
      if (loadId === currentTrackLoadId) {
        currentWaveformPeaks = generateLightweightBars(track, waveformData);
        drawWaveform();
      }
    });
  } catch (err) {
    console.warn('Blob load failed, falling back to streaming protocol:', err);
    if (loadId !== currentTrackLoadId) return;

    audio.src = `pmaudio://localhost/tracks/${trackId}`;
    audio.load();
    deckArtist.textContent = track.artist || 'Unknown Artist';
    btnPlay.disabled = false;

    if (autoplay) {
      audio.play().catch(playErr => {
        console.warn('Fallback stream play error:', playErr);
      });
    }

    currentWaveformPeaks = generateLightweightBars(track, waveformData);
    drawWaveform();
  }
}

function initAudio() {
  audio.volume = volumeSlider ? parseFloat(volumeSlider.value) : 0.9;
  audio.muted = false;

  audio.addEventListener('error', () => {
    console.error('Audio element error:', audio.error);
    const code = audio.error ? audio.error.code : 0;
    const msg = audio.error ? audio.error.message : '';
    if (activeTrack) {
      deckArtist.textContent = `${activeTrack.artist || ''} (⚠️ 再生エラー code ${code})`;
    }
    btnPlay.textContent = '▶ 再生';
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
    drawWaveform();
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
    drawWaveform();
  });

  audio.addEventListener('ended', () => {
    btnPlay.textContent = '▶ 再生';
    audio.currentTime = 0;
    playhead.style.left = '0%';
    drawWaveform();
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
    if (!isDeviceConnected) {
      showConfirmDialog('実機未接続', 'Pacemaker 実機が接続されていません。USBケーブルで実機を接続してください。', '🔌', '了解');
      return;
    }
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
    if (!isDeviceConnected) {
      showConfirmDialog('実機未接続', 'Pacemaker 実機が接続されていません。USBケーブルで実機を接続してください。', '🔌', '了解');
      return;
    }
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

  if (fileInput) {
    fileInput.addEventListener('click', (e) => {
      if (!isDeviceConnected) {
        e.preventDefault();
        showConfirmDialog('実機未接続', 'Pacemaker 実機が接続されていません。USBケーブルで実機を接続してから楽曲を追加してください。', '🔌', '了解', true);
      }
    });
    fileInput.addEventListener('change', async (e) => {
      const files = Array.from(e.target.files || []);
      fileInput.value = '';
      await handleLocalFiles(files);
    });
  }

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

  // YouTube / YouTube Music Import & Compare Modal Events
  if (btnOpenYtModal) btnOpenYtModal.addEventListener('click', openYtModal);
  if (btnYtModalClose) btnYtModalClose.addEventListener('click', closeYtModal);
  if (btnYtModalCloseBottom) btnYtModalCloseBottom.addEventListener('click', closeYtModal);
  if (ytSearchForm) ytSearchForm.addEventListener('submit', handleYtSearch);
  if (ytModal) {
    ytModal.addEventListener('click', (e) => {
      if (e.target === ytModal) closeYtModal();
    });
  }

  // Left comparison column events (Sort & Filter)
  if (btnSortLeftTitle) {
    btnSortLeftTitle.addEventListener('click', () => {
      if (leftSortKey === 'title') {
        leftSortAsc = !leftSortAsc;
      } else {
        leftSortKey = 'title';
        leftSortAsc = true;
      }
      btnSortLeftTitle.classList.add('active');
      const arrow = btnSortLeftTitle.querySelector('.sort-arrow');
      if (arrow) arrow.textContent = leftSortAsc ? '↑' : '↓';
      if (btnSortLeftArtist) {
        btnSortLeftArtist.classList.remove('active');
        const artArrow = btnSortLeftArtist.querySelector('.sort-arrow');
        if (artArrow) artArrow.textContent = '↕';
      }
      renderLeftCompareList();
    });
  }

  if (btnSortLeftArtist) {
    btnSortLeftArtist.addEventListener('click', () => {
      if (leftSortKey === 'artist') {
        leftSortAsc = !leftSortAsc;
      } else {
        leftSortKey = 'artist';
        leftSortAsc = true;
      }
      btnSortLeftArtist.classList.add('active');
      const arrow = btnSortLeftArtist.querySelector('.sort-arrow');
      if (arrow) arrow.textContent = leftSortAsc ? '↑' : '↓';
      if (btnSortLeftTitle) {
        btnSortLeftTitle.classList.remove('active');
        const titArrow = btnSortLeftTitle.querySelector('.sort-arrow');
        if (titArrow) titArrow.textContent = '↕';
      }
      renderLeftCompareList();
    });
  }

  if (filterLeftInput) {
    filterLeftInput.addEventListener('input', () => {
      filterLeftQuery = (filterLeftInput.value || '').trim().toLowerCase();
      renderLeftCompareList();
    });
  }

  // Right comparison column events (Sort)
  if (btnSortRightRelevance) {
    btnSortRightRelevance.addEventListener('click', () => {
      rightSortKey = 'relevance';
      btnSortRightRelevance.classList.add('active');
      if (btnSortRightTitle) {
        btnSortRightTitle.classList.remove('active');
        const a = btnSortRightTitle.querySelector('.sort-arrow');
        if (a) a.textContent = '↕';
      }
      if (btnSortRightArtist) {
        btnSortRightArtist.classList.remove('active');
        const a = btnSortRightArtist.querySelector('.sort-arrow');
        if (a) a.textContent = '↕';
      }
      sortAndRenderRightList();
    });
  }

  if (btnSortRightTitle) {
    btnSortRightTitle.addEventListener('click', () => {
      if (rightSortKey === 'title') {
        rightSortAsc = !rightSortAsc;
      } else {
        rightSortKey = 'title';
        rightSortAsc = true;
      }
      btnSortRightTitle.classList.add('active');
      const a = btnSortRightTitle.querySelector('.sort-arrow');
      if (a) a.textContent = rightSortAsc ? '↑' : '↓';
      if (btnSortRightRelevance) btnSortRightRelevance.classList.remove('active');
      if (btnSortRightArtist) {
        btnSortRightArtist.classList.remove('active');
        const artA = btnSortRightArtist.querySelector('.sort-arrow');
        if (artA) artA.textContent = '↕';
      }
      sortAndRenderRightList();
    });
  }

  if (btnSortRightArtist) {
    btnSortRightArtist.addEventListener('click', () => {
      if (rightSortKey === 'artist') {
        rightSortAsc = !rightSortAsc;
      } else {
        rightSortKey = 'artist';
        rightSortAsc = true;
      }
      btnSortRightArtist.classList.add('active');
      const a = btnSortRightArtist.querySelector('.sort-arrow');
      if (a) a.textContent = rightSortAsc ? '↑' : '↓';
      if (btnSortRightRelevance) btnSortRightRelevance.classList.remove('active');
      if (btnSortRightTitle) {
        btnSortRightTitle.classList.remove('active');
        const titA = btnSortRightTitle.querySelector('.sort-arrow');
        if (titA) titA.textContent = '↕';
      }
      sortAndRenderRightList();
    });
  }

  initYoutubeAuthHandlers();

  // Global Keydown Handler for Modals
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      if (confirmModal && !confirmModal.classList.contains('hidden')) {
        closeConfirm(false);
      } else if (promptModal && !promptModal.classList.contains('hidden')) {
        closePrompt(null);
      } else if (editModal && !editModal.classList.contains('hidden')) {
        closeEditModal();
      } else if (ytAuthModal && !ytAuthModal.classList.contains('hidden')) {
        closeYtAuthModal();
      } else if (ytModal && !ytModal.classList.contains('hidden') && (!btnYtSearch || !btnYtSearch.disabled)) {
        closeYtModal();
      }
    }
  });
}

async function handleLocalFiles(files) {
  if (!files || files.length === 0) return;
  if (!isDeviceConnected) {
    await showConfirmDialog(
      '実機未接続',
      'Pacemaker 実機が接続されていません。\nUSBケーブルで実機を接続してから楽曲を追加してください。',
      '🔌',
      '了解',
      true
    );
    return;
  }

  let successCount = 0;
  let failCount = 0;
  for (const file of files) {
    try {
      const buffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buffer));
      await callCommand('import_local_track', {
        fileName: file.name,
        fileBytes: bytes,
        caseName: null,
      });
      successCount++;
    } catch (e) {
      console.error(`Failed to import local file ${file.name}:`, e);
      failCount++;
    }
  }

  await loadDeviceStatus();
  await loadCases();
  await loadTracks();

  if (failCount > 0) {
    await showConfirmDialog(
      '楽曲の追加結果',
      `${successCount} 曲を追加しました（${failCount} 曲が失敗しました）。`,
      '⚠️',
      '閉じる',
      true
    );
  } else if (successCount > 0) {
    await showConfirmDialog(
      '追加完了',
      `${successCount} 曲を Pacemaker 実機に正常に追加・登録しました！`,
      '✓',
      '了解',
      true
    );
  }
}

function initDragDrop() {
  window.addEventListener('dragover', (e) => {
    e.preventDefault();
    if ((ytModal && !ytModal.classList.contains('hidden')) || (ytAuthModal && !ytAuthModal.classList.contains('hidden'))) {
      return;
    }
    dropOverlay.classList.remove('hidden');
  });

  dropOverlay.addEventListener('dragleave', (e) => {
    if (e.relatedTarget === null) dropOverlay.classList.add('hidden');
  });

  dropOverlay.addEventListener('drop', async (e) => {
    e.preventDefault();
    dropOverlay.classList.add('hidden');
    const files = Array.from(e.dataTransfer ? e.dataTransfer.files : []);
    await handleLocalFiles(files);
  });
}

function escapeHtml(str) {
  return String(str || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
