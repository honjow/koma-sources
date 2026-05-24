/* ── Home Page ── */
const HomePage = {
  template: `
  <div>
    <div v-if="store.loading" class="space-y-8">
      <div v-for="i in 3" :key="i">
        <div class="skeleton h-4 w-24 mb-3"></div>
        <div class="scroll-row">
          <div v-for="j in 6" :key="j" class="skeleton w-[120px]" style="aspect-ratio:3/4"></div>
        </div>
      </div>
    </div>
    <div v-else-if="store.homeSections.length===0" class="flex flex-col items-center justify-center py-20 text-fg-mute">
      <svg class="w-10 h-10 mb-3 opacity-30" fill="none" stroke="currentColor" viewBox="0 0 24 24"><rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/></svg>
      <p class="text-sm">No home sections available</p>
    </div>
    <div v-else v-for="section in store.homeSections" :key="section.title" class="mb-10">
      <h2 class="text-xs font-semibold text-fg-mute uppercase tracking-widest mb-4">{{ section.title }}</h2>
      <div class="scroll-row">
        <div v-for="item in section.items" :key="item.id" @click="openManga(item.id)" class="manga-card w-[130px] md:w-[150px] bg-bg-card flex-shrink-0">
          <img v-if="item.cover?.url" :src="proxyUrl(item.cover.url)" loading="lazy" @error="onImgError">
          <div v-else class="w-full bg-bg-hover flex items-center justify-center" style="aspect-ratio:3/4"><span class="text-fg-mute text-3xl">?</span></div>
          <div class="px-2.5 py-2"><div class="text-xs line-clamp-2 text-fg-dim leading-snug">{{ item.title }}</div></div>
        </div>
      </div>
    </div>
  </div>`,
  setup() {
    async function loadHome() {
      store.loading = true;
      const d = await apiRun('get_home', {});
      store.homeSections = d?.data?.sections || [];
      store.loading = false;
    }
    Vue.onMounted(() => loadHome());
    return { store, proxyUrl, openManga, onImgError(e) { e.target.style.display = 'none'; } };
  }
};
