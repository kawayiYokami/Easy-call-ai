<template>
  <div
    class="relationship-panel fixed right-4 top-16 z-50 rounded-box border border-base-300 bg-base-100 shadow-lg transition-all duration-300"
    :class="collapsed ? 'h-10 w-10 cursor-pointer overflow-hidden' : 'w-72 p-3'"
    @click="collapsed && toggleCollapse()"
  >
    <div v-if="collapsed" class="flex h-full w-full items-center justify-center text-sm font-bold" :style="{ color: dotColor }">
      <Heart class="h-5 w-5" :style="{ fill: dotColor }" />
    </div>

    <template v-else>
      <div class="mb-2 flex items-center justify-between">
        <span class="text-xs font-semibold text-base-content/70">💞 Relationship</span>
        <div class="flex gap-1">
          <button class="btn btn-ghost btn-xs h-5 min-h-0 w-5 p-0" title="刷新" @click.stop="refresh"><RotateCw class="h-3 w-3" /></button>
          <button class="btn btn-ghost btn-xs h-5 min-h-0 w-5 p-0" title="重置" @click.stop="reset"><Trash2 class="h-3 w-3" /></button>
          <button class="btn btn-ghost btn-xs h-5 min-h-0 w-5 p-0" title="折叠" @click.stop="toggleCollapse"><Minus class="h-3 w-3" /></button>
        </div>
      </div>

      <div class="mb-2 truncate text-[10px] text-base-content/40" :title="agentIdText">
        agent: {{ agentIdText }} · analyzer: {{ snapshot?.rules?.analyzerEnabled === false ? 'off' : 'on' }}
      </div>

      <div class="space-y-1.5">
        <div v-for="bar in displayedBars" :key="bar.key" class="flex items-center gap-1.5 text-xs">
          <span class="w-5 shrink-0 text-right">{{ bar.emoji }}</span>
          <span class="w-14 shrink-0 text-[11px] text-base-content/60">{{ bar.label }}</span>
          <div class="h-2 flex-1 overflow-hidden rounded-full bg-base-300">
            <div class="h-full rounded-full transition-all duration-500" :style="{ width: bar.value + '%', background: bar.color }"></div>
          </div>
          <span class="w-7 shrink-0 text-right font-mono text-[11px]" :style="{ color: bar.color }">{{ bar.value }}</span>
        </div>
      </div>

      <div v-if="lastEventText" class="mt-2 truncate text-[10px] italic text-base-content/50" :title="lastEventText">↳ {{ lastEventText }}</div>

      <details class="mt-2 text-[10px] text-base-content/60">
        <summary class="cursor-pointer select-none">Recent events</summary>
        <div class="mt-1 max-h-24 space-y-1 overflow-auto rounded bg-base-200 p-2">
          <div v-for="(event, index) in recentEvents" :key="`${event.eventType}-${index}`" class="border-b border-base-300/60 pb-1 last:border-0 last:pb-0">
            <div class="font-semibold">{{ event.eventType }} · {{ formatDelta(event.appliedDelta) }}</div>
            <div class="truncate opacity-70" :title="event.reason">{{ event.reason || 'no reason' }}</div>
          </div>
          <div v-if="recentEvents.length === 0" class="opacity-50">暂无事件</div>
        </div>
      </details>

      <details class="mt-2 text-[10px] text-base-content/60">
        <summary class="cursor-pointer select-none">Developer Controls</summary>
        <div class="mt-1 flex flex-wrap gap-1">
          <button v-for="type in eventTypes" :key="type" class="btn btn-outline btn-xs" @click.stop="simulate(type)">{{ type }}</button>
          <button class="btn btn-outline btn-xs" @click.stop="refreshRules">reload rules</button>
        </div>
      </details>

      <details class="mt-2 text-[10px] text-base-content/60">
        <summary class="cursor-pointer select-none">Block preview</summary>
        <pre class="mt-1 max-h-32 overflow-auto whitespace-pre-wrap rounded bg-base-200 p-2">{{ snapshot?.relationshipBlock }}</pre>
      </details>

      <details class="mt-2 text-[10px] text-base-content/60">
        <summary class="cursor-pointer select-none">Raw JSON</summary>
        <pre class="mt-1 max-h-32 overflow-auto whitespace-pre-wrap rounded bg-base-200 p-2">{{ rawJsonText }}</pre>
      </details>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { Heart, Minus, RotateCw, Trash2 } from "lucide-vue-next";
import { invokeTauri } from "../../../services/tauri-api";

interface RelationshipDimensions {
  affection: number;
  trust: number;
  tension: number;
  sadness: number;
  playfulness: number;
  attachment: number;
}

interface InteractionEvent {
  eventType: string;
  reason: string;
  appliedDelta: Partial<RelationshipDimensions>;
}

interface RelationshipRules {
  displayOrder: string[];
  analyzerEnabled: boolean;
  developerMode: boolean;
}

interface RelationshipPanelSnapshot {
  agentId: string;
  dimensions: RelationshipDimensions;
  lastEvent: InteractionEvent | null;
  recentEvents: InteractionEvent[];
  relationshipBlock: string;
  rawJson: unknown;
  rules: RelationshipRules;
}

interface BarDef {
  key: keyof RelationshipDimensions;
  label: string;
  emoji: string;
  color: string;
}

const props = defineProps<{
  conversationId: string;
  agentId?: string;
}>();

const collapsed = ref(true);
const snapshot = ref<RelationshipPanelSnapshot | null>(null);
let pollTimer: ReturnType<typeof setInterval> | null = null;

const bars: BarDef[] = [
  { key: "affection", label: "亲近", emoji: "❤️", color: "#ec4899" },
  { key: "trust", label: "信任", emoji: "🤝", color: "#10b981" },
  { key: "tension", label: "紧张", emoji: "⚡", color: "#ef4444" },
  { key: "sadness", label: "失落", emoji: "💧", color: "#3b82f6" },
  { key: "playfulness", label: "轻松", emoji: "😈", color: "#a855f7" },
  { key: "attachment", label: "陪伴", emoji: "🧷", color: "#f97316" },
];
const eventTypes = ["gratitude", "praise", "insult", "apology", "rejection", "repair", "neutral"];

const agentIdText = computed(() => String(props.agentId || snapshot.value?.agentId || "default_agent").trim() || "default_agent");
const recentEvents = computed(() => snapshot.value?.recentEvents || []);
const rawJsonText = computed(() => JSON.stringify(snapshot.value?.rawJson || {}, null, 2));

const displayedBars = computed(() => {
  const dimensions = snapshot.value?.dimensions;
  const order = snapshot.value?.rules?.displayOrder || bars.map((bar) => bar.key);
  return order
    .map((key) => bars.find((bar) => bar.key === key))
    .filter((bar): bar is BarDef => !!bar)
    .map((bar) => ({ ...bar, value: dimensions?.[bar.key] ?? 0 }));
});

const dotColor = computed(() => {
  const dimensions = snapshot.value?.dimensions;
  if (!dimensions) return "#a3a3a3";
  if (dimensions.tension >= 45) return "#ef4444";
  if (dimensions.affection >= 70) return "#ec4899";
  if (dimensions.trust >= 70) return "#10b981";
  return "#a3a3a3";
});

const lastEventText = computed(() => {
  const event = snapshot.value?.lastEvent;
  if (!event) return "";
  return event.reason || event.eventType;
});

function tauriArgs() {
  const args: { conversationId: string; agentId?: string } = { conversationId: props.conversationId };
  const agentId = String(props.agentId || "").trim();
  if (agentId) args.agentId = agentId;
  return args;
}

async function refresh() {
  try {
    if (!props.conversationId) return;
    snapshot.value = await invokeTauri<RelationshipPanelSnapshot>("get_relationship_panel_snapshot", tauriArgs());
  } catch (err) {
    console.warn("[RelationshipPanel] 刷新失败:", err);
  }
}

async function reset() {
  try {
    if (!props.conversationId) return;
    snapshot.value = await invokeTauri<RelationshipPanelSnapshot>("reset_relationship_state", tauriArgs());
  } catch (err) {
    console.warn("[RelationshipPanel] 重置失败:", err);
  }
}

async function simulate(eventType: string) {
  try {
    if (!props.conversationId) return;
    snapshot.value = await invokeTauri<RelationshipPanelSnapshot>("simulate_relationship_event", {
      ...tauriArgs(),
      eventType,
      intensity: 1,
    });
  } catch (err) {
    console.warn("[RelationshipPanel] 模拟事件失败:", err);
  }
}

async function refreshRules() {
  try {
    await invokeTauri<RelationshipRules>("refresh_relationship_rules", {});
    await refresh();
  } catch (err) {
    console.warn("[RelationshipPanel] 重载规则失败:", err);
  }
}

function formatDelta(delta: Partial<RelationshipDimensions> | undefined): string {
  if (!delta) return "0";
  const parts = Object.entries(delta)
    .filter(([, value]) => Number(value || 0) !== 0)
    .map(([key, value]) => `${key} ${Number(value) > 0 ? "+" : ""}${value}`);
  return parts.length ? parts.join(", ") : "0";
}

function toggleCollapse() {
  collapsed.value = !collapsed.value;
}

onMounted(() => {
  refresh();
  pollTimer = setInterval(refresh, 3000);
});

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
});

watch(() => [props.conversationId, props.agentId], () => {
  snapshot.value = null;
  refresh();
});
</script>
