// Koma Source Preview — Frontend
const API = '';
let currentPage = 'home';
let sourceInfo = null;

// --- API ---
async function api(endpoint, opts = {}) {
  const resp = await fetch(API + endpoint, opts);
  return resp.json();
}
async function apiRun(op, request = {}) {
  const data = await api('/api/run', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ op, request })
  });
  showJson(data);
  return data;
}
function proxyUrl(url) {
  return API + '/api/proxy?url=' + encodeURIComponent(url);
}

// --- Init ---
async function init() {
  await loadSources();
  sourceInfo = await api('/api/info');
  const info = sourceInfo?.data?.sourceInfo;
  if (info) {
    document.getElementById('source-name').textContent = `${info.name} v${info.version} (${info.language})`;
  }
  navigate('home');
}

// --- Source Switcher ---
async function loadSources() {
  const sources = await api('/api/sources');
  const select = document.getElementById('source-select');
  select.innerHTML = sources.map(s =>
    `<option value="${esc(s.file)}" ${s.active ? 'selected' : ''}>${esc(s.name)}</option>`
  ).join('');
}
async function switchSource() {
  const select = document.getElementById('source-select');
  const file = select.value;
  await api('/api/switch', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: file })
  });
  sourceInfo = await api('/api/info');
  const info = sourceInfo?.data?.sourceInfo;
  if (info) {
    document.getElementById('source-name').textContent = `${info.name} v${info.version} (${info.language})`;
  }
  navigate(currentPage);
}

// --- Navigation ---
function navigate(page, params = {}) {
  currentPage = page;
  document.querySelectorAll('.nav-link').forEach(el => {
    el.classList.toggle('active', el.dataset.page === page);
  });
  const content = document.getElementById('content');
  switch (page) {
    case 'home': renderHome(content); break;
    case 'search': renderSearch(content, params); break;
    case 'browse': renderBrowse(content); break;
    case 'manga': renderManga(content, params); break;
    case 'reader': renderReader(content, params); break;
  }
}

// --- Home ---
async function renderHome(el) {
  el.innerHTML = '<div class="text-center text-gray-500 py-8">Loading...</div>';
  const data = await apiRun('get_home', {});
  const sections = data?.data?.sections || [];
  if (sections.length === 0) {
    el.innerHTML = '<div class="text-center text-gray-500 py-8">No home sections available</div>';
    return;
  }
  el.innerHTML = sections.map(section => `
    <div class="mb-8">
      <h2 class="text-lg font-semibold mb-3">${esc(section.title)}</h2>
      <div class="section-scroll">
        ${(section.items || []).map(item => mangaCardSmall(item)).join('')}
      </div>
    </div>
  `).join('');
}

// --- Search ---
async function renderSearch(el, params = {}) {
  const query = params.query || '';
  el.innerHTML = `
    <div class="mb-6 flex gap-2">
      <input id="search-input" type="text" value="${esc(query)}" placeholder="Search manga..."
        class="flex-1 px-4 py-2 bg-gray-800 border border-gray-600 rounded text-sm focus:outline-none focus:border-blue-500"
        onkeydown="if(event.key==='Enter')doSearch()">
      <button onclick="doSearch()" class="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm">Search</button>
    </div>
    <div id="search-results" class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"></div>
  `;
  if (query) doSearch(query);
}
async function doSearch(q) {
  const query = q || document.getElementById('search-input').value;
  if (!query) return;
  const results = document.getElementById('search-results');
  results.innerHTML = '<div class="col-span-full text-center text-gray-500">Searching...</div>';
  const data = await apiRun('search', { query });
  const items = data?.data?.items || [];
  if (items.length === 0) {
    results.innerHTML = '<div class="col-span-full text-center text-gray-500">No results</div>';
    return;
  }
  results.innerHTML = items.map(item => mangaCard(item)).join('');
}

// --- Browse (manga_list) ---
async function renderBrowse(el) {
  el.innerHTML = '<div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4" id="browse-grid"></div>';
  const grid = document.getElementById('browse-grid');
  grid.innerHTML = '<div class="col-span-full text-center text-gray-500">Loading...</div>';
  const data = await apiRun('get_manga_list', { page: "1" });
  const items = data?.data?.items || [];
  grid.innerHTML = items.map(item => mangaCard(item)).join('');
}

// --- Manga Detail ---
async function renderManga(el, params) {
  el.innerHTML = '<div class="text-center text-gray-500 py-8">Loading...</div>';
  const data = await apiRun('get_manga', { mangaId: params.id });
  const manga = data?.data;
  if (!manga) { el.innerHTML = '<div class="text-red-400">Failed to load</div>'; return; }

  const chapData = await apiRun('get_chapters', { mangaId: params.id });
  const chapters = chapData?.data?.items || [];

  const coverUrl = manga.cover?.url ? proxyUrl(manga.cover.url) : '';
  el.innerHTML = `
    <div class="flex flex-col md:flex-row gap-6 mb-8">
      <div class="w-48 flex-shrink-0">
        ${coverUrl ? `<img src="${coverUrl}" class="w-full rounded-lg shadow-lg" style="aspect-ratio:3/4;object-fit:cover">` : ''}
      </div>
      <div class="flex-1">
        <h1 class="text-2xl font-bold mb-2">${esc(manga.title || '')}</h1>
        <div class="text-sm text-gray-400 mb-2">${(manga.authors || []).map(a => esc(a)).join(', ')}</div>
        <div class="flex flex-wrap gap-1 mb-3">
          ${(manga.tags || []).map(t => `<span class="px-2 py-0.5 bg-gray-700 rounded text-xs">${esc(t)}</span>`).join('')}
        </div>
        <div class="text-sm text-gray-400 mb-1">Status: ${esc(manga.status || 'unknown')}</div>
        <p class="text-sm text-gray-300 mt-3 max-h-32 overflow-y-auto">${esc(manga.description || '')}</p>
      </div>
    </div>
    <h2 class="text-lg font-semibold mb-3">Chapters (${chapters.length})</h2>
    <div class="space-y-1 max-h-96 overflow-y-auto">
      ${chapters.map(ch => `
        <div class="flex items-center justify-between px-3 py-2 bg-gray-800 rounded hover:bg-gray-700 cursor-pointer"
             onclick="navigate('reader', {mangaId:'${esc(params.id)}', chapterId:'${esc(ch.id)}'})">
          <span class="text-sm">${ch.chapterNumber ? 'Ch. ' + esc(ch.chapterNumber) : ''} ${ch.title ? esc(ch.title) : ''}</span>
          <span class="text-xs text-gray-500">${ch.pageCount || '?'} pages</span>
        </div>
      `).join('')}
    </div>
  `;
}

// --- Reader (single page, left/right navigation) ---
let readerPages = [];
let readerCurrentPage = 0;
let readerParams = {};

async function renderReader(el, params) {
  readerParams = params;
  el.innerHTML = '<div class="text-center text-gray-500 py-8">Loading pages...</div>';
  const data = await apiRun('get_pages', { chapterId: params.chapterId });
  readerPages = data?.data?.pages || [];
  readerCurrentPage = 0;
  if (readerPages.length === 0) {
    el.innerHTML = '<div class="text-red-400">No pages found</div>';
    return;
  }
  renderReaderPage(el);
}

function renderReaderPage(el) {
  if (!el) el = document.getElementById('content');
  const page = readerPages[readerCurrentPage];
  const url = page?.image?.url ? proxyUrl(page.image.url) : '';
  const total = readerPages.length;
  const current = readerCurrentPage + 1;

  el.innerHTML = `
    <div class="flex flex-col items-center h-[calc(100vh-8rem)]">
      <!-- Top bar -->
      <div class="flex items-center justify-between w-full max-w-3xl mb-2">
        <button onclick="navigate('manga', {id:'${esc(readerParams.mangaId)}'})" class="text-sm text-blue-400 hover:text-blue-300">← Back</button>
        <span class="text-sm text-gray-400">${current} / ${total}</span>
      </div>
      <!-- Image container -->
      <div class="flex-1 flex items-center justify-center w-full max-w-3xl relative select-none" id="reader-container">
        <!-- Left click zone -->
        <div class="absolute left-0 top-0 w-1/3 h-full cursor-pointer z-10" onclick="readerPrev()"></div>
        <!-- Right click zone -->
        <div class="absolute right-0 top-0 w-1/3 h-full cursor-pointer z-10" onclick="readerNext()"></div>
        ${url ? `<img src="${url}" class="max-h-full max-w-full object-contain rounded shadow-lg">` : '<div class="text-gray-500">No image</div>'}
      </div>
      <!-- Bottom controls -->
      <div class="flex items-center gap-4 mt-3">
        <button onclick="readerPrev()" class="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded text-sm disabled:opacity-30" ${current <= 1 ? 'disabled' : ''}>← Prev</button>
        <span class="text-sm text-gray-400 w-20 text-center">${current} / ${total}</span>
        <button onclick="readerNext()" class="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded text-sm disabled:opacity-30" ${current >= total ? 'disabled' : ''}>Next →</button>
      </div>
    </div>
  `;
}

function readerPrev() {
  if (readerCurrentPage > 0) {
    readerCurrentPage--;
    renderReaderPage();
  }
}
function readerNext() {
  if (readerCurrentPage < readerPages.length - 1) {
    readerCurrentPage++;
    renderReaderPage();
  }
}

// Keyboard navigation
document.addEventListener('keydown', (e) => {
  if (currentPage !== 'reader') return;
  if (e.key === 'ArrowLeft' || e.key === 'a') readerPrev();
  if (e.key === 'ArrowRight' || e.key === 'd') readerNext();
});

// --- Components ---
function mangaCard(item) {
  const cover = item.cover?.url ? proxyUrl(item.cover.url) : '';
  return `
    <div class="manga-card" onclick="navigate('manga', {id:'${esc(item.id)}'})">
      <div class="bg-gray-800 rounded-lg overflow-hidden">
        ${cover ? `<img src="${cover}" alt="" class="w-full">` : '<div class="w-full bg-gray-700" style="aspect-ratio:3/4"></div>'}
        <div class="p-2">
          <div class="text-xs font-medium truncate">${esc(item.title || '')}</div>
        </div>
      </div>
    </div>
  `;
}
function mangaCardSmall(item) {
  const cover = item.cover?.url ? proxyUrl(item.cover.url) : '';
  return `
    <div class="manga-card w-28" onclick="navigate('manga', {id:'${esc(item.id)}'})">
      ${cover ? `<img src="${cover}" alt="" class="w-28 rounded">` : '<div class="w-28 bg-gray-700 rounded" style="aspect-ratio:3/4"></div>'}
      <div class="text-xs mt-1 truncate">${esc(item.title || '')}</div>
    </div>
  `;
}

// --- Utils ---
function esc(s) { return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;'); }
function showJson(data) {
  document.getElementById('json-content').textContent = JSON.stringify(data, null, 2);
}
function toggleJsonPanel() {
  const panel = document.getElementById('json-panel');
  panel.classList.toggle('translate-y-full');
}
async function runTestAll() {
  const data = await api('/api/test-all');
  alert(data.results.map(r => `${r.status === 'pass' ? '✓' : '✗'} ${r.op}`).join('\n'));
}

// Start
init();
