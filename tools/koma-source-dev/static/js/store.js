/* ── Reactive Store ── */
const store = Vue.reactive({
  sources: [],
  currentSource: '',
  sourceInfo: null,
  currentPage: 'home',
  routeParams: {},
  loading: false,
  lastJson: '',
  showJson: true,
  testResults: null,

  /* page data */
  homeSections: [],
  searchQuery: '',
  searchResults: [],
  searchDone: false,
  browseItems: [],
  manga: null,
  mangaError: '',
  chapters: [],
  readerPages: [],
  readerPage: 0,
});

/* ── API ── */
async function apiCall(endpoint, opts) {
  try {
    const r = await fetch(endpoint, opts);
    return await r.json();
  } catch (e) {
    return { ok: false, error: e.message };
  }
}

async function apiRun(op, request = {}) {
  store.lastJson = '';
  const data = await apiCall('/api/run', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ op, request })
  });
  store.lastJson = JSON.stringify(data, null, 2);
  return data;
}

function proxyUrl(url) {
  return '/api/proxy?url=' + encodeURIComponent(url);
}

async function initSources() {
  store.sources = await apiCall('/api/sources') || [];
  const active = store.sources.find(s => s.active);
  if (active) store.currentSource = active.file;
}

async function refreshSourceInfo() {
  const d = await apiCall('/api/info');
  store.sourceInfo = d?.data?.sourceInfo || null;
}

async function switchSource(name) {
  await apiCall('/api/switch', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name })
  });
  await refreshSourceInfo();
}
