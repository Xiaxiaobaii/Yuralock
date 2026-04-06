<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

type CryptoResult = {
  output_path: string;
  message: string;
};

type ToastPayload = {
  message: string;
  type?: "success" | "error";
};

type ProgressPayload = {
  percent: number;
};

const TOAST_EVENT = "frontend://show-toast";
const PROGRESS_EVENT = "frontend://crypto-progress";
const MIN_PART = 0;
const MAX_PART = 100;

const inputPath = ref("");
const key = ref("");
const encryptPart = ref(20);
const progressPercent = ref(0);

const running = ref(false);
const isEncryptedFile = ref<boolean>(true);

const toastVisible = ref(false);
const toastText = ref("");
const toastType = ref<"success" | "error">("success");

const splashExiting = ref(false);
let splashExitTimer: number | null = null;
let toastTimer: number | null = null;
let unlistenToastEvent: UnlistenFn | null = null;
let unlistenProgressEvent: UnlistenFn | null = null;

function decodeUriSafely(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function normalizeDisplayPath(path: string): string {
  let decoded = decodeUriSafely(path);
  const rawPrefix = "raw:";
  if (decoded.includes(rawPrefix)) {
    decoded = decoded.split(rawPrefix).pop() ?? decoded;
  }
  return decoded;
}

function clampPercent(percent: number): number {
  return Math.max(0, Math.min(100, Math.trunc(percent)));
}

const fileName = computed(() => {
  const path = inputPath.value.trim();
  if (!path) {
    return "选择文件";
  }
  const parts = path.split(/[\\/]/);
  const result = normalizeDisplayPath(parts[parts.length - 1]);
  return result || path;
});

const displayPath = computed(() => normalizeDisplayPath(inputPath.value));

const submitText = computed(() => {
  if (running.value) return "处理中...";
  return `开始${isEncryptedFile.value ? "解密" : "加密"}`;
});

const encryptPartText = computed(() => `${encryptPart.value}%`);

const canSubmit = computed(
  () =>
    !running.value &&
    inputPath.value.trim().length > 0 &&
    key.value.length > 0,
);

onMounted(() => {
  splashExitTimer = window.setTimeout(() => {
    splashExiting.value = true;
  }, 914);

  void listen<ToastPayload>(TOAST_EVENT, (event) => {
    if (!event.payload?.message) return;
    showToast(
      event.payload.message,
      event.payload.type === "error" ? "error" : "success",
    );
  })
    .then((unlisten) => {
      unlistenToastEvent = unlisten;
    })
    .catch((error) => {
      console.error("toast 事件监听失败:", error);
    });

  void listen<ProgressPayload>(PROGRESS_EVENT, (event) => {
    const percent = Number(event.payload?.percent ?? 0);
    if (!Number.isFinite(percent)) return;
    progressPercent.value = clampPercent(percent);
  })
    .then((unlisten) => {
      unlistenProgressEvent = unlisten;
    })
    .catch((error) => {
      console.error("进度事件监听失败:", error);
    });
});

onBeforeUnmount(() => {
  if (splashExitTimer !== null) {
    window.clearTimeout(splashExitTimer);
    splashExitTimer = null;
  }
  if (toastTimer !== null) {
    window.clearTimeout(toastTimer);
    toastTimer = null;
  }
  if (unlistenToastEvent) {
    unlistenToastEvent();
    unlistenToastEvent = null;
  }
  if (unlistenProgressEvent) {
    unlistenProgressEvent();
    unlistenProgressEvent = null;
  }
});

function showToast(message: string, type: "success" | "error" = "success") {
  toastText.value = decodeUriSafely(message);
  toastType.value = type;
  toastVisible.value = true;

  if (toastTimer !== null) {
    window.clearTimeout(toastTimer);
  }

  toastTimer = window.setTimeout(() => {
    toastVisible.value = false;
    toastTimer = null;
  }, 2200);
}

async function selectInputPath() {
  const selected = await invoke<string>("pick_input_file");
  if (selected === "") return;
  inputPath.value = selected;
  isEncryptedFile.value = await invoke<boolean>("peek_file_from_path", {
    inputPath: selected,
  });
}

async function run() {
  if (!canSubmit.value) {
    return;
  }

  progressPercent.value = 0;
  running.value = true;

  try {
    let result = await invoke<CryptoResult>("process_file_from_path", {
      inputPath: inputPath.value,
      isencry: isEncryptedFile.value,
      key: key.value,
      encryptPart: isEncryptedFile.value ? null : encryptPart.value,
    });
    result.output_path = decodeUriSafely(result.output_path);
    showToast(`${result.message}\n${result.output_path}`, "success");
  } catch (error) {
    showToast(`操作失败: ${String(error)}`, "error");
  } finally {
    running.value = false;
  }
}
</script>

<template>
  <div class="page">
    <section class="splash">
      <img class="splash-icon" src="/icon.png" alt="YuraLock icon" />
    </section>

    <main class="app" :class="{ 'is-ready': splashExiting }">
      <div class="panel">
        <h1 class="title">
          <img class="title-image" src="/title2.png" alt="YuraLock" />
        </h1>

        <form class="form" @submit.prevent="run">
          <button
            type="button"
            class="file-btn"
            :disabled="running"
            @click="selectInputPath"
          >
            {{ fileName }}
          </button>
          <p v-if="inputPath" class="path-preview">{{ displayPath }}</p>

          <div class="key-row">
            <!-- type固定为text,不要修改为password -->
            <input
              v-model="key"
              type="text"
              placeholder="输入密钥"
            />
          </div>

          <div v-if="!isEncryptedFile" class="part-row">
            <div class="part-row-head">
              <span>加密比例</span>
              <span>{{ encryptPartText }}</span>
            </div>
            <input
              v-model.number="encryptPart"
              type="range"
              :min="MIN_PART"
              :max="MAX_PART"
              step="1"
            />
          </div>

          <button v-if="!running" type="submit" class="run-btn" :disabled="!canSubmit">
            {{ submitText }}
          </button>
          <div v-else class="run-progress">
            <div class="run-progress-head">
              <span>{{ isEncryptedFile ? "解密中" : "加密中" }}</span>
            </div>
            <div
              class="run-progress-track"
              :style="{ '--progress': `${progressPercent}%` }"
            >
              <div class="run-progress-fill"></div>
              <span class="run-progress-label run-progress-label--base">
                {{ progressPercent }}%
              </span>
              <span class="run-progress-label run-progress-label--fill">
                {{ progressPercent }}%
              </span>
            </div>
          </div>
        </form>
      </div>
    </main>

    <transition name="toast-fade">
      <div v-if="toastVisible" class="toast" :class="{ error: toastType === 'error' }">
        {{ toastText }}
      </div>
    </transition>
  </div>
</template>
