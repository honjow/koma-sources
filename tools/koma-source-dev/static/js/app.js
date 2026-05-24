/* ── Koma Source Dev App ── */
const App = {
  components: { Sidebar, MobileNav, HomePage, SearchPage, BrowsePage, MangaDetailPage, ReaderPage, ResponsePanel, TestModal },
  template: `
  <div class="h-full flex">
    <Sidebar />
    <div class="flex-1 flex flex-col min-w-0 md:pt-0 pt-12">
      <MobileNav />
      <main class="flex-1 overflow-y-auto min-h-0">
        <div class="max-w-5xl mx-auto px-4 py-6 md:py-8">
          <HomePage v-if="store.currentPage==='home'" ref="homeRef" />
          <SearchPage v-else-if="store.currentPage==='search'" />
          <BrowsePage v-else-if="store.currentPage==='browse'" ref="browseRef" />
          <MangaDetailPage v-else-if="store.currentPage==='detail'" ref="detailRef" />
          <ReaderPage v-else-if="store.currentPage==='reader'" ref="readerRef" />
        </div>
      </main>
      <ResponsePanel />
    </div>
    <TestModal />
  </div>`,
  setup() {
    function onHashChange() {
      const route = parseHash();
      applyRoute(route);
    }
    Vue.onMounted(async () => {
      window.addEventListener('hashchange', onHashChange);
      await initSources();
      await refreshSourceInfo();
      onHashChange();
    });
    Vue.onUnmounted(() => window.removeEventListener('hashchange', onHashChange));
    return { store };
  }
};
