/* ── Search Page ── */
const SearchPage = {
  template: `
  <div>
    <div class="max-w-lg mx-auto mb-8">
      <div class="relative">
        <input v-model="store.searchQuery" @keydown.enter="doSearch" placeholder="Search manga..."
          class="w-full pl-10 pr-4 py-3 bg-bg-card border border-bg-border rounded-xl text-sm focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 transition-all" autofocus>
        <svg class="w-4 h-4 text-fg-mute absolute left-3.5 top-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
      </div>
    </div>
    <div v-if="store.loading" class="card-grid">
      <div v-for="i in 8" :key="i" class="skeleton" style="aspect-ratio:3/4"></div>
    </div>
    <div v-else-if="store.searchResults.length" class="card-grid">
      <div v-for="item in store.searchResults" :key="item.id" @click="openManga(item.id)" class="manga-card bg-bg-card">
        <img v-if="item.cover?.url" :src="proxyUrl(item.cover.url)" loading="lazy" @error="onImgError">
        <div v-else class="w-full bg-bg-hover flex items-center justify-center" style="aspect-ratio:3/4"><span class="text-fg-mute text-3xl">?</span></div>
        <div class="px-2.5 py-2"><div class="text-xs line-clamp-2 text-fg-dim leading-snug">{{ item.title }}</div></div>
      </div>
    </div>
    <div v-else-if="store.searchDone" class="flex flex-col items-center justify-center py-20 text-fg-mute">
      <svg class="w-10 h-10 mb-3 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24"><circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/></svg>
      <p class="text-sm">No results for "{{ store.searchQuery }}"</p>
    </div>
  </div>`,
  setup() {
    async function doSearch() {
      if (!store.searchQuery.trim()) return;
      store.loading = true; store.searchDone = false;
      const d = await apiRun('search', { query: store.searchQuery });
      store.searchResults = d?.data?.items || [];
      store.searchDone = true; store.loading = false;
    }
    return { store, proxyUrl, openManga, doSearch, onImgError(e) { e.target.style.display = 'none'; } };
  }
};
