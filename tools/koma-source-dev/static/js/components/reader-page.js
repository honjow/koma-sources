/* ── Reader ── */
const ReaderPage = {
  template: `
  <div class="flex flex-col items-center" style="height:calc(100vh - 7rem);min-height:400px">
    <div class="flex items-center justify-between w-full max-w-3xl mb-3 flex-shrink-0">
      <button @click="goBack" class="text-sm text-fg-dim hover:text-fg transition-colors flex items-center gap-1.5">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path d="M19 12H5M12 19l-7-7 7-7"/></svg>Back
      </button>
      <span class="text-xs text-fg-mute font-mono tabular-nums">{{ store.readerPage + 1 }} / {{ store.readerPages.length || '?' }}</span>
      <div class="w-16"></div>
    </div>
    <div class="flex-1 flex items-center justify-center w-full max-w-3xl relative select-none bg-bg-card/50 rounded-xl overflow-hidden min-h-0 border border-bg-border">
      <div class="absolute left-0 top-0 w-[30%] h-full cursor-w-resize z-10" @click="prev"></div>
      <div class="absolute right-0 top-0 w-[30%] h-full cursor-e-resize z-10" @click="next"></div>
      <img v-if="currentUrl" :src="currentUrl" class="max-h-full max-w-full object-contain" :key="store.readerPage" @error="onImgError">
      <div v-else class="text-fg-mute flex flex-col items-center gap-2">
        <svg class="w-10 h-10 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="M21 15l-5-5L5 21"/></svg>
        <span class="text-sm">No image</span>
      </div>
    </div>
    <div class="flex items-center gap-4 mt-3 flex-shrink-0">
      <button @click="prev" :disabled="store.readerPage<=0" class="px-3 py-1.5 bg-bg-card hover:bg-bg-hover border border-bg-border rounded-lg text-xs transition-colors disabled:opacity-20 disabled:cursor-default">←</button>
      <input type="range" :min="0" :max="Math.max(0,store.readerPages.length-1)" v-model.number="store.readerPage" class="w-28 md:w-40">
      <button @click="next" :disabled="store.readerPage>=store.readerPages.length-1" class="px-3 py-1.5 bg-bg-card hover:bg-bg-hover border border-bg-border rounded-lg text-xs transition-colors disabled:opacity-20 disabled:cursor-default">→</button>
    </div>
    <div class="text-[10px] text-fg-mute mt-2 flex gap-3"><span>← → / A D keys</span><span>Click sides</span><span>Esc to exit</span></div>
  </div>`,
  setup() {
    const currentUrl = Vue.computed(() => {
      const p = store.readerPages[store.readerPage];
      return p?.image?.url ? proxyUrl(p.image.url) : '';
    });
    function prev() { if (store.readerPage > 0) store.readerPage--; }
    function next() { if (store.readerPage < store.readerPages.length - 1) store.readerPage++; }
    async function loadReader(chapterId) {
      store.readerPage = 0; store.readerPages = [];
      const d = await apiRun('get_pages', { chapterId });
      store.readerPages = d?.data?.pages || [];
    }
    function onKey(e) {
      if (store.currentPage !== 'reader') return;
      if (e.key === 'ArrowLeft' || e.key === 'a') prev();
      if (e.key === 'ArrowRight' || e.key === 'd') next();
      if (e.key === 'Escape') goBack();
    }
    Vue.onMounted(() => { const id = store.routeParams.id; if (id) loadReader(id); document.addEventListener('keydown', onKey); });
    Vue.onUnmounted(() => document.removeEventListener('keydown', onKey));
    return { store, currentUrl, prev, next, loadReader, goBack, onImgError(e) { e.target.style.display = 'none'; } };
  }
};
