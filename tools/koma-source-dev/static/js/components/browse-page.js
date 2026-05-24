/* ── Browse Page ── */
const BrowsePage = {
  template: `
  <div>
    <div v-if="store.loading" class="card-grid">
      <div v-for="i in 12" :key="i" class="skeleton" style="aspect-ratio:3/4"></div>
    </div>
    <div v-else class="card-grid">
      <div v-for="item in store.browseItems" :key="item.id" @click="openManga(item.id)" class="manga-card bg-bg-card">
        <img v-if="item.cover?.url" :src="proxyUrl(item.cover.url)" loading="lazy" @error="onImgError">
        <div v-else class="w-full bg-bg-hover flex items-center justify-center" style="aspect-ratio:3/4"><span class="text-fg-mute text-3xl">?</span></div>
        <div class="px-2.5 py-2"><div class="text-xs line-clamp-2 text-fg-dim leading-snug">{{ item.title }}</div></div>
      </div>
    </div>
  </div>`,
  setup() {
    async function loadBrowse() {
      store.loading = true;
      const d = await apiRun('get_manga_list', { page: "1" });
      store.browseItems = d?.data?.items || [];
      store.loading = false;
    }
    Vue.onMounted(() => loadBrowse());
    return { store, proxyUrl, openManga, onImgError(e) { e.target.style.display = 'none'; } };
  }
};
