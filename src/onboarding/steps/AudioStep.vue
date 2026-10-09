<script setup lang="ts">
import { computed } from "vue";
import {
  isPairedAudioRoute,
  type CaptureInputSnapshot,
  type AudioRouteSnapshot,
  type AudioEndpoint,
  type AudioSnapshot,
} from "../../lib/bridge";
import OptionCard from "../../components/onboarding/OptionCard.vue";

const props = defineProps<{
  endpoints: AudioEndpoint[];
  audio: AudioSnapshot | null;
  capture: CaptureInputSnapshot | null;
  route: AudioRouteSnapshot | null;
  scanning: boolean;
  message: string;
  selectingEndpointId: string;
  openingVbCablePage: boolean;
  /** 当前门禁失败码（只用于显示修复卡；判据仍由 flow.evaluateGate 独裁）。 */
  gateCode: string | null;
}>();

defineEmits<{
  select: [endpoint: AudioEndpoint];
  refresh: [];
  openDownload: [];
}>();

const recommended = computed(() => props.endpoints.filter(endpoint => endpoint.isVirtualCableCandidate));
const selectedId = computed(() => props.capture?.settings.endpointId ?? null);
const ready = computed(() => props.audio?.phase === "ready" && isPairedAudioRoute(props.route, props.capture, props.audio));

const FIX_TITLES: Record<string, string> = {
  "audio.no_output_device": "装了 VB-CABLE，但还没有检测到设备",
  "audio.not_ready": "语音设备还没有准备好",
};

const fixTitle = computed(() =>
  props.gateCode ? (FIX_TITLES[props.gateCode] ?? null) : null,
);
</script>

<template>
  <section class="onboarding-step">
    <h1>选择语音设备</h1>
    <p class="onboarding-lede">
      选择语音工具要使用的目标麦克风（CABLE Output）；无线麦会自动连接对应的声音写入端。
    </p>

    <div v-if="recommended.length === 0" class="onboarding-card warn">
      <h4>这台电脑还没有 VB-CABLE 语音设备</h4>
      <p>先安装，再继续：</p>
      <ul class="onboarding-numlist">
        <li>
          <span class="onboarding-num">1</span>
          <span>下载压缩包并完整解压，右键安装程序选择「以管理员身份运行」。</span>
        </li>
        <li>
          <span class="onboarding-num">2</span>
          <span>安装完成后重启电脑，再回来点「重新检测」。</span>
        </li>
      </ul>
      <div class="onboarding-chips">
        <button
          class="onboarding-chip strong"
          type="button"
          :disabled="openingVbCablePage"
          @click="$emit('openDownload')"
        >
          {{ openingVbCablePage ? "正在打开…" : "打开官方下载页" }}
        </button>
        <button class="onboarding-chip" type="button" :disabled="scanning" @click="$emit('refresh')">
          {{ scanning ? "正在检测…" : "重新检测" }}
        </button>
      </div>
    </div>

    <template v-else>
      <p class="onboarding-muted">选择目标麦克风：</p>
      <OptionCard
        v-for="endpoint in recommended"
        :key="endpoint.id"
        :selected="selectedId === endpoint.id"
        :disabled="selectingEndpointId === endpoint.id || (selectedId === endpoint.id && ready)"
        @select="$emit('select', endpoint)"
      >
        <strong>{{ endpoint.name }}</strong>
        <small v-if="selectedId === endpoint.id && ready">声音通道已配对。请在语音工具中选择同一个麦克风。</small>
        <template #trailing>
          {{
            selectedId === endpoint.id && ready
              ? "已就绪"
              : selectingEndpointId === endpoint.id
                ? "正在打开…"
                : "选择"
          }}
        </template>
      </OptionCard>
      <div class="onboarding-chips">
        <button class="onboarding-chip" type="button" :disabled="scanning" @click="$emit('refresh')">
          {{ scanning ? "正在检测…" : "重新检测" }}
        </button>
      </div>
    </template>

    <div v-if="fixTitle" class="onboarding-card warn">
      <h4>{{ fixTitle }}</h4>
      <div class="onboarding-chips">
        <button
          class="onboarding-chip strong"
          type="button"
          :disabled="scanning"
          @click="$emit('refresh')"
        >
          重新检测设备
        </button>
      </div>
    </div>

    <p v-if="message" class="onboarding-muted">{{ message }}</p>
  </section>
</template>
