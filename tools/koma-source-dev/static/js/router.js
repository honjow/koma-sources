/* ── Hash Router ── */
const PAGE_ACTIONS = {
  home:   () => { store.currentPage = 'home'; },
  search: () => { store.currentPage = 'search'; },
  browse: () => { store.currentPage = 'browse'; },
};

function parseHash() {
  const h = window.location.hash || '#/';
  if (h.startsWith('#/manga/')) return { page: 'detail', id: decodeURIComponent(h.slice('#/manga/'.length)) };
  if (h.startsWith('#/reader/')) return { page: 'reader', id: decodeURIComponent(h.slice('#/reader/'.length)) };
  const m = { '#/search': 'search', '#/browse': 'browse' };
  return { page: m[h] || 'home' };
}

function applyRoute(route) {
  store.routeParams = route;
  store.currentPage = route.page;
}

function navigateTo(hash) {
  if (hash) window.location.hash = hash;
}

function openManga(id) {
  window.location.hash = '#/manga/' + encodeURIComponent(id);
}

function openReader(chapterId) {
  window.location.hash = '#/reader/' + encodeURIComponent(chapterId);
}

function goBack() {
  window.history.back();
}
