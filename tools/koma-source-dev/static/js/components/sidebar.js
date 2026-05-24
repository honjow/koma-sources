/* ── Sidebar ── */
const Sidebar = {
  template: `
  <aside class="w-48 lg:w-56 flex-shrink-0 bg-bg-card border-r border-bg-border flex-col hidden md:flex">
    <div class="px-4 py-4 border-b border-bg-border">
      <div class="flex items-center gap-2 mb-3">
        <svg class="w-5 h-5 text-accent" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>
        <span class="font-semibold text-sm tracking-tight">Source Dev</span>
      </div>
      <select :value="store.currentSource" @change="onSwitch"
        class="w-full bg-bg border border-bg-border rounded-lg px-2.5 py-1.5 text-xs focus:border-accent focus:outline-none transition-colors">
        <option v-for="s in store.sources" :key="s.file" :value="s.file">{{ s.name }}</option>
      </select>
    </div>
    <nav class="flex-1 px-3 py-3 space-y-1">
      <a v-for="item in navItems" :key="item.id"
        :href="item.hash" @click.prevent="navigateTo(item.hash)"
        :class="['flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors',
          store.currentPage===item.id ? 'bg-accent/10 text-accent font-medium' : 'text-fg-dim hover:text-fg hover:bg-bg-hover']">
        <span>{{ item.icon }}</span><span>{{ item.label }}</span>
      </a>
    </nav>
    <div class="px-3 pb-3">
      <button @click="runTest" class="w-full py-2 text-xs bg-bg-hover hover:bg-bg-border rounded-lg transition-colors text-fg-dim hover:text-fg">Run Test All</button>
    </div>
  </aside>`,
  setup() {
    const navItems = [
      { id:'home', label:'Home', icon:'🏠', hash:'#/' },
      { id:'search', label:'Search', icon:'🔍', hash:'#/search' },
      { id:'browse', label:'Browse', icon:'📚', hash:'#/browse' },
    ];
    async function onSwitch(e) {
      store.currentSource = e.target.value;
      await switchSource(e.target.value);
      applyRoute(parseHash());
    }
    async function runTest() {
      const d = await apiCall('/api/test-all');
      store.testResults = d?.results || [];
    }
    return { store, navItems, navigateTo, onSwitch, runTest };
  }
};
