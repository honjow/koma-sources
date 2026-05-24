/* ── Response Panel ── */
const ResponsePanel = {
  template: `
  <div class="flex-shrink-0 border-t border-bg-border bg-bg-card">
    <div @click="store.showJson=!store.showJson" class="flex items-center justify-between px-4 py-2 cursor-pointer select-none hover:bg-bg-hover transition-colors">
      <span class="text-[11px] text-fg-mute font-mono tracking-wide">RESPONSE</span>
      <span class="text-[11px] text-fg-mute">{{ store.showJson ? '▾' : '▴' }}</span>
    </div>
    <pre v-if="store.showJson" class="px-4 pb-3 text-xs text-emerald-400/70 font-mono overflow-auto max-h-48 leading-relaxed">{{ store.lastJson || 'No response yet' }}</pre>
  </div>`,
  setup() { return { store }; }
};

/* ── Test Modal ── */
const TestModal = {
  template: `
  <transition name="page">
    <div v-if="store.testResults" class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 backdrop-blur-sm" @click.self="store.testResults=null">
      <div class="bg-bg-card border border-bg-border rounded-xl p-6 max-w-sm w-full mx-4 shadow-2xl">
        <h3 class="font-semibold text-sm mb-4">Test Results</h3>
        <div class="space-y-2">
          <div v-for="r in store.testResults" :key="r.op" class="flex items-center gap-2.5 text-sm">
            <span :class="r.status==='pass' ? 'text-emerald-400' : 'text-red-400'" class="font-mono text-xs w-4 text-center">{{ r.status==='pass' ? '✓' : '✗' }}</span>
            <span class="text-fg-dim text-xs flex-1">{{ r.op }}</span>
            <span v-if="r.status!=='pass'" class="text-xs text-red-400/60 truncate max-w-[120px]">{{ r.status }}</span>
          </div>
        </div>
        <button @click="store.testResults=null" class="mt-5 w-full py-2.5 bg-bg-hover hover:bg-bg-border rounded-lg text-xs transition-colors">Close</button>
      </div>
    </div>
  </transition>`,
  setup() { return { store }; }
};
