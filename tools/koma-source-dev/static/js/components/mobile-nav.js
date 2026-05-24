/* ── Mobile Nav ── */
const MobileNav = {
  template: `
  <nav class="md:hidden flex-shrink-0 bg-bg-card border-b border-bg-border px-2 flex">
    <a v-for="item in navItems" :key="item.id"
      :href="item.hash" @click.prevent="navigateTo(item.hash)"
      :class="['flex-1 flex flex-col items-center py-2 text-[10px] transition-colors',
        store.currentPage===item.id ? 'text-accent' : 'text-fg-mute']">
      <span class="text-lg">{{ item.icon }}</span>
      <span class="mt-0.5">{{ item.label }}</span>
    </a>
  </nav>`,
  setup() {
    const navItems = [
      { id:'home', label:'Home', icon:'🏠', hash:'#/' },
      { id:'search', label:'Search', icon:'🔍', hash:'#/search' },
      { id:'browse', label:'Browse', icon:'📚', hash:'#/browse' },
    ];
    return { store, navItems, navigateTo };
  }
};
