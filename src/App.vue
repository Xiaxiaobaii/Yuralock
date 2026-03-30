<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

type Mode = "encrypt" | "decrypt";

type CryptoResult = {
  output_path: string;
  message: string;
};

const mode = ref<Mode>("encrypt");
const inputPath = ref("");
const outputDir = ref("");
const key = ref("");
const part = ref(20);
const running = ref(false);
const status = ref("");
const logs = ref<string[]>([]);
const showKey = ref(false);

const showSplash = ref(true);
const splashExiting = ref(false);
const showMain = ref(false);
const splashTimers: number[] = [];
let unlistenLog: UnlistenFn | null = null;

const isEncrypt = computed(() => mode.value === "encrypt");
const outputText = computed(() => {
  const lines: string[] = [];
  if (logs.value.length > 0) {
    lines.push(`日志:\n${logs.value.join("\n")}`);
  }
  if (status.value) {
    lines.push(status.value);
  }
  return lines.join("\n\n");
});

watch(mode, () => {
  status.value = "";
  logs.value = [];
});

onMounted(() => {
  splashTimers.push(
    window.setTimeout(() => {
      splashExiting.value = true;
    }, 1120),
  );

  splashTimers.push(
    window.setTimeout(() => {
      showSplash.value = false;
      showMain.value = true;
    }, 1750),
  );

  void listen<string>("crypto-log", (event) => {
    logs.value.push(event.payload);
  }).then((unlisten) => {
    unlistenLog = unlisten;
  });
});

onBeforeUnmount(() => {
  splashTimers.forEach((id) => window.clearTimeout(id));
  if (unlistenLog) {
    unlistenLog();
    unlistenLog = null;
  }
});

async function selectInputPath() {
  const selected = await invoke<string | null>("pick_input_file");
  if (selected) {
    inputPath.value = selected;
  }
}

async function selectOutputDir() {
  const selected = await invoke<string | null>("pick_output_dir");
  if (selected) {
    outputDir.value = selected;
  }
}

function toggleShowKey() {
  showKey.value = !showKey.value;
}

async function run() {
  if (running.value) {
    return;
  }

  running.value = true;
  status.value = "";
  logs.value = [];

  try {
    let result: CryptoResult;

    if (isEncrypt.value) {
      result = await invoke<CryptoResult>("encrypt_part_file_from_path", {
        inputPath: inputPath.value,
        outputDir: outputDir.value,
        key: key.value,
        part: Number(part.value),
      });
    } else {
      result = await invoke<CryptoResult>("decrypt_part_file_from_path", {
        inputPath: inputPath.value,
        outputDir: outputDir.value,
        key: key.value,
      });
    }

    status.value = `${result.message}\n${result.output_path}`;
  } catch (error) {
    status.value = `失败: ${String(error)}`;
  } finally {
    running.value = false;
  }
}
</script>

<template>
  <div class="page">
    <section v-if="showSplash" class="splash" :class="{ 'is-exiting': splashExiting }">
      <div class="splash-spin">
        <img class="splash-icon" src="/icon.png" alt="YuraLock icon" />
      </div>
    </section>

    <main v-if="showMain" class="app app-enter">
      <h1 class="title">
        <img class="title-icon" src="/icon.png" alt="YuraLock icon" />
        YuraLock
      </h1>

      <form class="form" @submit.prevent="run">
        <label>
          模式
          <select v-model="mode">
            <option value="encrypt">加密</option>
            <option value="decrypt">解密</option>
          </select>
        </label>

        <label>
          源文件路径
          <div class="path-row">
            <input v-model.trim="inputPath" placeholder="/path/to/file" />
            <button
              type="button"
              class="picker"
              :disabled="running"
              @click="selectInputPath"
            >
              选择
            </button>
          </div>
        </label>

        <label>
          输出目录（留空为当前目录）
          <div class="path-row">
            <input v-model.trim="outputDir" placeholder="/path/to/output" />
            <button
              type="button"
              class="picker"
              :disabled="running"
              @click="selectOutputDir"
            >
              选择
            </button>
          </div>
        </label>

        <label>
          密钥
          <div class="path-row">
            <input
              v-model="key"
              :type="showKey ? 'text' : 'password'"
              placeholder="请输入密钥"
            />
            <button
              type="button"
              class="picker"
              :disabled="running"
              @click="toggleShowKey"
            >
              {{ showKey ? "隐藏" : "显示" }}
            </button>
          </div>
        </label>

        <label v-if="isEncrypt">
          加密比例（0-100）
          <input v-model.number="part" type="number" min="0" max="100" />
        </label>

        <button type="submit" :disabled="running">
          {{ running ? "处理中..." : "执行" }}
        </button>
      </form>

      <pre v-if="outputText" class="status">{{ outputText }}</pre>
    </main>
  </div>
</template>

<style scoped>
.page {
  width: 100%;
  min-height: 100vh;
  min-height: 100dvh;
  overflow-x: hidden;
}

.splash {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: grid;
  place-items: center;
  background: #fff;
  opacity: 1;
  transition: opacity 360ms ease;
}

.splash.is-exiting {
  opacity: 0;
}

.splash-icon {
  width: 100%;
  height: 100%;
  object-fit: contain;
  will-change: transform, opacity;
  animation: splash-pop 1.02s cubic-bezier(0.08, 0.95, 0.28, 1) forwards;
}

.splash-spin {
  width: clamp(300px, 52vmin, 560px);
  height: clamp(300px, 52vmin, 560px);
  transform-origin: center;
  will-change: transform;
  animation: splash-spin 1.02s linear forwards;
}

@keyframes splash-spin {
  0% {
    transform: rotate(0deg);
  }

  100% {
    transform: rotate(360deg);
  }
}

@keyframes splash-pop {
  0% {
    transform: scale(0.12);
    opacity: 0;
  }

  20% {
    opacity: 1;
  }

  70% {
    transform: scale(1.52);
    opacity: 1;
  }

  100% {
    transform: scale(1.52);
    opacity: 1;
  }
}

.app {
  box-sizing: border-box;
  width: 100%;
  min-height: 100vh;
  min-height: 100dvh;
  margin: 0;
  padding: 24px;
  background: #fff;
}

.app-enter {
  animation: app-fade-in 280ms ease-out;
}

@keyframes app-fade-in {
  0% {
    opacity: 0;
    transform: translateY(4px);
  }

  100% {
    opacity: 1;
    transform: translateY(0);
  }
}

.title {
  margin: 0 0 16px;
  font-size: 20px;
  display: flex;
  align-items: center;
  gap: 10px;
}

.title-icon {
  width: 24px;
  height: 24px;
  object-fit: contain;
}

.form {
  display: grid;
  gap: 12px;
}

.path-row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
}

label {
  display: grid;
  gap: 6px;
  font-size: 14px;
}

input,
select,
button {
  box-sizing: border-box;
  width: 100%;
  padding: 8px 10px;
  border: 1px solid #bbb;
  border-radius: 6px;
  font-size: 14px;
}

button {
  cursor: pointer;
  background: #111;
  color: #fff;
  border-color: #111;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.picker {
  width: auto;
  min-width: 64px;
  background: #fff;
  color: #111;
  border-color: #bbb;
}

.status {
  margin: 16px 0 0;
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 6px;
  white-space: pre-wrap;
  word-break: break-all;
  background: #fafafa;
}

@media (orientation: portrait) and (max-width: 900px) {
  .app {
    padding: 16px 14px calc(16px + env(safe-area-inset-bottom, 0px));
  }

  .title {
    margin-bottom: 12px;
    font-size: 18px;
    gap: 8px;
  }

  .title-icon {
    width: 20px;
    height: 20px;
  }

  .form {
    gap: 10px;
  }

  label {
    gap: 5px;
    font-size: 13px;
  }

  input,
  select,
  button {
    padding: 10px 12px;
    font-size: 16px;
  }

  .path-row {
    grid-template-columns: 1fr;
    gap: 6px;
  }

  .picker {
    width: 100%;
    min-width: 0;
  }

  .status {
    margin-top: 12px;
    max-height: 34dvh;
    overflow: auto;
  }

  .splash-spin {
    width: clamp(220px, 62vw, 420px);
    height: clamp(220px, 62vw, 420px);
  }
}

@media (max-width: 420px) and (orientation: portrait) {
  .app {
    padding: 12px 10px calc(12px + env(safe-area-inset-bottom, 0px));
  }
}
</style>

<style>
:root {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  color: #111;
  background: #fff;
}

body {
  margin: 0;
}

#app {
  padding: 0;
}
</style>
