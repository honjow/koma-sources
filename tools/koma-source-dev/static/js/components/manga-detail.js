/* ── Manga Detail ── */
const MangaDetailPage = {
  template: `
  <div>
    <button @click="goBack" class="text-sm text-fg-dim hover:text-fg mb-6 transition-colors flex items-center gap-1.5">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path d="M19 12H5M12 19l-7-7 7-7"/></svg>Back
    </button>
    <div v-if="store.loading" class="flex gap-5 mb-6">
      <div class="skeleton w-32 md:w-44" style="aspect-ratio:3/4"></div>
      <div class="flex-1 space-y-3 pt-2">
        <div class="skeleton h-6 w-3/4"></div><div class="skeleton h-4 w-1/3"></div>
        <div class="skeleton h-3 w-full"></div><div class="skeleton h-3 w-5/6"></div>
      </div>
    </div>
    <div v-else-if="store.mangaError" class="flex flex-col items-center py-20">
      <div class="bg-red-500/5 border border-red-500/10 rounded-xl px-6 py-5 max-w-md text-center">
        <svg class="w-8 h-8 text-red-400/60 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/></svg>
        <p class="text-red-400 text-sm font-medium">Failed to load</p>
        <p class="text-fg-mute text-xs mt-1">{{ store.mangaError }}</p>
      </div>
    </div>
    <div v-else-if="store.manga">
      <div class="flex flex-col sm:flex-row gap-5 md:gap-8 mb-8">
        <div class="flex-shrink-0 mx-auto sm:mx-0">
          <img v-if="store.manga.cover?.url" :src="proxyUrl(store.manga.cover.url)" class="w-36 md:w-48 rounded-xl shadow-2xl" style="aspect-ratio:3/4;object-fit:cover" @error="onImgError">
          <div v-else class="w-36 md:w-48 bg-bg-card rounded-xl flex items-center justify-center" style="aspect-ratio:3/4"><span class="text-fg-mute text-4xl">?</span></div>
        </div>
        <div class="flex-1 min-w-0 text-center sm:text-left">
          <h1 class="text-xl md:text-2xl font-bold tracking-tight mb-1.5">{{ store.manga.title }}</h1>
          <div class="text-sm text-fg-dim mb-2" v-if="store.manga.authors?.length">{{ store.manga.authors.join(' · ') }}</div>
          <div class="flex flex-wrap justify-center sm:justify-start gap-1.5 mb-3">
            <span v-if="store.manga.status" class="px-2.5 py-0.5 bg-bg-card border border-bg-border rounded-md text-xs text-fg-dim capitalize">{{ store.manga.status }}</span>
            <span v-if="store.manga.contentRating" class="px-2.5 py-0.5 bg-bg-card border border-bg-border rounded-md text-xs text-fg-dim">{{ store.manga.contentRating }}</span>
            <span v-if="store.manga.language" class="px-2.5 py-0.5 bg-bg-card border border-bg-border rounded-md text-xs text-fg-dim uppercase">{{ store.manga.language }}</span>
          </div>
          <div class="flex flex-wrap justify-center sm:justify-start gap-1 mb-3" v-if="store.manga.tags?.length">
            <span v-for="t in store.manga.tags.slice(0,10)" :key="t" class="px-2 py-0.5 bg-accent/5 border border-accent/10 rounded-full text-[11px] text-fg-dim">{{ t }}</span>
          </div>
          <p class="text-sm text-fg-dim leading-relaxed max-h-24 overflow-y-auto whitespace-pre-line" v-if="store.manga.description">{{ store.manga.description }}</p>
          <div class="mt-3 flex flex-wrap gap-1.5" v-if="store.manga.links?.length">
            <a v-for="(link,i) in store.manga.links" :key="i" :href="link.url" target="_blank"
              class="inline-flex items-center gap-1 px-2.5 py-1 bg-bg-hover hover:bg-bg-border rounded-lg text-xs text-accent transition-colors">
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3"/></svg>
              {{ link.kind === 'source' ? 'Source' : link.kind }}
            </a>
          </div>
        </div>
      </div>
      <div class="bg-bg-card rounded-xl border border-bg-border overflow-hidden">
        <div class="px-4 py-3 border-b border-bg-border flex items-center justify-between">
          <h2 class="text-sm font-semibold">Chapters</h2>
          <span class="text-xs text-fg-mute">{{ store.chapters.length }} chapters</span>
        </div>
        <div v-if="store.chapters.length===0" class="px-4 py-8 text-center text-fg-mute text-sm">No chapters available</div>
        <div v-else class="divide-y divide-bg-border max-h-[55vh] overflow-y-auto">
          <div v-for="ch in store.chapters" :key="ch.id" @click="openReader(ch.id)"
            class="px-4 py-3 hover:bg-bg-hover cursor-pointer transition-colors flex items-center justify-between group">
            <div class="min-w-0">
              <div class="text-sm text-fg group-hover:text-white transition-colors truncate">
                <span v-if="ch.chapterNumber" class="text-accent font-mono text-xs mr-2">Ch.{{ ch.chapterNumber }}</span>
                {{ ch.title || 'Untitled' }}
              </div>
              <div class="text-[11px] text-fg-mute mt-0.5" v-if="ch.language">🌐 {{ ch.language }}</div>
            </div>
            <div class="flex items-center gap-2 flex-shrink-0">
              <span class="text-xs text-fg-mute" v-if="ch.pageCount">{{ ch.pageCount }}p</span>
              <svg class="w-4 h-4 text-fg-mute opacity-0 group-hover:opacity-100 transition-opacity" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path d="M9 18l6-6-6-6"/></svg>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>`,
  setup() {
    async function loadManga(id) {
      store.loading = true; store.manga = null; store.mangaError = ''; store.chapters = [];
      const d = await apiRun('get_manga', { mangaId: id });
      if (!d.ok || (!d.data?.manga && !d.data?.title)) {
        store.mangaError = d.error?.message || d.error || 'Unknown error';
        store.loading = false; return;
      }
      store.manga = d.data.manga || d.data;
      const cd = await apiRun('get_chapters', { mangaId: id });
      store.chapters = cd?.data?.items || [];
      store.loading = false;
    }
    Vue.onMounted(() => { const id = store.routeParams.id; if (id) loadManga(id); });
    return { store, proxyUrl, goBack, openReader, onImgError(e) { e.target.style.display = 'none'; } };
  }
};
