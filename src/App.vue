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

const inputPath = ref("");
const key = ref("");

const running = ref(false);
const isEncryptedFile = ref<boolean>(false);

const toastVisible = ref(false);
const toastText = ref("");
const toastType = ref<"success" | "error">("success");

const splashExiting = ref(false);
let splashExitTimer: number | null = null;
let toastTimer: number | null = null;
let unlistenToastEvent: UnlistenFn | null = null;

const fileName = computed(() => {
  const path = inputPath.value.trim();
  if (!path) {
    return "选择文件";
  }
  let parts = path.split(/[\\/]/);
  // parts = parts[parts.length - 1].split(/[_]/);
  // let result = parts.slice(1, parts.length).join("");
  let result = parts[parts.length - 1]
  result = ComplateAndroidPath(result)

  return result || path;
});

const submitText = computed(() => {
  if (running.value) return "处理中...";
  return `开始${isEncryptedFile.value ? "解密" : "加密"}`;
});

const canSubmit = computed(
  () =>
    !running.value &&
    inputPath.value.trim().length > 0 &&
    key.value.length > 0 &&
    isEncryptedFile.value !== null,
);

function ComplateAndroidPath(str: string): string {
  let destr = decodeURIComponent(str);
  if (destr.split("raw:").length > 1) {
    destr = destr.split("raw:")[1];
  }
  
  return destr
}

onMounted(() => {
  splashExitTimer = window.setTimeout(() => {
    splashExiting.value = true;
  }, 914);

  void listen<ToastPayload>("frontend://show-toast", (event) => {
    if (!event.payload?.message) {
      return;
    }
    showToast(
      event.payload.message,
      event.payload.type === "error" ? "error" : "success",
    );
  }).then((unlisten) => {
    unlistenToastEvent = unlisten;
  }).catch((error) => {
    console.error("toast 事件监听失败:", error);
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
});

function showToast(message: string, type: "success" | "error" = "success") {
  toastText.value = decodeURIComponent(message);
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
  if (selected == "") return;

  // 原始文件名
  inputPath.value = selected;
  isEncryptedFile.value = await invoke<boolean>("peek_file_from_path", {
    inputPath: selected,
  });
}

async function run() {
  if (!canSubmit.value) {
    return;
  }

  running.value = true;

  try {
    let result = await invoke<CryptoResult>("process_file_from_path", {
      inputPath: inputPath.value,
      isencry: isEncryptedFile.value,
      key: key.value,
    });
    result.output_path = decodeURIComponent(result.output_path);
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
          <p v-if="inputPath" class="path-preview">{{ ComplateAndroidPath(inputPath) }}</p>

          <div class="key-row">
            <!-- type固定为text,不要修改为password -->
            <input
              v-model="key"

              type="text"
              placeholder="输入密钥"
            />
          </div>

          <button type="submit" class="run-btn" :disabled="!canSubmit">
            {{ submitText }}
          </button>
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
